pub struct Watchdog;

impl Watchdog {
    pub fn new() -> Self {
        // TODO: implement watchdog initialization
        Watchdog
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        // TODO: implement watchdog start
        println!("TODO: implement Watchdog::start");
        Ok(())
    }

    pub async fn monitor(&self) -> anyhow::Result<()> {
        // TODO: implement process monitoring
        println!("TODO: implement Watchdog::monitor");
        Ok(())
    }
}