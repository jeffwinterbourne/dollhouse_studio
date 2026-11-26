fn main() {
    #[cfg(target_os = "windows")]
    {
        if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
            println!("cargo:rerun-if-changed=../../assets/icon.ico");
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../../assets/icon.ico");
            res.set("ProductName", "Dollhouse Studio");
            res.set(
                "FileDescription",
                "Dollhouse Studio - 3D Printable Miniature Architectural CAD",
            );
            res.set("LegalCopyright", "MIT License");
            if let Err(e) = res.compile() {
                panic!("Failed to compile Windows application resources: {e}");
            }
        }
    }
}
