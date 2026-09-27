use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteFolder {
    pub name: String,
    pub path: String,
}

pub trait FavoritesProvider {
    fn get_favorites(&self) -> Vec<FavoriteFolder>;
}

pub struct OsFavoritesService;

impl OsFavoritesService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OsFavoritesService {
    fn default() -> Self {
        Self::new()
    }
}

impl FavoritesProvider for OsFavoritesService {
    fn get_favorites(&self) -> Vec<FavoriteFolder> {
        read_os_favorites()
    }
}

pub fn read_os_favorites() -> Vec<FavoriteFolder> {
    #[cfg(target_os = "macos")]
    {
        macos::MacOsFavoritesProvider::new().get_favorites()
    }
    #[cfg(target_os = "windows")]
    {
        windows::WindowsFavoritesProvider::new().get_favorites()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Vec::new()
    }
}

fn name_from_path(path: &str) -> String {
    let trimmed = path.trim();
    if (trimmed.len() == 2 && trimmed.ends_with(':'))
        || (trimmed.len() == 3 && trimmed.ends_with(r":\"))
        || (trimmed.len() == 3 && trimmed.ends_with(":/"))
    {
        let drive = trimmed.chars().next().unwrap_or('C').to_ascii_uppercase();
        return format!("Local Disk ({}:)", drive);
    }
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
    use super::{push_unique, FavoriteFolder, FavoritesProvider};
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

    pub struct MacOsFavoritesProvider;

    impl MacOsFavoritesProvider {
        pub fn new() -> Self {
            Self
        }
    }

    impl FavoritesProvider for MacOsFavoritesProvider {
        fn get_favorites(&self) -> Vec<FavoriteFolder> {
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
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{name_from_path, push_unique, FavoriteFolder, FavoritesProvider};
    use std::ffi::c_void;
    use std::path::{Path, PathBuf};

    type HRESULT = i32;
    type HWND = *mut c_void;
    const S_OK: HRESULT = 0;

    const SFGAO_FOLDER: u32 = 0x2000_0000;
    const SFGAO_STREAM: u32 = 0x0040_0000;
    const SHGDN_NORMAL: u32 = 0;
    const SHGDN_FORPARSING: u32 = 0x8000;
    const SHCONTF_FOLDERS: u32 = 0x0020;
    const SHCONTF_NONFOLDERS: u32 = 0x0040;

    #[repr(C)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const IID_ISHELL_FOLDER: GUID = GUID {
        data1: 0x000214E6,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    #[repr(C)]
    struct STRRET {
        u_type: u32,
        data: [u8; 264],
    }

    #[repr(C)]
    struct IShellFolderVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        parse_display_name: unsafe extern "system" fn(
            *mut c_void,
            HWND,
            *mut c_void,
            *const u16,
            *mut u32,
            *mut *mut c_void,
            *mut u32,
        ) -> HRESULT,
        enum_objects: unsafe extern "system" fn(*mut c_void, HWND, u32, *mut *mut c_void) -> HRESULT,
        bind_to_object: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            *mut c_void,
            *const GUID,
            *mut *mut c_void,
        ) -> HRESULT,
        bind_to_storage: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        compare_ids: unsafe extern "system" fn(*mut c_void, isize, *const c_void, *const c_void) -> HRESULT,
        create_view_object: unsafe extern "system" fn(*mut c_void, HWND, *const GUID, *mut *mut c_void) -> HRESULT,
        get_attributes_of: unsafe extern "system" fn(*mut c_void, u32, *const *const c_void, *mut u32) -> HRESULT,
        get_ui_object_of: unsafe extern "system" fn(*mut c_void, HWND, u32, *const *const c_void, *const GUID, *mut u32, *mut *mut c_void) -> HRESULT,
        get_display_name_of: unsafe extern "system" fn(*mut c_void, *const c_void, u32, *mut STRRET) -> HRESULT,
        set_name_of: unsafe extern "system" fn(*mut c_void, HWND, *const c_void, *const u16, u32, *mut *mut c_void) -> HRESULT,
    }

    #[repr(C)]
    struct IEnumIDListVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        next: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void, *mut u32) -> HRESULT,
        skip: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT,
        reset: unsafe extern "system" fn(*mut c_void) -> HRESULT,
        clone: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetDesktopFolder(ppshf: *mut *mut c_void) -> HRESULT;
    }

    #[link(name = "shlwapi")]
    extern "system" {
        fn StrRetToBufW(pstr: *mut STRRET, pidl: *const c_void, pszBuf: *mut u16, cchBuf: u32) -> HRESULT;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pvReserved: *mut c_void, dwCoInit: u32) -> HRESULT;
        fn CoUninitialize();
        fn CoTaskMemFree(pv: *mut c_void);
    }

    struct ComGuard {
        initialized: bool,
    }

    impl ComGuard {
        fn enter() -> Self {
            let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), 0) };
            Self {
                initialized: hr >= 0,
            }
        }
    }

    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.initialized {
                unsafe { CoUninitialize() };
            }
        }
    }

    struct ComPtr<T, V> {
        raw: *mut T,
        vtbl: *mut V,
    }

    impl<T, V> ComPtr<T, V> {
        fn from_raw(raw: *mut T) -> Option<Self> {
            if raw.is_null() {
                None
            } else {
                let vtbl = unsafe { *(raw as *mut *mut V) };
                Some(Self { raw, vtbl })
            }
        }

        fn as_raw(&self) -> *mut T {
            self.raw
        }

        fn vtbl(&self) -> &V {
            unsafe { &*self.vtbl }
        }
    }

    impl<T, V> Drop for ComPtr<T, V> {
        fn drop(&mut self) {
            if !self.raw.is_null() {
                unsafe {
                    let unk = *(self.raw as *mut *mut [unsafe extern "system" fn(*mut c_void) -> u32; 3]);
                    let release = (*unk)[2];
                    release(self.raw as *mut c_void);
                }
            }
        }
    }

    struct CoMem<T>(*mut T);

    impl<T> CoMem<T> {
        fn new(ptr: *mut T) -> Self {
            Self(ptr)
        }
        fn as_raw(&self) -> *mut T {
            self.0
        }
    }

    impl<T> Drop for CoMem<T> {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CoTaskMemFree(self.0 as *mut c_void) };
            }
        }
    }

    struct QuickAccessShellReader;

    impl QuickAccessShellReader {
        fn new() -> Self {
            Self
        }

        fn read_items(&self, out: &mut Vec<FavoriteFolder>) -> bool {
            let _com = ComGuard::enter();

            let mut raw_desktop: *mut c_void = std::ptr::null_mut();
            if unsafe { SHGetDesktopFolder(&mut raw_desktop) } != S_OK {
                return false;
            }
            let desktop = match ComPtr::<c_void, IShellFolderVtbl>::from_raw(raw_desktop) {
                Some(p) => p,
                None => return false,
            };

            let qa_path: Vec<u16> = "shell:::{679f85cb-0220-4080-b29b-5540cc05aab6}"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let mut raw_pidl_qa: *mut c_void = std::ptr::null_mut();
            let mut eaten = 0u32;
            let mut attribs = 0u32;
            let hr = unsafe {
                (desktop.vtbl().parse_display_name)(
                    desktop.as_raw(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    qa_path.as_ptr(),
                    &mut eaten,
                    &mut raw_pidl_qa,
                    &mut attribs,
                )
            };
            if hr != S_OK {
                return false;
            }
            let pidl_qa = CoMem::new(raw_pidl_qa);

            let mut raw_qa_folder: *mut c_void = std::ptr::null_mut();
            let hr = unsafe {
                (desktop.vtbl().bind_to_object)(
                    desktop.as_raw(),
                    pidl_qa.as_raw(),
                    std::ptr::null_mut(),
                    &IID_ISHELL_FOLDER,
                    &mut raw_qa_folder,
                )
            };
            if hr != S_OK {
                return false;
            }
            let qa_folder = match ComPtr::<c_void, IShellFolderVtbl>::from_raw(raw_qa_folder) {
                Some(p) => p,
                None => return false,
            };

            let mut raw_enum: *mut c_void = std::ptr::null_mut();
            let hr = unsafe {
                (qa_folder.vtbl().enum_objects)(
                    qa_folder.as_raw(),
                    std::ptr::null_mut(),
                    SHCONTF_FOLDERS | SHCONTF_NONFOLDERS,
                    &mut raw_enum,
                )
            };
            if hr != S_OK {
                return false;
            }
            let enum_list = match ComPtr::<c_void, IEnumIDListVtbl>::from_raw(raw_enum) {
                Some(p) => p,
                None => return false,
            };

            let mut child_pidl: *mut c_void = std::ptr::null_mut();
            let mut fetched = 0u32;
            let mut buf = vec![0u16; 1024];
            let mut count_added = 0;

            while unsafe { (enum_list.vtbl().next)(enum_list.as_raw(), 1, &mut child_pidl, &mut fetched) } == S_OK
                && fetched == 1
            {
                let child_guard = CoMem::new(child_pidl);
                if child_pidl.is_null() {
                    continue;
                }

                let mut attrs = SFGAO_FOLDER | SFGAO_STREAM;
                let apidl = [child_pidl as *const c_void];
                let _ = unsafe {
                    (qa_folder.vtbl().get_attributes_of)(qa_folder.as_raw(), 1, apidl.as_ptr(), &mut attrs)
                };
                let is_folder = (attrs & SFGAO_FOLDER) != 0 && (attrs & SFGAO_STREAM) == 0;
                if !is_folder {
                    continue;
                }

                let mut strret_path: STRRET = unsafe { std::mem::zeroed() };
                let mut path_str = String::new();
                if unsafe { (qa_folder.vtbl().get_display_name_of)(qa_folder.as_raw(), child_pidl, SHGDN_FORPARSING, &mut strret_path) } == S_OK {
                    if unsafe { StrRetToBufW(&mut strret_path, child_pidl, buf.as_mut_ptr(), buf.len() as u32) } == S_OK {
                        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                        path_str = String::from_utf16_lossy(&buf[..len]);
                    }
                }

                let mut strret_name: STRRET = unsafe { std::mem::zeroed() };
                let mut name_str = String::new();
                if unsafe { (qa_folder.vtbl().get_display_name_of)(qa_folder.as_raw(), child_pidl, SHGDN_NORMAL, &mut strret_name) } == S_OK {
                    if unsafe { StrRetToBufW(&mut strret_name, child_pidl, buf.as_mut_ptr(), buf.len() as u32) } == S_OK {
                        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                        name_str = String::from_utf16_lossy(&buf[..len]);
                    }
                }

                if !path_str.is_empty() && Path::new(&path_str).is_dir() {
                    let label = if !name_str.is_empty() {
                        name_str
                    } else {
                        name_from_path(&path_str)
                    };
                    let cleaned = crate::commands::path::clean_verbatim_path(&path_str);
                    if !cleaned.is_empty() && !out.iter().any(|f| f.path.eq_ignore_ascii_case(&cleaned)) {
                        out.push(FavoriteFolder {
                            name: label,
                            path: cleaned,
                        });
                        count_added += 1;
                    }
                }
                drop(child_guard);
            }

            count_added > 0
        }
    }

    struct KnownFoldersResolver;

    impl KnownFoldersResolver {
        const RESOLVERS: &'static [fn() -> Option<PathBuf>] = &[
            dirs::desktop_dir,
            dirs::download_dir,
            dirs::document_dir,
            dirs::picture_dir,
            dirs::audio_dir,
            dirs::video_dir,
        ];

        fn resolve(&self, out: &mut Vec<FavoriteFolder>) {
            for path in Self::RESOLVERS.iter().filter_map(|resolve| resolve()) {
                push_unique(out, path);
            }
        }
    }

    struct LegacyLinksResolver;

    impl LegacyLinksResolver {
        fn resolve(&self, out: &mut Vec<FavoriteFolder>) {
            if let Some(links_dir) = dirs::home_dir().map(|h| h.join("Links")) {
                if let Ok(entries) = std::fs::read_dir(links_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            push_unique(out, path);
                        }
                    }
                }
            }
        }
    }

    pub struct WindowsFavoritesProvider {
        qa_reader: QuickAccessShellReader,
        known_resolver: KnownFoldersResolver,
        legacy_resolver: LegacyLinksResolver,
    }

    impl WindowsFavoritesProvider {
        pub fn new() -> Self {
            Self {
                qa_reader: QuickAccessShellReader::new(),
                known_resolver: KnownFoldersResolver,
                legacy_resolver: LegacyLinksResolver,
            }
        }
    }

    impl Default for WindowsFavoritesProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl FavoritesProvider for WindowsFavoritesProvider {
        fn get_favorites(&self) -> Vec<FavoriteFolder> {
            let mut out = Vec::new();
            let _ = self.qa_reader.read_items(&mut out);
            self.known_resolver.resolve(&mut out);
            self.legacy_resolver.resolve(&mut out);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_from_path_uses_basename() {
        assert_eq!(name_from_path("/Users/me/Developer"), "Developer");
        assert_eq!(name_from_path(r"C:\Users\me\Desktop"), "Desktop");
        assert_eq!(name_from_path(r"D:\"), "Local Disk (D:)");
        assert_eq!(name_from_path(r"D:"), "Local Disk (D:)");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_read_returns_known_and_pinned_folders() {
        let provider = windows::WindowsFavoritesProvider::new();
        let favs = provider.get_favorites();
        assert!(!favs.is_empty());
        let paths: Vec<&str> = favs.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.eq_ignore_ascii_case("D:\\") || p.eq_ignore_ascii_case("D:")));
        assert!(paths.iter().any(|p| p.to_lowercase().ends_with("shelf")));
        if let Some(desktop) = dirs::desktop_dir() {
            let d_str = crate::commands::path::clean_verbatim_path(&desktop.to_string_lossy());
            assert!(paths.iter().any(|p| p.eq_ignore_ascii_case(&d_str)));
        }
    }
}
