//! Rust translation of the C `driver` shared library (c_src/).
//!
//! The layout mirrors the C sources one-to-one:
//!   * `logger`       <- src/logger.c       / include/logger.h
//!   * `task_manager` <- src/task_manager.c / include/task_manager.h
//!   * `driver`       <- src/driver.c
//!
//! Behaviour (including the quirks and bugs of the original) is reproduced
//! exactly: same order of checks, same messages, same return codes, same
//! `malloc` success/failure semantics, same `atoi`/`strncpy`/`%s` semantics.

pub mod cstdio;
pub mod cutil;
pub mod driver;
pub mod logger;
pub mod stdio_stream;
pub mod task_manager;
