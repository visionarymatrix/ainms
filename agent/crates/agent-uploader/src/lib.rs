pub struct Uploader;

impl Uploader {
    pub fn new() -> Self {
        // TODO: implement uploader initialization
        Uploader
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        // TODO: implement dual-channel upload loop
        println!("TODO: implement Uploader::start");
        Ok(())
    }

    pub async fn upload_priority(&self) -> anyhow::Result<()> {
        // TODO: implement priority upload
        println!("TODO: implement Uploader::upload_priority");
        Ok(())
    }

    pub async fn upload_bulk(&self) -> anyhow::Result<()> {
        // TODO: implement bulk upload
        println!("TODO: implement Uploader::upload_bulk");
        Ok(())
    }
}