use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

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

        Arc::new(Self {
            log_info_table: RwLock::new(log_info_table),
        })
    }

    pub fn insert_log(&self, log_type: LogType, priority: u8, format: fmt::Arguments) {
        assert!(priority <= 10);

        let table = self.log_info_table.read().unwrap();
        let cur_log_info = table.get(&log_type).expect("missing log type");
        if cur_log_info.priority <= priority {
            print!("{}{}{}", cur_log_info.prefix, format, cur_log_info.suffix);
        }
    }

    pub fn change_log_priority(&self, log_type: LogType, new_priority: u8) -> u8 {
        let mut table = self.log_info_table.write().unwrap();
        let cur_log_info = table.get_mut(&log_type).expect("missing log type");
        let old_priority = cur_log_info.priority;
        cur_log_info.priority = new_priority;
        old_priority
    }
}
