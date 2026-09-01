use std::fs;
use std::path::Path;
use std::process::Command;

use crate::services::ServiceResult;

pub fn compress_zip(path: &str) -> ServiceResult<String> {
    let src = fs::canonicalize(path)?;
    let parent = src.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?;
    let base = src
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("no basename"))?;

    let mut candidate = parent.join(format!("{}.zip", base.to_string_lossy()));
    let mut suffix = 0u32;
    while candidate.exists() {
        suffix += 1;
        candidate = parent.join(format!("{} ({}).zip", base.to_string_lossy(), suffix));
    }

    run_zip_command(&src, parent, base, &candidate)?;
    Ok(candidate.to_string_lossy().to_string())
}

#[cfg(target_os = "macos")]
fn run_zip_command(
    src: &Path,
    _parent: &Path,
    _base: &std::ffi::OsStr,
    candidate: &Path,
) -> ServiceResult<()> {
    let status = Command::new("ditto")
        .args(["-c", "-k", "--sequesterRsrc"])
        .arg(src)
        .arg(candidate)
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("ditto failed to create archive"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_zip_command(
    src: &Path,
    _parent: &Path,
    _base: &std::ffi::OsStr,
    candidate: &Path,
) -> ServiceResult<()> {
    let ps = format!(
        "Compress-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        src.to_string_lossy().replace('\'', "''"),
        candidate.to_string_lossy().replace('\'', "''")
    );
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Compress-Archive failed"));
    }
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn run_zip_command(
    _src: &Path,
    parent: &Path,
    base: &std::ffi::OsStr,
    candidate: &Path,
) -> ServiceResult<()> {
    let base_str = base.to_string_lossy();
    // Prefix "./" so a file/dir named like "-x" or "--out=…" cannot be parsed
    // as a zip option (argument injection). `current_dir(parent)` makes the
    // relative "./name" resolve correctly.
    let status = Command::new("zip")
        .args(["-r", "-q"])
        .arg(candidate)
        .arg(format!("./{}", base_str))
        .current_dir(parent)
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("zip failed"));
    }
    Ok(())
}
