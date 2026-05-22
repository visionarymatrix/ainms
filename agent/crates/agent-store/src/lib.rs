use agent_proto::events::AppUsageEvent;

pub struct Store;

impl Store {
    pub fn new() -> Self {
        // TODO: implement encrypted SQLite store
        Store
    }

    pub async fn insert_event(&self, _event: &AppUsageEvent) -> anyhow::Result<()> {
        // TODO: implement insert_event
        println!("TODO: implement Store::insert_event");
        Ok(())
    }

    pub async fn get_pending_bulk(&self) -> anyhow::Result<Vec<AppUsageEvent>> {
        // TODO: implement get_pending_bulk
        println!("TODO: implement Store::get_pending_bulk");
        Ok(vec![])
    }

    pub async fn get_pending_priority(&self) -> anyhow::Result<Vec<AppUsageEvent>> {
        // TODO: implement get_pending_priority
        println!("TODO: implement Store::get_pending_priority");
        Ok(vec![])
    }

    pub async fn mark_uploaded(&self, _ids: &[uuid::Uuid]) -> anyhow::Result<()> {
        // TODO: implement mark_uploaded
        println!("TODO: implement Store::mark_uploaded");
        Ok(())
    }
}