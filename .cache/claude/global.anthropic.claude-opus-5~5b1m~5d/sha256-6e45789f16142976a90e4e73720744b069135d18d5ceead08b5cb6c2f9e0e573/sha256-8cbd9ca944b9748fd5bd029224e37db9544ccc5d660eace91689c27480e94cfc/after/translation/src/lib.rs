//! Rust translation of the C `driver` shared library found in `c_src/`.
//!
//! The C build globs `src/task_manager.c`, `src/logger.c` and `src/driver.c`
//! into a single shared object exporting ten public symbols:
//!
//! * `logger.h`       — `initialize_logger`, `log_info`, `log_warning`,
//!                      `log_error`, `finalize_logger`
//! * `task_manager.h` — `create_task_manager`, `add_task`, `print_tasks`,
//!                      `destroy_task_manager`
//! * `driver.c`       — `driver`
//!
//! All of them are re-exported here with the identical C ABI, and every
//! observable behaviour (including the original's quirks) is preserved.

pub mod cstd;
pub mod driver;
pub mod logger;
pub mod task_manager;
