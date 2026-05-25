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
        capture_screen_png()
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