use Graph_recogniser::log::{Logger, LogType};

#[test]
fn test_logger_default_priority_is_zero() {
    let logger = Logger::new();
    let old = logger.change_log_priority(LogType::Log, 5);
    assert_eq!(old, 0);
    let old2 = logger.change_log_priority(LogType::UnitTest, 5);
    assert_eq!(old2, 0);
    let old3 = logger.change_log_priority(LogType::OpenHash, 5);
    assert_eq!(old3, 0);
    let old4 = logger.change_log_priority(LogType::CuckooHash, 5);
    assert_eq!(old4, 0);
}

#[test]
fn test_logger_change_priority_round_trip() {
    let logger = Logger::new();
    let prev = logger.change_log_priority(LogType::OpenHash, 7);
    assert_eq!(prev, 0);
    let prev2 = logger.change_log_priority(LogType::OpenHash, 11);
    assert_eq!(prev2, 7);
    let prev3 = logger.change_log_priority(LogType::OpenHash, 0);
    assert_eq!(prev3, 11);
}

#[test]
fn test_log_type_discriminants() {
    assert_eq!(LogType::Log as u8, 0);
    assert_eq!(LogType::UnitTest as u8, 1);
    assert_eq!(LogType::OpenHash as u8, 2);
    assert_eq!(LogType::CuckooHash as u8, 3);
}

#[test]
fn test_insert_log_does_not_panic() {
    let logger = Logger::new();
    // Suppress all output by raising priorities above 10.
    logger.change_log_priority(LogType::Log, 11);
    logger.change_log_priority(LogType::UnitTest, 11);
    logger.change_log_priority(LogType::OpenHash, 11);
    logger.change_log_priority(LogType::CuckooHash, 11);
    logger.insert_log(LogType::Log, 5, format_args!("hello {}", "world"));
    logger.insert_log(LogType::OpenHash, 0, format_args!("debug message"));
}

fn main() {}
