fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "TUI Runner");
        res.set("FileDescription", "TUI Runner Application");
        res.set("LegalCopyright", "Copyright (c) 2024");
        res.set("InternalName", "tui-runner.exe");
        res.compile().unwrap();
    }
}
