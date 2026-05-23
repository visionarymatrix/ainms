use std::process::Command;
use tracing::{info, warn};

enum CaptureMethod {
    Scrot,
    Import,
    XdgScreenshot,
    None,
}

pub struct ScreenshotCommander {
    method: CaptureMethod,
}

impl ScreenshotCommander {
    pub fn new() -> Self {
        let method = Self::detect_method();
        match &method {
            CaptureMethod::Scrot => info!("screenshot: using scrot"),
            CaptureMethod::Import => info!("screenshot: using ImageMagick import"),
            CaptureMethod::XdgScreenshot => info!("screenshot: using xdg-screencapture"),
            CaptureMethod::None => warn!("screenshot: no capture tool available"),
        }
        ScreenshotCommander { method }
    }

    fn detect_method() -> CaptureMethod {
        if Self::which("scrot") {
            return CaptureMethod::Scrot;
        }
        if Self::which("import") {
            return CaptureMethod::Import;
        }
        if Self::which("xdg-screencapture") {
            return CaptureMethod::XdgScreenshot;
        }
        CaptureMethod::None
    }

    fn which(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn capture(&self) -> anyhow::Result<Vec<u8>> {
        if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
            anyhow::bail!("No display available for screenshot");
        }

        match &self.method {
            CaptureMethod::None => anyhow::bail!("No screenshot tool available"),
            CaptureMethod::Scrot => self.capture_with_scrot(),
            CaptureMethod::Import => self.capture_with_import(),
            CaptureMethod::XdgScreenshot => self.capture_with_xdg(),
        }
    }

    fn capture_with_scrot(&self) -> anyhow::Result<Vec<u8>> {
        let tmp = Self::tmp_path();
        let status = Command::new("scrot")
            .arg("-z")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            anyhow::bail!("scrot failed with exit code {:?}", status.code());
        }
        Self::read_and_delete(&tmp)
    }

    fn capture_with_import(&self) -> anyhow::Result<Vec<u8>> {
        let tmp = Self::tmp_path();
        let status = Command::new("import")
            .arg("-window")
            .arg("root")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            anyhow::bail!("import failed with exit code {:?}", status.code());
        }
        Self::read_and_delete(&tmp)
    }

    fn capture_with_xdg(&self) -> anyhow::Result<Vec<u8>> {
        let tmp = Self::tmp_path();
        let status = Command::new("xdg-screencapture")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            anyhow::bail!("xdg-screencapture failed with exit code {:?}", status.code());
        }
        Self::read_and_delete(&tmp)
    }

    fn tmp_path() -> String {
        format!("/tmp/ainms_screenshot_{}.png", std::process::id())
    }

    fn read_and_delete(path: &str) -> anyhow::Result<Vec<u8>> {
        let data = std::fs::read(path)?;
        let _ = std::fs::remove_file(path);
        Ok(data)
    }

    pub async fn classify_and_upload(&self) -> anyhow::Result<()> {
        let data = self.capture().await?;
        let size = data.len();
        tracing::info!("screenshot captured, {} bytes", size);
        // Upload to backend will be wired later
        Ok(())
    }
}