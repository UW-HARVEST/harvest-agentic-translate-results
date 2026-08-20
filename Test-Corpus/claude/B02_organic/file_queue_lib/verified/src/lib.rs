//! Rust translation of the `driver` C shared library found in `c_src/`.
//!
//! The C build globs `src/file-queue.c`, `src/read-alert.c` and `src/driver.c`
//! into a single `libdriver.so`.  Because `include/shared.h` defines three
//! non-`static` functions in the header itself, those end up as public symbols
//! too.  The complete exported surface is:
//!
//! ```text
//! FreeAlertData  GetAlertData  Init_FileQueue  Read_FileMon
//! driver  merror  os_calloc  os_realloc  os_strdup
//! ```
//!
//! Every one of them is re-exported here with `#[unsafe(no_mangle)]` and the
//! original C signature.  None of the headers apply a namespacing macro, so the
//! linker names are identical to the source-level names.

pub mod cbits;
pub mod driver;
pub mod file_queue;
pub mod read_alert;
pub mod shared;
