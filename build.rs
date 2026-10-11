fn main() {
    // winresource drives rc.exe (MSVC) / windres (GNU); the target-OS gate
    // keeps non-Windows hosts free of that toolchain (icons-and-windows.md A2).
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app-icon/gpui-starter.ico");
        res.compile()
            .expect("embed assets/app-icon/gpui-starter.ico");
        println!("cargo:rerun-if-changed=assets/app-icon/gpui-starter.ico");
    }
}
