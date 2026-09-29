use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::Emitter;

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenEditorIn {
    pub path: String,
    pub editor: Option<String>,
    pub custom_cmd: Option<String>,
}

fn try_launch_editor(path: &str, editor: &str, custom_cmd: Option<&str>) -> Result<(), String> {
    let ed = editor.trim().to_lowercase();
    match ed.as_str() {
        "cursor" => {
            #[cfg(target_os = "macos")]
            {
                if Command::new("open")
                    .args(["-a", "Cursor", path])
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            #[cfg(target_os = "windows")]
            {
                if Command::new("cursor.cmd").arg(path).spawn().is_ok()
                    || Command::new("cursor").arg(path).spawn().is_ok()
                {
                    return Ok(());
                }
            }
            if Command::new("cursor").arg(path).spawn().is_ok() {
                return Ok(());
            }
            Err(
                "Cursor not found. Ensure 'cursor' is installed or Cursor.app is in Applications."
                    .into(),
            )
        }
        "sublime" => {
            #[cfg(target_os = "macos")]
            {
                if Command::new("open")
                    .args(["-a", "Sublime Text", path])
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            #[cfg(target_os = "windows")]
            {
                if Command::new("subl.exe").arg(path).spawn().is_ok()
                    || Command::new("sublime_text.exe").arg(path).spawn().is_ok()
                {
                    return Ok(());
                }
            }
            if Command::new("subl").arg(path).spawn().is_ok() {
                return Ok(());
            }
            Err("Sublime Text not found. Ensure 'subl' is installed or Sublime Text.app is in Applications.".into())
        }
        "zed" => {
            #[cfg(target_os = "macos")]
            {
                if Command::new("open")
                    .args(["-a", "Zed", path])
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            if Command::new("zed").arg(path).spawn().is_ok() {
                return Ok(());
            }
            Err("Zed not found. Ensure 'zed' is installed or Zed.app is in Applications.".into())
        }
        "custom" => {
            let cmd_str = custom_cmd.unwrap_or("").trim();
            if cmd_str.is_empty() {
                return Err("No custom editor command specified in Preferences.".into());
            }
            #[cfg(target_os = "windows")]
            {
                let full = if cmd_str.contains("{path}") {
                    cmd_str.replace("{path}", path)
                } else {
                    format!("{} \"{}\"", cmd_str, path)
                };
                Command::new("cmd")
                    .args(["/C", &full])
                    .spawn()
                    .map_err(|e| format!("Failed to launch custom command '{cmd_str}': {e}"))?;
                Ok(())
            }
            #[cfg(not(target_os = "windows"))]
            {
                let full = if cmd_str.contains("{path}") {
                    cmd_str.replace("{path}", path)
                } else {
                    format!("{} \"{}\"", cmd_str, path)
                };
                Command::new("/bin/sh")
                    .args(["-c", &full])
                    .spawn()
                    .map_err(|e| format!("Failed to launch custom command '{cmd_str}': {e}"))?;
                Ok(())
            }
        }
        _ => {
            // "vscode" or default
            #[cfg(target_os = "macos")]
            {
                if Command::new("open")
                    .args(["-a", "Visual Studio Code", path])
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            #[cfg(target_os = "windows")]
            {
                if Command::new("code.cmd").arg(path).spawn().is_ok()
                    || Command::new("code").arg(path).spawn().is_ok()
                {
                    return Ok(());
                }
            }
            if Command::new("code").arg(path).spawn().is_ok() {
                return Ok(());
            }
            Err("VS Code not found. Ensure 'code' is installed or Visual Studio Code.app is in Applications.".into())
        }
    }
}

#[tauri::command]
pub fn shell_open_editor(input: OpenEditorIn) -> Result<Ack, String> {
    let ed = input.editor.as_deref().unwrap_or("vscode");
    try_launch_editor(&input.path, ed, input.custom_cmd.as_deref())?;
    Ok(ack())
}

#[tauri::command]
pub fn shell_open_vscode(input: ShellPath) -> Result<Ack, String> {
    shell_open_editor(OpenEditorIn {
        path: input.path,
        editor: Some("vscode".to_string()),
        custom_cmd: None,
    })
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

pub(crate) struct PtySession {
    pub(crate) writer: Mutex<Box<dyn Write + Send>>,
    pub(crate) master: Mutex<Box<dyn MasterPty + Send>>,
    pub(crate) killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
    }
}

#[derive(Default, Clone)]
pub struct TerminalSessions(pub(crate) Arc<Mutex<HashMap<String, Arc<PtySession>>>>);

impl Drop for TerminalSessions {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.clear();
        }
    }
}

impl TerminalSessions {
    pub fn start(
        &self,
        cwd: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<(String, Box<dyn Read + Send>, Box<dyn Child + Send + Sync>), String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: rows.clamp(2, 300),
                cols: cols.clamp(2, 500),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "windows"))]
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        #[cfg(target_os = "windows")]
        let shell = "powershell.exe".to_string();
        let mut command = CommandBuilder::new(shell);
        #[cfg(not(target_os = "windows"))]
        command.arg("-l");
        #[cfg(target_os = "windows")]
        command.arg("-NoLogo");
        if let Some(cwd) = cwd.filter(|p| !p.trim().is_empty()) {
            command.cwd(cwd);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        let reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        drop(pair.slave);
        let id = format!(
            "{}-{}",
            std::process::id(),
            NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed)
        );
        let killer = child.clone_killer();
        let session = Arc::new(PtySession {
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            killer: Mutex::new(killer),
        });
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.clone(), session);
        Ok((id, reader, child))
    }

    pub fn write(&self, session_id: &str, data: &str) -> Result<(), String> {
        let session = self
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .get(session_id)
            .cloned()
            .ok_or_else(|| "Terminal session not found".to_string())?;

        let mut writer = session.writer.lock().map_err(|e| e.to_string())?;
        writer
            .write_all(data.as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|e| e.to_string())
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let session = self
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .get(session_id)
            .cloned()
            .ok_or_else(|| "Terminal session not found".to_string())?;

        let result = session
            .master
            .lock()
            .map_err(|e| e.to_string())?
            .resize(PtySize {
                rows: rows.clamp(2, 300),
                cols: cols.clamp(2, 500),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string());
        result
    }

    pub fn stop(&self, session_id: &str) -> Result<(), String> {
        let session = self
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .remove(session_id);
        if let Some(session) = session {
            if let Ok(mut killer) = session.killer.lock() {
                let _ = killer.kill();
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStartIn {
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionIn {
    pub session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWriteIn {
    pub session_id: String,
    pub data: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizeIn {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn decode_utf8_chunk(carry: &mut Vec<u8>, chunk: &[u8]) -> String {
    carry.extend_from_slice(chunk);
    let mut out = String::new();
    let mut offset = 0;
    while offset < carry.len() {
        match std::str::from_utf8(&carry[offset..]) {
            Ok(valid_str) => {
                out.push_str(valid_str);
                offset = carry.len();
                break;
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to > 0 {
                    if let Ok(valid_str) =
                        std::str::from_utf8(&carry[offset..offset + valid_up_to])
                    {
                        out.push_str(valid_str);
                    }
                    offset += valid_up_to;
                }
                match e.error_len() {
                    Some(invalid_len) => {
                        out.push('\u{FFFD}');
                        offset += invalid_len;
                    }
                    None => break,
                }
            }
        }
    }
    carry.drain(..offset);
    out
}

#[tauri::command(async)]
pub fn shell_terminal_start(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, TerminalSessions>,
    input: TerminalStartIn,
) -> Result<String, String> {
    let (id, mut reader, mut child) =
        sessions.start(input.cwd.as_deref(), input.cols, input.rows)?;
    let output_id = id.clone();
    let sessions_map = Arc::clone(&sessions.0);
    let exit_done = Arc::new(AtomicBool::new(false));

    let emit_exit = {
        let exit_done = Arc::clone(&exit_done);
        let app = app.clone();
        let sessions_map = Arc::clone(&sessions_map);
        let output_id = output_id.clone();
        Arc::new(move || {
            if !exit_done.swap(true, Ordering::SeqCst) {
                let _ = sessions_map.lock().ok().and_then(|mut s| s.remove(&output_id));
                let _ = app.emit("terminal-exit", serde_json::json!({"sessionId": output_id}));
            }
        })
    };

    // Monitor child process independently of reader.read()
    let child_exit = Arc::clone(&emit_exit);
    std::thread::spawn(move || {
        let _ = child.wait();
        // Give reader thread a brief window to flush remaining buffered output
        std::thread::sleep(std::time::Duration::from_millis(50));
        child_exit();
    });

    // Reader thread
    let reader_exit = Arc::clone(&emit_exit);
    let reader_app = app.clone();
    let reader_id = output_id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut carry: Vec<u8> = Vec::new();
        loop {
            if exit_done.load(Ordering::Relaxed) {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = decode_utf8_chunk(&mut carry, &buf[..n]);
                    if !data.is_empty()
                        && reader_app
                            .emit(
                                "terminal-output",
                                serde_json::json!({"sessionId": reader_id, "data": data}),
                            )
                            .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if !carry.is_empty() {
            let data = String::from_utf8_lossy(&carry).into_owned();
            let _ = reader_app.emit(
                "terminal-output",
                serde_json::json!({"sessionId": reader_id, "data": data}),
            );
        }
        reader_exit();
    });
    Ok(id)
}

#[tauri::command]
pub async fn shell_terminal_write(
    sessions: tauri::State<'_, TerminalSessions>,
    input: TerminalWriteIn,
) -> Result<(), String> {
    let sessions = sessions.inner().clone();
    tokio::task::spawn_blocking(move || sessions.write(&input.session_id, &input.data))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command(async)]
pub fn shell_terminal_resize(
    sessions: tauri::State<'_, TerminalSessions>,
    input: TerminalResizeIn,
) -> Result<(), String> {
    sessions.resize(&input.session_id, input.cols, input.rows)
}

#[tauri::command(async)]
pub fn shell_terminal_stop(
    sessions: tauri::State<'_, TerminalSessions>,
    input: TerminalSessionIn,
) -> Result<(), String> {
    sessions.stop(&input.session_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_exec() {
        let res = shell_exec(ShellExecIn {
            cmd: "echo test_output_123".into(),
            cwd: None,
        })
        .await
        .unwrap();

        assert_eq!(res["ok"], true);
        assert!(res["stdout"].as_str().unwrap().contains("test_output_123"));
        assert_eq!(res["code"], 0);
    }

    #[test]
    fn test_shell_open_editor_custom_cmd() {
        let res = shell_open_editor(OpenEditorIn {
            path: "/tmp/oryn_test_file.txt".into(),
            editor: Some("custom".into()),
            custom_cmd: Some("true".into()),
        });
        assert!(res.is_ok());
    }

    #[test]
    fn test_shell_open_editor_custom_cmd_empty_fails() {
        let res = shell_open_editor(OpenEditorIn {
            path: "/tmp/oryn_test_file.txt".into(),
            editor: Some("custom".into()),
            custom_cmd: Some("   ".into()),
        });
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("No custom editor command"));
    }

    #[test]
    fn test_decode_utf8_chunk_split_sequence() {
        let mut carry = Vec::new();
        let full = "Привет".as_bytes();
        let part1 = &full[..3];
        let part2 = &full[3..];

        let out1 = decode_utf8_chunk(&mut carry, part1);
        assert_eq!(out1, "П");
        assert_eq!(carry.len(), 1);

        let out2 = decode_utf8_chunk(&mut carry, part2);
        assert_eq!(out2, "ривет");
        assert!(carry.is_empty());
    }

    #[test]
    fn test_decode_utf8_chunk_multibyte_emoji() {
        let mut carry = Vec::new();
        let emoji = "🚀".as_bytes();
        let out1 = decode_utf8_chunk(&mut carry, &emoji[..2]);
        assert_eq!(out1, "");
        assert_eq!(carry.len(), 2);

        let out2 = decode_utf8_chunk(&mut carry, &emoji[2..]);
        assert_eq!(out2, "🚀");
        assert!(carry.is_empty());
    }

    #[test]
    fn test_decode_utf8_chunk_invalid_bytes() {
        let mut carry = Vec::new();
        let invalid = [0xFF, 0xFE];
        let out = decode_utf8_chunk(&mut carry, &invalid);
        assert!(out.contains('\u{FFFD}'));
        assert!(carry.is_empty());
    }

    #[test]
    fn test_decode_utf8_chunk_invalid_byte_before_split_char() {
        let mut carry = Vec::new();
        // 0xFF is an invalid byte.
        // 0xF0, 0x9F are the first two bytes of 4-byte emoji 😀 (0xF0, 0x9F, 0x98, 0x80).
        let part1 = decode_utf8_chunk(&mut carry, &[0xFF, 0xF0, 0x9F]);
        assert_eq!(part1, "\u{FFFD}");
        assert_eq!(carry, vec![0xF0, 0x9F]);

        let part2 = decode_utf8_chunk(&mut carry, &[0x98, 0x80]);
        assert_eq!(part2, "😀");
        assert!(carry.is_empty());
    }

    #[test]
    fn test_terminal_sessions_lifecycle() {
        let sessions = TerminalSessions::default();
        drop(sessions);
    }

    #[test]
    fn test_terminal_sessions_full_lifecycle() {
        let sessions = TerminalSessions::default();
        let (id, mut reader, _child) = sessions.start(None, 80, 24).expect("start pty");
        assert!(!id.is_empty());

        assert!(sessions.write(&id, "echo hello\r\n").is_ok());
        assert!(sessions.resize(&id, 100, 30).is_ok());

        let mut buf = [0u8; 1024];
        let _ = reader.read(&mut buf);

        assert!(sessions.stop(&id).is_ok());
        assert!(sessions.write(&id, "fail").is_err());
        assert!(sessions.resize(&id, 80, 24).is_err());
    }

    #[test]
    fn test_terminal_sessions_with_cwd() {
        let sessions = TerminalSessions::default();
        let tmp = std::env::temp_dir();
        let (id, _, _child) = sessions.start(tmp.to_str(), 80, 24).expect("start pty with cwd");
        assert!(sessions.stop(&id).is_ok());
    }

    #[test]
    fn test_terminal_sessions_not_found_errors() {
        let sessions = TerminalSessions::default();
        assert_eq!(
            sessions.write("non_existent", "abc").unwrap_err(),
            "Terminal session not found"
        );
        assert_eq!(
            sessions.resize("non_existent", 80, 24).unwrap_err(),
            "Terminal session not found"
        );
        assert!(sessions.stop("non_existent").is_ok());
    }

    #[test]
    fn test_terminal_sessions_drop_kills_active() {
        let sessions = TerminalSessions::default();
        let (_id, _, _child) = sessions.start(None, 80, 24).expect("start pty");
        drop(sessions);
    }

    #[test]
    fn test_terminal_sessions_independent_child_wait() {
        let sessions = TerminalSessions::default();
        let (id, _reader, mut child) = sessions.start(None, 80, 24).expect("start pty");
        // Kill session via stop
        assert!(sessions.stop(&id).is_ok());
        // Child wait should complete cleanly because killer terminated the child
        let status = child.wait();
        assert!(status.is_ok());
    }
}
