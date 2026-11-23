use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use oryn_core::update::{Fetch, LATEST_URL, Plan, Platform, Release, Updater, parse_latest, plan};

const USER_AGENT: &str = concat!("Oryn/", env!("CARGO_PKG_VERSION"));
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

pub fn latest() -> Option<Release> {
    let body = fetch(LATEST_URL)?;
    parse_latest(&body, env!("CARGO_PKG_VERSION"))
}

pub fn plan_for(release: &Release) -> Plan {
    match std::env::current_exe() {
        Ok(exe) => plan(Platform::current(), &exe, &release.version),
        Err(_) => Plan::Manual("Oryn can't find its own program file."),
    }
}

pub fn install(release: &Release) -> Result<PathBuf> {
    let Plan::Automatic(target) = plan_for(release) else {
        anyhow::bail!("Oryn can't update itself here.");
    };
    let work = std::env::temp_dir().join(format!("oryn-update-{}", release.version));
    Updater::new(GitHub::new(), work).install(release, &target)
}

fn fetch(url: &str) -> Option<String> {
    GitHub::new().api(url).ok()
}

struct GitHub {
    api: ureq::Agent,
    downloads: ureq::Agent,
}

impl GitHub {
    fn new() -> Self {
        let api = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .into();
        let downloads = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_per_read(Some(Duration::from_secs(30)))
            .build()
            .into();
        Self { api, downloads }
    }

    fn api(&self, url: &str) -> Result<String> {
        Ok(self
            .api
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", USER_AGENT)
            .call()?
            .body_mut()
            .read_to_string()?)
    }
}

impl Fetch for GitHub {
    fn text(&self, url: &str) -> Result<String> {
        Ok(self
            .downloads
            .get(url)
            .header("User-Agent", USER_AGENT)
            .call()?
            .body_mut()
            .read_to_string()?)
    }

    fn save(&self, url: &str, dest: &Path) -> Result<()> {
        let mut response = self
            .downloads
            .get(url)
            .header("Accept", "application/octet-stream")
            .header("User-Agent", USER_AGENT)
            .call()?;
        let mut reader = response
            .body_mut()
            .with_config()
            .limit(MAX_DOWNLOAD)
            .reader();
        let mut file =
            File::create(dest).with_context(|| format!("can't write {}", dest.display()))?;
        std::io::copy(&mut reader, &mut file)?;
        file.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Fetch, GitHub, fetch};
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

    #[test]
    fn save_streams_a_download_to_disk() {
        let (url, server) = serve_once("200 OK", "binary");
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("asset");
        GitHub::new().save(&url, &dest).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"binary");
        let headers = server.join().unwrap();
        assert!(
            headers
                .iter()
                .any(|h| h.eq_ignore_ascii_case("accept: application/octet-stream"))
        );
    }

    #[test]
    fn text_fails_for_an_error_status() {
        let (url, server) = serve_once("500 Internal Server Error", "x");
        assert!(GitHub::new().text(&url).is_err());
        server.join().unwrap();
    }
}
