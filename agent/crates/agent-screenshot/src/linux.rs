use std::process::Command;
use tracing::{info, warn};

enum CaptureMethod {
    Portal,
    Scrot,
    Import,
    Grim,
    GnomeScreenshot,
    None,
}

pub struct ScreenshotCommander {
    method: CaptureMethod,
}

impl ScreenshotCommander {
    pub fn new() -> Self {
        let method = Self::detect_method();
        match &method {
            CaptureMethod::Portal => info!("screenshot: using xdg-desktop-portal"),
            CaptureMethod::Scrot => info!("screenshot: using scrot"),
            CaptureMethod::Import => info!("screenshot: using ImageMagick import"),
            CaptureMethod::Grim => info!("screenshot: using grim"),
            CaptureMethod::GnomeScreenshot => info!("screenshot: using gnome-screenshot"),
            CaptureMethod::None => warn!("screenshot: no capture tool available"),
        }
        ScreenshotCommander { method }
    }

    fn detect_method() -> CaptureMethod {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return CaptureMethod::Portal;
        }
        if Self::which("scrot") {
            return CaptureMethod::Scrot;
        }
        if Self::which("import") {
            return CaptureMethod::Import;
        }
        if Self::which("grim") {
            return CaptureMethod::Grim;
        }
        if Self::which("gnome-screenshot") {
            return CaptureMethod::GnomeScreenshot;
        }
        if std::env::var("DISPLAY").is_ok() {
            return CaptureMethod::Portal;
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
            CaptureMethod::None => anyhow::bail!(
                "No screenshot method available. Install one of: scrot, imagemagick, grim, gnome-screenshot"
            ),
            CaptureMethod::Portal => self.capture_with_portal().await,
            CaptureMethod::Scrot => self.capture_with_scrot(),
            CaptureMethod::Import => self.capture_with_import(),
            CaptureMethod::Grim => self.capture_with_grim(),
            CaptureMethod::GnomeScreenshot => self.capture_with_gnome(),
        }
    }

    async fn capture_with_portal(&self) -> anyhow::Result<Vec<u8>> {
        use ashpd::desktop::screenshot::Screenshot;

        let response = Screenshot::request()
            .interactive(false)
            .send()
            .await?
            .response()?;

        let uri = response.uri();
        let path = uri.to_file_path()
            .map_err(|_| anyhow::anyhow!("Invalid screenshot URI path"))?;

        let data = std::fs::read(&path)?;
        let _ = std::fs::remove_file(&path);
        Ok(data)
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

    fn capture_with_grim(&self) -> anyhow::Result<Vec<u8>> {
        let tmp = Self::tmp_path();
        let status = Command::new("grim")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            anyhow::bail!("grim failed with exit code {:?}", status.code());
        }
        Self::read_and_delete(&tmp)
    }

    fn capture_with_gnome(&self) -> anyhow::Result<Vec<u8>> {
        let tmp = Self::tmp_path();
        let status = Command::new("gnome-screenshot")
            .arg("-f")
            .arg(&tmp)
            .status()?;
        if !status.success() {
            anyhow::bail!("gnome-screenshot failed with exit code {:?}", status.code());
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
}