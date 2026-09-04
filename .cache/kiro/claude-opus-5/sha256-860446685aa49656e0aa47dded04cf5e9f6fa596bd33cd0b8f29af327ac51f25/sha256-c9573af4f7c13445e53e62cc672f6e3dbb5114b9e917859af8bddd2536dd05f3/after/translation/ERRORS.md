# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from every `if (...) { ... return ...; }`, every guarded
no-op (`if (log_file)`), every allocation check and every implicit
unchecked-pointer dereference in `c_src/src/*.c`. There are no `assert`s, no
error enums, no `RETURN_ERROR`-style macros and no explicit range checks in this
library; the complete rejection vocabulary is `-1`, `NULL`, `EXIT_FAILURE` (== 1
from `<stdlib.h>`), silent `return;`, and undefined behaviour on NULL input.

`grep` basis (`c_src`):

```
driver.c:34  if (res != 0) return EXIT_FAILURE;
driver.c:39  if (!manager) return EXIT_FAILURE;
driver.c:54  if (!task) { fprintf(stderr,...); destroy; finalize; return EXIT_FAILURE; }
logger.c:38  if (!log_file) { fprintf(stderr,...); return -1; }
logger.c:48  if (log_file)   -> log_info    guarded no-op
logger.c:54  if (log_file)   -> log_warning guarded no-op
logger.c:60  if (log_file)   -> log_error   guarded no-op
logger.c:66  if (log_file)   -> finalize_logger guarded no-op
task_manager.c:34 if (!manager) { log_error(...); return NULL; }
task_manager.c:43 if (!manager->tasks) { log_error(...); free(manager); return NULL; }
task_manager.c:54 if (task_count >= max_tasks) { log_warning(...); return; }
```

## Table

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| E1  | `initialize_logger` | `$LOG_FILE` names a path `fopen(path,"a")` cannot open — non-existent directory component (`/nonexistent-dir-xyz/log.txt`) | writes `Failed to open log file: <path>\n` to `stderr`; returns `-1`; static `log_file` left `NULL` |
| E2  | `initialize_logger` | `$LOG_FILE` set to the empty string `""` (`getenv` returns non-NULL, so `""` is used as the path — the `?:` tests the pointer, not emptiness) | `fopen("", "a")` fails ⇒ same as E1: `stderr` message with an empty path, returns `-1` |
| E3  | `initialize_logger` | `$LOG_FILE` names an existing **directory** | `fopen` fails (EISDIR) ⇒ returns `-1`, `stderr` message |
| E4  | `initialize_logger` | `$LOG_FILE` names a file with no write permission (mode `0444`) | `fopen` fails (EACCES) ⇒ returns `-1`, `stderr` message |
| E5  | `log_info` | called while `log_file == NULL` (never initialised, or after a failed `initialize_logger`) | silent no-op, no output anywhere, `void` |
| E6  | `log_warning` | called while `log_file == NULL` | silent no-op, no output, `void` |
| E7  | `log_error` | called while `log_file == NULL` | silent no-op, no output, `void` |
| E8  | `finalize_logger` | called while `log_file == NULL` | silent no-op — in particular **no** `[INFO] Logger finalized.` line and no `fclose` |
| E9  | `create_task_manager` | `malloc(sizeof(TaskManager))` (16 bytes) returns `NULL` | `log_error("Failed to allocate memory for TaskManager.")`, returns `NULL`. Not reachable by input; covered by inspection + the identical 16-byte `malloc` call in both builds |
| E10 | `create_task_manager` | `$MAX_TASKS` negative, e.g. `-1`: `max_tasks * sizeof(Task)` converts the `int` to `size_t` (sign-extended) and wraps ⇒ astronomically large request, `malloc` returns `NULL` | `log_error("Failed to allocate memory for tasks.")` to the log file, `free(manager)`, returns `NULL`. `manager->max_tasks` was already written as `-1` before the failure |
| E11 | `create_task_manager` | `$MAX_TASKS` a huge positive value (e.g. `2000000000`) whose `*260` byte size cannot be allocated | same as E10: `NULL` returned after `log_error` + `free` |
| E12 | `create_task_manager` | `$MAX_TASKS` = `INT_MIN` (`-2147483648`) — most negative wrap case | same as E10: `NULL` |
| E13 | `add_task` | `manager->task_count >= manager->max_tasks` reached by filling the manager (`MAX_TASKS=n`, then the *(n+1)*-th `add_task`) | `log_warning("Cannot add task: Maximum task limit reached.")`; returns without touching `tasks`; `task_count` stays `n` |
| E14 | `add_task` | `$MAX_TASKS=0` (or a non-numeric value ⇒ `atoi` = 0), so the *first* `add_task` is already over the limit | `log_warning(...)`, `task_count` stays `0` |
| E15 | `add_task` | manager whose `max_tasks` field is negative and `task_count == 0` (`0 >= -1` is true) | `log_warning(...)`, no write. Reachable only by hand-built struct, since E10 makes `create_task_manager` return `NULL` for negative `MAX_TASKS` |
| E16 | `driver` | `initialize_logger()` returns non-zero, i.e. any of E1–E4 (unopenable `$LOG_FILE`) | returns `EXIT_FAILURE` (`1`) immediately; **no** task manager created, `print_tasks` never runs, nothing on stdout |
| E17 | `driver` | `create_task_manager()` returns `NULL`, i.e. E10–E12 (negative/huge `$MAX_TASKS`) | returns `EXIT_FAILURE` (`1`); note the quirk: `finalize_logger()` is **not** called, so no `[INFO] Logger finalized.` line and the log file stays open |
| E18 | `driver` | `malloc(length + 1)` for one extracted line returns `NULL` | `Error: Failed to allocate memory for task.\n` on `stderr`, `destroy_task_manager`, `finalize_logger`, returns `EXIT_FAILURE`. Not reachable by input (lines are short); covered by inspection — both builds issue the identical `malloc(length+1)` |
| E19 | `add_task` | `manager == NULL` — no null check, `manager->task_count` dereferences NULL | undefined behaviour: `SIGSEGV`. Rust must fault identically (verified in a forked child) |
| E20 | `print_tasks` | `manager == NULL` — no null check | prints `Tasks:\n` **first**, then `SIGSEGV` on `manager->task_count` |
| E21 | `destroy_task_manager` | `manager == NULL` | `free(manager->tasks)` dereferences NULL ⇒ `SIGSEGV` |
| E22 | `driver` | `tasks == NULL` — no null check; `*start` dereferences NULL after the logger and manager are set up | `SIGSEGV` (after the log file has been created and written to) |
| E23 | `add_task` | `description == NULL` — passed straight to `strncpy` | `SIGSEGV` inside `strncpy` |
| E24 | `log_info` / `log_warning` / `log_error` | `message == NULL` while `log_file != NULL` | **not** a crash: glibc `printf` `%s` prints the literal `(null)` ⇒ e.g. `[INFO] (null)\n`. Must match byte-for-byte |
| E25 | `add_task` | `description` longer than 255 bytes (over-long input, the only length limit in the library: `sizeof(task->description) - 1`) | silently truncated to 255 bytes + `'\0'`; still `log_info("Task added successfully.")`; **not** an error return |
| E26 | out-of-range enum across FFI | the API declares **no enums** — `priority` is a plain `int` and every one of the 2^32 values is valid (`INT_MIN`, `-1`, `0`, `INT_MAX` all stored verbatim), and there is no mode/flag parameter anywhere | no rejection exists; tested as valid input in `CONFIGS.md` (C1x rows) instead of here |

## Result

All rows E1–E26 have a differential test in `tests/differential.rs`
(`phase_c_*`). E9 and E18 are allocation-failure branches that cannot be
triggered through the public API; they are covered by source inspection plus a
test asserting the surrounding observable behaviour is identical.

## Notes on how the rows are asserted

* The crash rows (E19–E23) run the call in a **forked child** with core dumps
  disabled and compare the raw wait status. Each test also asserts that the *C*
  child really died of `SIGSEGV` (11) before comparing, so the row cannot pass
  vacuously by both sides exiting 0.
* The per-library scratch directory path is normalised to `<SUBDIR>` in captured
  output before comparison: `logger.c:39` echoes the failing path back on
  `stderr`, and the two runs legitimately use sibling directories. Everything
  else is compared byte-for-byte.
* Each library is loaded from a **unique copy** of its `.so`, so every test starts
  from a pristine `static FILE *log_file == NULL`; test order cannot leak logger
  state and the "uninitialised" rows (E5–E8) are genuinely observable.
* Harness sensitivity was confirmed by a negative control: three deliberate bugs
  injected into the Rust source (`[WARNING]`→`[WARN]`, `strncpy` length 255→200,
  `driver` priority start 1→0) made 31/42 Phase B, 9/26 Phase C and 2/2 Phase D
  tests fail. The tests are not vacuous.
* `tests/phase_d_abi.rs` additionally mixes the two libraries inside one
  pipeline (C allocates → Rust populates → C renders → C frees, and the reverse)
  to prove `sizeof(Task)` = 260, `sizeof(TaskManager)` = 16 and every field
  offset agree across the FFI boundary — something no single-library test can
  detect.
