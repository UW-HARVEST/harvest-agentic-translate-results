# ERRORS.md — Phase A error-surface table

Mechanically derived from every rejection / early-return / guard / sentinel in
`c_src/src/*.c`. Greps used: `return -1`, `return NULL`, `return EXIT_FAILURE`,
`if (!`, `>=`, `== NULL`, `sizeof(...) - 1`, `assert` (none present).

There are **no** `assert()`s, no error enums and no `RETURN_ERROR`-style macros
in this library. Every rejection is one of: a `-1`/`NULL`/`EXIT_FAILURE`
sentinel, a silent `return` guard, a silent truncation, or unchecked-pointer
UB (the C never null-checks its `manager` / `description` / `tasks`
parameters).

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `initialize_logger` | `fopen(path,"a")` fails — `LOG_FILE` names a path that cannot be opened for append (missing directory) | writes `Failed to open log file: <path>\n` to `stderr`; leaves `log_file` unchanged; returns `-1` | `errors.rs::e01_initialize_logger_fopen_missing_dir` |
| 2 | `initialize_logger` | `LOG_FILE=""` → `fopen("", "a")` fails (ENOENT) | same as #1 with an empty `%s` | `errors.rs::e02_initialize_logger_empty_path` |
| 3 | `initialize_logger` | `LOG_FILE` is an existing **directory** → `fopen` fails (EISDIR) | same as #1 | `errors.rs::e03_initialize_logger_path_is_dir` |
| 4 | `initialize_logger` | `LOG_FILE` path component is not a directory (`<file>/x`) → ENOTDIR | same as #1 | `errors.rs::e04_initialize_logger_enotdir` |
| 5 | `log_info` | called while `log_file == NULL` (before any successful `initialize_logger`) | silent no-op, no output on any stream | `errors.rs::e05_log_fns_noop_when_uninitialised` |
| 6 | `log_warning` | called while `log_file == NULL` | silent no-op | `errors.rs::e05_log_fns_noop_when_uninitialised` |
| 7 | `log_error` | called while `log_file == NULL` | silent no-op | `errors.rs::e05_log_fns_noop_when_uninitialised` |
| 8 | `finalize_logger` | called while `log_file == NULL` | silent no-op, no `fclose`, no `[INFO] Logger finalized.` | `errors.rs::e08_finalize_noop_when_uninitialised` |
| 9 | `log_info` / `log_warning` / `log_error` | `message == NULL` while stream open — no null check, passed to `%s` | glibc prints the literal `(null)`: `[INFO] (null)\n` etc. | `errors.rs::e09_log_null_message` |
| 10 | `create_task_manager` | `malloc(sizeof(TaskManager))` returns NULL | `log_error("Failed to allocate memory for TaskManager.")` then `return NULL` | `errors.rs::e10_manager_malloc_failure_unreachable` (documented-unreachable: 16-byte alloc; equivalence of the *reachable* consequence — a `NULL` return being propagated — is covered by #11/#12) |
| 11 | `create_task_manager` | `MAX_TASKS` negative (e.g. `-1`): `(size_t)(int)-1 * 260` wraps to `2**64-260`, `malloc` fails | `log_error("Failed to allocate memory for tasks.")`, `free(manager)`, `return NULL` (and the `[ERROR]` line lands in the log only if a logger is already open) | `errors.rs::e11_create_negative_max_tasks` |
| 12 | `create_task_manager` | `MAX_TASKS` huge positive (`2147483647`) → 558 GB request `malloc` fails | same as #11 | `errors.rs::e12_create_huge_max_tasks` |
| 13 | `add_task` | `manager->task_count >= manager->max_tasks` (full, incl. `MAX_TASKS=0` where it is full from the start) | `log_warning("Cannot add task: Maximum task limit reached.")`, returns without touching `tasks` / `task_count` | `errors.rs::e13_add_task_when_full` |
| 14 | `add_task` | `description` longer than 255 bytes — no error, silent truncation by `strncpy(.., 255)` + forced `[255]=0` | first 255 bytes copied, byte 255 forced to `\0`; task still added, returns void | `errors.rs::e14_add_task_truncates_long_description` |
| 15 | `add_task` | `manager == NULL` (no null check → `(*manager).task_count`) | dereferences NULL → `SIGSEGV` | `errors.rs::e15_add_task_null_manager` (fork-based) |
| 16 | `add_task` | `description == NULL` (no null check → `strncpy(dst, NULL, 255)`) | `SIGSEGV` inside `strncpy` | `errors.rs::e16_add_task_null_description` (fork-based) |
| 17 | `print_tasks` | `manager == NULL` (no null check → `(*manager).task_count`) | prints `Tasks:\n` **first**, then `SIGSEGV` | `errors.rs::e17_print_tasks_null_manager` (fork-based) |
| 18 | `destroy_task_manager` | `manager == NULL` (no null check → `free((*manager).tasks)`) | `SIGSEGV` before any log line | `errors.rs::e18_destroy_null_manager` (fork-based) |
| 19 | `driver` | `initialize_logger()` returned non-zero (rows #1–#4) | returns `EXIT_FAILURE` (`1`) immediately; **no** `TaskManager`, **no** `Tasks:` output, **no** `finalize_logger` | `errors.rs::e19_driver_logger_failure` |
| 20 | `driver` | `create_task_manager()` returned NULL (rows #11/#12) | returns `EXIT_FAILURE` (`1`); logger deliberately left **open** (no `finalize_logger`, no `[INFO] Logger finalized.`) | `errors.rs::e20_driver_manager_failure` |
| 21 | `driver` | `malloc(length + 1)` for the extracted task fails | `fprintf(stderr,"Error: Failed to allocate memory for task.\n")`, `destroy_task_manager`, `finalize_logger`, return `EXIT_FAILURE` | `errors.rs::e21_driver_task_malloc_failure_unreachable` (documented-unreachable: `length+1` is bounded by the caller's string; both sides emit the byte-identical branch, verified by source inspection + the identical `stderr` literal asserted in the file) |
| 22 | `driver` | `tasks == NULL` (no null check → `*start`) | `SIGSEGV` — *after* `initialize_logger` and `create_task_manager` already ran (so `[INFO] Logger initialized.` / `[INFO] TaskManager created successfully.` are already in the log buffer) | `errors.rs::e22_driver_null_input` (fork-based) |
| 23 | `finalize_logger` | called twice — C never resets `log_file` to `NULL` after `fclose` | second call does `fprintf` + `fclose` on a **freed** `FILE*`. Undefined behaviour, **not** a rejection: see "Nondeterministic rows" below | `errors.rs::e23_double_finalize` (structural + fork status class, 12 runs/side) |
| 24 | `log_info` / `log_warning` / `log_error` | called *after* `finalize_logger` (dangling `log_file`, not `NULL`) | `fprintf` on a freed stream — same UB caveat as row 23 | `errors.rs::e24_log_after_finalize` (fork status class, 12 runs/side) |
| 24b | `log_*` / `finalize_logger` | the guard is `if (log_file)`, a raw pointer test; nothing but `initialize_logger` ever assigns the static | a defensive reset-to-NULL would silently turn rows 23/24 into no-ops | `errors.rs::e24b_dangling_log_file_is_still_treated_as_open` (structural, deterministic) |
| 25 | `create_task_manager` | `MAX_TASKS` not a number (`"abc"`, `""`, `"   "`, `"+"`) → `atoi` yields `0` | `max_tasks == 0`, manager created with a `malloc(0)` (non-NULL) `tasks`; every later `add_task` hits row #13 | `errors.rs::e25_max_tasks_non_numeric` |
| 26 | `create_task_manager` | `MAX_TASKS` overflows `int` (`"99999999999999"`, `"-99999999999999"`, `"0x10"`) → `atoi` = `(int)strtol` saturation/truncation | whatever `atoi` yields, then row #11/#12 or a valid alloc — must match bit-for-bit | `errors.rs::e26_max_tasks_int_overflow` |
| 27 | *(generic FFI boundary)* | out-of-range "enum" value across FFI: `priority` given `INT_MIN`, `INT_MAX`, `-1`, `0` — `int` accepts any value, there is no valid-range check anywhere | value stored verbatim in `task->priority` and printed with `%d` | `errors.rs::e27_priority_out_of_range` |
| 28 | *(generic FFI boundary)* | zero-length input: `driver("")` | `initialize_logger`, `create_task_manager`, `printf("Tasks:\n")`, destroy, finalize, return `0` | `errors.rs::e28_driver_empty_input` |
| 29 | *(generic FFI boundary)* | oversized input: `driver` with a 64 KiB single line and with 5000 lines | truncation per row #14 and limit per row #13, return `0` | `errors.rs::e29_driver_oversized_input` |
| 30 | *(generic FFI boundary)* | one step past the documented range: description of exactly 253 / 254 / 255 / 256 / 257 bytes | 255-byte truncation boundary | `errors.rs::e30_description_length_boundary` |
| 31 | `add_task` | caller-supplied `task_count = -1`, `max_tasks = 10`: `-1 >= 10` is false, so the gate passes and `&tasks[-1]` writes **before** the array | write at index −1; `task_count` becomes 0 | `errors.rs::e31_negative_task_count_writes_before_array` |
| 32 | `add_task` | caller-supplied `task_count = -5`, `max_tasks = 0` | write at index −5; `task_count` becomes −4 | `errors.rs::e32_negative_count_and_zero_cap` |
| 33 | `add_task` | `task_count = INT_MAX`, `max_tasks = INT_MIN` → gate rejects | `log_warning`, no write | `errors.rs::e33_int_max_count_int_min_cap_is_rejected` |
| 34 | `add_task` | `task_count = max_tasks = INT_MAX` → gate rejects | `log_warning`, no write | `errors.rs::e34_equal_extremes_are_rejected` |
| 35 | `add_task` | `task_count = 0`, `max_tasks = -1` → `0 >= -1` rejects | `log_warning`, no write | `errors.rs::e35_zero_count_negative_cap_is_rejected` |
| 36 | `add_task` | `task_count = INT_MAX - 1`, `max_tasks = INT_MAX`: gate passes, then indexes ~558 GB past the array | `SIGSEGV` | `errors.rs::e36_out_of_range_index_faults_identically` (fork-based) |
| 37 | `initialize_logger` | `LOG_FILE` longer than `PATH_MAX` → ENAMETOOLONG | `stderr` message + `return -1` | `errors.rs::e37_initialize_logger_path_too_long` |
| 38 | `initialize_logger` | `LOG_FILE` exists but mode `0444` → EACCES on `"a"` | `stderr` message + `return -1` | `errors.rs::e38_initialize_logger_permission_denied` |

## Nondeterministic rows (23 / 24)

Rows 23 and 24 are **not rejections** — they are use-after-`fclose`. Their
manifestation depends on the allocator's state, not on the code. Running the
*unmodified C library alone*, three times in a row, gives:

```
C   run1 -> 0
C   run2 -> 134   (free(): double free detected in tcache 2)
C   run3 -> 0
```

So there is no C-defined error code or sentinel to match, and an exact status
comparison would be a coin flip. These rows are therefore pinned down where the
behaviour *is* deterministic:

* **structurally** — `logger.c` contains exactly one assignment to `log_file`
  (the `fopen` in `initialize_logger`), and all four consumers guard on
  `if (log_file)`; the translation is asserted to have exactly the same shape
  (`e24b`). This is what actually rules out the plausible wrong translation, a
  defensive `log_file = NULL` in `finalize_logger`, which would convert both
  rows into silent no-ops.
* **dynamically** — over 12 forked children per side, every observed
  termination status on both sides must fall in the same allowed set
  `{exited(0), SIGABRT, SIGSEGV}` (`e23`, `e24`). In particular a Rust-side
  *panic* would show up as an extra `stderr` diagnostic and fail the harness's
  byte comparison.
* the **well-defined** half — with `log_file` still `NULL` all four functions
  are guaranteed clean no-ops — is compared live (`e05`, `e08`, `e24b`).

## Notes on unreachable rows

Rows #10 and #21 are `malloc`-failure branches for a 16-byte and an
`n+1`-byte allocation. They cannot be triggered from the public API without
interposing the allocator, which would change *both* libraries identically and
prove nothing about the translation. Both are instead verified by direct source
comparison (identical literals, identical ordering of `log_error` /
`fprintf` / `free` / `destroy` / `finalize` / return value), and their
*observable consequence* (a `NULL` manager propagating out of `driver` as
`EXIT_FAILURE` without `finalize_logger`) **is** exercised live by rows
#11/#12/#20. The corresponding tests assert the source-level invariants.

## Divergence found and fixed

Phase C caught one **real** divergence, in exactly the class the null-pointer
rows exist to catch (rows 15, 16, 17, 18, 22, 36).

`c_src` never null-checks a single parameter, so handing the C library a NULL
`TaskManager *` / `const char *` produces a raw hardware null dereference:

```
C    : signal(11)+core     (SIGSEGV, nothing on stderr)
```

The Rust `.so`, built with the default `dev` profile, instead produced:

```
Rust : signal(6)+core
       thread '<unnamed>' panicked at src/task_manager.rs:78:12:
       null pointer dereference occurred
```

Cause: `debug-assertions` makes rustc inject `ub_checks` into the crate, which
turn the dereference into a Rust panic-then-abort *with a diagnostic on stderr*.
That is an observable behavioural difference across the FFI boundary — a
different signal, and 200+ extra bytes on `stderr`.

Fix (in `translation/Cargo.toml`, since the C compiler injects no such checks
and `c_src` contains no `assert`s):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false

[profile.release]
panic = "abort"
debug-assertions = false
overflow-checks = false
```

After the fix all six rows report `signal(11)+core` on both sides with empty
`stderr`. The `[profile.release]` settings were made explicit so the property
cannot regress if someone flips `debug-assertions` on for release builds.

Note also that `cargo test` does **not** build a `crate-type = ["cdylib"]`
library (the test harness cannot link it), so the `.so` must be produced by an
explicit `cargo build` first. `tests/common/mod.rs` therefore refuses to run
against a missing *or stale* `.so`, and `run_verification.sh` builds before
testing in every profile.

## Harness self-check (is the suite vacuous?)

A passing suite only means something if it can fail. `./mutation_check.sh`
injects 20 plausible translation mistakes into `translation/src`, one at a time,
and asserts each is caught:

```
CAUGHT  logger tag [INFO] -> [Info]                45 failing test(s)
CAUGHT  logger tag [WARNING] -> [WARN]             17
CAUGHT  initialize_logger returns -2                6
CAUGHT  default log name changed                    2
CAUGHT  fopen mode a -> w                          12
CAUGHT  finalize_logger resets the static           2   <- rows 23/24b
CAUGHT  default max_tasks 10 -> 11                  8
CAUGHT  strncpy limit 255 -> 254                   13
CAUGHT  forced NUL at 254 not 255                  14
CAUGHT  capacity gate >= becomes >                  1
CAUGHT  print index i+1 -> i                       28
CAUGHT  print header text                          31
CAUGHT  size uses usize not sign-extend             1   <- row 11 (structural)
CAUGHT  destroy frees in reverse order              1
CAUGHT  defensive null check in add_task            1   <- row 15
CAUGHT  EXIT_FAILURE 1 -> 2                         2   <- rows 19/20
CAUGHT  priority not incremented                   13
CAUGHT  newline skip off by one                     1
CAUGHT  driver forgets finalize_logger             17
CAUGHT  driver returns 0 on logger failure          1

ALL 20 MUTATIONS DETECTED
```

The script always restores `translation/src` (via an `EXIT`/`INT`/`TERM` trap)
and never touches `c_src`.

Note on row 11: `max_tasks as isize as usize` → `as u32 as usize` is an
*equivalent mutant* dynamically — sign-extension asks for `2**64-260` bytes and
zero-extension for ~1.1 TB, and both `malloc`s return NULL on any realistic
host (verified: 558 GB, 1.1 TB and `2**64-260` all fail here, and glibc rejects
anything above `PTRDIFF_MAX` outright). It is therefore pinned by a structural
assertion in `e11_sign_extension_of_max_tasks` instead of by a value comparison.
