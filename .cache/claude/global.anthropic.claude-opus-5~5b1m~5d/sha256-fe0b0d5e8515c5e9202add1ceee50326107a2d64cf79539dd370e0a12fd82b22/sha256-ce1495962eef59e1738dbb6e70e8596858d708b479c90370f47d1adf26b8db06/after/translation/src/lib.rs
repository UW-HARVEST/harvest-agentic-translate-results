//! Rust translation of the `driver` C shared library.
//!
//! Sources translated (1:1, file for file):
//!   * `c_src/include/shared.h`      -> `shared.rs`
//!   * `c_src/include/read-alert.h`  -> `read_alert.rs`
//!   * `c_src/include/file-queue.h`  -> `file_queue.rs`
//!   * `c_src/src/read-alert.c`      -> `read_alert.rs`
//!   * `c_src/src/file-queue.c`      -> `file_queue.rs`
//!   * `c_src/src/driver.c`          -> `driver.rs`
//!
//! Exported ABI (matches `nm -D` on the C `libdriver.so`):
//!   merror, Init_FileQueue, Read_FileMon,
//!   os_calloc, os_realloc, os_strdup,
//!   FreeAlertData, GetAlertData, driver

#![allow(clippy::missing_safety_doc)]

pub mod cbind;
#[macro_use]
pub mod shared;
pub mod driver;
pub mod file_queue;
pub mod read_alert;
