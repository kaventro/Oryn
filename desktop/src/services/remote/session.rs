use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use ssh2::{Session, Sftp};
use std::collections::HashMap;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use super::profile::{AuthMethod, RemoteProfile};

pub struct RemoteSession {
    #[allow(dead_code)]
    pub profile: RemoteProfile,
    session: Session,
    sftp: Sftp,
}

impl RemoteSession {
    pub fn connect(profile: &RemoteProfile) -> Result<Self> {
        let addr = format!("{}:{}", profile.host, profile.port);
        let tcp = TcpStream::connect_timeout(
            &addr.parse().or_else(|_| {
                use std::net::ToSocketAddrs;
                addr.to_socket_addrs()?
                    .next()
                    .ok_or_else(|| anyhow!("Failed to resolve address {}", addr))
            })?,
            Duration::from_secs(10),
        )
        .with_context(|| format!("Failed to connect to {}", addr))?;

        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;

        // Host key verification
        verify_ssh_host_key(&mut session, profile)?;

        match profile.auth_type {
            AuthMethod::Password => {
                let password = profile
                    .password
                    .as_deref()
                    .ok_or_else(|| anyhow!("Password is required for password authentication"))?;
                session
                    .userauth_password(&profile.username, password)
                    .context("SSH password authentication failed")?;
            }
            AuthMethod::Key => {
                let key_path_str = profile
                    .key_path
                    .as_deref()
                    .ok_or_else(|| anyhow!("Private key path is required"))?;
                let expanded_key_path = if let Some(stripped) = key_path_str.strip_prefix("~/") {
                    if let Some(home) = dirs::home_dir() {
                        home.join(stripped)
                    } else {
                        Path::new(key_path_str).to_path_buf()
                    }
                } else {
                    Path::new(key_path_str).to_path_buf()
                };

                if !expanded_key_path.exists() {
                    return Err(anyhow!(
                        "Private key file not found: {}",
                        expanded_key_path.display()
                    ));
                }

                session
                    .userauth_pubkey_file(
                        &profile.username,
                        None,
                        &expanded_key_path,
                        profile.passphrase.as_deref(),
                    )
                    .with_context(|| {
                        format!(
                            "SSH public key authentication failed for key: {}",
                            expanded_key_path.display()
                        )
                    })?;
            }
            AuthMethod::Agent => {
                let mut agent = session.agent().context("Failed to connect to SSH agent")?;
                agent.connect().context("SSH agent connection failed")?;
                agent
                    .list_identities()
                    .context("Failed to list SSH agent identities")?;
                let mut authed = false;
                for identity in agent.identities()? {
                    if agent.userauth(&profile.username, &identity).is_ok() {
                        authed = true;
                        break;
                    }
                }
                if !authed {
                    return Err(anyhow!(
                        "SSH agent authentication failed for user {}",
                        profile.username
                    ));
                }
            }
        }

        if !session.authenticated() {
            return Err(anyhow!("SSH authentication was not completed successfully"));
        }

        let sftp = session
            .sftp()
            .context("Failed to initialize SFTP subsystem")?;

        Ok(Self {
            profile: profile.clone(),
            session,
            sftp,
        })
    }

    pub fn sftp(&self) -> &Sftp {
        &self.sftp
    }
}

#[derive(Default, Clone)]
pub struct SessionPool {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<RemoteSession>>>>>,
}

impl SessionPool {
    pub fn get_or_connect(&self, profile: &RemoteProfile) -> Result<Arc<Mutex<RemoteSession>>> {
        let mut pool = self.sessions.lock();
        if let Some(existing) = pool.get(&profile.id) {
            // Test if still connected
            let session_guard = existing.lock();
            if session_guard.session.authenticated() {
                drop(session_guard);
                return Ok(Arc::clone(existing));
            }
        }

        let session = RemoteSession::connect(profile)?;
        let arc_session = Arc::new(Mutex::new(session));
        pool.insert(profile.id.clone(), Arc::clone(&arc_session));
        Ok(arc_session)
    }

    pub fn disconnect(&self, profile_id: &str) {
        let mut pool = self.sessions.lock();
        pool.remove(profile_id);
    }
}

pub fn compute_ssh_key_fingerprint(key: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(hasher.finalize())
}

pub fn verify_ssh_host_key(session: &mut Session, profile: &RemoteProfile) -> Result<()> {
    let (key, key_type) = session
        .host_key()
        .ok_or_else(|| anyhow!("Failed to retrieve host key from remote server"))?;

    let fingerprint = compute_ssh_key_fingerprint(key);

    let mut known_hosts = session
        .known_hosts()
        .context("Failed to init known_hosts manager")?;

    let known_hosts_path = dirs::home_dir().map(|h| h.join(".ssh").join("known_hosts"));
    if let Some(ref kh_path) = known_hosts_path {
        if kh_path.exists() {
            known_hosts
                .read_file(kh_path, ssh2::KnownHostFileKind::OpenSSH)
                .with_context(|| format!("Failed to read known_hosts file at {:?}", kh_path))?;
        }
    }

    let check_res = known_hosts.check_port(&profile.host, profile.port, key);
    match check_res {
        ssh2::CheckResult::Match => {
            if let Some(ref expected) = profile.expected_fingerprint {
                let clean_expected = expected.trim_start_matches("SHA256:").trim();
                if clean_expected != fingerprint {
                    return Err(anyhow!(
                        "Host key fingerprint mismatch for {}:{}. Expected SHA256:{}, got SHA256:{}",
                        profile.host,
                        profile.port,
                        clean_expected,
                        fingerprint
                    ));
                }
            }
            Ok(())
        }
        ssh2::CheckResult::NotFound => {
            match evaluate_unknown_host(
                profile.expected_fingerprint.as_deref(),
                profile.accept_unknown_host == Some(true),
                &fingerprint,
            ) {
                HostTrust::PinnedMismatch => {
                    return Err(anyhow!(
                        "SECURITY WARNING: SSH host key for {}:{} does not match the pinned fingerprint (expected {}, got SHA256:{}). If you intentionally changed the server key, clear or update the pinned fingerprint in the connection settings.",
                        profile.host,
                        profile.port,
                        profile.expected_fingerprint.as_deref().unwrap_or(""),
                        fingerprint
                    ));
                }
                HostTrust::Untrusted => {
                    return Err(anyhow!(
                        "Unknown SSH host key for {}:{} (Fingerprint: SHA256:{}). To connect, verify this host fingerprint and enable 'Accept Unknown Host' or set the expected fingerprint in your connection settings.",
                        profile.host,
                        profile.port,
                        fingerprint
                    ));
                }
                HostTrust::Trusted => {}
            }

            if let Some(ref kh_path) = known_hosts_path {
                if let Some(parent) = kh_path.parent() {
                    std::fs::create_dir_all(parent)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(
                            parent,
                            std::fs::Permissions::from_mode(0o700),
                        );
                    }
                }

                let fmt = match key_type {
                    ssh2::HostKeyType::Rsa => ssh2::KnownHostKeyFormat::SshRsa,
                    ssh2::HostKeyType::Dss => ssh2::KnownHostKeyFormat::SshDss,
                    ssh2::HostKeyType::Ecdsa256 => ssh2::KnownHostKeyFormat::Ecdsa256,
                    ssh2::HostKeyType::Ecdsa384 => ssh2::KnownHostKeyFormat::Ecdsa384,
                    ssh2::HostKeyType::Ecdsa521 => ssh2::KnownHostKeyFormat::Ecdsa521,
                    ssh2::HostKeyType::Ed25519 => ssh2::KnownHostKeyFormat::Ed25519,
                    _ => ssh2::KnownHostKeyFormat::Unknown,
                };

                known_hosts
                    .add(&profile.host, key, &format!("{}:{}", profile.host, profile.port), fmt)
                    .context("Failed to add host key to known_hosts")?;

                known_hosts
                    .write_file(kh_path, ssh2::KnownHostFileKind::OpenSSH)
                    .with_context(|| {
                        format!("Failed to write known_hosts file at {:?}", kh_path)
                    })?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        kh_path,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
            }

            if profile.expected_fingerprint.is_none() {
                super::profile::pin_fingerprint(&profile.id, &fingerprint).context(
                    "Trusted a new SSH host key but could not persist its fingerprint; refusing to continue so TOFU cannot silently re-accept an unknown key on the next connection",
                )?;
            }
            Ok(())
        }
        ssh2::CheckResult::Mismatch => {
            Err(anyhow!(
                "SECURITY WARNING: Remote host identification has changed for {}:{}! Possible Man-in-the-Middle attack. Remote fingerprint: SHA256:{}",
                profile.host,
                profile.port,
                fingerprint
            ))
        }
        ssh2::CheckResult::Failure => {
            Err(anyhow!(
                "SSH host key verification failed or host key file was invalid for {}:{}",
                profile.host,
                profile.port
            ))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum HostTrust {
    Trusted,
    PinnedMismatch,
    Untrusted,
}

fn evaluate_unknown_host(
    expected_fingerprint: Option<&str>,
    accept_unknown: bool,
    presented: &str,
) -> HostTrust {
    match expected_fingerprint.map(|s| s.trim_start_matches("SHA256:").trim() == presented) {
        Some(true) => HostTrust::Trusted,
        Some(false) => HostTrust::PinnedMismatch,
        None if accept_unknown => HostTrust::Trusted,
        None => HostTrust::Untrusted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_host_trust_decisions() {
        assert_eq!(evaluate_unknown_host(None, true, "abc"), HostTrust::Trusted);
        assert_eq!(
            evaluate_unknown_host(None, false, "abc"),
            HostTrust::Untrusted
        );
        assert_eq!(
            evaluate_unknown_host(Some("abc"), false, "abc"),
            HostTrust::Trusted
        );
        assert_eq!(
            evaluate_unknown_host(Some("SHA256:abc"), false, "abc"),
            HostTrust::Trusted
        );
        assert_eq!(
            evaluate_unknown_host(Some("abc"), true, "def"),
            HostTrust::PinnedMismatch
        );
    }

    #[test]
    fn test_remote_session_pool_lifecycle() {
        let pool = SessionPool::default();
        pool.disconnect("nonexistent");
    }

    #[test]
    fn test_compute_ssh_key_fingerprint() {
        let dummy_key = b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKey";
        let fp = compute_ssh_key_fingerprint(dummy_key);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 43);
    }
}
