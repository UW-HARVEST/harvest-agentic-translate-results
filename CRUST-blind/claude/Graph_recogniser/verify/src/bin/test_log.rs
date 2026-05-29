use Graph_recogniser::log::{LogType, Logger};

#[test]
fn test_logger_new_returns_arc() {
    let logger = Logger::new();
    // Should be usable as an Arc.
    let _clone = logger.clone();
}

#[test]
fn test_change_log_priority_returns_old_value() {
    // C: each log type starts with priority 0.
    let logger = Logger::new();
    let old0 = logger.change_log_priority(LogType::Log, 5);
    assert_eq!(old0, 0);
    let old1 = logger.change_log_priority(LogType::Log, 7);
    assert_eq!(old1, 5);
    let old2 = logger.change_log_priority(LogType::Log, 0);
    assert_eq!(old2, 7);
}

#[test]
fn test_change_log_priority_unit_test() {
    let logger = Logger::new();
    assert_eq!(logger.change_log_priority(LogType::UnitTest, 11), 0);
    assert_eq!(logger.change_log_priority(LogType::UnitTest, 3), 11);
}

#[test]
fn test_change_log_priority_open_hash() {
    let logger = Logger::new();
    assert_eq!(logger.change_log_priority(LogType::OpenHash, 11), 0);
    assert_eq!(logger.change_log_priority(LogType::OpenHash, 9), 11);
}

#[test]
fn test_change_log_priority_cuckoo_hash() {
    let logger = Logger::new();
    assert_eq!(logger.change_log_priority(LogType::CuckooHash, 11), 0);
    assert_eq!(logger.change_log_priority(LogType::CuckooHash, 1), 11);
}

#[test]
fn test_logtype_discriminants() {
    // C macros: LOG_LOG=0, UNIT_TEST_LOG=1, OPEN_HASH_LOG=2, CUCKOO_HASH_LOG=3
    assert_eq!(LogType::Log as u32, 0);
    assert_eq!(LogType::UnitTest as u32, 1);
    assert_eq!(LogType::OpenHash as u32, 2);
    assert_eq!(LogType::CuckooHash as u32, 3);
}

#[test]
fn test_insert_log_does_not_panic_below_threshold() {
    let logger = Logger::new();
    // Bump priority high so message is filtered out (no print collision).
    logger.change_log_priority(LogType::Log, 11);
    logger.insert_log(LogType::Log, 5, format_args!("hidden message: {}", 42));
}

#[test]
fn test_insert_log_does_not_panic_above_threshold() {
    let logger = Logger::new();
    // Bump priority high then back down so an emit can happen
    logger.change_log_priority(LogType::Log, 11);
    logger.change_log_priority(LogType::Log, 0);
    logger.insert_log(LogType::Log, 0, format_args!("test message"));
}

#[test]
fn test_change_log_priority_independent_per_type() {
    let logger = Logger::new();
    // Modifying one log type does not affect others.
    logger.change_log_priority(LogType::Log, 10);
    let old_other = logger.change_log_priority(LogType::UnitTest, 4);
    assert_eq!(old_other, 0);
    let still_log = logger.change_log_priority(LogType::Log, 1);
    assert_eq!(still_log, 10);
}

fn main() {}
