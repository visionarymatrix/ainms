pub struct ScreenshotCommander;

impl ScreenshotCommander {
    pub fn new() -> Self {
        // TODO: implement screenshot commander initialization
        ScreenshotCommander
    }

    pub async fn capture(&self) -> anyhow::Result<Vec<u8>> {
        // TODO: implement screenshot capture
        println!("TODO: implement ScreenshotCommander::capture");
        Ok(vec![])
    }

    pub async fn classify_and_upload(&self) -> anyhow::Result<()> {
        // TODO: implement classify and upload
        println!("TODO: implement ScreenshotCommander::classify_and_upload");
        Ok(())
    }
}