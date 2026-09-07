# Error surface

Rows 1–7 are the explicit rejection branches mechanically found in the C
sources. Rows 8–13 record generic invalid FFI inputs required by Phase C; the
C API contains no length parameters or enum parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `initialize_logger` | `fopen(LOG_FILE or "default.log", "a") == NULL` | prints `Failed to open log file: <path>` to stderr and returns `-1` | [x] |
| 2 | `create_task_manager` | first `malloc(sizeof(TaskManager)) == NULL` | logs `Failed to allocate memory for TaskManager.` when a logger is open and returns `NULL` | [x] |
| 3 | `create_task_manager` | `malloc(manager->max_tasks * sizeof(Task)) == NULL` after the manager allocation succeeds | logs `Failed to allocate memory for tasks.`, frees the manager, and returns `NULL` | [x] |
| 4 | `add_task` | `manager->task_count >= manager->max_tasks` | logs `Cannot add task: Maximum task limit reached.` and returns without changing the manager | [x] |
| 5 | `driver` | `initialize_logger() != 0` | returns `EXIT_FAILURE` (`1`) | [x] |
| 6 | `driver` | `create_task_manager() == NULL` | returns `EXIT_FAILURE` (`1`) | [x] |
| 7 | `driver` | per-line `malloc(length + 1) == NULL` | prints `Error: Failed to allocate memory for task.` to stderr, destroys the manager, finalizes the logger, and returns `EXIT_FAILURE` (`1`) | [x] |
| 8 | `add_task` | `manager == NULL` | no C guard; process terminates with the same signal in isolated C/Rust calls | [x] |
| 9 | `add_task` | `description == NULL` while capacity remains | no C guard; process terminates with the same signal in isolated C/Rust calls | [x] |
| 10 | `print_tasks` | `manager == NULL` | no C guard; process termination/stdout behavior must match in isolated calls | [x] |
| 11 | `destroy_task_manager` | `manager == NULL` | no C guard; process terminates with the same signal in isolated C/Rust calls | [x] |
| 12 | `driver` | `tasks == NULL` after logger/manager setup succeeds | no C guard; process termination and side effects must match in isolated calls | [x] |
| 13 | `log_info`, `log_warning`, `log_error` | `message == NULL`, both with logger closed and open | closed logger is a no-op; open logger delegates `%s` handling to the same C stdio and output must match | [x] |

Generic boundary applicability:

- Zero-sized task collections are covered through `MAX_TASKS=0`.
- Oversized C strings are covered above and in `CONFIGS.md`; descriptions are
  truncated to 255 bytes.
- There are no public length arguments and no public enum arguments, so
  one-past-range lengths/enums do not exist for this API.
