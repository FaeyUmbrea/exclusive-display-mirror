// Tauri command for launching bootstrapper.exe and handling its output
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn run_bootstrapper_with_pid(app: AppHandle) -> Result<String, String> {
    // Find path to bootstrapper.exe next to tauri exe
    let exe_path = env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe dir")?;
    let bootstrap_path = exe_dir.join("bootstrapper.exe");
    if !bootstrap_path.exists() {
        return Err(format!(
            "bootstrapper.exe not found at {}",
            bootstrap_path.display()
        ));
    }
    // Spawn the bootstrapper without extra args; elevation is handled by the launcher when needed
    let mut child = Command::new(&bootstrap_path)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to launch bootstrapper.exe: {e}"))?;
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.contains("no update required") {
            app.emit("bootstrap_done", ()).ok();
            break;
        }
        if line.contains("waiting for parent to exit") {
            // Older bootstrapper message; emit restart if present
            app.emit("bootstrap_restart_required", ()).ok();
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok("bootstrapper.exe finished successfully".to_string())
    } else {
        Err(format!(
            "bootstrapper.exe exited with code {:?}",
            status.code()
        ))
    }
}

#[tauri::command]
pub fn check_bootstrap_manifest() -> Result<bool, String> {
    let exe_path = env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe dir")?;
    let manifest_path = exe_dir.join("bootstrap_manifest.json");
    Ok(manifest_path.exists())
}

// Launch the bundled bootstrapper (same as run_bootstrapper but fire-and-forget)
fn dir_writable(dir: &std::path::Path) -> bool {
    // Try to create and remove a temporary file in the directory
    use std::io::Write;
    let test_path = dir.join(".__edm_write_test");
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&test_path)
    {
        Ok(mut f) => {
            let _ = f.write_all(b"test");
            let _ = std::fs::remove_file(&test_path);
            true
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub fn launch_bootstrapper_downloader() -> Result<(), String> {
    let exe_path = env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = exe_path.parent().ok_or("Failed to get exe dir")?;
    let bootstrap_path = exe_dir.join("bootstrapper.exe");
    if !bootstrap_path.exists() {
        return Err(format!(
            "bootstrapper.exe not found at {}",
            bootstrap_path.display()
        ));
    }

    // If the executable directory is not writable, attempt to launch bootstrapper elevated
    if !dir_writable(exe_dir) {
        #[cfg(target_os = "windows")]
        {
            // Use the same elevation pattern as bootstrapper/src/elevate.rs
            use std::env;
            use windows::core::w;
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::Shell::ShellExecuteW;
            use windows::Win32::UI::WindowsAndMessaging::SHOW_WINDOW_CMD;

            let exe_path_str = bootstrap_path
                .to_str()
                .ok_or_else(|| "Bootstrapper path is not valid UTF-8".to_string())?;

            // Collect command-line arguments (excluding program name) from the current process
            let args: Vec<String> = env::args().skip(1).collect();
            let args_string = args.join(" ");

            unsafe {
                let result = ShellExecuteW(
                    Option::from(HWND::default()),
                    w!("runas"),
                    &windows::core::HSTRING::from(exe_path_str),
                    &windows::core::HSTRING::from(args_string.as_str()),
                    &windows::core::HSTRING::default(),
                    SHOW_WINDOW_CMD(5), // SW_SHOW
                );

                if result.0 as usize > 32 {
                    return Ok(());
                } else {
                    return Err(format!("ShellExecuteW failed with code: {:?}", result.0));
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            // On unix, try pkexec or sudo
            if which::which("pkexec").is_ok() {
                let mut c = Command::new("pkexec");
                c.arg(bootstrap_path);
                let s = c.spawn();
                return s.map(|_| ()).map_err(|e| e.to_string());
            } else {
                let mut c = Command::new("sudo");
                c.arg(bootstrap_path);
                let s = c.spawn();
                return s.map(|_| ()).map_err(|e| e.to_string());
            }
        }
    }

    // Otherwise, launch normally
    let spawn = Command::new(&bootstrap_path).spawn();
    spawn.map_err(|e| format!("Failed to launch bootstrapper: {e}"))?;
    Ok(())
}
