use std::path::PathBuf;

use super::{ensure_shortcut_at, same_path};

fn temp_programs(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gpui-starter-toast-identity-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn fake_exe(dir: &std::path::Path, name: &str) -> PathBuf {
    let exe = dir.join(name);
    std::fs::write(&exe, b"stub exe").expect("stub exe");
    exe
}

#[test]
fn shortcut_is_written_and_reused() {
    let programs = temp_programs("reuse");
    let exe = fake_exe(&programs, "app.exe");

    ensure_shortcut_at(&programs, "com.example.app", &exe).expect("first write");
    let lnk = programs.join("gpui-starter.lnk");
    assert!(lnk.exists(), "shortcut must exist after ensure");

    // Idempotent: a second run with identical inputs must not rewrite the
    // file (mtime unchanged).
    let before = std::fs::metadata(&lnk)
        .expect("metadata")
        .modified()
        .expect("mtime");
    std::thread::sleep(std::time::Duration::from_millis(20));
    ensure_shortcut_at(&programs, "com.example.app", &exe).expect("second run");
    let after = std::fs::metadata(&lnk)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(
        before, after,
        "unchanged exe + AUMID must leave the shortcut untouched"
    );
}

#[test]
fn shortcut_is_refreshed_when_exe_moves() {
    let programs = temp_programs("exe-moves");
    let exe_a = fake_exe(&programs, "app-a.exe");
    let exe_b = fake_exe(&programs, "app-b.exe");

    ensure_shortcut_at(&programs, "com.example.app", &exe_a).expect("write for exe a");
    ensure_shortcut_at(&programs, "com.example.app", &exe_b).expect("rewrite for exe b");

    let (target, aumid) = read_back(&programs);
    // read_back yields the shell-normalized long form, which never textually
    // equals the temp_dir-built path when TEMP is the 8.3 short form.
    assert!(
        same_path(&target, &exe_b),
        "shortcut must follow the current exe: {} != {}",
        target.display(),
        exe_b.display()
    );
    assert_eq!(aumid.as_deref(), Some("com.example.app"));
}

#[test]
fn shortcut_is_refreshed_when_aumid_changes() {
    let programs = temp_programs("aumid-changes");
    let exe = fake_exe(&programs, "app.exe");

    ensure_shortcut_at(&programs, "com.old.app", &exe).expect("write old AUMID");
    ensure_shortcut_at(&programs, "com.new.app", &exe).expect("rewrite AUMID");

    let (target, aumid) = read_back(&programs);
    assert!(
        same_path(&target, &exe),
        "shortcut must follow the current exe: {} != {}",
        target.display(),
        exe.display()
    );
    assert_eq!(aumid.as_deref(), Some("com.new.app"));
}

/// Decode the shortcut again through the shell APIs, independent of the
/// shortcut_matches fast path, so the assertions observe the persisted file.
fn read_back(programs: &std::path::Path) -> (PathBuf, Option<String>) {
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
        IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::IShellLinkW;
    use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
    use windows::core::{HSTRING, Interface};

    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    assert!(hr.is_ok(), "COM init in test: {hr}");

    let result = (|| {
        let link: IShellLinkW =
            unsafe { CoCreateInstance(&super::CLSID_SHELL_LINK, None, CLSCTX_ALL) }
                .expect("shell link instance");
        let persist: IPersistFile = link.cast().expect("IPersistFile");
        let lnk = programs.join("gpui-starter.lnk");
        unsafe {
            persist
                .Load(&HSTRING::from(lnk.as_os_str()), STGM_READ)
                .expect("load lnk")
        };

        let mut buffer = [0u16; 1024];
        let mut find_data = WIN32_FIND_DATAW::default();
        unsafe {
            link.GetPath(&mut buffer, &mut find_data, 0)
                .expect("read target")
        };
        let target =
            String::from_utf16_lossy(&buffer[..buffer.iter().position(|&c| c == 0).unwrap()]);

        let store: IPropertyStore = link.cast().expect("IPropertyStore");
        let mut propvar = unsafe {
            store
                .GetValue(&super::PKEY_APP_USER_MODEL_ID)
                .expect("read AUMID")
        };
        let aumid = super::propvariant_string(&propvar);
        unsafe { PropVariantClear(&mut propvar).expect("clear propvariant") };
        (PathBuf::from(target), aumid)
    })();

    unsafe { CoUninitialize() };
    result
}
