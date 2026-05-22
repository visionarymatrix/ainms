use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PopupType {
    Toast,
    ModalExplain,
    SoftBlock,
}

pub struct PopupManager;

impl PopupManager {
    pub fn new() -> Self {
        println!("TODO: implement PopupManager::new");
        PopupManager
    }
}