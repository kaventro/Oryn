#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod fs_safe;
mod services;
pub mod vfs;

use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let filter = if cfg!(debug_assertions) {
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new("Oryn=debug,tauri=info,wry=warn")
        })
    } else {
        tracing_subscriber::EnvFilter::new("warn")
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();

    tracing::info!("Oryn starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(services::fs_transfer::TransferControl::default())
        .manage(commands::system_cmd::SystemStatsState::default())
        .manage(Arc::new(commands::search_cmd::SearchService::default()))
        .manage(services::remote::SessionPool::default())
        .manage(Arc::new(services::fs_watcher::FsWatcherService::default()))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                use objc2::AnyThread;
                use objc2_app_kit::{NSApplication, NSImage};
                use objc2_foundation::{MainThreadMarker, NSData};

                if let Some(mtm) = MainThreadMarker::new() {
                    let app_kit_app = NSApplication::sharedApplication(mtm);
                    const ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");
                    let data = NSData::with_bytes(ICON_BYTES);
                    if let Some(ns_image) = NSImage::initWithData(NSImage::alloc(), &data) {
                        unsafe {
                            app_kit_app.setApplicationIconImage(Some(&ns_image));
                        }
                    }
                }
            }

            if let Some(win) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = win.set_icon(icon.clone());
                }
            }
            let search_service = Arc::clone(
                app.state::<Arc<commands::search_cmd::SearchService>>()
                    .inner(),
            );
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    search_service.prune_expired();
                }
            });
            let watcher_service = Arc::clone(
                app.state::<Arc<services::fs_watcher::FsWatcherService>>()
                    .inner(),
            );
            watcher_service.set_app_handle(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::path::path_join,
            commands::path::path_dirname,
            commands::path::path_basename,
            commands::path::path_normalize,
            commands::path::app_get_home,
            commands::fs::config_load,
            commands::fs::fs_read_dir,
            commands::fs::fs_watch_dirs,
            commands::fs::fs_read_flat_branch,
            commands::fs::fs_stat_props,
            commands::fs::fs_get_dir_size,
            commands::fs::fs_analyze_dir,
            commands::fs::fs_scan_duplicates,
            commands::fs::fs_checksum,
            commands::fs::fs_read_file_text,
            commands::fs::fs_probe_text,
            commands::fs::fs_read_office,
            commands::fs::fs_read_media_data_url,
            commands::fs::fs_mkdir,
            commands::fs::fs_create_file,
            commands::fs::fs_write_file_text,
            commands::fs::fs_rename,
            commands::fs::fs_delete,
            commands::fs::fs_compress_zip,
            commands::fs::fs_compress,
            commands::fs::fs_extract,
            commands::fs::fs_copy,
            commands::fs::fs_move,
            commands::fs::fs_copy_conflicts,
            commands::fs::fs_cancel_copy,
            commands::remote_cmd::remote_list_profiles,
            commands::remote_cmd::remote_save_profile,
            commands::remote_cmd::remote_delete_profile,
            commands::remote_cmd::remote_test_connection,
            commands::remote_cmd::remote_connect,
            commands::remote_cmd::remote_disconnect,
            commands::remote_cmd::remote_read_dir,
            commands::remote_cmd::remote_read_file_text,
            commands::remote_cmd::remote_write_file_text,
            commands::remote_cmd::remote_mkdir,
            commands::remote_cmd::remote_create_file,
            commands::remote_cmd::remote_rename,
            commands::remote_cmd::remote_delete,
            commands::remote_cmd::remote_download,
            commands::remote_cmd::remote_upload,
            commands::shell_cmd::shell_open_path,
            commands::shell_cmd::shell_show_in_folder,
            commands::shell_cmd::shell_open_vscode,
            commands::shell_cmd::shell_open_terminal,
            commands::shell_cmd::clipboard_write,
            commands::shell_cmd::shell_exec,
            commands::dialog_cmd::dialog_confirm,
            commands::dialog_cmd::dialog_info,
            commands::dialog_cmd::dialog_pick_folder,
            commands::search_cmd::search_clear_cache,
            commands::search_cmd::search_start,
            commands::search_cmd::search_get_page,
            commands::search_cmd::search_cancel,
            commands::search_cmd::search_release,
            commands::replace_cmd::fs_find_replace,
            commands::zip_cmd::zip_extract,
            commands::compare_cmd::compare_dirs,
            commands::compare_cmd::compare_files,
            commands::git_cmd::git_is_repo,
            commands::git_cmd::git_status,
            commands::git_cmd::git_log,
            commands::git_cmd::git_diff,
            commands::git_cmd::git_blame,
            commands::git_cmd::git_add,
            commands::git_cmd::git_commit,
            commands::git_cmd::git_push,
            commands::git_cmd::git_pull,
            commands::git_cmd::git_branches,
            commands::git_cmd::git_checkout,
            commands::git_cmd::git_stage_file,
            commands::git_cmd::git_restore,
            commands::git_cmd::git_stash,
            commands::git_cmd::git_stash_pop,
            commands::git_cmd::git_stash_list,
            commands::system_cmd::system_get_stats,
            commands::system_cmd::system_get_path_space,
            commands::system_cmd::system_get_locations,
            commands::window_cmd::window_close,
            commands::window_cmd::window_minimize,
            commands::window_cmd::window_toggle_maximize,
            commands::window_cmd::set_dock_icon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn package_name_matches() {
        assert_eq!(env!("CARGO_PKG_NAME"), "Oryn");
    }
}
