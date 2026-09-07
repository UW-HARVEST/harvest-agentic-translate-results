# ERRORS.md — error-surface table (Phase C)

Derived mechanically from the C sources. Every early-return, every error print,
every guard `if`, every clamp/truncation constant is one row.

Greps used:

```sh
grep -n 'return -1\|return NULL\|return EXIT_FAILURE\|return;\|assert\|if (!\|if (log_file)\|>=\|sizeof' c_src/src/*.c
```

There are **no** `assert`s, **no** error enums and **no** error-code macros in
this library. The complete rejection vocabulary is: `-1`, `NULL`,
`EXIT_FAILURE` (== `1`), silent no-op, and silent truncation.

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| E1 | `initialize_logger` (`logger.c:38`) | `fopen(path,"a")` fails — `LOG_FILE` names an unopenable path (non-existent directory, e.g. `/nonexistent-dir-xyz/f.log`) | prints `Failed to open log file: <path>\n` to `stderr`; returns `-1`; `log_file` left `NULL` | `err_e1_initialize_logger_fopen_fails` | [x] |
| E2 | `initialize_logger` (`logger.c:38`) | `LOG_FILE=""` (empty string — `fopen("")` fails with ENOENT) | same as E1, returns `-1` | `err_e2_initialize_logger_empty_path` | [x] |
| E3 | `initialize_logger` (`logger.c:38`) | `LOG_FILE` names a directory (`fopen(dir,"a")` fails EISDIR) | same as E1, returns `-1` | `err_e3_initialize_logger_path_is_dir` | [x] |
| E4 | `log_info` (`logger.c:48`) | called while `log_file == NULL` (before any `initialize_logger`, or after a failed one) | guard `if (log_file)` false → no output at all, no crash, `void` | `err_e4_log_fns_before_init` | [x] |
| E5 | `log_warning` (`logger.c:54`) | called while `log_file == NULL` | no output, no crash | `err_e4_log_fns_before_init` | [x] |
| E6 | `log_error` (`logger.c:60`) | called while `log_file == NULL` | no output, no crash | `err_e4_log_fns_before_init` | [x] |
| E7 | `finalize_logger` (`logger.c:66`) | called while `log_file == NULL` (never initialized / init failed) | guard false → no `Logger finalized.` line, no `fclose`, `void`, no crash | `err_e7_finalize_without_init` | [x] |
| E8 | `create_task_manager` (`task_manager.c:34`) | `malloc(sizeof(TaskManager))` (16 bytes) returns `NULL` | `log_error("Failed to allocate memory for TaskManager.")`; returns `NULL` | not reachable in-process (16-byte malloc cannot be made to fail without replacing the allocator, which would also change the C side); code path verified identical by inspection — both call `log_error` with the identical literal then return null | [x] (unreachable, inspected) |
| E9 | `create_task_manager` (`task_manager.c:43`) | `malloc(max_tasks * sizeof(Task))` returns `NULL` — `MAX_TASKS` so large the product exceeds addressable memory (e.g. `MAX_TASKS=1000000000` → 260 GB) | `log_error("Failed to allocate memory for tasks.")`; `free(manager)`; returns `NULL` | `err_e9_create_tasks_alloc_fails` | [x] |
| E10 | `create_task_manager` (`task_manager.c:42`) | `MAX_TASKS` negative (e.g. `-1`) → `(size_t)(-1) * 260` wraps to a huge value → `malloc` fails | same as E9: `log_error(...tasks.)`, `free(manager)`, `NULL` | `err_e10_create_negative_max_tasks` | [x] |
| E11 | `add_task` (`task_manager.c:54`) | `manager->task_count >= manager->max_tasks` (list already full — reached by adding `max_tasks+k` tasks) | `log_warning("Cannot add task: Maximum task limit reached.")`; returns without writing; `task_count` unchanged | `err_e11_add_task_full` | [x] |
| E12 | `add_task` (`task_manager.c:54`) | `MAX_TASKS=0` → `max_tasks == 0`, so the **first** `add_task` is already rejected | `log_warning(...)`, `task_count` stays `0` | `err_e12_add_task_max_zero` | [x] |
| E13 | `add_task` (`task_manager.c:54`) | `MAX_TASKS` negative-but-allocatable is impossible, but `MAX_TASKS` non-numeric (`atoi` → `0`) reaches the same guard | `log_warning(...)`, `task_count` stays `0` | `err_e13_add_task_max_nonnumeric` | [x] |
| E14 | `add_task` (`task_manager.c:60-61`) | `description` longer than 255 bytes | silent truncation: `strncpy(dst,src,255)` then `dst[255]='\0'` → exactly the first 255 bytes retained | `err_e14_add_task_truncation` | [x] |
| E15 | `add_task` (`task_manager.c:60`) | `description` is the empty string `""` | `strncpy` zero-fills all 255 bytes → all-zero description | `err_e15_add_task_empty_desc` | [x] |
| E16 | `driver` (`driver.c:34`) | `initialize_logger()` returned non-zero (unopenable `LOG_FILE`) | returns `EXIT_FAILURE` (`1`); nothing printed to stdout; no `TaskManager` created | `err_e16_driver_logger_fails` | [x] |
| E17 | `driver` (`driver.c:39`) | `create_task_manager()` returned `NULL` (huge/negative `MAX_TASKS`) | returns `EXIT_FAILURE` (`1`); nothing on stdout; `finalize_logger` **not** called (log left open, no `Logger finalized.` line) | `err_e17_driver_manager_fails` | [x] |
| E18 | `driver` (`driver.c:54`) | `malloc(length+1)` for one task line returns `NULL` | prints `Error: Failed to allocate memory for task.\n` to stderr; `destroy_task_manager`; `finalize_logger`; returns `EXIT_FAILURE` | not reachable in-process (needs a small malloc to fail); path verified identical by inspection | [x] (unreachable, inspected) |
| E19 | `driver` (`driver.c:45`) | `tasks` is the empty string `""` | loop body never runs; prints only `Tasks:\n`; returns `0` | `err_e19_driver_empty_input` | [x] |
| E20 | `driver` (`driver.c:63`) | more input lines than `max_tasks` | surplus lines each emit the E11 warning; only the first `max_tasks` appear on stdout; `priority` still increments for every line; returns `0` | `err_e20_driver_overflow_lines` | [x] |

## Generic FFI boundary cases (also covered, not in the table above)

| # | case | expected | test | status |
|---|------|----------|------|--------|
| G1 | `priority` = `INT_MIN`, `INT_MAX`, `0`, `-1` (int is unconstrained — there is **no enum** in this API, so "out-of-range enum value" degenerates to arbitrary `int`, which both sides must store and print verbatim with `%d`) | value stored and printed verbatim | `bnd_g1_priority_extremes` | [x] |
| G2 | `log_*` message containing `printf` conversion specifiers (`%s`, `%n`, `%d`) — it is an *argument*, not the format, so it must appear literally | literal text in log | `bnd_g2_log_format_specifiers` | [x] |
| G3 | `log_*` message of length 0 and of length 100 000 | full text written | `bnd_g3_log_lengths` | [x] |
| G4 | `add_task` description exactly 254 / 255 / 256 / 257 bytes (one step either side of the truncation boundary) | 254→254 bytes; ≥255→first 255 bytes | `bnd_g4_desc_boundary_lengths` | [x] |
| G5 | `driver` input consisting of only `"\n"`, `"\n\n\n"`, no trailing newline, trailing newline, `\n` at index 0 | empty task strings added with incrementing priority; exact stdout match | `bnd_g5_driver_newline_shapes` | [x] |
| G6 | `MAX_TASKS` = `"0"`, `""`, `"abc"`, `"7x"`, `" 9"`, `"+3"`, `"2147483647"`, `"-2147483648"`, `"99999999999999999999"` (`atoi` overflow is UB in C but both sides call the *same* libc `atoi`, so results are identical by construction) | identical `max_tasks` field | `bnd_g6_max_tasks_parsing` | [x] |
| G7 | NULL pointer arguments (`add_task(NULL,…)`, `print_tasks(NULL)`, `destroy_task_manager(NULL)`, `driver(NULL)`, `log_info(NULL)`) | The C **dereferences unchecked** → `SIGSEGV` for the manager/driver functions. `log_info(NULL)` passes `NULL` to `%s` (glibc prints `(null)`). Documented below; the manager/driver NULL cases are verified by inspection (both sides perform the identical unchecked deref, so both fault); `log_info(NULL)` is tested. | `bnd_g7_log_null_message` | [x] |

### Note on G7 / NULL-deref rows

`add_task`, `print_tasks`, `destroy_task_manager` and `driver` contain **no**
null checks in the C. Passing `NULL` is therefore undefined behaviour that
crashes the process in both implementations. Those rows cannot be asserted
inside a shared test process without killing the test runner; the Rust code was
confirmed to perform the same unchecked dereference at the same point (no added
`if ptr.is_null()` guard exists anywhere in `translation/src/`, verified with
`grep -n 'is_null' translation/src/*.rs` — the only null checks are the ones the
C also has: `manager`/`tasks` malloc results, `LOG_FILE`, `strchr` result,
`getenv` result, task `malloc` result). `log_info(NULL)` is safe (the pointer is
only forwarded to `printf`) and **is** tested.

## Update: stderr is now part of every comparison

The first version of this suite compared only stdout and the log file. Mutation
`M23` (changing the C's `"Failed to open log file: %s\n"` diagnostic) was **not
caught**, which proved the stderr surface was untested. `capture_out_err` in
`tests/common/mod.rs` now redirects fd 1 *and* fd 2, and every comparator
(`diff`, `diff_err`, `diff_err_fixed_env`) asserts stderr byte-equality. Rows
E1, E2, E3 and E16 — the only rows that produce stderr output — go through
`diff_err_fixed_env`, which pins `LOG_FILE` to the same unopenable value for
both implementations and compares the diagnostic bytes.

E1 additionally covers a third `fopen` failure mode discovered while writing the
test: a **read-only regular file** (mode `0444`, `EACCES`) — a different `errno`
from the `ENOENT`/`EISDIR` cases but the same `-1` result. The test asserts the
mode is actually usable (it is skipped only under uid 0) so it cannot silently
degrade.

Both `assert_eq!(rc, -1)` (the absolute sentinel) and `assert_eq!(rc_c, rc_r)`
(the differential) are asserted for E1/E2/E3, and `assert_eq!(rc, 1)` for E16,
so these rows cannot pass by "both failed somehow".
