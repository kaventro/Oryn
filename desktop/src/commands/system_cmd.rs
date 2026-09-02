use parking_lot::Mutex;
use serde_json::json;
use std::path::PathBuf;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

pub struct SystemStatsState {
    system: Mutex<System>,
}

impl Default for SystemStatsState {
    fn default() -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        Self {
            system: Mutex::new(system),
        }
    }
}

impl SystemStatsState {
    fn snapshot(&self) -> serde_json::Value {
        let mut system = self.system.lock();
        system.refresh_cpu_usage();
        system.refresh_memory();

        let cpu_pct = system.global_cpu_usage().round() as u32;
        // sysinfo 0.32+: memory values are already bytes.
        let ram_total = system.total_memory();
        let ram_used = system.used_memory();
        let ram_pct = if ram_total > 0 {
            ((ram_used as f64 / ram_total as f64) * 100.0).round() as u32
        } else {
            0
        };

        json!({
            "cpuPct": cpu_pct.min(100),
            "ramUsed": ram_used,
            "ramTotal": ram_total,
            "ramPct": ram_pct.min(100),
            "uptimeSec": System::uptime()
        })
    }
}

#[tauri::command]
pub fn system_get_stats(state: tauri::State<'_, SystemStatsState>) -> serde_json::Value {
    state.snapshot()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathSpaceIn {
    pub path: String,
}

#[tauri::command]
pub fn system_get_path_space(input: PathSpaceIn) -> serde_json::Value {
    let probe = PathBuf::from(&input.path);
    let disks = Disks::new_with_refreshed_list();

    let mut best_match_len = 0usize;
    let mut best_total = 0u64;
    let mut best_free = 0u64;

    for disk in disks.list() {
        let mount = disk.mount_point();
        if probe.starts_with(mount) {
            let len = mount.as_os_str().to_string_lossy().len();
            if len >= best_match_len {
                best_match_len = len;
                best_total = disk.total_space();
                best_free = disk.available_space();
            }
        }
    }

    if best_match_len == 0 {
        return json!({ "ok": false, "error": "No matching disk" });
    }

    json!({
        "ok": true,
        "path": input.path,
        "total": best_total,
        "free": best_free
    })
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskLocation {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
    pub file_system: String,
}

#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemLocationsOut {
    pub os: String,
    pub username: String,
    pub home: Option<String>,
    pub desktop: Option<String>,
    pub documents: Option<String>,
    pub downloads: Option<String>,
    pub pictures: Option<String>,
    pub music: Option<String>,
    pub videos: Option<String>,
    pub applications: Option<String>,
    pub favorites: Vec<crate::services::os_favorites::FavoriteFolder>,
    pub drives: Vec<DiskLocation>,
}

fn is_internal_mount(mount_point: &str) -> bool {
    if cfg!(target_os = "macos") {
        mount_point.starts_with("/System/Volumes")
            || mount_point.starts_with("/private/var")
            || mount_point.starts_with("/Library/Developer")
            || mount_point.starts_with("/dev")
            || mount_point == "/Volumes/Recovery"
            || mount_point.starts_with("/Volumes/.timemachine")
    } else if cfg!(target_os = "linux") {
        mount_point.starts_with("/proc")
            || mount_point.starts_with("/sys")
            || mount_point.starts_with("/dev")
            || mount_point.starts_with("/snap")
            || mount_point.starts_with("/var/lib/docker")
            || mount_point.starts_with("/var/lib/flatpak")
            || mount_point.starts_with("/boot")
            || (mount_point.starts_with("/run") && !mount_point.starts_with("/run/media"))
    } else {
        false
    }
}

fn path_to_clean_string(p: std::path::PathBuf) -> String {
    let s = p.to_string_lossy().to_string();
    crate::commands::path::clean_verbatim_path(&s)
}

#[tauri::command]
pub fn system_get_locations() -> SystemLocationsOut {
    let os_name = std::env::consts::OS.to_string();

    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .or_else(|| {
            dirs::home_dir().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        })
        .unwrap_or_else(|| "User".to_string());

    let home = dirs::home_dir().map(path_to_clean_string);
    let desktop = dirs::desktop_dir().map(path_to_clean_string);
    let documents = dirs::document_dir().map(path_to_clean_string);
    let downloads = dirs::download_dir().map(path_to_clean_string);
    let pictures = dirs::picture_dir().map(path_to_clean_string);
    let music = dirs::audio_dir().map(path_to_clean_string);
    let videos = dirs::video_dir().map(path_to_clean_string);

    let applications = if cfg!(target_os = "macos") {
        let app_dir = std::path::Path::new("/Applications");
        if app_dir.exists() {
            Some("/Applications".to_string())
        } else {
            None
        }
    } else if cfg!(target_os = "windows") {
        std::env::var("ProgramFiles")
            .ok()
            .filter(|p| std::path::Path::new(p).exists())
            .or_else(|| {
                if std::path::Path::new(r"C:\Program Files").exists() {
                    Some(r"C:\Program Files".to_string())
                } else {
                    None
                }
            })
    } else {
        let app_dir = std::path::Path::new("/usr/share/applications");
        if app_dir.exists() {
            Some("/usr/share/applications".to_string())
        } else {
            None
        }
    };

    let disks = Disks::new_with_refreshed_list();
    let mut drives: Vec<DiskLocation> = Vec::new();

    for disk in disks.list() {
        let raw_mount = disk.mount_point().to_string_lossy().to_string();
        let mount_point = crate::commands::path::clean_verbatim_path(&raw_mount);
        let raw_name = disk.name().to_string_lossy().to_string();
        let is_removable = disk.is_removable();
        let file_system = disk.file_system().to_string_lossy().to_string();

        if is_internal_mount(&mount_point) || drives.iter().any(|d| d.mount_point == mount_point) {
            continue;
        }

        let name = if cfg!(target_os = "windows") {
            if !raw_name.trim().is_empty() {
                format!(
                    "{} ({})",
                    raw_name.trim(),
                    mount_point.trim_end_matches('\\')
                )
            } else {
                format!("Local Disk ({})", mount_point.trim_end_matches('\\'))
            }
        } else if cfg!(target_os = "macos") {
            if mount_point == "/" {
                if raw_name.trim().is_empty() {
                    "Macintosh HD".to_string()
                } else {
                    raw_name
                }
            } else if !raw_name.trim().is_empty() {
                raw_name
            } else {
                std::path::Path::new(&mount_point)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| mount_point.clone())
            }
        } else if mount_point == "/" {
            "File System (/)".to_string()
        } else if !raw_name.trim().is_empty() {
            raw_name
        } else {
            std::path::Path::new(&mount_point)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| mount_point.clone())
        };

        drives.push(DiskLocation {
            name,
            mount_point,
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            is_removable,
            file_system,
        });
    }

    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let drive_root = format!("{}:\\", letter as char);
            if std::path::Path::new(&drive_root).exists() {
                let drive_letter_prefix = format!("{}:", letter as char);
                if !drives.iter().any(|d| {
                    d.mount_point.eq_ignore_ascii_case(&drive_root)
                        || d.mount_point
                            .trim_end_matches('\\')
                            .eq_ignore_ascii_case(&drive_letter_prefix)
                }) {
                    drives.push(DiskLocation {
                        name: format!("Local Disk ({}:)", letter as char),
                        mount_point: drive_root,
                        total_space: 0,
                        available_space: 0,
                        is_removable: false,
                        file_system: "NTFS".to_string(),
                    });
                }
            }
        }
    }

    if drives.is_empty() {
        if cfg!(target_os = "windows") {
            if std::path::Path::new(r"C:\").exists() {
                drives.push(DiskLocation {
                    name: "Local Disk (C:)".to_string(),
                    mount_point: r"C:\".to_string(),
                    total_space: 0,
                    available_space: 0,
                    is_removable: false,
                    file_system: "NTFS".to_string(),
                });
            }
        } else if cfg!(target_os = "macos") {
            drives.push(DiskLocation {
                name: "Macintosh HD".to_string(),
                mount_point: "/".to_string(),
                total_space: 0,
                available_space: 0,
                is_removable: false,
                file_system: "APFS".to_string(),
            });
        } else {
            drives.push(DiskLocation {
                name: "File System (/)".to_string(),
                mount_point: "/".to_string(),
                total_space: 0,
                available_space: 0,
                is_removable: false,
                file_system: "ext4".to_string(),
            });
        }
    }

    SystemLocationsOut {
        os: os_name,
        username,
        home,
        desktop,
        documents,
        downloads,
        pictures,
        music,
        videos,
        applications,
        favorites: crate::services::os_favorites::read_os_favorites(),
        drives,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_snapshot_has_bounded_percentages_and_memory() {
        let stats = SystemStatsState::default().snapshot();
        assert!(stats["cpuPct"].as_u64().is_some_and(|v| v <= 100));
        assert!(stats["ramPct"].as_u64().is_some_and(|v| v <= 100));
        assert!(stats["ramTotal"].as_u64().is_some_and(|v| v > 0));
        assert!(stats["uptimeSec"].as_u64().is_some_and(|v| v > 0));
    }

    #[test]
    fn system_locations_returns_valid_data() {
        let locs = system_get_locations();
        assert!(!locs.os.is_empty());
        assert!(!locs.username.is_empty());
        assert!(locs.home.is_some());
        assert!(!locs.drives.is_empty());
        for fav in &locs.favorites {
            assert!(!fav.path.is_empty());
            assert!(!fav.name.is_empty());
        }
    }
}
