//! Rust translation of the C library in `c_src/`.
//!
//! Exported ABI (matches `nm -D` of the CMake-built `libdriver.so`):
//!
//! | symbol                 | source file            |
//! |------------------------|------------------------|
//! | `initialize_logger`    | `src/logger.c`         |
//! | `log_info`             | `src/logger.c`         |
//! | `log_warning`          | `src/logger.c`         |
//! | `log_error`            | `src/logger.c`         |
//! | `finalize_logger`      | `src/logger.c`         |
//! | `create_task_manager`  | `src/task_manager.c`   |
//! | `add_task`             | `src/task_manager.c`   |
//! | `print_tasks`          | `src/task_manager.c`   |
//! | `destroy_task_manager` | `src/task_manager.c`   |
//! | `driver`               | `src/driver.c`         |
//!
//! No header in `c_src/include/` uses a namespace/renaming macro, so the
//! linker names are identical to the source-level names.

pub mod cffi;
pub mod driver;
pub mod logger;
pub mod task_manager;
