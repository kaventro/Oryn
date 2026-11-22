use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
    Password,
    Key,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: AuthMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_unknown_host: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_fingerprint: Option<String>,
}

impl Default for RemoteProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "New Connection".into(),
            host: "localhost".into(),
            port: 22,
            username: "root".into(),
            auth_type: AuthMethod::Password,
            password: None,
            key_path: None,
            passphrase: None,
            initial_path: Some("/".into()),
            accept_unknown_host: None,
            expected_fingerprint: None,
        }
    }
}

pub fn get_profiles_file_path() -> Result<PathBuf> {
    let mut config_dir = dirs::config_dir().context("Failed to find system config directory")?;
    config_dir.push("Oryn");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context("Failed to create Oryn config directory")?;
    }
    config_dir.push("remote_profiles.json");
    Ok(config_dir)
}

pub fn load_profiles() -> Result<Vec<RemoteProfile>> {
    let path = get_profiles_file_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read profiles from {}", path.display()))?;
    let profiles: Vec<RemoteProfile> = serde_json::from_str(&content).unwrap_or_default();
    Ok(profiles)
}

impl RemoteProfile {
    pub fn sanitized(&self) -> Self {
        let mut copy = self.clone();
        if copy.password.is_some() {
            copy.password = Some("••••••••".into());
        }
        if copy.passphrase.is_some() {
            copy.passphrase = Some("••••••••".into());
        }
        copy
    }
}

pub fn save_profiles(profiles: &[RemoteProfile]) -> Result<()> {
    let path = get_profiles_file_path()?;
    let json = serde_json::to_string_pretty(profiles)?;
    fs::write(&path, json)
        .with_context(|| format!("Failed to write profiles to {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn save_profile(mut profile: RemoteProfile) -> Result<Vec<RemoteProfile>> {
    let mut profiles = load_profiles().unwrap_or_default();
    if let Some(pos) = profiles.iter().position(|p| p.id == profile.id) {
        if profile.password.as_deref() == Some("••••••••") {
            profile.password = profiles[pos].password.clone();
        }
        if profile.passphrase.as_deref() == Some("••••••••") {
            profile.passphrase = profiles[pos].passphrase.clone();
        }
        match profile.expected_fingerprint.as_deref() {
            None => profile.expected_fingerprint = profiles[pos].expected_fingerprint.clone(),
            Some("") => profile.expected_fingerprint = None,
            Some(_) => {}
        }
        profiles[pos] = profile;
    } else {
        profiles.push(profile);
    }
    save_profiles(&profiles)?;
    Ok(profiles)
}

pub fn delete_profile(id: &str) -> Result<Vec<RemoteProfile>> {
    let mut profiles = load_profiles().unwrap_or_default();
    profiles.retain(|p| p.id != id);
    save_profiles(&profiles)?;
    Ok(profiles)
}

pub fn pin_fingerprint(id: &str, fingerprint: &str) -> Result<()> {
    if id.is_empty() {
        return Ok(());
    }
    let mut profiles = load_profiles().unwrap_or_default();
    let Some(prof) = profiles.iter_mut().find(|p| p.id == id) else {
        return Ok(());
    };
    if prof.expected_fingerprint.is_some() {
        return Ok(());
    }
    prof.expected_fingerprint = Some(format!("SHA256:{}", fingerprint));
    save_profiles(&profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_serialization() {
        let p = RemoteProfile {
            id: "srv1".into(),
            name: "Server 1".into(),
            host: "192.168.1.100".into(),
            port: 22,
            username: "admin".into(),
            auth_type: AuthMethod::Password,
            password: Some("secret".into()),
            key_path: None,
            passphrase: None,
            initial_path: Some("/home/admin".into()),
            accept_unknown_host: Some(true),
            expected_fingerprint: Some("SHA256:abc123xyz".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let parsed: RemoteProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "srv1");
        assert_eq!(parsed.accept_unknown_host, Some(true));
        assert_eq!(parsed.host, "192.168.1.100");
    }
}
