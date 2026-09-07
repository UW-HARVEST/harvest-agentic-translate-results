# CONFIGS.md — configuration-surface table (Phase B)

## Axes derived from the C source

**Runtime options.** The public API takes no option/flag arguments; the *only*
runtime configuration the C branches on is the environment:

| option | read at | branch in C | values that matter |
|--------|---------|-------------|--------------------|
| `LOG_FILE` | `logger.c:34` `getenv("LOG_FILE")` | ternary: env vs `"default.log"`; then `fopen` success/failure at `logger.c:38` | unset / writable path / unopenable path |
| `MAX_TASKS` | `task_manager.c:39` `getenv("MAX_TASKS")` | ternary: `atoi(env)` vs `10`; then `malloc` success/failure at `:43`; then the capacity guard at `:54` | unset / `"0"` / `"1"` / `"3"` / `"10"` / non-numeric / negative / huge |

No `#ifdef`s guard any behaviour (`grep -n '#if' c_src/src/*.c c_src/include/*.h`
finds only the two header include guards). `translation/Cargo.toml` has **no
`[features]` section**, so there is exactly one feature combination (default).

**Public entry points (all of them, low-level included).**
`initialize_logger`, `log_info`, `log_warning`, `log_error`, `finalize_logger`
(`logger.h`); `create_task_manager`, `add_task`, `print_tasks`,
`destroy_task_manager` (`task_manager.h`); `driver` (`driver.c`, the one-shot
convenience wrapper). Phase B drives the nine low-level functions directly *and*
the `driver` wrapper.

**Input shapes the C special-cases.**
`add_task` description length vs the 255-byte `strncpy` bound
(`task_manager.c:60`); `task_count` vs `max_tasks` (`:54`); `task_count == 0` in
`print_tasks` (`:69`); `priority` as an arbitrary `int` printed with `%d`;
`driver`'s `strchr(start,'\n')` hit vs miss (`driver.c:46-49`) and the
`*end == '\n'` re-anchor (`:65`), i.e. line count 0/1/many, empty lines, leading
/ trailing / no newline; byte values 0x01–0xFF inside descriptions.

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `initialize_logger` → `finalize_logger` | `LOG_FILE` = fresh writable temp file; no other calls | [x] |
| C2 | `initialize_logger` → `finalize_logger` | `LOG_FILE` **unset** → default `default.log` in CWD (append mode, pre-existing content) | [x] |
| C3 | `initialize_logger` ×2 → `finalize_logger` | double init on the same writable path (C leaks the first handle; both must emit two `Logger initialized.` lines) | [x] |
| C4 | `log_info` / `log_warning` / `log_error` | after successful init; randomized ASCII messages (len 0…512), 200 iterations, interleaved across the three severities | [x] |
| C5 | `log_info` / `log_warning` / `log_error` | after successful init; messages containing arbitrary non-NUL bytes 0x01–0xFF (randomized, 200 iterations) | [x] |
| C6 | `create_task_manager` | `MAX_TASKS` unset → `max_tasks == 10`; inspect all three struct fields | [x] |
| C7 | `create_task_manager` | `MAX_TASKS` = randomized decimal string in 1…4096 (200 iterations) | [x] |
| C8 | `create_task_manager` | `MAX_TASKS` = `"0"` → `malloc(0)`; `max_tasks == 0`, pointer non-NULL | [x] |
| C9 | `create_task_manager` | `MAX_TASKS` non-numeric / partially numeric: `""`, `"abc"`, `"7x"`, `" 9"`, `"+3"`, `"-0"`, `"0x10"`, `"2147483647"` | [x] |
| C10 | `create_task_manager` → `destroy_task_manager` | full lifecycle, no tasks; compare log lines | [x] |
| C11 | `create_task_manager` + `add_task` ×1 | `MAX_TASKS=10`; randomized description (len 1…60) + randomized `priority`; compare all 260 struct bytes of slot 0 | [x] |
| C12 | `create_task_manager` + `add_task` ×N | `MAX_TASKS=10`, N randomized in 1…10; randomized descriptions/priorities; compare every written slot byte-for-byte (200 iterations) | [x] |
| C13 | `create_task_manager` + `add_task` ×N | `MAX_TASKS=1` (capacity boundary), N = 1 then 2 → second rejected | [x] |
| C14 | `create_task_manager` + `add_task` ×N | `MAX_TASKS=3`, N = 5 → 3 accepted, 2 rejected; compare struct + log | [x] |
| C15 | `add_task` | description length exactly 254 / 255 / 256 / 257 / 1024 (truncation boundary) | [x] |
| C16 | `add_task` | description containing randomized bytes 0x01–0xFF (non-UTF-8), len 1…400 (200 iterations) | [x] |
| C17 | `add_task` | `priority` = `0`, `1`, `-1`, `i32::MIN`, `i32::MAX`, randomized `i32` (200 iterations) | [x] |
| C18 | `print_tasks` | `task_count == 0` (empty manager) → only the `Tasks:` header | [x] |
| C19 | `print_tasks` | `task_count == 1` | [x] |
| C20 | `print_tasks` | `task_count` = many (10), randomized descriptions incl. `%`-specifiers and extreme priorities; stdout compared byte-for-byte (100 iterations) | [x] |
| C21 | `print_tasks` | after the capacity guard fired (`task_count == max_tasks < N added`) | [x] |
| C22 | full low-level pipeline | `initialize_logger` → `create_task_manager` → `add_task`×N → `print_tasks` → `destroy_task_manager` → `finalize_logger`, randomized `MAX_TASKS`∈1…16 and N∈0…24 (100 iterations); stdout **and** log file both compared | [x] |
| C23 | `driver` | `LOG_FILE` writable, `MAX_TASKS` unset (10); single line, no trailing `\n` | [x] |
| C24 | `driver` | single line **with** trailing `\n` | [x] |
| C25 | `driver` | multi-line input, no trailing `\n`; randomized 1…9 lines × randomized ASCII (200 iterations) | [x] |
| C26 | `driver` | multi-line input **with** trailing `\n` (extra empty final segment is *not* produced — loop exits) (200 randomized iterations) | [x] |
| C27 | `driver` | input containing empty lines: `"\n"`, `"\n\n\n"`, `"a\n\nb"`, leading `\n` | [x] |
| C28 | `driver` | lines longer than 255 bytes (truncation inside `add_task`), randomized 256…600 bytes | [x] |
| C29 | `driver` | `MAX_TASKS` smaller than the line count (`"3"`) → warnings + `priority` keeps incrementing past the cap | [x] |
| C30 | `driver` | `MAX_TASKS="0"` → every line rejected, stdout is just `Tasks:` | [x] |
| C31 | `driver` | `MAX_TASKS` randomized 1…20 × randomized line count 0…30 × randomized line lengths 0…300, 150 iterations; stdout + log + return code | [x] |
| C32 | `driver` ×2 sequentially | second call re-inits the logger after `finalize_logger` (append mode → both runs' lines in one file) | [x] |
| C33 | `driver` | input with non-ASCII / arbitrary bytes 0x01–0xFF (excluding NUL), randomized (150 iterations) | [x] |

There is no binary executable target in either build (`c_src/CMakeLists.txt`
declares only `add_library(driver SHARED …)`; `translation/Cargo.toml` declares
only `[lib] crate-type = ["cdylib"]`), so the "compare C and Rust binary stdout"
item is not applicable. The `driver` symbol — the library's driver entry point —
is exercised end-to-end with stdout comparison in rows C23–C33 instead.

## Rows added after the first pass

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C34 | `create_task_manager` + `add_task` + `print_tasks` | `MAX_TASKS` = 20000 and 100000 (large but allocatable), filled to *exactly* capacity then 3 more — exercises `tasks[i]` at the far end of the allocation, the only place an index/offset computation could diverge | [x] |
| C35 | `driver` | one 20 000-byte line (far past the 255 truncation bound); and 500 lines × 0…400 bytes with `MAX_TASKS=64` | [x] |

## Harness notes (why these rows are not vacuous)

* `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library. The
  first run of this suite passed against a *stale* `.so`. `tests/common/mod.rs`
  now refuses to run if either `.so` is older than its sources
  (`assert_fresh`), and `run_all.sh` always builds both first.
* stdout **and stderr** are both captured (`capture_out_err`) and compared; the
  C writes to stderr in `initialize_logger` and `driver`. Comparing only stdout
  was a real blind spot, caught by mutation `M23`.
* `mutate.sh` injects 41 known bugs into the Rust and checks the suite catches
  each: **39 caught, 2 observationally equivalent**:
  * `M01` `strncpy(dst,src,255)` → `strncpy(dst,src,256)`. Because
    `dst[255]='\0'` runs afterwards, the resulting 256 bytes are identical for
    every source length — verified exhaustively for lengths 0…600 with a
    standalone C program.
  * `M10` allocating `max_tasks*(sizeof(Task)+1)` instead of
    `max_tasks*sizeof(Task)`. Over-allocation is not observable through the
    public API (no size is exposed) and both sizes fail `malloc` for every
    `MAX_TASKS` that makes the allocation fail at all.
