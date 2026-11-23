use std::time::Duration;

use oryn_core::update::{LATEST_URL, Release, parse_latest};

pub fn latest() -> Option<Release> {
    let body = fetch(LATEST_URL)?;
    parse_latest(&body, env!("CARGO_PKG_VERSION"))
}

fn fetch(url: &str) -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("Oryn/", env!("CARGO_PKG_VERSION")))
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::fetch;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once(
        status: &'static str,
        body: &'static str,
    ) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/latest", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    break;
                }
                headers.push(line);
            }
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            headers
        });
        (url, handle)
    }

    #[test]
    fn fetch_returns_the_body_and_sends_the_github_headers() {
        let (url, server) = serve_once("200 OK", "{\"tag_name\":\"v9.9.9\"}");
        assert_eq!(fetch(&url).as_deref(), Some("{\"tag_name\":\"v9.9.9\"}"));
        let headers: Vec<String> = server
            .join()
            .unwrap()
            .into_iter()
            .map(|h| h.to_ascii_lowercase())
            .collect();
        assert!(
            headers
                .iter()
                .any(|h| h == "accept: application/vnd.github+json")
        );
        assert!(headers.iter().any(|h| h.starts_with("user-agent: oryn/")));
    }

    #[test]
    fn fetch_gives_nothing_for_an_error_status() {
        let (url, server) = serve_once("404 Not Found", "missing");
        assert_eq!(fetch(&url), None);
        server.join().unwrap();
    }
}
