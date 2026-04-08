use Graph_recogniser::log::{Logger, LogType};

#[test]
fn test_logger_new() {
    let logger = Logger::new();
    // Should not panic - logger is created successfully
    assert!(std::sync::Arc::strong_count(&logger) == 1);
}

#[test]
fn test_change_log_priority() {
    let logger = Logger::new();
    // Default priority is 0 for all types
    let old = logger.change_log_priority(LogType::Log, 5);
    assert_eq!(old, 0);
    // Now change again, should return 5
    let old = logger.change_log_priority(LogType::Log, 10);
    assert_eq!(old, 5);
}

#[test]
fn test_change_log_priority_all_types() {
    let logger = Logger::new();
    for &lt in &[LogType::Log, LogType::UnitTest, LogType::OpenHash, LogType::CuckooHash] {
        let old = logger.change_log_priority(lt, 7);
        assert_eq!(old, 0);
        let old = logger.change_log_priority(lt, 3);
        assert_eq!(old, 7);
    }
}

#[test]
fn test_insert_log_does_not_panic() {
    let logger = Logger::new();
    // Suppress output by setting high priority threshold
    logger.change_log_priority(LogType::Log, 11);
    logger.insert_log(LogType::Log, 5, format_args!("test message"));
    // Should not panic
}

#[test]
fn test_log_type_enum_values() {
    assert_eq!(LogType::Log as u8, 0);
    assert_eq!(LogType::UnitTest as u8, 1);
    assert_eq!(LogType::OpenHash as u8, 2);
    assert_eq!(LogType::CuckooHash as u8, 3);
}

fn main() {}
