use serde::Deserialize;
use std::process::Command;

use crate::commands::response::{ack, Ack};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellPath {
    pub path: String,
}

#[tauri::command]
pub fn shell_open_path(input: ShellPath) -> Result<Ack, String> {
    let path = input.path.trim();
    let first_err = match opener::open(path) {
        Ok(()) => return Ok(ack()),
        Err(e) => e.to_string(),
    };
    // LaunchServices may refuse files it has no default handler for; fall back
    // to the default text editor for regular files.
    #[cfg(target_os = "macos")]
    {
        let is_file = std::fs::metadata(path)
            .map(|m| m.is_file())
            .unwrap_or(false);
        if is_file {
            let r = Command::new("/usr/bin/open")
                .args(["-t", "--", path])
                .output();
            if let Ok(out) = r {
                if out.status.success() {
                    return Ok(ack());
                }
            }
        }
    }
    Err(format!("{first_err} (path: {path:?})"))
}

#[tauri::command]
pub fn shell_show_in_folder(input: ShellPath) -> Result<Ack, String> {
    let path = &input.path;
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args([&format!("/select,{}", path.replace('/', "\\"))])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(parent) = std::path::Path::new(path).parent() {
            opener::open(parent).map_err(|e| e.to_string())?;
        }
    }
    Ok(ack())
}

#[tauri::command]
pub fn shell_open_vscode(input: ShellPath) -> Result<Ack, String> {
    #[cfg(target_os = "windows")]
    {
        if Command::new("code.cmd").arg(&input.path).spawn().is_ok() {
            return Ok(ack());
        }
        if Command::new("code").arg(&input.path).spawn().is_ok() {
            return Ok(ack());
        }
    }
    let r = Command::new("code").arg(&input.path).spawn();
    if r.is_ok() {
        return Ok(ack());
    }
    #[cfg(target_os = "macos")]
    {
        let r2 = Command::new("open")
            .args(["-a", "Visual Studio Code", &input.path])
            .spawn();
        if r2.is_ok() {
            return Ok(ack());
        }
    }
    Err("VS Code (code) not found in PATH".to_string())
}

#[tauri::command]
pub fn shell_open_terminal(input: ShellPath) -> Result<Ack, String> {
    let path = input.path.trim();
    #[cfg(target_os = "macos")]
    {
        if Command::new("open")
            .args(["-a", "Terminal", path])
            .spawn()
            .is_ok()
        {
            return Ok(ack());
        }
        return Ok(ack());
    }
    #[cfg(target_os = "windows")]
    {
        // 1. Try Windows Terminal
        if Command::new("wt").args(["-d", path]).spawn().is_ok() {
            return Ok(ack());
        }
        // 2. Try PowerShell using -LiteralPath
        let ps_path = path.replace('\'', "''");
        if Command::new("powershell")
            .args([
                "-NoExit",
                "-Command",
                &format!("Set-Location -LiteralPath '{}'", ps_path),
            ])
            .spawn()
            .is_ok()
        {
            return Ok(ack());
        }
        // 3. Try CMD with current_dir
        if Command::new("cmd").current_dir(path).spawn().is_ok() {
            return Ok(ack());
        }
        return Ok(ack());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let terminals = [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "xterm",
        ];
        for term in terminals {
            if Command::new(term).current_dir(path).spawn().is_ok() {
                return Ok(ack());
            }
        }
        return Ok(ack());
    }
    #[allow(unreachable_code)]
    Err("Could not launch external terminal".to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecIn {
    pub cmd: String,
    pub cwd: Option<String>,
}

#[tauri::command]
pub async fn shell_exec(input: ShellExecIn) -> Result<serde_json::Value, String> {
    // Unix: use user's default shell (zsh/bash) with login flag and enriched PATH.
    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let mut c = tokio::process::Command::new(&shell);
        c.arg("-l").arg("-c").arg(&input.cmd);
        c.process_group(0);

        let home = std::env::var("HOME").unwrap_or_default();
        let existing_path = std::env::var("PATH").unwrap_or_default();
        let extra_paths = format!(
            "{}/.cargo/bin:{}/.nvm/current/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
            home, home
        );
        let combined_path = if existing_path.is_empty() {
            extra_paths
        } else {
            format!("{}:{}", extra_paths, existing_path)
        };
        c.env("PATH", combined_path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = tokio::process::Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-Command", &input.cmd]);
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
        c
    };
    if let Some(ref cwd) = input.cwd {
        cmd.current_dir(cwd);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.kill_on_drop(true);

    #[cfg(target_os = "windows")]
    let job = {
        use win32job::{ExtendedLimitInfo, Job};
        let mut info = ExtendedLimitInfo::new();
        info.limit_kill_on_job_close();
        Job::create_with_limit_info(&info).map_err(|e| e.to_string())?
    };

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    if let Some(handle) = child.raw_handle() {
        let _ = job.assign_process(handle as isize);
    }

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    const MAX_SHELL_OUTPUT_BYTES: usize = 1_048_576; // 1 MB streaming limit

    let read_stdout = async {
        if let Some(mut stream) = stdout_pipe {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let mut limited = (&mut stream).take(MAX_SHELL_OUTPUT_BYTES as u64);
            let _ = limited.read_to_end(&mut buf).await;
            buf
        } else {
            Vec::new()
        }
    };

    let read_stderr = async {
        if let Some(mut stream) = stderr_pipe {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let mut limited = (&mut stream).take(MAX_SHELL_OUTPUT_BYTES as u64);
            let _ = limited.read_to_end(&mut buf).await;
            buf
        } else {
            Vec::new()
        }
    };

    let wait_process = async {
        let (stdout_bytes, stderr_bytes, status) =
            tokio::join!(read_stdout, read_stderr, child.wait());
        let status = status.map_err(|e| e.to_string())?;
        Ok::<_, String>((stdout_bytes, stderr_bytes, status.code().unwrap_or(-1)))
    };

    let (stdout_bytes, stderr_bytes, code) =
        match tokio::time::timeout(std::time::Duration::from_secs(60), wait_process).await {
            Ok(res) => res?,
            Err(_) => {
                #[cfg(unix)]
                if let Some(pid) = child.id() {
                    unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
                }
                let _ = child.kill().await;
                return Err(
                    "Command execution timed out after 60 seconds and was terminated".to_string(),
                );
            }
        };

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
    Ok(serde_json::json!({
        "ok": code == 0,
        "stdout": stdout,
        "stderr": stderr,
        "code": code
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardIn {
    pub text: String,
}

#[tauri::command]
pub fn clipboard_write(input: ClipboardIn) -> Result<(), String> {
    arboard::Clipboard::new()
        .map_err(|e| e.to_string())?
        .set_text(input.text)
        .map_err(|e| e.to_string())
}
