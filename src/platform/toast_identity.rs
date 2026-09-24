//! Toast identity for unpackaged Windows apps. Windows drops any toast sent
//! under an AppUserModelID that resolves to no app identity — no MSIX
//! registration and no Start Menu shortcut carrying `System.AppUserModel.ID`.
//! `ToastNotifier::Show` still reports success, so the drop is silent.
//! [`ensure_toast_identity`] installs that shortcut for the running exe
//! (refreshing it when the exe moves, as dev builds do between target dirs)
//! and sets the process AUMID. Both steps are idempotent and best-effort.

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{E_OUTOFMEMORY, PROPERTYKEY, RPC_E_CHANGED_MODE};
use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemAlloc,
    CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READ,
};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, IShellLinkW, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath,
    SetCurrentProcessExplicitAppUserModelID,
};
use windows::core::{GUID, HSTRING, Interface, PWSTR};

const LOG: &str = "gpui_starter::toast_identity";
const SHORTCUT_NAME: &str = "gpui-starter.lnk";

// `System.AppUserModel_ID`; the windows crate ships it only behind the
// oversized Win32_Storage_EnhancedStorage feature.
const PKEY_APP_USER_MODEL_ID: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3),
    pid: 5,
};

// IShellLink coclass; the windows crate does not generate this CLSID const.
const CLSID_SHELL_LINK: GUID = GUID::from_u128(0x00021401_0000_0000_c000_000000000046);

/// Install the toast identity for `app_id`: explicit process AUMID plus a
/// Start Menu shortcut pointing at the running exe. `Err(reason)` never
/// blocks notification sending — callers log and continue.
pub fn ensure_toast_identity(app_id: &str) -> Result<(), String> {
    // Best-effort: the process AUMID affects taskbar grouping; the shortcut
    // below is what makes toasts display.
    unsafe {
        if let Err(err) = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(app_id)) {
            tracing::warn!(target: LOG, error = %err, "explicit process AUMID rejected");
        }
    }

    let programs = programs_dir()?;
    let exe = std::env::current_exe().map_err(|err| format!("current_exe: {err}"))?;
    ensure_shortcut_at(&programs, app_id, &exe)
}

/// Shortcut lives in the user's Start Menu Programs folder (no admin rights).
fn programs_dir() -> Result<PathBuf, String> {
    unsafe {
        let pwstr = SHGetKnownFolderPath(&FOLDERID_Programs, KNOWN_FOLDER_FLAG(0), None)
            .map_err(|err| format!("SHGetKnownFolderPath(Programs): {err}"))?;
        let decoded = pwstr_to_string(pwstr);
        CoTaskMemFree(Some(pwstr.as_ptr().cast()));
        decoded.map(PathBuf::from)
    }
}

pub(crate) fn ensure_shortcut_at(programs: &Path, app_id: &str, exe: &Path) -> Result<(), String> {
    let shortcut = programs.join(SHORTCUT_NAME);
    with_com(|| {
        if shortcut_matches(&shortcut, app_id, exe)? {
            return Ok(());
        }
        write_shortcut(&shortcut, app_id, exe)
    })
}

fn with_com(work: impl FnOnce() -> Result<(), windows::core::Error>) -> Result<(), String> {
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // S_FALSE = already initialized (usable, still must balance);
    // RPC_E_CHANGED_MODE = foreign apartment (usable, not ours to tear down).
    let balance = hr.is_ok();
    if !balance && hr != RPC_E_CHANGED_MODE {
        return Err(format!("CoInitializeEx: {hr}"));
    }
    let result = work();
    if balance {
        unsafe { CoUninitialize() };
    }
    result.map_err(|err| err.to_string())
}

/// A missing, unreadable, or stale shortcut is simply rewritten.
fn shortcut_matches(
    shortcut: &Path,
    app_id: &str,
    exe: &Path,
) -> Result<bool, windows::core::Error> {
    let Ok(link) = load_shortcut(shortcut) else {
        return Ok(false);
    };

    let mut buffer = [0u16; 1024];
    let mut find_data = WIN32_FIND_DATAW::default();
    unsafe { link.GetPath(&mut buffer, &mut find_data, 0)? };
    let target = pwstr_to_string(PWSTR(buffer.as_mut_ptr()))
        .map(PathBuf::from)
        .unwrap_or_default();
    if !same_path(&target, exe) {
        return Ok(false);
    }

    let store: IPropertyStore = link.cast()?;
    let mut propvar = unsafe { store.GetValue(&PKEY_APP_USER_MODEL_ID)? };
    let aumid = propvariant_string(&propvar);
    unsafe { PropVariantClear(&mut propvar)? };
    Ok(aumid.as_deref() == Some(app_id))
}

fn load_shortcut(shortcut: &Path) -> Result<IShellLinkW, windows::core::Error> {
    let link: IShellLinkW = unsafe { CoCreateInstance(&CLSID_SHELL_LINK, None, CLSCTX_ALL)? };
    let persist: IPersistFile = link.cast()?;
    unsafe { persist.Load(&HSTRING::from(shortcut.as_os_str()), STGM_READ)? };
    Ok(link)
}

fn write_shortcut(shortcut: &Path, app_id: &str, exe: &Path) -> Result<(), windows::core::Error> {
    let link: IShellLinkW = unsafe { CoCreateInstance(&CLSID_SHELL_LINK, None, CLSCTX_ALL)? };
    let exe_path = HSTRING::from(exe.as_os_str());
    unsafe {
        link.SetPath(&exe_path)?;
        link.SetDescription(&HSTRING::from("GPUI Starter"))?;
        if let Some(dir) = exe.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }
        link.SetIconLocation(&exe_path, 0)?;
    }

    let store: IPropertyStore = link.cast()?;
    let mut propvar = propvariant_from_str(app_id)?;
    unsafe {
        store.SetValue(&PKEY_APP_USER_MODEL_ID, &propvar)?;
        PropVariantClear(&mut propvar)?;
    }

    let persist: IPersistFile = link.cast()?;
    unsafe { persist.Save(&HSTRING::from(shortcut.as_os_str()), true)? };
    tracing::info!(
        target: LOG,
        shortcut = %shortcut.display(),
        "start-menu shortcut with toast AUMID written"
    );
    Ok(())
}

/// PROPVARIANT::VT_LPWSTR pointing at a CoTaskMemAlloc'd copy of `text`, so
/// `PropVariantClear` frees it correctly.
fn propvariant_from_str(text: &str) -> Result<PROPVARIANT, windows::core::Error> {
    let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();
    let buffer = unsafe { CoTaskMemAlloc(bytes) };
    if buffer.is_null() {
        return Err(windows::core::Error::from_hresult(E_OUTOFMEMORY));
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), buffer.cast::<u8>(), bytes);
        let mut propvar = PROPVARIANT::default();
        let inner = &mut *propvar.Anonymous.Anonymous;
        inner.vt = VT_LPWSTR;
        inner.Anonymous.pwszVal = PWSTR(buffer.cast());
        Ok(propvar)
    }
}

fn propvariant_string(propvar: &PROPVARIANT) -> Option<String> {
    unsafe {
        let inner = &*propvar.Anonymous.Anonymous;
        if inner.vt != VT_LPWSTR {
            return None;
        }
        pwstr_to_string(inner.Anonymous.pwszVal).ok()
    }
}

fn pwstr_to_string(pwstr: PWSTR) -> Result<String, String> {
    let mut len = 0usize;
    unsafe {
        while *pwstr.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16(std::slice::from_raw_parts(pwstr.0, len))
            .map_err(|err| format!("non-UTF16 path: {err}"))
    }
}

// Registry/shell paths compare case-insensitively; deep_link_registration
// reuses this for its stored command value.
pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

#[cfg(test)]
#[path = "toast_identity.test.rs"]
mod toast_identity_test;
