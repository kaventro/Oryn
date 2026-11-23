use serde::Deserialize;

pub const LATEST_URL: &str = "https://api.github.com/repos/kaventro/Oryn/releases/latest";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

#[derive(Deserialize)]
struct Raw {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<Asset>,
}

pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

pub fn parse_latest(json: &str, current: &str) -> Option<Release> {
    let raw: Raw = serde_json::from_str(json).ok()?;
    if raw.draft || raw.prerelease || !is_newer(&raw.tag_name, current) {
        return None;
    }
    Some(Release {
        version: raw.tag_name.trim_start_matches(['v', 'V']).to_string(),
        url: raw.html_url,
        assets: raw.assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse() {
        assert_eq!(parse_version("v0.0.8"), Some((0, 0, 8)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("0.1.0-spike"), Some((0, 1, 0)));
        assert_eq!(parse_version("x"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }

    #[test]
    fn newer_compares_numerically() {
        assert!(is_newer("v0.0.10", "0.0.9"));
        assert!(!is_newer("v0.0.8", "0.0.8"));
        assert!(!is_newer("v0.0.7", "0.0.8"));
        assert!(!is_newer("junk", "0.0.8"));
    }

    #[test]
    fn latest_skips_drafts_and_old() {
        let new = r#"{"tag_name":"v0.0.9","html_url":"https://example.com/r","draft":false,"prerelease":false}"#;
        assert_eq!(
            parse_latest(new, "0.0.8"),
            Some(Release {
                version: "0.0.9".into(),
                url: "https://example.com/r".into(),
                assets: Vec::new(),
            })
        );
        assert_eq!(parse_latest(new, "0.0.9"), None);
        let pre = r#"{"tag_name":"v0.1.0","html_url":"u","prerelease":true}"#;
        assert_eq!(parse_latest(pre, "0.0.8"), None);
        assert_eq!(parse_latest("not json", "0.0.8"), None);
    }

    #[test]
    fn latest_keeps_the_downloads() {
        let json = r#"{"tag_name":"v0.0.11","html_url":"u","assets":[
            {"name":"Oryn_0.0.11_aarch64.app.zip","browser_download_url":"https://d/a.zip","size":3},
            {"name":"SHA256SUMS-macos.txt","browser_download_url":"https://d/s.txt"}]}"#;
        let release = parse_latest(json, "0.0.10").unwrap();
        assert_eq!(release.assets.len(), 2);
        assert_eq!(
            release
                .asset("SHA256SUMS-macos.txt")
                .map(|a| a.url.as_str()),
            Some("https://d/s.txt")
        );
        assert_eq!(release.asset("missing"), None);
    }
}
