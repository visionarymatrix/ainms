use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandMessage {
    Screenshot,
    PolicyUpdate,
    Uninstall,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventMessage {
    Usage(agent_proto::events::AppUsageEvent),
    Popup(agent_proto::events::PopupEvent),
    Priority(agent_proto::events::PriorityEvent),
    Tamper(agent_proto::events::TamperEvent),
}

pub fn create_channels() -> (
    tokio::sync::mpsc::Sender<CommandMessage>,
    tokio::sync::mpsc::Receiver<CommandMessage>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel::<CommandMessage>(256);
    (tx, rx)
}