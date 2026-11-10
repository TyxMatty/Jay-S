//! The taskbar icon, drawn by the page in the current style (frontend/src/ui/window-icon.js).
//! Copied from Jay-C (frontend/src-tauri/src/taskbar_icon.rs), without its child-window grouping
//! (Jay-S has no PiP).
//!
//! `ICON_BIG` alone isn't enough once the app is installed: Windows ties `jay-s.exe` to its Start
//! menu shortcut and shows the shortcut's icon (the .exe's), ignoring the window's (measured in
//! Jay-C). So the window also gets its own AppUserModelID, which unties it, and "relaunch"
//! properties: the taskbar (and the right-click menu, and a pin made from it) then use the relaunch
//! icon, an .ico of the same picture. Explorer keeps what it first drew for an app ID, a blank page
//! included, and clearing the ID and setting the same one back doesn't make it look again (measured
//! in Jay-S, 2026-11-09: one window, one valid .ico, a blank page under the old fixed ID and the
//! picture under any new one). So the ID carries the picture's hash: a new picture is a new ID.

use std::hash::{Hash, Hasher};
use std::path::Path;

use tauri::Manager;
use windows::Win32::Foundation::{HWND, LPARAM, PROPERTYKEY, WPARAM};
use windows::Win32::Storage::EnhancedStorage::{
    PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand, PKEY_AppUserModel_RelaunchDisplayNameResource,
    PKEY_AppUserModel_RelaunchIconResource,
};
use windows::Win32::System::Com::CoTaskMemAlloc;
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::WindowsAndMessaging::{CreateIcon, DestroyIcon, HICON, ICON_BIG, SendMessageW, WM_SETICON};
use windows::core::PWSTR;

/// Start of the window's own app ID (the picture's hash follows): not the Start menu shortcut's
/// (`io.github.tyxmatty.jays`, set by the installer), or Windows ties them again. Dev and release
/// differ, so a dev build never shares a taskbar button (or what Explorer remembers about it) with
/// the installed app.
/// shortcut: a button pinned from the window keeps the ID of the picture it was pinned with, so
/// after a style change the running window opens beside the pin; pin the Start menu shortcut.
const WINDOW_APP_ID: &str = if cfg!(debug_assertions) { "io.github.tyxmatty.jays.window.dev" } else { "io.github.tyxmatty.jays.window" };

/// `rgba`: `size` x `size` pixels, 4 bytes each (straight alpha), as from a canvas.
#[tauri::command]
pub fn set_taskbar_icon(window: tauri::WebviewWindow, rgba: Vec<u8>, size: u32) -> Result<(), String> {
    let pixels = (size * size) as usize;
    if size == 0 || size > 256 || rgba.len() != pixels * 4 {
        return Err(format!("expected {size}x{size} RGBA pixels, got {} bytes", rgba.len()));
    }
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    // Named after its content, so a changed picture is never served from an icon cache. Explorer
    // loads this file itself, so it must be where Explorer sees it: an app started from a packaged
    // (MSIX) program gets a private copy of AppData, the file lands there, Explorer finds nothing at
    // this path and the button shows a blank page (measured in Jay-S, 2026-11-09: the same window
    // showed the picture once the property pointed at the private copy's real path). Dev and release
    // get different names so the installed app never inherits a path Explorer once failed to load;
    // the base name also changed from "taskbar-" for the same reason (Jay-A does the same).
    let kind = if cfg!(debug_assertions) { "dev" } else { "app" };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    rgba.hash(&mut hasher);
    let dir = window.app_handle().path().app_local_data_dir().map_err(|e| e.to_string())?;
    let hash = hasher.finish();
    let ico_path = dir.join(format!("window-icon-{kind}-{hash:016x}.ico"));
    if !ico_path.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(&ico_path, ico(&rgba, size)).map_err(|e| e.to_string())?;
    }
    // CreateIcon takes BGRA color bits and an AND mask (here one byte per pixel: the inverted alpha).
    let mut bgra = rgba;
    let mut and_mask = Vec::with_capacity(pixels);
    for px in bgra.chunks_exact_mut(4) {
        and_mask.push(px[3].wrapping_sub(u8::MAX));
        px.swap(0, 2);
    }
    unsafe {
        let icon = CreateIcon(None, size as i32, size as i32, 1, 32, and_mask.as_ptr(), bgra.as_ptr()).map_err(|e| e.to_string())?;
        // The previous big icon is one this command made (nothing else sets ICON_BIG): free it.
        let old = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(icon.0 as isize)));
        if old.0 != 0 {
            let _ = DestroyIcon(HICON(old.0 as _));
        }
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // The app ID and relaunch icon only untie the installed app from its Start menu shortcut. Run
    // from anywhere else (dev, a build folder) there is no shortcut to untie and ICON_BIG alone
    // shows the picture; it also keeps a dev run started from a packaged program (see above) from
    // pointing Explorer at a file it can't see.
    if !installed(&exe) {
        return Ok(());
    }
    relaunch_with_icon(hwnd, &exe, &ico_path, &app_id(hash)).map_err(|e| format!("taskbar properties: {e}"))
}

fn app_id(hash: u64) -> String {
    format!("{WINDOW_APP_ID}.{hash:016x}")
}

/// Where the installer (NSIS, current user) puts the app: `%LOCALAPPDATA%\Jay-S\jay-s.exe`.
fn installed(exe: &Path) -> bool {
    let Some(local) = std::env::var_os("LOCALAPPDATA") else { return false };
    exe.parent().is_some_and(|dir| dir.as_os_str().eq_ignore_ascii_case(Path::new(&local).join("Jay-S").as_os_str()))
}

/// The relaunch properties, then the app ID: the taskbar reads them when the ID changes.
fn relaunch_with_icon(hwnd: HWND, exe: &Path, ico_path: &Path, app_id: &str) -> windows::core::Result<()> {
    let store: IPropertyStore = unsafe { SHGetPropertyStoreForWindow(hwnd)? };
    set_text(&store, &PKEY_AppUserModel_RelaunchCommand, &format!("\"{}\"", exe.display()))?;
    set_text(&store, &PKEY_AppUserModel_RelaunchDisplayNameResource, "Jay-S")?;
    // "file,index", as the property is documented (a bare path loads too: measured in Jay-S).
    set_text(&store, &PKEY_AppUserModel_RelaunchIconResource, &format!("{},0", ico_path.display()))?;
    set_text(&store, &PKEY_AppUserModel_ID, app_id)?;
    unsafe { store.Commit() }
}

fn set_text(store: &IPropertyStore, key: &PROPERTYKEY, text: &str) -> windows::core::Result<()> {
    let mut value = string_value(text);
    let result = unsafe { store.SetValue(key, &value) };
    let _ = unsafe { PropVariantClear(&mut value) };
    result
}

/// A VT_LPWSTR value (its string is freed by PropVariantClear).
fn string_value(text: &str) -> PROPVARIANT {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let mut value = PROPVARIANT::default();
    unsafe {
        let ptr = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
        ptr.copy_from_nonoverlapping(wide.as_ptr(), wide.len());
        let inner = &mut *value.Anonymous.Anonymous;
        inner.vt = VT_LPWSTR;
        inner.Anonymous.pwszVal = PWSTR(ptr);
    }
    value
}

/// A one-picture .ico (32-bit BGRA bitmap, bottom-up, empty AND mask) from straight-alpha RGBA.
fn ico(rgba: &[u8], size: u32) -> Vec<u8> {
    let side = size as usize;
    let mask_row = side.div_ceil(32) * 4; // 1 bit per pixel, rows padded to 32 bits
    let image_len = 40 + rgba.len() + mask_row * side;
    let dim = if size >= 256 { 0 } else { size as u8 }; // 0 means 256
    let mut out = Vec::with_capacity(22 + image_len);
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]); // ICONDIR: an icon, 1 picture
    out.extend_from_slice(&[dim, dim, 0, 0]); // ICONDIRENTRY: size, no palette
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&(image_len as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes()); // where the picture starts
    out.extend_from_slice(&40u32.to_le_bytes()); // BITMAPINFOHEADER
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&(2 * size as i32).to_le_bytes()); // color + mask
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]); // no compression, sizes and palette unused
    for row in rgba.chunks_exact(side * 4).rev() {
        for px in row.chunks_exact(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    out.resize(22 + image_len, 0); // the AND mask: transparency comes from the alpha
    out
}

#[cfg(test)]
mod tests {
    use super::{app_id, ico};

    #[test]
    fn app_id_follows_the_picture() {
        assert_ne!(app_id(1), app_id(2));
        // Windows allows 128 characters, no spaces.
        assert!(app_id(u64::MAX).len() <= 128 && !app_id(u64::MAX).contains(' '));
    }

    #[test]
    fn ico_layout() {
        // 2x2: top row red, green; bottom row blue, transparent.
        let rgba = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0];
        let file = ico(&rgba, 2);
        assert_eq!(&file[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!((file[6], file[7]), (2, 2));
        let image_len = u32::from_le_bytes(file[14..18].try_into().unwrap()) as usize;
        assert_eq!(file.len(), 22 + image_len);
        assert_eq!(image_len, 40 + 16 + 4 * 2, "header, pixels, mask rows of 4 bytes");
        assert_eq!(i32::from_le_bytes(file[30..34].try_into().unwrap()), 4, "height counts the mask");
        // Bottom-up BGRA: the bottom row (blue, transparent) comes first.
        assert_eq!(&file[62..70], &[255, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(&file[70..78], &[0, 0, 255, 255, 0, 255, 0, 255]);
    }
}
