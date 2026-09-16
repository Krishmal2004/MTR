fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "TUI Runner");
        res.set("FileDescription", "TUI Runner - developer project scaffolder and process runner");
        res.set("LegalCopyright", "Copyright (c) 2024 Krishmal Dinidu");
        res.set("CompanyName", "Krishmal Dinidu");
        res.set("InternalName", "tui-runner.exe");
        res.set("OriginalFilename", "tui-runner.exe");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.compile().unwrap();
    }
}
