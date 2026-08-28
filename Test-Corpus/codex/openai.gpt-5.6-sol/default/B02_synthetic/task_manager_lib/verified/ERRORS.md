# Error Surface

Derived mechanically from every error-return and rejection branch in
`c_src/src/*.c`. Allocation failures are induced by the differential test's
preloaded allocator shim so the exact otherwise-unreachable branches execute.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `initialize_logger` | `fopen(LOG_FILE or "default.log", "a")` returns `NULL` | writes `Failed to open log file: <path>` to stderr and returns `-1` | [x] |
| 2 | `create_task_manager` | first `malloc(sizeof(TaskManager))` returns `NULL` | logs `Failed to allocate memory for TaskManager.` when logging is active and returns `NULL` | [x] |
| 3 | `create_task_manager` | task-array `malloc(max_tasks * sizeof(Task))` returns `NULL` | logs `Failed to allocate memory for tasks.`, frees the manager, and returns `NULL` | [x] |
| 4 | `add_task` | `manager->task_count >= manager->max_tasks` | logs `Cannot add task: Maximum task limit reached.`, returns without changing the manager | [x] |
| 5 | `driver` | `initialize_logger()` returns nonzero because its `fopen` failed | returns `EXIT_FAILURE` (`1`) without creating a manager | [x] |
| 6 | `driver` | `create_task_manager()` returns `NULL` after either manager allocation fails or task-array allocation fails | returns `EXIT_FAILURE` (`1`) | [x] |
| 7 | `driver` | per-line `malloc(length + 1)` returns `NULL` | writes `Error: Failed to allocate memory for task.` to stderr, destroys the manager, finalizes logging, and returns `EXIT_FAILURE` (`1`) | [x] |

There are no C `assert` statements, enum parameters, explicit numeric range
rejections, or public length parameters. Null pointers not explicitly checked
by C are covered as generic FFI-boundary behavior in the tests rather than
listed as C rejection branches.
