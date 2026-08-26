# Error Surface

This table is derived from every failure return, null check, and capacity check
in `c_src/src/*.c`. There are no assertions, public enums, explicit numeric
range checks, or error-return macros.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `initialize_logger` | `fopen(LOG_FILE or "default.log", "a")` returns `NULL` | returns `-1` and writes `Failed to open log file: <path>\n` to `stderr` [x] |
| 2 | `create_task_manager` | first `malloc(sizeof(TaskManager))` returns `NULL` | logs `Failed to allocate memory for TaskManager.`, returns `NULL` [x] |
| 3 | `create_task_manager` | `malloc(max_tasks * sizeof(Task))` returns `NULL` | logs `Failed to allocate memory for tasks.`, frees the manager, returns `NULL` [x] |
| 4 | `add_task` | `manager->task_count >= manager->max_tasks` | logs warning, returns without changing the manager [x] |
| 5 | `driver` | `initialize_logger()` returns nonzero | returns `EXIT_FAILURE` (`1`) immediately [x] |
| 6 | `driver` | `create_task_manager()` returns `NULL` | returns `EXIT_FAILURE` (`1`) without finalizing the logger [x] |
| 7 | `driver` | per-line `malloc(length + 1)` returns `NULL` | writes allocation error to `stderr`, destroys manager, finalizes logger, returns `EXIT_FAILURE` (`1`) [x] |

## FFI Boundary Preconditions

The C API does not reject null pointers for `add_task`, `print_tasks`,
`destroy_task_manager`, or `driver`; it dereferences them and therefore has
undefined behavior. A null `message` is ignored while the logger is closed and
is passed to glibc `fprintf("%s")` while open. Tests exercise these generic
boundaries in isolated subprocesses and compare observable C/Rust outcomes,
without treating undefined behavior as a portable API contract.

Lengths are implicit NUL-terminated-string lengths. Zero-length task input and
descriptions, descriptions around the 255-byte copy boundary, large
descriptions, zero capacity, and capacity plus one are covered in
`CONFIGS.md`. There are no enum parameters to test out of range.
