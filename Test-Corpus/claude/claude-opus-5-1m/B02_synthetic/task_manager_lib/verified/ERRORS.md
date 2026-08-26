# ERRORS.md — Phase A: error-surface table

Every distinct way the C library rejects / errors on input, derived by grepping
`c_src/src/*.c` for **every** `return -1` / `return NULL` / `return
EXIT_FAILURE` / early `return;` / null check / range check / `assert` / min-max
constant. Line numbers refer to the C sources.

Inventory of rejection sites found (nothing else in the library rejects
anything):

```
logger.c:38-40   if (!log_file)                  -> fprintf(stderr,...); return -1
logger.c:48      if (log_file)  [log_info]       -> guard: silently no-op when false
logger.c:54      if (log_file)  [log_warning]    -> guard: silently no-op when false
logger.c:60      if (log_file)  [log_error]      -> guard: silently no-op when false
logger.c:66      if (log_file)  [finalize]       -> guard: silently no-op when false
task_manager.c:34-36  if (!manager)              -> log_error(...); return NULL
task_manager.c:43-46  if (!manager->tasks)       -> log_error(...); free(manager); return NULL
task_manager.c:54-57  task_count >= max_tasks    -> log_warning(...); return  (task dropped)
task_manager.c:60-61  strncpy(..., 255); [255]=0 -> silent truncation at 255 chars
driver.c:34-35   if (res != 0)                   -> return EXIT_FAILURE (== 1)
driver.c:39-40   if (!manager)                   -> return EXIT_FAILURE (== 1)
driver.c:54-58   if (!task)                      -> fprintf(stderr,...); destroy; finalize; return EXIT_FAILURE
```

There are **no `assert()`s** and **no `enum` types** anywhere in the public API
(`c_src/include/*.h` declares only `Task`/`TaskManager` structs and plain
`int`/`const char *` parameters), so there is no "out-of-range enum value across
the FFI boundary" case to construct. The nearest equivalents — out-of-range
`int` values for `priority` and for the `MAX_TASKS`-derived `max_tasks`
(including negative and `INT_MIN`/`INT_MAX`) — are covered by rows 12-15 and by
`CONFIGS.md` rows 8-12.

Magic constants / limits: `sizeof(Task::description) == 256` (copy limit 255),
default `max_tasks == 10`, `EXIT_FAILURE == 1`, error return `-1`.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|----|----------|---------------------------------------------|-------------------|------|----|
| 1  | `initialize_logger` | `LOG_FILE` names an unopenable path (`fopen(path,"a")` fails) — nonexistent directory component, e.g. `LOG_FILE=/nonexistent_dir_xyz/l.log` | writes `Failed to open log file: <path>\n` to `stderr`; sets static `log_file = NULL`; **returns `-1`** | `err_01_init_logger_bad_path` | [x] |
| 2  | `initialize_logger` | `LOG_FILE` is a **directory** (`fopen` fails with `EISDIR`) | same as #1: stderr message, **returns `-1`** | `err_02_init_logger_dir_path` | [x] |
| 3  | `initialize_logger` | `LOG_FILE` is the **empty string** `""` (`fopen("", "a")` fails `ENOENT`) | same as #1: stderr message (empty path printed), **returns `-1`** | `err_03_init_logger_empty_path` | [x] |
| 4  | `initialize_logger` | `LOG_FILE` names a file in a **read-only / non-writable** directory (mode 0500) | same as #1: stderr message, **returns `-1`** | `err_04_init_logger_perm_denied` | [x] |
| 5  | `log_info` | called while `log_file == NULL` (never initialized, or after a failed `initialize_logger`) | guard false: **no output at all**, returns void, no crash | `c_errors::err_05_log_fns_before_init` + `c_process::fresh_log_before_init` | [x] |
| 6  | `log_warning` | called while `log_file == NULL` | guard false: **no output**, void | `c_errors::err_05_log_fns_before_init` + `c_process::fresh_log_before_init` | [x] |
| 7  | `log_error` | called while `log_file == NULL` | guard false: **no output**, void | `c_errors::err_05_log_fns_before_init` + `c_process::fresh_log_before_init` | [x] |
| 8  | `finalize_logger` | called while `log_file == NULL` (never initialized / init failed) | guard false: **no `[INFO] Logger finalized.`**, no `fclose`, void, no crash | `c_errors::err_06_finalize_before_init` + `c_process::fresh_log_before_init` | [x] |
| 9  | `log_info` / `log_warning` / `log_error` | `message == NULL` (no null check in C) — `fprintf(f,"[INFO] %s\n", NULL)` | glibc prints the literal `(null)`: `[INFO] (null)\n` etc.; returns void | `c_errors::err_07_log_null_message` | [x] |
| 10 | `create_task_manager` | `malloc(sizeof(TaskManager))` (16 bytes) returns `NULL` — forced in a re-executed child process by exhausting the heap under `RLIMIT_AS` | `log_error("Failed to allocate memory for TaskManager.")`; **returns `NULL`** (no `free`) | `c_process::oom_create_manager` | [x] |
| 11 | `create_task_manager` | `MAX_TASKS` huge positive so `max_tasks * 260` cannot be allocated, e.g. `MAX_TASKS=2000000000` (520 GB) | `log_error("Failed to allocate memory for tasks.")`; `free(manager)`; **returns `NULL`** | `c_errors::err_09_create_tm_huge_max_tasks` + `c_process::oom_create_tasks_array` | [x] |
| 12 | `create_task_manager` | `MAX_TASKS` **negative**, e.g. `-1`: the `int` is converted to `size_t` (sign-extended) before `* sizeof(Task)`, wrapping to a huge value → `malloc` fails | `log_error("Failed to allocate memory for tasks.")`; `free(manager)`; **returns `NULL`** | `c_errors::err_10_create_tm_negative_max_tasks` + `c_process::oom_create_tasks_array` | [x] |
| 13 | `create_task_manager` | `MAX_TASKS=-2147483648` (`INT_MIN`) — extreme of #12 | **returns `NULL`** (tasks alloc fails) | `c_errors::err_10_create_tm_negative_max_tasks` + `c_process::oom_create_tasks_array` | [x] |
| 14 | `create_task_manager` | `MAX_TASKS=2147483647` (`INT_MAX`, 558 GB) | **returns `NULL`** (tasks alloc fails) | `c_errors::err_09_create_tm_huge_max_tasks` + `c_process::oom_create_tasks_array` | [x] |
| 15 | `add_task` | `manager->task_count >= manager->max_tasks` — reached by adding an 11th task with the default `max_tasks == 10` | `log_warning("Cannot add task: Maximum task limit reached.")`; **returns without storing**; `task_count` **not** incremented | `c_errors::err_11_add_task_limit_reached` | [x] |
| 16 | `add_task` | `max_tasks == 0` (`MAX_TASKS=0`, or `MAX_TASKS` non-numeric so `atoi` yields 0) — first `add_task` already exceeds the limit | `log_warning(...)` on **every** call; nothing ever stored | `c_errors::err_12_add_task_zero_max` | [x] |
| 17 | `add_task` | `max_tasks` **negative** on a caller-supplied `TaskManager` (`task_count = 0 >= -1`) | `log_warning(...)`; **returns**, nothing stored | `c_errors::err_13_add_task_negative_max` | [x] |
| 18 | `add_task` | `description` longer than 255 bytes (256, 300, 1000) — `strncpy(...,255)` does **not** NUL-terminate, then `description[255] = '\0'` | silent **truncation to exactly 255 bytes**, no error, no warning, task **is** stored | `c_errors::err_14_add_task_truncation` | [x] |
| 19 | `add_task` | `manager == NULL` — C dereferences `manager->task_count` with no null check | **SIGSEGV** (undefined behaviour, deterministic crash) | `c_process::null_add_task_manager` | [x] |
| 20 | `add_task` | `description == NULL` with room in the manager — `strncpy(dst, NULL, 255)` | **SIGSEGV** | `c_process::null_add_task_desc` | [x] |
| 21 | `add_task` | `description == NULL` **and** limit already reached | the limit check fires **first**, so `description` is never dereferenced: `log_warning(...)`, **no crash** | `c_errors::err_16_null_desc_short_circuit` | [x] |
| 22 | `print_tasks` | `manager == NULL` — `manager->task_count` dereferenced with no null check | prints `Tasks:\n` **first**, then **SIGSEGV** | `c_process::null_print_manager` | [x] |
| 23 | `print_tasks` | `manager->tasks == NULL` while `task_count > 0` | prints `Tasks:\n`, then **SIGSEGV** reading `tasks[0]` | `c_process::null_print_tasks_array` | [x] |
| 24 | `print_tasks` | `task_count` **negative** (caller-supplied struct) — loop condition `0 < negative` is false | prints only `Tasks:\n`, no rows, no crash | `c_errors::err_17_print_negative_count` | [x] |
| 25 | `destroy_task_manager` | `manager == NULL` — `free(manager->tasks)` dereferences NULL | **SIGSEGV** | `c_process::null_destroy` | [x] |
| 26 | `destroy_task_manager` | `manager->tasks == NULL` (valid manager, null tasks) — `free(NULL)` is a no-op | no error; frees `manager`; `log_info("TaskManager destroyed successfully.")` | `c_errors::err_18_destroy_null_tasks` | [x] |
| 27 | `driver` | `initialize_logger()` returned non-zero (`LOG_FILE` unopenable) | stderr message from #1; **returns `EXIT_FAILURE` (1)**; `create_task_manager` never called | `c_errors::err_19_driver_logger_fail` | [x] |
| 28 | `driver` | `create_task_manager()` returned `NULL` (`MAX_TASKS` huge or negative) | **returns `EXIT_FAILURE` (1)**; logger left **open, not finalized** (no `[INFO] Logger finalized.` in the log) — C leaks it, and the log ends with `[ERROR] Failed to allocate memory for tasks.` | `c_errors::err_20_driver_create_fail` | [x] |
| 29 | `driver` | `malloc(length + 1)` for the extracted task line returns `NULL` — forced in a re-executed child process: build a 32 MiB line, then clamp `RLIMIT_AS` so the 32 MiB copy cannot be allocated but the 16 B + 2600 B manager allocations still can | `fprintf(stderr, "Error: Failed to allocate memory for task.\n")`; `destroy_task_manager(manager)`; `finalize_logger()`; **returns `EXIT_FAILURE` (1)** | `c_process::oom_driver_task_line` | [x] |
| 30 | `driver` | `tasks == NULL` — `while (*start != '\0')` dereferences NULL, but only **after** `initialize_logger` + `create_task_manager` succeeded | **SIGSEGV** (and the log file already contains the init + create entries) | `c_process::null_driver` | [x] |
| 31 | `driver` | more input lines than `max_tasks` (e.g. 15 lines, `MAX_TASKS=10`) | per-line `log_warning` for the overflow lines; **still returns `0`**; only the first 10 tasks printed; `priority` keeps incrementing for dropped lines | `c_errors::err_22_driver_more_lines_than_max` | [x] |
| 32 | *generic FFI boundary* | zero-length / empty inputs: `driver("")`, `add_task(m, "", p)` | no rejection: `driver("")` prints only `Tasks:\n` and returns `0`; `add_task` stores an empty description | `c_errors::err_23_generic_empty_and_bounds` | [x] |
| 33 | *generic FFI boundary* | values one step past the documented range: description length exactly `255` (last fully-kept) vs `256` (first truncated); `task_count == max_tasks - 1` (accepted) vs `== max_tasks` (rejected) | boundary behaves exactly as #15/#18 describe, off-by-one-free | `c_errors::err_23_generic_empty_and_bounds` | [x] |
| 34 | *generic FFI boundary* | extreme `int` arguments across FFI: `priority = INT_MIN`, `INT_MAX`, `-1`, `0` (no validation in C) | stored verbatim and printed by `%d` verbatim | `c_errors::err_23_generic_empty_and_bounds` | [x] |

### Where each row runs

* `c_errors::` = `tests/phase_c_errors.rs` — in-process, both `.so`s loaded with
  `libloading`; compares return value + stdout + stderr + log-file bytes +
  raw `TaskManager`/`Task` bytes.
* `c_process::` = `tests/phase_c_process.rs` — out-of-process (`harness = false`,
  the test binary re-executes itself), needed because:
  * rows 19, 20, 22, 23, 25, 30 are null dereferences: C has no null checks, so
    the observable is the **termination signal**. The test asserts the C
    reference really died with `SIGSEGV` (11) *and* that Rust died with the same
    signal, with identical stdout/stderr/log — a real differential assertion, not
    a "both failed somehow" pass.
  * rows 10, 11, 12, 29 are `malloc` failures. They are genuinely **reached**,
    not assumed: `RLIMIT_AS` is clamped to "currently mapped + slack" and (for
    row 10) the heap is exhausted first, which is confirmed by the resulting log
    records (`[ERROR] Failed to allocate memory for TaskManager.` /
    `... for tasks.`) and by row 29's `Error: Failed to allocate memory for
    task.` on stderr.
  * rows 5-8 additionally run in a **pristine** process, the only place where
    `static FILE *log_file` is genuinely still `NULL`.

### Deliberately-not-executed rows

None. All 34 rows have an executed, passing differential test.

### Finding: `debug-assertions` broke the null-dereference rows

Rows 19, 22, 23, 25 and 30 initially **diverged**: the C `.so` died with
`SIGSEGV` (exit 139, empty stderr) while the Rust `.so` died with `SIGABRT`
(exit 134) after printing

```
thread '<unnamed>' panicked at src/task_manager.rs:151:19:
null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

Rust's implicit UB checks — enabled by `debug-assertions`, which is on by
default in the `dev`/`test` profiles — intercept exactly the inputs the C code
happily faults on. Fixed by disabling `debug-assertions` and `overflow-checks`
for every profile in `Cargo.toml` (causality verified: rebuilding the cdylib
with `RUSTFLAGS=-Cdebug-assertions=on` reproduces exit 134, and removing it
restores exit 139 for both libraries).
