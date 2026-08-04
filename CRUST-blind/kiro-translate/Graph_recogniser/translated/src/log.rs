use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::fmt;
use crate::check;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogType {
    Log = 0,
    UnitTest = 1,
    OpenHash = 2,
    CuckooHash = 3,
}
pub struct LogInfo {
    priority: u8,
    prefix: &'static str,
    suffix: &'static str,
}
pub struct Logger {
    log_info_table: RwLock<HashMap<LogType, LogInfo>>,
}
impl Logger {
    pub fn new() -> Arc<Self> {
        let mut table = HashMap::new();
        table.insert(LogType::Log, LogInfo { priority: 0, prefix: "log log:\t", suffix: "\n" });
        table.insert(LogType::UnitTest, LogInfo { priority: 0, prefix: "unit testing:\t", suffix: "\n" });
        table.insert(LogType::OpenHash, LogInfo { priority: 0, prefix: "open hash table:\t", suffix: "\n" });
        table.insert(LogType::CuckooHash, LogInfo { priority: 0, prefix: "cuckoo hash table:\t", suffix: "\n" });
        Arc::new(Self { log_info_table: RwLock::new(table) })
    }
    pub fn insert_log(&self, log_type: LogType, priority: u8, format: fmt::Arguments) {
        check!(priority <= 10);
        let table = self.log_info_table.read().unwrap();
        let info = table.get(&log_type).unwrap();
        if info.priority <= priority {
            print!("{}{}{}", info.prefix, format, info.suffix);
        }
    }
    pub fn change_log_priority(&self, log_type: LogType, new_priority: u8) -> u8 {
        let mut table = self.log_info_table.write().unwrap();
        let info = table.get_mut(&log_type).unwrap();
        let old = info.priority;
        info.priority = new_priority;
        old
    }
}
