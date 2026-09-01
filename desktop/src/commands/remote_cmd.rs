use std::path::Path;
use tauri::State;

use crate::commands::response::{ack, Ack};
use crate::services::fs_vfs::DirListing;
use crate::services::remote::{ops, profile, RemoteProfile, RemoteSession, SessionPool};

fn resolve_profile_credentials(mut profile: RemoteProfile) -> RemoteProfile {
    let stored = profile::load_profiles()
        .ok()
        .and_then(|saved| saved.into_iter().find(|p| p.id == profile.id));

    if let Some(existing) = stored {
        if profile.password.as_deref() == Some("••••••••") {
            profile.password = existing.password.clone();
        }
        if profile.passphrase.as_deref() == Some("••••••••") {
            profile.passphrase = existing.passphrase.clone();
        }
        match profile.expected_fingerprint.as_deref() {
            None => profile.expected_fingerprint = existing.expected_fingerprint.clone(),
            Some("") => profile.expected_fingerprint = None,
            Some(_) => {}
        }
    } else if profile.expected_fingerprint.as_deref() == Some("") {
        profile.expected_fingerprint = None;
    }
    profile
}

#[tauri::command]
pub fn remote_list_profiles() -> Result<Vec<RemoteProfile>, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    Ok(profiles.into_iter().map(|p| p.sanitized()).collect())
}

#[tauri::command]
pub fn remote_save_profile(profile: RemoteProfile) -> Result<Vec<RemoteProfile>, String> {
    let profiles = profile::save_profile(profile).map_err(|e| e.to_string())?;
    Ok(profiles.into_iter().map(|p| p.sanitized()).collect())
}

#[tauri::command]
pub fn remote_delete_profile(
    pool: State<'_, SessionPool>,
    id: String,
) -> Result<Vec<RemoteProfile>, String> {
    pool.disconnect(&id);
    let profiles = profile::delete_profile(&id).map_err(|e| e.to_string())?;
    Ok(profiles.into_iter().map(|p| p.sanitized()).collect())
}

#[tauri::command]
pub fn remote_test_connection(profile: RemoteProfile) -> Result<Ack, String> {
    let resolved = resolve_profile_credentials(profile);
    RemoteSession::connect(&resolved).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_connect(
    pool: State<'_, SessionPool>,
    profile: RemoteProfile,
) -> Result<DirListing, String> {
    let resolved = resolve_profile_credentials(profile);
    let session = pool.get_or_connect(&resolved).map_err(|e| e.to_string())?;
    let guard = session.lock();
    let initial_path = resolved.initial_path.as_deref().unwrap_or("/");
    ops::list_dir(&guard, initial_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_disconnect(pool: State<'_, SessionPool>, profile_id: String) -> Result<Ack, String> {
    pool.disconnect(&profile_id);
    Ok(ack())
}

#[tauri::command]
pub fn remote_read_dir(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
) -> Result<DirListing, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::list_dir(&guard, &path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_read_file_text(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
) -> Result<String, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::read_file_text(&guard, &path, 5 * 1024 * 1024).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remote_write_file_text(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
    content: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::write_file_text(&guard, &path, &content).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_mkdir(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::mkdir(&guard, &path).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_create_file(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::create_file(&guard, &path).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_rename(
    pool: State<'_, SessionPool>,
    profile_id: String,
    src_path: String,
    dst_path: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::rename(&guard, &src_path, &dst_path).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_delete(
    pool: State<'_, SessionPool>,
    profile_id: String,
    path: String,
    is_dir: bool,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    if is_dir {
        ops::delete_dir_recursive(&guard, &path).map_err(|e| e.to_string())?;
    } else {
        ops::delete_file(&guard, &path).map_err(|e| e.to_string())?;
    }
    Ok(ack())
}

#[tauri::command]
pub fn remote_download(
    pool: State<'_, SessionPool>,
    profile_id: String,
    remote_path: String,
    local_dst: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::download_file(&guard, &remote_path, Path::new(&local_dst)).map_err(|e| e.to_string())?;
    Ok(ack())
}

#[tauri::command]
pub fn remote_upload(
    pool: State<'_, SessionPool>,
    profile_id: String,
    local_src: String,
    remote_dst: String,
) -> Result<Ack, String> {
    let profiles = profile::load_profiles().map_err(|e| e.to_string())?;
    let prof = profiles
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| format!("Profile not found: {}", profile_id))?;

    let session = pool.get_or_connect(&prof).map_err(|e| e.to_string())?;
    let guard = session.lock();
    ops::upload_file(&guard, Path::new(&local_src), &remote_dst).map_err(|e| e.to_string())?;
    Ok(ack())
}
