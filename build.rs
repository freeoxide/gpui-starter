fn main() {
    // #[cfg(windows)] sees the host and pairs with the target-gated build-dep
    // (icons-and-windows.md A2); a Windows target still needs rc.exe/windres.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app-icon/gpui-starter.ico");
        res.compile()
            .expect("embed assets/app-icon/gpui-starter.ico");
        println!("cargo:rerun-if-changed=assets/app-icon/gpui-starter.ico");
    }
}
