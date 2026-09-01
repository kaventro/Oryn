use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmOpts {
    pub title: Option<String>,
    pub message: String,
    pub ok_label: Option<String>,
    pub cancel_label: Option<String>,
    #[allow(dead_code)]
    pub danger: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoOpts {
    pub title: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickFolderInput {
    pub default_path: Option<String>,
}

#[tauri::command]
pub async fn dialog_confirm(opts: ConfirmOpts) -> Result<bool, String> {
    let title = opts.title.unwrap_or_else(|| "Oryn".into());
    let ok = opts.ok_label.unwrap_or_else(|| "OK".into());
    let cancel = opts.cancel_label.unwrap_or_else(|| "Cancel".into());
    // OkCancelCustom maps both buttons to MessageDialogResult::Custom(label) on macOS/GTK — not Ok/Yes.
    let buttons = rfd::MessageButtons::OkCancelCustom(ok.clone(), cancel);
    let answer = rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&opts.message)
        .set_buttons(buttons)
        .show();
    let confirmed = match answer {
        rfd::MessageDialogResult::Yes | rfd::MessageDialogResult::Ok => true,
        rfd::MessageDialogResult::No | rfd::MessageDialogResult::Cancel => false,
        rfd::MessageDialogResult::Custom(s) => s == ok,
    };
    Ok(confirmed)
}

#[tauri::command]
pub async fn dialog_info(opts: InfoOpts) -> Result<(), String> {
    let title = opts.title.unwrap_or_else(|| "Oryn".into());
    rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&opts.message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    Ok(())
}

#[tauri::command]
pub async fn dialog_pick_folder(input: PickFolderInput) -> Result<Value, String> {
    let mut d = rfd::FileDialog::new();
    if let Some(ref p) = input.default_path {
        if !p.is_empty() {
            d = d.set_directory(p);
        }
    }
    match d.pick_folder() {
        Some(p) => Ok(json!({ "ok": true, "path": p.to_string_lossy().to_string() })),
        None => Ok(json!({ "ok": false, "cancelled": true })),
    }
}
