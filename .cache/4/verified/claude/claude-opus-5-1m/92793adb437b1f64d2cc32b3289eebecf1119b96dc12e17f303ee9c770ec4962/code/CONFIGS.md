# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for inputs the C library **accepts**. Axes below are
derived mechanically from the branches the C source actually takes.

## Axes the C code branches on

### Runtime options (this library's only "options" are environment variables)

| axis | read at | C branch | states |
|------|---------|----------|--------|
| `LOG_FILE` | `logger.c:34-35` | `log_file_env ? log_file_env : "default.log"` | **L0** unset → `"default.log"` (relative to CWD) · **L1** set to a fresh path · **L2** set to an already-existing non-empty file (`fopen(...,"a")` must **append**, not truncate) |
| `MAX_TASKS` | `task_manager.c:39-40` | `max_tasks_env ? atoi(max_tasks_env) : 10` | **M0** unset → `10` · **M1** `"0"` · **M2** `"1"` · **M3** `"3"` · **M4** `"10"` · **M5** `"64"` · **M6** `"abc"` → `atoi`=0 · **M7** `"   7"` (leading whitespace) · **M8** `"+7"` · **M9** `"7abc"` (trailing garbage) · **M10** `"0x10"` → 0 · **M11** `"99999999999999"` (`atoi` overflow — glibc truncates the `long` to `int`) |

`atoi` is called through the *same* libc by both implementations, so every
parsing quirk above must agree bit-for-bit.

### Input shapes the code special-cases

| axis | C branch | shapes |
|------|----------|--------|
| task-list text (`driver.c:45-66`) | `while (*start != '\0')`, `strchr(start,'\n')`, `end==NULL`, `(*end=='\n') ? end+1 : end` | **S0** `""` (loop never entered) · **S1** one line, no trailing `\n` (`strchr` → NULL path) · **S2** one line **with** trailing `\n` · **S3** many lines, no trailing `\n` · **S4** many lines **with** trailing `\n` · **S5** leading `\n` (empty first task) · **S6** consecutive `\n\n` (empty middle task) · **S7** only newlines · **S8** CRLF (`\r` stays in the description) · **S9** bytes ≥ 0x80 (`c_char` is signed → sign-extension traps) · **S10** text containing `%s`/`%d`/`%n` · **S11** tabs & other control bytes |
| description length (`task_manager.c:60-61`) | `strncpy(...,255)` + forced NUL at `[255]` | **D0** 0 · **D1** 1 · **D2** 254 · **D3** **255** (last length kept whole) · **D4** **256** (first truncated) · **D5** 300 · **D6** 1000 |
| task count vs limit (`task_manager.c:54`) | `task_count >= max_tasks` | **N0** 0 tasks · **N1** 1 · **N2** `max_tasks-1` · **N3** exactly `max_tasks` · **N4** `max_tasks+k` (overflow → dropped) |
| `priority` (`int`, stored verbatim, printed with `%d`) | none | **P0** 1.. (as `driver` generates) · **P1** 0 · **P2** −1 · **P3** `INT_MIN` · **P4** `INT_MAX` |
| `TaskManager` shape given to `print_tasks` (caller-owned struct) | `for (i=0; i<task_count; i++)` | **T0** `task_count=0` · **T1** 1 · **T2** many · **T3** `task_count < max_tasks` (partially filled array) |

### Full set of public entry points

Low-level (called directly, **not** only through the `driver` wrapper):
`initialize_logger`, `log_info`, `log_warning`, `log_error`, `finalize_logger`,
`create_task_manager`, `add_task`, `print_tasks`, `destroy_task_manager`.
Composed one-shot wrapper: `driver`.

Observables compared for every row: return value · captured **stdout** bytes ·
captured **stderr** bytes · **log-file** bytes · and, where a `TaskManager` is
reachable, the raw struct fields (`max_tasks`, `task_count`) and all 260 bytes
of every `Task` slot.

## Table

| #  | entry point(s) | configuration (options set + input shape) | test | ok |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `initialize_logger`, `finalize_logger` | L1 fresh path; open → finalize; assert log is exactly `[INFO] Logger initialized.` + `[INFO] Logger finalized.` | `cfg_01_logger_init_finalize` | [x] |
| 2  | `initialize_logger`, `log_info`/`log_warning`/`log_error`, `finalize_logger` | L1; every severity, randomized message bodies (100 seeds) incl. empty, 1-byte, long, `%`-bearing, non-ASCII, embedded `\t`/`\r` | `cfg_02_logger_all_severities_random` | [x] |
| 3  | `initialize_logger` … | **L2** LOG_FILE already contains data → `fopen(...,"a")` must append; assert pre-existing bytes preserved | `cfg_03_logger_append_existing` | [x] |
| 4  | `initialize_logger` … | **L0** `LOG_FILE` unset → writes `./default.log` relative to CWD (test chdirs into a private dir) | `cfg_04_logger_default_path` | [x] |
| 5  | `initialize_logger` ×2, `finalize_logger` | L1; **re-initialise without finalising** (C overwrites the static, leaking the first handle) → `[INFO] Logger initialized.` twice | `cfg_05_logger_double_init` | [x] |
| 6  | `initialize_logger`, `finalize_logger`, `initialize_logger`, `finalize_logger` | L1; two full open/close cycles on the same path → 4 lines appended in order | `cfg_06_logger_two_cycles` | [x] |
| 7  | `create_task_manager`, `destroy_task_manager` | **M0** unset → `max_tasks==10`, `task_count==0`; log gets `TaskManager created successfully.` / `destroyed successfully.` | `cfg_07_create_default_max` | [x] |
| 8  | `create_task_manager` | **M1–M5** numeric `MAX_TASKS` (`0`,`1`,`3`,`10`,`64`) → `max_tasks` equals the value; struct fields compared raw | `cfg_08_create_numeric_max` | [x] |
| 9  | `create_task_manager` | **M6–M10** `atoi` quirks: `"abc"`→0, `"   7"`→7, `"+7"`→7, `"7abc"`→7, `"0x10"`→0 | `cfg_09_create_atoi_quirks` | [x] |
| 10 | `create_task_manager` | **M11** `"99999999999999"` → `atoi` overflow (glibc: `long`→`int` truncation); whatever `max_tasks` results must match, and so must success/failure of the `max_tasks*260` allocation | `cfg_10_create_atoi_overflow` | [x] |
| 11 | `create_task_manager`, `add_task`, `print_tasks`, `destroy_task_manager` | M0; **N0** no tasks → `print_tasks` emits only `Tasks:\n` | `cfg_11_tm_zero_tasks` | [x] |
| 12 | …same 4 | M0; **N1** exactly one task, **D1** short description, **P0** | `cfg_12_tm_one_task` | [x] |
| 13 | …same 4 | M0; **N2/N3** fill to `max_tasks-1` then exactly `max_tasks`; randomized descriptions + priorities (60 seeds) | `cfg_13_tm_fill_to_limit_random` | [x] |
| 14 | …same 4 | **M5** `MAX_TASKS=64`, **N4** 80 `add_task` calls → 64 stored + 16 warnings, `task_count` stays 64 | `cfg_14_tm_overflow_beyond_max` | [x] |
| 15 | `add_task`, `print_tasks` | **D0–D6** description lengths 0,1,254,**255**,**256**,300,1000 — verifies `strncpy` 255-byte truncation and the forced `[255]=0`, plus `strncpy`'s NUL **zero-padding** of the rest of the 256-byte field (all 260 struct bytes compared) | `cfg_15_tm_description_lengths` | [x] |
| 16 | `add_task`, `print_tasks` | **P1–P4** priorities `0`, `-1`, `INT_MIN`, `INT_MAX` (+ random `i32`s) — `%d` rendering compared | `cfg_16_tm_priority_extremes` | [x] |
| 17 | `add_task`, `print_tasks` | **S9/S10/S11** descriptions holding bytes ≥ 0x80, `%s`/`%d`/`%n` sequences, tabs, `\r`, and every byte 0x01–0xFF | `cfg_17_tm_byte_range_descriptions` | [x] |
| 18 | `print_tasks` **on a caller-crafted `TaskManager`** | **T0–T3**: `task_count` = 0 / 1 / 32, partially-filled array (`task_count < max_tasks`), randomized contents — exercises struct-layout compatibility from the caller side | `cfg_18_print_caller_struct_random` | [x] |
| 19 | `add_task` **on a caller-crafted `TaskManager`** (test-`malloc`'d) | M-independent: `max_tasks` 1/2/8, `task_count` pre-set to 0/1/`max-1`, randomized descriptions; then `print_tasks`, then `destroy_task_manager` (frees the test's `malloc` blocks) | `cfg_19_add_caller_struct_random` | [x] |
| 20 | `create_task_manager` from one impl, `add_task`/`print_tasks` from the same impl, interleaved lifecycles ×3 | M3; three managers alive at once, tasks added round-robin, printed, destroyed in a different order | `cfg_20_tm_multiple_managers` | [x] |
| 21 | `driver` | L1, M0; **S0** `""` → only `Tasks:\n`, returns 0 | `cfg_21_driver_empty_input` | [x] |
| 22 | `driver` | L1, M0; **S1** single line, **no** trailing newline | `cfg_22_driver_single_no_nl` | [x] |
| 23 | `driver` | L1, M0; **S2** single line **with** trailing newline | `cfg_23_driver_single_with_nl` | [x] |
| 24 | `driver` | L1, M0; **S3/S4** 2..9 lines, with and without trailing newline | `cfg_24_driver_multi_lines` | [x] |
| 25 | `driver` | L1, M0; **S5/S6/S7** leading `\n`, `a\n\nb`, `"\n"`, `"\n\n\n"` → empty tasks are still added | `cfg_25_driver_empty_lines` | [x] |
| 26 | `driver` | L1, M0; **S8** CRLF input `a\r\nb\r\n` (the `\r` must stay in the stored description) | `cfg_26_driver_crlf` | [x] |
| 27 | `driver` | L1, M0; **D2–D6** lines of length 254/255/256/300/1000 → truncation inside the composed pipeline | `cfg_27_driver_long_lines` | [x] |
| 28 | `driver` | L1, M0; **S9/S10/S11** UTF-8 + high bytes + `%s%d%n` + tabs in the task text | `cfg_28_driver_odd_bytes` | [x] |
| 29 | `driver` | L1, **M1** `MAX_TASKS=0` with 3 lines → 3 warnings, empty task list, returns 0 | `cfg_29_driver_max_zero` | [x] |
| 30 | `driver` | L1, **M2/M3** `MAX_TASKS=1`/`3` with **N3** exactly that many lines and **N4** more → `priority` keeps counting through dropped lines | `cfg_30_driver_max_boundary` | [x] |
| 31 | `driver` ×2 in one process | L1 same path, M0 → second run **appends**; log holds both cycles in order | `cfg_31_driver_twice_appends` | [x] |
| 32 | `driver` | **L0** `LOG_FILE` unset (`./default.log`) + M0, real multi-line input | `cfg_32_driver_default_log` | [x] |
| 33 | `driver` | L1, **M0/M2/M3/M5** × randomized task text (200 seeds: random line count 0-30, random line lengths 0-400, random bytes incl. `\n`, `\r`, `%`, 0x80-0xFF) — the full property-style fuzz over the composed pipeline | `cfg_33_driver_fuzz_random` | [x] |
| 34 | mixed low-level sequence | L1, M3: `initialize_logger` → `log_warning` → `create_task_manager` → `add_task`×4 → `print_tasks` → `log_error` → `destroy_task_manager` → `finalize_logger`, i.e. the pipeline hand-assembled from the lowest-level exports rather than via `driver`; randomized payloads (50 seeds) | `cfg_34_manual_pipeline_random` | [x] |
| 35 | `driver` after a manual `initialize_logger` | L1, M0: logger already open, then `driver` re-opens it (leaks the first handle) → log content ordering | `cfg_35_driver_after_manual_init` | [x] |

Row 33 and rows 2/13/16/17/18/19/34 are the property-style rows: each uses a
fixed-seed PRNG (`common::Rng::new(SEED)`) and many generated inputs, so a row
is only checked off after the whole randomized sweep agrees.

## Result

All 35 rows pass: for each row the scenario is driven against **both** `.so`s
through `libloading`/`dlsym` and these observables are compared byte-for-byte:

* the return value of every call, in call order;
* every byte written to `stdout` (`printf` from `print_tasks`);
* every byte written to `stderr` (`fprintf(stderr, ...)`);
* every byte of the log file the run produced (including `fopen(..., "a")`
  append behaviour and the records left behind by leaked/unfinalized handles);
* the raw bytes of the `TaskManager` struct and of all populated 260-byte `Task`
  slots — which pins `strncpy`'s NUL zero-padding and the C struct layout.

Rows 2, 13, 16, 17, 18, 19, 33 and 34 are property-style: each drives a
fixed-seed `Rng` over many generated inputs (row 33 alone runs 200 randomized
task lists crossed with four `MAX_TASKS` settings), so a row is only checked off
once the whole sweep agrees.

Verified in `dev` and `release`, for every declared feature combination
(`./check_all_features.sh test`).
