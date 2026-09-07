# ERRORS.md — Phase C error-surface table

Every distinct rejection / error path in the C source, found by grepping for
`return -1`, `return NULL`, `return EXIT_FAILURE`, `if (!...)`, every `log_error`
/ `log_warning` call, every guard `if`, and every size constant. There are no
`assert`s and no error enums in this library; rejection is signalled by sentinel
return values (`-1`, `NULL`, `EXIT_FAILURE`) or by a silent no-op.

Constants that bound the input surface:
`sizeof(Task::description) == 256` (so 255 usable chars + NUL),
default `max_tasks == 10`, default log path `"default.log"`,
`EXIT_FAILURE == 1`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `initialize_logger` | `fopen(path,"a")` fails — `LOG_FILE` names a path that cannot be opened for append (missing directory component, e.g. `/nonexistent-dir-xyz/l.log`) | writes `Failed to open log file: <path>\n` to `stderr`; returns `-1`; `log_file` stays `NULL` | [x] |
| E2 | `initialize_logger` | `fopen` fails because `LOG_FILE` is an existing **directory** | same as E1: `stderr` message, returns `-1` | [x] |
| E3 | `initialize_logger` | `fopen` fails because `LOG_FILE` is the empty string `""` | same as E1: `stderr` message, returns `-1` | [x] |
| E4 | `log_info` | called while `log_file == NULL` (before `initialize_logger`, or after a failed one) | silent no-op, no output, no crash | [x] |
| E5 | `log_warning` | called while `log_file == NULL` | silent no-op | [x] |
| E6 | `log_error` | called while `log_file == NULL` | silent no-op | [x] |
| E7 | `finalize_logger` | called while `log_file == NULL` (never initialised / init failed) | silent no-op — does **not** write `Logger finalized.` and does **not** `fclose` | [x] |
| E8 | `create_task_manager` | `malloc(sizeof(TaskManager))` returns `NULL` | `log_error("Failed to allocate memory for TaskManager.")`; returns `NULL` | [x] (not reachable: a 16-byte `malloc` does not fail. Asserted mechanically that the branch's string literal is present in BOTH `.so`s; its sibling bail-out path E9 is executed for real) |
| E9 | `create_task_manager` | `malloc(max_tasks * sizeof(Task))` returns `NULL` — `MAX_TASKS` is huge (e.g. `2000000000` → 520 000 000 000 bytes) | `log_error("Failed to allocate memory for tasks.")`; `free(manager)`; returns `NULL` | [x] |
| E10 | `create_task_manager` | `MAX_TASKS` **negative** (e.g. `-1`): `(size_t)(int)-1 * 260` wraps to a colossal size | `malloc` fails → `log_error("Failed to allocate memory for tasks.")`, `free`, returns `NULL` | [x] |
| E11 | `create_task_manager` | `MAX_TASKS` = `INT_MIN` / `-2147483648` (extreme sign-extension + wrap) | same wrap-then-fail path as E10 → `NULL` | [x] |
| E12 | `add_task` | `manager->task_count >= manager->max_tasks` (capacity full) | `log_warning("Cannot add task: Maximum task limit reached.")`; returns without touching `task_count` or the array | [x] |
| E13 | `add_task` | `max_tasks <= 0` (e.g. `MAX_TASKS=0`, or `MAX_TASKS=abc` → `atoi` = 0), so the *first* `add_task` already has `0 >= 0` | `log_warning(...)`, task rejected; `task_count` stays `0` | [x] |
| E14 | `add_task` | `description` longer than 255 bytes | **not** an error: silently truncated to 255 bytes + forced NUL at `description[255]` | [x] |
| E15 | `add_task` / `print_tasks` / `destroy_task_manager` | `manager == NULL` | dereferences `NULL` → `SIGSEGV`. Undefined behaviour, identically UB in Rust (raw-pointer deref); neither side has a null guard. **Executed** as a differential test in a forked child, asserting both terminate with the identical signal (`signal(11)` for the C and for the release Rust cdylib) — see "How the UB rows are compared" below. | [x] |
| E16 | `log_info` / `log_warning` / `log_error` | `message == NULL` while `log_file != NULL` | passed as `%s` to glibc `fprintf` → glibc renders `(null)`, so the line is `[INFO] (null)`. Rust calls the same `fprintf`, so identical. | [x] |
| E17 | `driver` | `initialize_logger()` returned non-zero (i.e. trigger E1/E2/E3 via `LOG_FILE`) | returns `EXIT_FAILURE` (`1`) immediately; no `TaskManager`, nothing on `stdout` | [x] |
| E18 | `driver` | `create_task_manager()` returned `NULL` (trigger E9/E10/E11 via `MAX_TASKS`) | returns `EXIT_FAILURE` (`1`); nothing on `stdout`; the log file is left **open and un-finalised** (no `Logger finalized.` line) — bug-for-bug behaviour | [x] |
| E19 | `driver` | `malloc(length + 1)` for the extracted task returns `NULL` | `fprintf(stderr, "Error: Failed to allocate memory for task.\n")`; `destroy_task_manager`; `finalize_logger`; returns `EXIT_FAILURE` | [x] (not reachable: the allocation is at most `strlen(tasks)+1`. Asserted mechanically instead: the branch's string literal must be present in BOTH `.so`s, so a dropped or stubbed branch fails the test) |
| E20 | `driver` | `tasks == NULL` | `*start` dereferences `NULL` → `SIGSEGV`, after the logger and manager have been set up. Identically UB on both sides; **executed** in a forked child and the termination signal compared, exactly as E15. | [x] |
| E21 | `driver` | `tasks` is the empty string `""` | loop body never runs; prints just `Tasks:\n`; returns `0` | [x] |
| E22 | `finalize_logger` | called **twice** — the C never resets `log_file` to `NULL`, so the 2nd call `fprintf`s to and `fclose`s an already-closed `FILE*` | Undefined behaviour (use-after-free inside glibc). **Measured to be nondeterministic in the C itself**: the same C `.so` produced both `exited(0)` and `SIGABRT` depending on whether the freed `FILE` had been recycled, so there is no stable result to compare. Asserted instead at the source level: neither `finalize_logger` clears its static after `fclose`, so the Rust reaches the identical use-after-free state and does not "fix" the bug (`e22_double_finalize_logger_state_mirrors_c`). | [x] (source-mirror assertion) |
| E23 | `create_task_manager` | the `max_tasks * sizeof(Task)` conversion: C sign-extends the `int` to `size_t` and lets the product wrap mod 2^64 | For every negative `int32` the sign-extending and a hypothetical zero-extending translation both yield ≥ 558 GB, so `malloc` returns `NULL` either way — **no input can distinguish them at run time** (measured). Differentially asserted `NULL` for all boundary + 12 random negative capacities, and the conversion pinned at the source level (`size_conversion_semantics_mirror_c`). | [x] |

## Generic FFI-boundary boundaries also covered by `tests/phase_c.rs`

* null pointers: `message == NULL` (E16, executed in-process), `description == NULL`
  (E16b) and `manager == NULL` / `tasks == NULL` (E15/E20) — the UB ones executed
  in a forked child with the termination signal compared.
* zero lengths: `MAX_TASKS=0` (E13), empty `description` `""`, empty `tasks` `""` (E21),
  empty `LOG_FILE` `""` (E3).
* oversized lengths: `description` of 255 / 256 / 257 / 1024 / 100 000 bytes (E14),
  `MAX_TASKS=2000000000`, `MAX_TASKS=99999999999999999999` (`atoi` overflow).
* one step past a valid range: `add_task` call number `max_tasks + 1` (E12);
  `MAX_TASKS = -1` i.e. one below the smallest sane capacity (E10);
  `priority = INT_MIN` / `INT_MAX` and the `priority++` overflow in `driver`.
* out-of-range enum values across the FFI boundary: **this library declares no
  `enum` and takes no mode/flag parameter** — every parameter is a `const char*`,
  a `TaskManager*`, or a plain `int` whose whole 32-bit range is legal input.
  The `int` domain is therefore fuzzed over its extremes instead
  (`priority` ∈ {`INT_MIN`, `-1`, `0`, `1`, `INT_MAX`} plus random `i32`s), which
  is the exact analogue of the "value with no valid variant" case here.

## Result

All 23 rows checked (E1–E23). `tests/phase_c.rs`: **26 tests, 26 passing**, in every
feature combination and in both the `debug` and `release` profiles.

Row → test mapping:

| rows | test |
|------|------|
| E1 | `e01_initialize_logger_missing_directory` |
| E2 | `e02_initialize_logger_path_is_directory` |
| E3 | `e03_initialize_logger_empty_path` |
| E4, E5, E6 | `e04_log_info_uninitialised`, `e05_log_warning_uninitialised`, `e06_log_error_uninitialised` |
| E7 | `e07_finalize_logger_uninitialised` |
| E8 | `e08_taskmanager_alloc_failure_branch_present` |
| E9 | `e09_create_task_manager_alloc_too_large` |
| E10 | `e10_create_task_manager_negative_capacity` |
| E11 | `e11_create_task_manager_int_min_capacity` |
| E12 | `e12_add_task_past_capacity` |
| E13 | `e13_add_task_zero_capacity` |
| E14 | `e14_add_task_description_truncated` |
| E15 | `e15_null_manager_pointer` |
| E16 | `e16_log_null_message`, `e16b_add_task_null_description` |
| E17 | `e17_driver_logger_failure` |
| E18 | `e18_driver_manager_failure` |
| E19 | `e19_driver_task_alloc_failure_branch_present` |
| E20 | `e20_driver_null_input` |
| E21 | `e21_driver_empty_string` |
| E22 | `e22_double_finalize_logger_state_mirrors_c` |
| E23 | `size_conversion_semantics_mirror_c` |
| generic boundaries | `generic_int_domain_across_ffi`, `generic_max_tasks_boundaries` |

### How the UB rows are compared

E15, E16b and E20 are genuine null-dereferences. They are run in a **forked
child** (`common::fork_status`) and the two implementations' *termination status*
is compared, so the assertion is a real equality check rather than "both failed
somehow", and a crash cannot take the harness down.

One documented, build-profile-only difference exists on those rows: with
`-C debug-assertions` (cargo's `dev`/`test` profile) Rust turns `*ptr` on a null
raw pointer into a panic → `SIGABRT`, where the C takes a hardware fault →
`SIGSEGV`. The **release cdylib — the artifact this crate ships** — faults
identically to the C (`signal(11)` on both), which the same tests assert.
`common::rust_has_debug_assertions()` detects the situation mechanically (by
looking for the precondition panic message inside the `.so`) and
`assert_same_ub_status` permits *only* that exact `signal(11)` ↔ `signal(6)`
pairing; every other combination, including either side exiting normally, fails.

Debug assertions are deliberately left **enabled** so Phases B and C also prove
the translation never relies on a panicking arithmetic operation where the C
wraps (the translation uses `wrapping_*` / `as isize as usize` throughout).
