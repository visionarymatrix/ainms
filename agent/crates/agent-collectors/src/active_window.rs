#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub title: String,
    pub process_name: String,
    pub process_id: i32,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: i32,
    pub cmdline: String,
    pub cpu_percent: f64,
    pub memory_kb: u64,
    pub is_user_facing: bool,
}