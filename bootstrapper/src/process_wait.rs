#[cfg(unix)]
pub fn wait_for_pid(pid: u32) -> anyhow::Result<()> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    use std::thread::sleep;
    use std::time::Duration;
    while kill(Pid::from_raw(pid as i32), None).is_ok() {
        sleep(Duration::from_millis(500));
    }
    Ok(())
}

#[cfg(windows)]
pub fn wait_for_pid(pid: u32) -> anyhow::Result<()> {
    use windows::Win32::Foundation::WAIT_OBJECT_0;
    use windows::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, INFINITE, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE,
    };

    unsafe {
        // Request SYNCHRONIZE access so WaitForSingleObject can wait on the process handle.
        let desired_access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE;
        let handle = match OpenProcess(desired_access, false, pid) {
            Ok(h) => h,
            Err(e) => {
                // On Windows, trying to open a non-existent or invalid PID can return
                // ERROR_INVALID_PARAMETER (HRESULT 0x80070057). Treat that as "no process to wait for".
                let hr = e.code().0;
                const HR_ERROR_INVALID_PARAMETER: i32 = 0x80070057u32 as i32;
                if hr == HR_ERROR_INVALID_PARAMETER || hr == (ERROR_INVALID_PARAMETER.0 as i32) {
                    return Ok(());
                }
                return Err(e.into());
            }
        };

        let wait_result = WaitForSingleObject(handle, INFINITE);

        if wait_result != WAIT_OBJECT_0 {
            CloseHandle(handle)?;
            anyhow::bail!(
                "WaitForSingleObject returned unexpected value: {:?}",
                wait_result
            );
        }

        CloseHandle(handle)?;
    }

    Ok(())
}
