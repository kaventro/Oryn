use std::time::Duration;

use oryn_core::update::{LATEST_URL, Release, parse_latest};

pub fn latest() -> Option<Release> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let body = agent
        .get(LATEST_URL)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", concat!("Oryn/", env!("CARGO_PKG_VERSION")))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    parse_latest(&body, env!("CARGO_PKG_VERSION"))
}
