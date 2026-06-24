fn main() {
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("bluearchivelogo.ico");
        resource.set("ProductName", "BluePresence");
        resource.set("FileDescription", "Blue Archive Discord Rich Presence");
        resource.set("LegalCopyright", "© 2026 Y2KDevelopment. All rights reserved.");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
