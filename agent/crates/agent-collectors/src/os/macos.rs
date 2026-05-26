// TODO: implement macos-specific collector

use crate::active_window::{ActiveWindow, ProcessInfo};
use agent_proto::events::NetworkConnection;

pub fn get_active_window() -> Option<ActiveWindow> {
    todo!()
}

pub fn get_idle_seconds() -> f64 {
    todo!()
}

pub fn get_running_applications() -> Vec<ProcessInfo> {
    Vec::new()
}

pub fn get_all_running_applications() -> Vec<ProcessInfo> {
    Vec::new()
}

pub fn get_network_connections() -> Vec<NetworkConnection> {
    Vec::new()
}