use std::time::Duration;

use oryn_core::update::{LATEST_URL, Release, parse_latest};

pub fn latest() -> Option<Release> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let body = agent
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("Oryn/", env!("CARGO_PKG_VERSION")))
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    parse_latest(&body, env!("CARGO_PKG_VERSION"))
}
