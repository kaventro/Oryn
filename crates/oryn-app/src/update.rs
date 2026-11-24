use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use oryn_core::update::{Fetch, LATEST_URL, Plan, Platform, Release, Updater, parse_latest, plan};

const USER_AGENT: &str = concat!("Oryn/", env!("CARGO_PKG_VERSION"));
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

pub trait UpdateSource: Send + Sync {
    fn latest(&self) -> Option<Release>;
    fn plan(&self, release: &Release) -> Plan;
    fn install(&self, release: &Release) -> Result<PathBuf>;
    fn open_page(&self, url: &str);
}

pub struct GitHubUpdates {
    latest_url: String,
    current: String,
    exe: Option<PathBuf>,
    platform: Platform,
    work: PathBuf,
}

impl Default for GitHubUpdates {
    fn default() -> Self {
        Self {
            latest_url: LATEST_URL.into(),
            current: env!("CARGO_PKG_VERSION").into(),
            exe: std::env::current_exe().ok(),
            platform: Platform::current(),
            work: std::env::temp_dir().join("oryn-update"),
        }
    }
}

impl UpdateSource for GitHubUpdates {
    fn latest(&self) -> Option<Release> {
        parse_latest(&fetch(&self.latest_url)?, &self.current)
    }

    fn plan(&self, release: &Release) -> Plan {
        match &self.exe {
            Some(exe) => plan(self.platform, exe, &release.version),
            None => Plan::Manual("Oryn can't find its own program file."),
        }
    }

    fn install(&self, release: &Release) -> Result<PathBuf> {
        let Plan::Automatic(target) = self.plan(release) else {
            anyhow::bail!("Oryn can't update itself here.");
        };
        Updater::new(GitHub::new(), self.work.clone()).install(release, &target)
    }

    fn open_page(&self, url: &str) {
        let _ = open::that_detached(url);
    }
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
    use super::{Fetch, GitHub, GitHubUpdates, UpdateSource, fetch};
    use oryn_core::update::{Arch, Asset, Os, Plan, Platform, Release};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
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

    fn serve_routes(routes: Vec<(&'static str, Vec<u8>)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            for _ in 0..routes.len() {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim_end().is_empty() {
                        break;
                    }
                }
                let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
                let body = routes
                    .iter()
                    .find(|(p, _)| *p == path)
                    .map(|(_, b)| b.clone());
                let (status, body) = match body {
                    Some(b) => ("200 OK", b),
                    None => ("404 Not Found", Vec::new()),
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        base
    }

    fn source(latest_url: String, exe: Option<PathBuf>, work: PathBuf) -> GitHubUpdates {
        GitHubUpdates {
            latest_url,
            current: "1.0.0".into(),
            exe,
            platform: Platform {
                os: Os::Windows,
                arch: Arch::X86_64,
            },
            work,
        }
    }

    fn release(base: &str) -> Release {
        let asset = |name: &str, path: &str| Asset {
            name: name.into(),
            url: format!("{base}{path}"),
        };
        Release {
            version: "2.0.0".into(),
            url: "page".into(),
            assets: vec![
                asset("Oryn_2.0.0_x64_portable.exe", "/exe"),
                asset("SHA256SUMS-windows.txt", "/sums"),
            ],
        }
    }

    #[test]
    fn latest_reads_the_newest_release() {
        let base = serve_routes(vec![(
            "/latest",
            br#"{"tag_name":"v2.0.0","html_url":"page","assets":[]}"#.to_vec(),
        )]);
        let dir = tempfile::tempdir().unwrap();
        let found = source(format!("{base}/latest"), None, dir.path().into()).latest();
        assert_eq!(found.map(|r| r.version), Some("2.0.0".into()));
    }

    #[test]
    fn plan_needs_the_program_file() {
        let dir = tempfile::tempdir().unwrap();
        let release = release("http://unused");
        let missing = source(String::new(), None, dir.path().into());
        assert!(matches!(missing.plan(&release), Plan::Manual(_)));
        let found = source(
            String::new(),
            Some(dir.path().join("Oryn.exe")),
            dir.path().into(),
        );
        assert!(matches!(found.plan(&release), Plan::Automatic(_)));
        assert!(missing.install(&release).is_err());
    }

    #[test]
    fn install_downloads_checks_and_replaces_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let body = b"new oryn".to_vec();
        std::fs::write(dir.path().join("probe"), &body).unwrap();
        let hash = oryn_core::update::sha256_file(&dir.path().join("probe")).unwrap();
        let sums = format!("{hash}  Oryn_2.0.0_x64_portable.exe\n").into_bytes();
        let base = serve_routes(vec![("/sums", sums), ("/exe", body.clone())]);
        let exe = dir.path().join("Oryn.exe");
        std::fs::write(&exe, b"old").unwrap();
        let updates = source(String::new(), Some(exe.clone()), dir.path().join("work"));
        assert_eq!(updates.install(&release(&base)).unwrap(), exe);
        assert_eq!(std::fs::read(&exe).unwrap(), body);
    }

    #[test]
    fn the_default_source_points_at_github() {
        let updates = GitHubUpdates::default();
        assert_eq!(updates.latest_url, oryn_core::update::LATEST_URL);
        assert_eq!(updates.current, env!("CARGO_PKG_VERSION"));
    }
}
