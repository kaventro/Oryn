use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_THUMBNAILONLY,
};

use super::{encode_rgba, FrameRequest, FrameSource};

pub struct ShellFrames;

impl FrameSource for ShellFrames {
    fn render(&self, request: &FrameRequest) -> Option<PathBuf> {
        let (width, height, bgra) = shell_bitmap(&request.video, request.size)?;
        let png = encode_rgba(width, height, &to_rgba(bgra))?;
        std::fs::write(&request.out, png).ok()?;
        Some(request.out.clone())
    }
}

fn plain_path(path: &Path) -> Vec<u16> {
    let text = path.as_os_str().to_string_lossy();
    let text = match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => rest.to_string(),
        Some(rest) => format!(r"\\{}", &rest[4..]),
        None => text.into_owned(),
    };
    std::ffi::OsStr::new(&text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn to_rgba(mut px: Vec<u8>) -> Vec<u8> {
    let opaque = px.chunks_exact(4).all(|p| p[3] == 0);
    for p in px.chunks_exact_mut(4) {
        p.swap(0, 2);
        if opaque {
            p[3] = 255;
        }
    }
    px
}

fn shell_bitmap(video: &Path, size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let wide = plain_path(video);
    let side = i32::try_from(size).ok()?;
    unsafe {
        let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = (|| {
            let factory: IShellItemImageFactory =
                SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).ok()?;
            let bitmap = factory
                .GetImage(
                    SIZE { cx: side, cy: side },
                    SIIGBF_BIGGERSIZEOK | SIIGBF_THUMBNAILONLY,
                )
                .ok()?;
            let pixels = read_bitmap(bitmap);
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            pixels
        })();
        if com.is_ok() {
            CoUninitialize();
        }
        result
    }
}

unsafe fn read_bitmap(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut c_void),
        )
    };
    let (w, h) = (info.bmWidth, info.bmHeight.abs());
    if got == 0 || w <= 0 || h <= 0 || w > 4096 || h > 4096 {
        return None;
    }
    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; w as usize * h as usize * 4];
    let lines = unsafe {
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            bitmap,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        lines
    };
    (lines == h).then_some((w as u32, h as u32, pixels))
}
