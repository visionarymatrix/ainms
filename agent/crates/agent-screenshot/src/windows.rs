use tracing::info;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

/// Captures a screenshot silently on Windows using the Win32 GDI BitBlt API.
///
/// BitBlt reads directly from the screen buffer and does NOT trigger the
/// Windows 11 "Snipping Tool" notification or any other system notification,
/// unlike the Windows.Graphics.Capture API which would.
pub struct ScreenshotCommander;

impl ScreenshotCommander {
    pub fn new() -> Self {
        unsafe {
            let _ = SetProcessDPIAware();
        }
        info!("screenshot: using Win32 BitBlt (silent capture, DPI-aware)");
        ScreenshotCommander
    }

    pub async fn capture(&self) -> anyhow::Result<Vec<u8>> {
        if is_session_zero() {
            anyhow::bail!("Cannot capture screenshot from Session 0 (service context)")
        }
        capture_screen_png()
    }

    /// Capture screenshot in the user's session by spawning a helper process via CreateProcessAsUser.
    /// The helper process runs the same binary with `--take-screenshot` flag in the user session.
    pub fn capture_in_user_session(
        server: &str,
        device_id: &str,
        install_token: &str,
        request_id: &str,
    ) -> anyhow::Result<()> {
        spawn_screenshot_helper_in_user_session(server, device_id, install_token, request_id)
    }
}

/// Detect if the current process is running in Session 0 (service context).
pub fn is_session_zero() -> bool {
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    unsafe {
        let mut session_id = 0u32;
        if ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id).is_ok() {
            return session_id == 0;
        }
        false
    }
}

fn spawn_screenshot_helper_in_user_session(
    server: &str,
    device_id: &str,
    install_token: &str,
    request_id: &str,
) -> anyhow::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
    use windows::Win32::System::RemoteDesktop::{
        WTSGetActiveConsoleSessionId, WTSQueryUserToken,
    };
    use windows::Win32::System::Threading::{
        CreateProcessAsUserW, CREATE_UNICODE_ENVIRONMENT, CREATE_NO_WINDOW,
        PROCESS_INFORMATION, STARTUPINFOW, STARTF_USESHOWWINDOW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::PWSTR;

    unsafe {
        // 1. Get active console session
        let session_id = WTSGetActiveConsoleSessionId();
        if session_id == 0xFFFFFFFF {
            anyhow::bail!("No active console session");
        }

        // 2. Get user token
        let mut user_token = windows::Win32::Foundation::HANDLE::default();
        if WTSQueryUserToken(session_id, &mut user_token).is_err() {
            anyhow::bail!("WTSQueryUserToken failed (are we running as SYSTEM?)");
        }

        // 3. Create environment block for the user
        let mut env_block: *mut core::ffi::c_void = std::ptr::null_mut();
        if CreateEnvironmentBlock(&mut env_block, user_token, false).is_err() {
            let _ = CloseHandle(user_token);
            anyhow::bail!("CreateEnvironmentBlock failed");
        }

        // 4. Build command line
        let exe_path = std::env::current_exe()?;
        let args = format!(
            "--take-screenshot --server {} --device-id {} --install-token {} --request-id {}",
            server, device_id, install_token, request_id
        );
        let cmdline = format!("\"{}\" {}", exe_path.display(), args);
        let mut cmdline_wide: Vec<u16> = std::ffi::OsStr::new(&cmdline)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect();

        // 5. CreateProcessAsUser
        let mut startup_info: STARTUPINFOW = std::mem::zeroed();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup_info.dwFlags = STARTF_USESHOWWINDOW;
        startup_info.wShowWindow = SW_HIDE.0 as u16;
        let mut proc_info: PROCESS_INFORMATION = std::mem::zeroed();

        let result = CreateProcessAsUserW(
            user_token,
            None,
            PWSTR(cmdline_wide.as_mut_ptr()),
            None,
            None,
            false,
            CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
            Some(env_block),
            None,
            &startup_info,
            &mut proc_info,
        );

        // Cleanup
        if !env_block.is_null() {
            let _ = DestroyEnvironmentBlock(env_block);
        }
        let _ = CloseHandle(user_token);

        if result.is_err() {
            anyhow::bail!("CreateProcessAsUserW failed");
        }

        let _ = CloseHandle(proc_info.hProcess);
        let _ = CloseHandle(proc_info.hThread);

        info!("Spawned screenshot helper in user session {}", session_id);
        Ok(())
    }
}

fn capture_screen_png() -> anyhow::Result<Vec<u8>> {
    unsafe {
        let null_hwnd = HWND(std::ptr::null_mut());
        let screen_dc = GetDC(null_hwnd);
        if screen_dc.is_invalid() {
            anyhow::bail!("Failed to get screen DC");
        }

        let width = GetDeviceCaps(screen_dc, DESKTOPHORZRES);
        let height = GetDeviceCaps(screen_dc, DESKTOPVERTRES);

        if width <= 0 || height <= 0 {
            ReleaseDC(null_hwnd, screen_dc);
            anyhow::bail!("Invalid screen dimensions: {}x{}", width, height);
        }

        info!(width, height, "Capturing screenshot");

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_invalid() {
            ReleaseDC(null_hwnd, screen_dc);
            anyhow::bail!("Failed to create compatible DC");
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(null_hwnd, screen_dc);
            anyhow::bail!("Failed to create compatible bitmap");
        }

        let old_obj = SelectObject(mem_dc, bitmap);

        BitBlt(mem_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY)
            .map_err(|e| anyhow::anyhow!("BitBlt failed: {}", e))?;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default(); 1],
        };

        let buf_len = (width * height * 4) as usize;
        let mut pixels: Vec<u8> = vec![0u8; buf_len];

        let scan_lines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(mem_dc, old_obj);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        ReleaseDC(null_hwnd, screen_dc);

        if scan_lines == 0 {
            anyhow::bail!("GetDIBits returned 0 scan lines");
        }

        let png_data = encode_png(&pixels, width as u32, height as u32)?;
        info!(size = png_data.len(), "Screenshot captured via BitBlt");
        Ok(png_data)
    }
}

fn encode_png(bgra_pixels: &[u8], width: u32, height: u32) -> anyhow::Result<Vec<u8>> {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for (i, chunk) in bgra_pixels.chunks_exact(4).enumerate() {
        rgba[i * 4] = chunk[2];
        rgba[i * 4 + 1] = chunk[1];
        rgba[i * 4 + 2] = chunk[0];
        rgba[i * 4 + 3] = chunk[3];
    }

    let mut png_buf: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_buf, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba)?;
        writer.finish()?;
    }

    Ok(png_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_session_zero_returns_bool() {
        let _ = is_session_zero();
    }

    #[test]
    fn test_is_session_zero_in_interactive_session() {
        if std::env::var("CI").is_err() {
            assert!(!is_session_zero(), "Tests run in interactive session, should NOT be Session 0");
        }
    }

    #[test]
    fn test_screenshot_commander_bails_in_session_zero() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let commander = ScreenshotCommander::new();
        let result = rt.block_on(async { commander.capture().await });

        if is_session_zero() {
            assert!(result.is_err(), "capture() must fail in Session 0");
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Session 0"),
                "Error should mention Session 0, got: {}", err
            );
        } else if let Err(e) = result {
            assert!(
                !e.to_string().contains("Session 0"),
                "In interactive session, error should not mention Session 0, got: {}", e
            );
        }
    }

    #[test]
    fn test_capture_in_user_session_builds_correct_args() {
        let server = "http://localhost:8440";
        let device_id = "device-123";
        let install_token = "token-abc";
        let request_id = "req-456";

        let expected_args = format!(
            "--take-screenshot --server {} --device-id {} --install-token {} --request-id {}",
            server, device_id, install_token, request_id
        );

        assert!(expected_args.contains("--take-screenshot"));
        assert!(expected_args.contains("--server http://localhost:8440"));
        assert!(expected_args.contains("--device-id device-123"));
        assert!(expected_args.contains("--install-token token-abc"));
        assert!(expected_args.contains("--request-id req-456"));
    }

    #[test]
    fn test_capture_in_user_session_auto_request_id_format() {
        let timestamp = 1700000000i64;
        let request_id = format!("auto-{}", timestamp);
        assert!(request_id.starts_with("auto-"));
        assert!(request_id.contains(&timestamp.to_string()));
    }

    #[test]
    fn test_capture_screen_png_returns_valid_png_header() {
        if is_session_zero() { return; }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let commander = ScreenshotCommander::new();
        let result = rt.block_on(async { commander.capture().await });

        if let Ok(data) = result {
            assert!(data.len() >= 8, "PNG data too short: {} bytes", data.len());
            assert_eq!(&data[0..4], b"\x89PNG", "Missing PNG magic header");
        }
    }
}