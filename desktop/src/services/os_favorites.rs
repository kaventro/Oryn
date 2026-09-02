use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteFolder {
    pub name: String,
    pub path: String,
}

pub fn read_os_favorites() -> Vec<FavoriteFolder> {
    #[cfg(target_os = "macos")]
    {
        return macos::read();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::read();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

fn name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty() && !s.contains('\\'))
        .or_else(|| {
            path.replace('\\', "/")
                .rsplit('/')
                .find(|s| !s.is_empty())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| path.to_string())
}

fn push_unique(out: &mut Vec<FavoriteFolder>, path: PathBuf) {
    let Ok(meta) = std::fs::symlink_metadata(&path) else {
        return;
    };
    if !meta.is_dir() && !meta.file_type().is_symlink() {
        return;
    }
    if meta.file_type().is_symlink() {
        let Ok(resolved) = std::fs::canonicalize(&path) else {
            return;
        };
        if !resolved.is_dir() {
            return;
        }
    }
    let cleaned = crate::commands::path::clean_verbatim_path(&path.to_string_lossy());
    if cleaned.is_empty() {
        return;
    }
    if out.iter().any(|f| f.path.eq_ignore_ascii_case(&cleaned)) {
        return;
    }
    out.push(FavoriteFolder {
        name: name_from_path(&cleaned),
        path: cleaned,
    });
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{push_unique, FavoriteFolder};
    use std::ffi::{c_char, c_void, CString};
    use std::path::PathBuf;

    type CfTypeRef = *const c_void;
    type CfStringRef = *const c_void;
    type CfArrayRef = *const c_void;
    type CfUrlRef = *const c_void;
    type LsSharedFileListRef = *mut c_void;
    type LsSharedFileListItemRef = *mut c_void;

    const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const K_CFURL_POSIX_PATH_STYLE: i32 = 0;
    const K_LS_NO_UI_NO_MOUNT: u32 = 1 | 4;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: CfTypeRef);
        fn CFArrayGetCount(the_array: CfArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(the_array: CfArrayRef, idx: isize) -> *const c_void;
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const c_char,
            encoding: u32,
        ) -> CfStringRef;
        fn CFStringGetCString(
            the_string: CfStringRef,
            buffer: *mut u8,
            buffer_size: isize,
            encoding: u32,
        ) -> u8;
        fn CFURLCopyFileSystemPath(an_url: CfUrlRef, path_style: i32) -> CfStringRef;
    }

    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn LSSharedFileListCreate(
            allocator: *const c_void,
            list_type: CfStringRef,
            list_options: CfTypeRef,
        ) -> LsSharedFileListRef;
        fn LSSharedFileListCopySnapshot(
            in_list: LsSharedFileListRef,
            io_seed: *mut u32,
        ) -> CfArrayRef;
        fn LSSharedFileListItemCopyResolvedURL(
            in_item: LsSharedFileListItemRef,
            in_flags: u32,
            out_error: *mut CfTypeRef,
        ) -> CfUrlRef;
    }

    fn cfstring(s: &str) -> Option<CfStringRef> {
        let c = CString::new(s).ok()?;
        let cf = unsafe {
            CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), K_CFSTRING_ENCODING_UTF8)
        };
        if cf.is_null() {
            None
        } else {
            Some(cf)
        }
    }

    fn cfstring_to_rust(cf: CfStringRef) -> Option<String> {
        if cf.is_null() {
            return None;
        }
        let mut buf = vec![0u8; 4096];
        let ok = unsafe {
            CFStringGetCString(
                cf,
                buf.as_mut_ptr(),
                buf.len() as isize,
                K_CFSTRING_ENCODING_UTF8,
            )
        };
        if ok == 0 {
            return None;
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8(buf[..end].to_vec()).ok()
    }

    pub fn read() -> Vec<FavoriteFolder> {
        let mut out = Vec::new();
        unsafe {
            let Some(list_id) = cfstring("com.apple.LSSharedFileList.FavoriteItems") else {
                return out;
            };
            let list = LSSharedFileListCreate(std::ptr::null(), list_id, std::ptr::null());
            CFRelease(list_id);
            if list.is_null() {
                return out;
            }
            let mut seed: u32 = 0;
            let snapshot = LSSharedFileListCopySnapshot(list, &mut seed);
            CFRelease(list as CfTypeRef);
            if snapshot.is_null() {
                return out;
            }
            let count = CFArrayGetCount(snapshot);
            for i in 0..count {
                let item = CFArrayGetValueAtIndex(snapshot, i) as LsSharedFileListItemRef;
                if item.is_null() {
                    continue;
                }
                let mut err: CfTypeRef = std::ptr::null();
                let url = LSSharedFileListItemCopyResolvedURL(item, K_LS_NO_UI_NO_MOUNT, &mut err);
                if !err.is_null() {
                    CFRelease(err);
                }
                if url.is_null() {
                    continue;
                }
                let path_cf = CFURLCopyFileSystemPath(url, K_CFURL_POSIX_PATH_STYLE);
                CFRelease(url as CfTypeRef);
                let path = cfstring_to_rust(path_cf);
                if !path_cf.is_null() {
                    CFRelease(path_cf);
                }
                if let Some(p) = path {
                    push_unique(&mut out, PathBuf::from(p));
                }
            }
            CFRelease(snapshot as CfTypeRef);
        }
        out
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{push_unique, FavoriteFolder};
    use std::path::{Path, PathBuf};

    fn utf16le_dirs_in(bytes: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == 0 && bytes[i + 1] == 0 {
                i += 2;
                continue;
            }
            let mut units: Vec<u16> = Vec::new();
            let mut j = i;
            while j + 1 < bytes.len() {
                let u = u16::from_le_bytes([bytes[j], bytes[j + 1]]);
                if u == 0 {
                    break;
                }
                if (u < 32 && u != 9) || u == 0xFFFE {
                    units.clear();
                    break;
                }
                units.push(u);
                j += 2;
            }
            if units.len() >= 3 {
                if let Ok(s) = String::from_utf16(&units) {
                    let t = s.trim();
                    if (t.starts_with('\\') || t.chars().nth(1) == Some(':'))
                        && Path::new(t).is_dir()
                    {
                        out.push(t.to_string());
                    }
                }
            }
            i += 2;
        }
        out
    }

    pub fn read() -> Vec<FavoriteFolder> {
        let mut out = Vec::new();
        let Some(home) = dirs::home_dir() else {
            return out;
        };
        let links = home.join("Links");
        let Ok(entries) = std::fs::read_dir(&links) else {
            return out;
        };
        let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        for path in files {
            if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("lnk")) == Some(true)
            {
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Some(target) = utf16le_dirs_in(&bytes).into_iter().next() {
                        push_unique(&mut out, PathBuf::from(target));
                    }
                }
            } else if path.is_dir() {
                push_unique(&mut out, path);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_from_path_uses_basename() {
        assert_eq!(name_from_path("/Users/me/Developer"), "Developer");
        assert_eq!(name_from_path(r"C:\Users\me\Desktop"), "Desktop");
    }
}
