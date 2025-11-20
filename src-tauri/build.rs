fn main() {
    #[cfg(all(target_os = "windows", target_env = "msvc"))]
    {
        // Defer loading of obs.dll until first function call
        println!("cargo:rustc-link-arg=/DELAYLOAD:obs.dll");

        // Link against the delay-load helper library (required)
        println!("cargo:rustc-link-lib=delayimp");
    }
    // Preserve existing tauri build behavior
    tauri_build::build();
}
