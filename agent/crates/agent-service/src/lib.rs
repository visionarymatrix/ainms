pub mod platform;

pub struct AgentService;

impl AgentService {
    pub fn new() -> Self {
        // TODO: implement service initialization
        AgentService
    }

    pub async fn install(&self) -> anyhow::Result<()> {
        // TODO: implement service install
        println!("TODO: implement AgentService::install");
        Ok(())
    }

    pub async fn uninstall(&self) -> anyhow::Result<()> {
        // TODO: implement service uninstall
        println!("TODO: implement AgentService::uninstall");
        Ok(())
    }
}