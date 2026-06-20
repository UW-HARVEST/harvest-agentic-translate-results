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
       static LOGGER: std::sync::OnceLock<Arc<Logger>> = std::sync::OnceLock::new();

       LOGGER
           .get_or_init(|| {
               let log_info_table = HashMap::from([
                   (
                       LogType::Log,
                       LogInfo {
                           priority: 0,
                           prefix: "log log:\t",
                           suffix: "\n",
                       },
                   ),
                   (
                       LogType::UnitTest,
                       LogInfo {
                           priority: 0,
                           prefix: "unit testing:\t",
                           suffix: "\n",
                       },
                   ),
                   (
                       LogType::OpenHash,
                       LogInfo {
                           priority: 0,
                           prefix: "open hash table:\t",
                           suffix: "\n",
                       },
                   ),
                   (
                       LogType::CuckooHash,
                       LogInfo {
                           priority: 0,
                           prefix: "cuckoo hash table:\t",
                           suffix: "\n",
                       },
                   ),
               ]);

               Arc::new(Logger {
                   log_info_table: RwLock::new(log_info_table),
               })
           })
           .clone()
    }
    pub fn insert_log(&self, log_type: LogType, priority: u8, format: fmt::Arguments) {
        debug_assert!(priority <= 10);

        let table = self.log_info_table.read().expect("logger lock poisoned");
        if let Some(info) = table.get(&log_type) {
            if info.priority <= priority {
                print!("{}{}{}", info.prefix, format, info.suffix);
            }
        }
    }
    pub fn change_log_priority(&self, log_type: LogType, new_priority: u8) -> u8 {
        let mut table = self.log_info_table.write().expect("logger lock poisoned");
        let info = table.get_mut(&log_type).expect("unknown log type");
        let old_priority = info.priority;
        info.priority = new_priority;
        old_priority
    }
}
