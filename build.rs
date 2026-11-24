// Embeds the app icon into Klar.exe on Windows. Does nothing on other systems.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon/klar.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/klar.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
