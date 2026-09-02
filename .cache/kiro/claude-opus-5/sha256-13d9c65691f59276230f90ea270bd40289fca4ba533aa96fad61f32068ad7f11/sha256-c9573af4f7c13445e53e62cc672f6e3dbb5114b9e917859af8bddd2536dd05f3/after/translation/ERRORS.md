# ERRORS.md — error-surface table (Phase A) / error-path tests (Phase C)

Derived mechanically from every conditional in the C sources.  The complete set
of `if` statements in `c_src/src` is:

```
$ grep -n 'if (\|return\|assert\|NULL' c_src/src/*.c
driver.c:34       if (res != 0)                            -> return EXIT_FAILURE
driver.c:39       if (!manager)                            -> return EXIT_FAILURE
driver.c:47       if (end == NULL)                         -> (not an error: last-line handling)
driver.c:54       if (!task)                               -> fprintf(stderr) + cleanup + EXIT_FAILURE
logger.c:38       if (!log_file)                           -> fprintf(stderr) + return -1
logger.c:48/54/60 if (log_file)                            -> guard; else silent no-op
logger.c:66       if (log_file)                            -> guard; else silent no-op
task_manager.c:34 if (!manager)                            -> log_error + return NULL
task_manager.c:43 if (!manager->tasks)                     -> log_error + free + return NULL
task_manager.c:54 if (task_count >= max_tasks)             -> log_warning + return
```

There are **no** `assert`s, **no** error enums and **no** error-code macros
other than `EXIT_FAILURE` (== 1) and the literal `-1`/`NULL` sentinels.  The
public API also declares **no enum parameters**: the only `int` inputs are
`add_task`'s `priority` (completely unconstrained — every 32-bit value is valid
and is covered in `CONFIGS.md` rows 20–21) and the `int` return values.  So the
"out-of-range enum across FFI" class collapses to "extreme `int` values", which
rows 30–31 of `CONFIGS.md` cover.

Rows 16–20 and 24–26 are *implicit* rejections: the C performs no NULL/liveness
check at all, so the "expected C result" is a process-level fault.  Those are
compared by forking and comparing the child's termination status
(`common::assert_same_term`), because "returns an error" is not something the C
does there and inventing a graceful error in Rust would itself be a divergence.

Rows 11–14 and 25 need `malloc` to fail on demand.  The test binaries are linked
with `-rdynamic` (see `build.rs`) so that a `#[no_mangle] malloc` in
`tests/common/mod.rs` interposes on the allocator calls of both dlopen'ed `.so`s;
a test arms a failure for one specific allocation size and asserts the failure
really fired.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `initialize_logger` | `$LOG_FILE` names an existing **directory** → `fopen(dir,"a")` = NULL (EISDIR) | `stderr` gets `Failed to open log file: <path>\n`; returns `-1`; `log_file` stays NULL | `err01_fopen_directory` | [x] |
| 2 | `initialize_logger` | `$LOG_FILE=""` → `fopen("","a")` = NULL (ENOENT) | same message with an empty path; returns `-1` | `err02_fopen_empty_path` | [x] |
| 3 | `initialize_logger` | `$LOG_FILE` under a **non-existent directory** (`nope/deeper/x.log`) | same message; returns `-1` | `err03_fopen_missing_parent` | [x] |
| 4 | `initialize_logger` | `$LOG_FILE` is an existing file with mode `0444` → EACCES (skipped when euid==0) | same message; returns `-1` | `err04_fopen_readonly` | [x] |
| 5 | `initialize_logger` | `$LOG_FILE` component is a file, used as a directory (`f.txt/x.log`, ENOTDIR) | same message; returns `-1` | `err05_fopen_notdir` | [x] |
| 6 | `initialize_logger` | `$LOG_FILE` longer than `PATH_MAX` (ENAMETOOLONG) | same message; returns `-1` | `err06_fopen_name_too_long` | [x] |
| 7 | `log_info` | called while `log_file == NULL` (before any `initialize_logger`) | guard fails → **silent no-op**, nothing written anywhere | `err07_preinit_guards` | [x] |
| 8 | `log_warning` | ditto | silent no-op | `err07_preinit_guards` | [x] |
| 9 | `log_error` | ditto | silent no-op | `err07_preinit_guards` | [x] |
| 10 | `finalize_logger` | ditto (`log_file == NULL`) | silent no-op: **no** `[INFO] Logger finalized.` line and no `fclose` | `err07_preinit_guards` | [x] |
| 11 | `create_task_manager` | `malloc(sizeof(TaskManager))` (16 B) returns NULL | `log_error("Failed to allocate memory for TaskManager.")` then returns NULL; **no** `free` | `err11_manager_malloc_fails` | [x] |
| 12 | `create_task_manager` | `malloc(max_tasks*sizeof(Task))` returns NULL because `$MAX_TASKS=-1` makes the size wrap to `0xFFFFFFFFFFFFFEFC` | `log_error("Failed to allocate memory for tasks.")`, `free(manager)`, returns NULL — note `max_tasks`/`task_count` were already written | `err12_tasks_size_wraps` | [x] |
| 13 | `create_task_manager` | same branch via a huge positive `$MAX_TASKS` (`2000000000` → 520 GB) | as row 12 | `err13_tasks_too_big` | [x] |
| 14 | `create_task_manager` | same branch via interposed failure of the 2600-byte `malloc` (`$MAX_TASKS` = 10) | as row 12 | `err14_tasks_malloc_fails` | [x] |
| 15 | `add_task` | `manager->task_count >= manager->max_tasks` after exactly `max_tasks` successful adds | `log_warning("Cannot add task: Maximum task limit reached.")`, returns without touching the array or `task_count` | `err15_limit_reached` | [x] |
| 16 | `add_task` | `$MAX_TASKS=0` → the *first* add is already over the limit | as row 15 (every add rejected; `print_tasks` prints only the header) | `err16_zero_capacity` | [x] |
| 17 | `add_task` | hand-built manager with `max_tasks < 0` (`0 >= -1`) | as row 15 | `err17_negative_capacity` | [x] |
| 18 | `add_task` | `manager == NULL` → `(*manager).task_count` read at offset 12 of NULL | no check exists → `SIGSEGV` (11) | `err18_add_task_null_manager` | [x] |
| 19 | `add_task` | `description == NULL` with capacity available → `strncpy(dst, NULL, 255)` | no check exists → `SIGSEGV` (11) | `err19_add_task_null_description` | [x] |
| 20 | `print_tasks` | `manager == NULL` | `Tasks:\n` **is** emitted, then `SIGSEGV` (11) | `err20_print_tasks_null_manager` | [x] |
| 21 | `print_tasks` | `manager->task_count > 0` but `manager->tasks == NULL` | `Tasks:\n` emitted, then `SIGSEGV` (11) | `err21_print_tasks_null_array` | [x] |
| 22 | `destroy_task_manager` | `manager == NULL` → `free((*manager).tasks)` | no check exists → `SIGSEGV` (11) | `err22_destroy_null_manager` | [x] |
| 23 | `driver` | `initialize_logger()` returned non-zero (`$LOG_FILE` is a directory) | `stderr` gets the `Failed to open log file:` line; returns `EXIT_FAILURE` (1); **nothing** on stdout, no manager created | `err23_driver_logger_fails` | [x] |
| 24 | `driver` | `create_task_manager()` returned NULL (`$MAX_TASKS=-1`) | returns `EXIT_FAILURE` (1); stdout empty; the log holds `Logger initialized.` + `Failed to allocate memory for tasks.` and **no** `Logger finalized.` (the C leaks the open handle on this path) | `err24_driver_manager_fails` | [x] |
| 25 | `driver` | `malloc(length + 1)` for the per-line copy returns NULL (interposed) | `stderr` gets `Error: Failed to allocate memory for task.\n`, then `destroy_task_manager` + `finalize_logger` run and it returns `EXIT_FAILURE` (1); stdout has **no** `Tasks:` header | `err25_driver_task_malloc_fails` | [x] |
| 26 | `driver` | `tasks == NULL` → `*start` dereferences NULL | no check exists → `SIGSEGV` (11), after the logger has been opened and the manager allocated | `err26_driver_null_input` | [x] |
| 27 | `log_info` / `log_warning` / `log_error` | called **after** `finalize_logger` — `log_file` is `fclose`d but never reset to NULL, so the guard passes on a freed `FILE *` | observed on this glibc: the write is silently dropped and the process continues (`Exited(0)`); C and Rust identical and reproducible | `err27_log_after_finalize` | [x] |
| 28 | `finalize_logger` | called twice → second `fclose` on the already-freed handle | observed: `Exited(0)` (no output, no fault); C and Rust identical and reproducible | `err28_double_finalize` | [x] |
| 29 | `initialize_logger` | called twice → the first `FILE *` is overwritten without being closed | second call succeeds and returns `0`; two `Logger initialized.` lines; the first handle is leaked (still open) | `err29_double_initialize` | [x] |
| 30 | `destroy_task_manager` | called twice on the same pointer → double `free` | observed: `SIGSEGV` (11) — glibc's tcache corrupts before the abort check; C and Rust identical and reproducible | `err30_double_destroy` | [x] |
| 31 | `add_task` | called after `destroy_task_manager` (use-after-free of the manager) | observed: `SIGSEGV` (11); C and Rust identical and reproducible | `err31_add_after_destroy` | [x] |
| 32 | `add_task` | hand-built manager with capacity available but `tasks == NULL` | `SIGSEGV` (11) | `err_generic_null_and_extreme_sweep` | [x] |
| 33 | `add_task` | `manager` **and** `description` both NULL, `priority = INT_MIN` | `SIGSEGV` (11) | `err_generic_null_and_extreme_sweep` | [x] |
| 34 | `destroy_task_manager` | non-NULL but unmapped pointer (`0x1`) | fatal signal | `err_generic_null_and_extreme_sweep` | [x] |

All 34 rows have a passing differential test in `tests/phase_c_errors.rs`,
`tests/phase_c_preinit.rs` and `tests/phase_c_crashes.rs`.

Rows 27, 28, 30 and 31 are undefined behaviour in C, so "the expected result" is
not defined by the standard — only by what this glibc actually does.  Each is
therefore recorded twice per implementation and asserted to be both *stable* and
*identical across the two builds*, rather than compared against a hard-coded
expectation.

## Not reachable through the public API

For completeness, these C conditionals are not separate rejections:

* `driver.c:47` `if (end == NULL)` is the last-line case of the splitting loop,
  not an error; covered as `CONFIGS.md` rows 26/31/35.
* There is no way to make `getenv` or `atoi` fail — `atoi` has no error return,
  which is why every malformed `$MAX_TASKS` string is a *valid* configuration
  (`CONFIGS.md` rows 17-18) rather than an error row.

## Self-check

`./mutation_check.sh` injects 29 known divergences into the Rust source (wrong
error return, missing `free`, an *added* NULL guard the C does not have, …) and
confirms the suite fails on every one, plus 2 provably unobservable changes it
correctly ignores. Current result: 29/29 caught.
