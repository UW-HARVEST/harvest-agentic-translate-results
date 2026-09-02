/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Rust translation of the `driver` C shared library (`c_src/`).
//!
//! Exported ABI (matches `nm -D libdriver.so` of the C build):
//!
//! * `task_manager.h`: `create_task_manager`, `add_task`, `print_tasks`,
//!   `destroy_task_manager`
//! * `logger.h`: `initialize_logger`, `log_info`, `log_warning`, `log_error`,
//!   `finalize_logger`
//! * `driver.c`: `driver`
//!
//! Behaviour - including the original's quirks (unchecked `MAX_TASKS`,
//! `NULL`-unsafe `add_task`/`print_tasks`/`destroy_task_manager`, the
//! non-reset `log_file` handle) - is reproduced as-is rather than corrected.

pub mod cbind;
pub mod driver;
pub mod logger;
pub mod task_manager;
