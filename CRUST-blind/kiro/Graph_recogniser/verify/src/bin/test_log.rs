use Graph_recogniser::log::{Logger, LogType};

#[test]
fn test_change_log_priority_returns_old() {
    let logger = Logger::new();
    // Default priority is 0
    let old = logger.change_log_priority(LogType::Log, 5);
    assert_eq!(old, 0);
    let old = logger.change_log_priority(LogType::Log, 3);
    assert_eq!(old, 5);
}

#[test]
fn test_change_log_priority_all_types() {
    let logger = Logger::new();
    assert_eq!(logger.change_log_priority(LogType::UnitTest, 10), 0);
    assert_eq!(logger.change_log_priority(LogType::OpenHash, 7), 0);
    assert_eq!(logger.change_log_priority(LogType::CuckooHash, 2), 0);
}

#[test]
fn test_change_log_priority_roundtrip() {
    let logger = Logger::new();
    let old = logger.change_log_priority(LogType::Log, 8);
    assert_eq!(old, 0);
    let old = logger.change_log_priority(LogType::Log, 0);
    assert_eq!(old, 8);
}

#[test]
fn test_insert_log_does_not_panic() {
    let logger = Logger::new();
    // Suppress output by setting high priority threshold
    logger.change_log_priority(LogType::Log, 11);
    logger.insert_log(LogType::Log, 5, format_args!("test message"));
}

fn main() {}
