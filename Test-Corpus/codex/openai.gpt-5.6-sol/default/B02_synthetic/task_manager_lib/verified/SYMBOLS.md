# Dynamic Symbol Surface

Source: `nm -D --defined-only ../c_src/build/libdriver.so`, built from the
unmodified C source.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `add_task` | `T` | `add_task` | [x] |
| 2 | `create_task_manager` | `T` | `create_task_manager` | [x] |
| 3 | `destroy_task_manager` | `T` | `destroy_task_manager` | [x] |
| 4 | `driver` | `T` | `driver` | [x] |
| 5 | `finalize_logger` | `T` | `finalize_logger` | [x] |
| 6 | `initialize_logger` | `T` | `initialize_logger` | [x] |
| 7 | `log_error` | `T` | `log_error` | [x] |
| 8 | `log_info` | `T` | `log_info` | [x] |
| 9 | `log_warning` | `T` | `log_warning` | [x] |
| 10 | `print_tasks` | `T` | `print_tasks` | [x] |

Missing C-defined symbols in Rust: **0**.

The C library's remaining dynamic symbols are undefined libc/toolchain
imports, not public symbols defined by this library.
