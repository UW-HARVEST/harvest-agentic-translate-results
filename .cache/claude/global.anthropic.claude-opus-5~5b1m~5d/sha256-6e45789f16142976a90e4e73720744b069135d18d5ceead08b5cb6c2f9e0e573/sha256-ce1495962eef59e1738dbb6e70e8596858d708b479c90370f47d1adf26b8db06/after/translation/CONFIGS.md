# CONFIGS.md — Phase A configuration-surface table

## Axes the C actually branches on

Derived from the public headers plus every `if` / `?:` / loop condition in
`c_src/src/*.c`. There are no `#ifdef`s and no compile-time options.

**Runtime options (both are environment variables — the library's only "flags"):**

| axis | read by | branch it drives |
|------|---------|------------------|
| `LOG_FILE` | `initialize_logger` | `log_file_env ? log_file_env : "default.log"`; then `fopen` success/failure |
| `MAX_TASKS` | `create_task_manager` | `max_tasks_env ? atoi(env) : 10`; then `malloc(max_tasks * 260)` success/failure; then the `task_count >= max_tasks` gate in `add_task` |

**Logger state axis:** `log_file == NULL` (never initialised, or a failed
`initialize_logger`) vs. open vs. closed-but-dangling (after
`finalize_logger`). Every `log_*` and `finalize_logger` branches on it.

**Input-shape axes for `driver(tasks)`** (the `while (*start)` /
`strchr(start,'\n')` / `(*end=='\n') ? end+1 : end` loop):
empty string; no trailing `\n`; trailing `\n`; leading `\n`; consecutive `\n`
(empty tasks); 1 vs. many lines; line length vs. the 255-byte `strncpy`
boundary; line count vs. `max_tasks`; byte values (high/8-bit bytes, `\r`,
`%`-signs that reach `printf` as a `%s` *argument*).

**Input-shape axes for `add_task`:** description length 0/1/254/255/256/257/big;
`priority` = 0 / 1 / -1 / `INT_MIN` / `INT_MAX`.

**Input-shape axes for `print_tasks`:** `task_count` = 0 / 1 / many.

**Entry points — the full set, low-level first:**
`initialize_logger`, `log_info`, `log_warning`, `log_error`,
`finalize_logger`, `create_task_manager`, `add_task`, `print_tasks`,
`destroy_task_manager`, and the one-shot wrapper `driver`.

**Feature combinations:** `Cargo.toml` has no `[features]` table → the single
default configuration is the whole cross-product. Rows are additionally re-run
under `--release` and `--no-default-features`.

## Configuration rows

Every row is driven through **both** `.so`s via `libloading` and compared
byte-for-byte on: return value, `TaskManager` fields (`max_tasks`,
`task_count`), every stored `Task` (all 256 description bytes + `priority`),
captured `stdout`, captured `stderr`, and the resulting log-file bytes.
Randomised rows use a fixed-seed xorshift64\* PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `initialize_logger` → `finalize_logger` | `LOG_FILE` **unset** → `default.log` created in CWD | [x] |
| 2 | `initialize_logger` → `finalize_logger` | `LOG_FILE` = fresh writable path (file does not exist) | [x] |
| 3 | `initialize_logger` → `finalize_logger` | `LOG_FILE` = **existing** file → `"a"` append semantics (pre-seeded content preserved, new bytes appended) | [x] |
| 4 | `initialize_logger` ×2 → `finalize_logger` | re-initialise while already open (leaks the first `FILE*`, `log_file` overwritten) — same path | [x] |
| 5 | `initialize_logger` ×2 → `finalize_logger` | re-initialise with a **different** `LOG_FILE` between the two calls | [x] |
| 6 | `log_info` / `log_warning` / `log_error` | logger open; 200 randomised messages, lengths 0…4096, bytes 1…255 (incl. `%`, `\n`, high bytes) | [x] |
| 7 | `log_info` / `log_warning` / `log_error` | logger open; interleaved in random order so tag ordering in the file is checked | [x] |
| 8 | `create_task_manager` → `destroy_task_manager` | `MAX_TASKS` unset → `max_tasks == 10` | [x] |
| 9 | `create_task_manager` → `destroy_task_manager` | `MAX_TASKS` = `"0"` → `malloc(0)`, immediately full | [x] |
| 10 | `create_task_manager` → `destroy_task_manager` | `MAX_TASKS` = `"1"` | [x] |
| 11 | `create_task_manager` → `destroy_task_manager` | `MAX_TASKS` randomised in `1..=512` (30 seeds) | [x] |
| 12 | `create_task_manager` → `destroy_task_manager` | `MAX_TASKS` = `"1000000"` (260 MB, expected to succeed) | [x] |
| 13 | `create_task_manager` | `MAX_TASKS` with `atoi` quirks: `" 7"`, `"+7"`, `"7abc"`, `"0x10"`, `"abc"`, `""`, `"007"`, `"2147483647"`, `"-1"`, `"99999999999999"` | [x] |
| 14 | `create_task_manager` | logger **not** initialised (so its `log_info` is a no-op) — proves the `log_file==NULL` path through a *different* module | [x] |
| 15 | `create_task_manager` | logger initialised first → `[INFO] TaskManager created successfully.` reaches the log | [x] |
| 16 | `add_task` | 1 task, description length 0 | [x] |
| 17 | `add_task` | 1 task, description lengths 1, 10, 254, 255, 256, 257, 1024 (the `strncpy` boundary) | [x] |
| 18 | `add_task` | fill to exactly `max_tasks`, then 5 more (limit gate) with `MAX_TASKS` ∈ {0,1,2,10} | [x] |
| 19 | `add_task` | `priority` ∈ {0, 1, -1, `INT_MIN`, `INT_MAX`} and 100 randomised `i32`s | [x] |
| 20 | `add_task` | 200 randomised (description, priority) pairs into a `MAX_TASKS`-randomised manager, then full struct compare | [x] |
| 21 | `print_tasks` | `task_count == 0` (fresh manager) → only `Tasks:\n` | [x] |
| 22 | `print_tasks` | `task_count == 1` | [x] |
| 23 | `print_tasks` | `task_count == max_tasks` (full), randomised descriptions incl. high bytes and `%s`/`%d` literals inside descriptions | [x] |
| 24 | `print_tasks` | called twice in a row (idempotent, doubles the stdout bytes) | [x] |
| 25 | full low-level pipeline | `initialize_logger` → `create_task_manager` → N× `add_task` → `print_tasks` → `destroy_task_manager` → `finalize_logger`, randomised N and `MAX_TASKS` (40 seeds) | [x] |
| 26 | full low-level pipeline | same, but `create_task_manager` **before** `initialize_logger` (so early log lines are dropped) | [x] |
| 27 | full low-level pipeline | same, but extra `log_warning`/`log_error` calls interleaved between `add_task`s | [x] |
| 28 | `driver` | `tasks = ""` (empty) | [x] |
| 29 | `driver` | single line, no trailing newline | [x] |
| 30 | `driver` | single line, with trailing newline | [x] |
| 31 | `driver` | `"\n"`, `"\n\n"`, `"\n\n\n"` (only empty tasks) | [x] |
| 32 | `driver` | leading newline + content, consecutive interior newlines | [x] |
| 33 | `driver` | many lines, count < `max_tasks` (`MAX_TASKS` unset → 10) | [x] |
| 34 | `driver` | many lines, count == `max_tasks` | [x] |
| 35 | `driver` | many lines, count > `max_tasks` → `[WARNING]` lines + `priority` keeps incrementing | [x] |
| 36 | `driver` | `MAX_TASKS = "0"` → every line rejected, `Tasks:\n` only | [x] |
| 37 | `driver` | line longer than 255 bytes (truncation) and a mix of short/long lines | [x] |
| 38 | `driver` | lines with 8-bit bytes (0x80–0xFF), `\r`, tabs, and `%s %d %n` text | [x] |
| 39 | `driver` | 300 fully randomised inputs (random length 0…600, random alphabet incl. `\n`) × randomised `MAX_TASKS` | [x] |
| 40 | `driver` | `LOG_FILE` = existing file (append) while running a randomised input | [x] |
| 41 | `driver` | called twice in the same process, second call appends to the same log (exercises the *not-reset* `log_file` static across a successful `finalize_logger`) | [x] |
| 42 | `driver` | 64 KiB single line, and 5000 short lines (oversized) | [x] |

## Additional rows added while driving the surface

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 43 | `driver` | `LOG_FILE` **unset** → `default.log` relative to the cwd, through the full one-shot pipeline | [x] |
| 44 | mixed | low-level pipeline (`initialize_logger` → `create` → `add` → `print` → `destroy`) **followed by** two `driver` calls, sharing the module-level `log_file` static across both entry paths | [x] |
| 45 | `add_task` / `print_tasks` | long description then short description then empty, 20 randomised rounds — proves `strncpy`'s zero-padding of the 256-byte buffer (catches a translation that leaves dirty bytes) | [x] |
| 46 | `log_info` | messages of 255 / 256 / 257 / 4095 / 4096 / 4097 / 65536 bytes — the logger, unlike `add_task`, has no length limit at all | [x] |
| 47 | *(layout)* | `sizeof`/`alignof`/`offsetof` of `Task` and `TaskManager` match the C exactly (260 / 16 / 4 / 256 / 8 / 12) | [x] |

Ordering constraint discovered in row 44: `driver` ends with `finalize_logger`,
which `fclose`s `log_file` **without** resetting it, so any low-level call made
*after* `driver` returns is a use-after-free. That is genuine undefined
behaviour and is handled by `ERRORS.md` rows 23/24 instead of being asserted
byte-for-byte here.

## How to run

```
cd translation && ./run_verification.sh
```

The script rebuilds the C reference `.so`, then for each profile
(`dev`, `release`) and each feature combination (extracted mechanically from
`Cargo.toml`; there are none declared, so `default` and `--no-default-features`)
it runs `cargo build` — required, because `cargo test` does not build a
`cdylib`-only lib target — followed by
`cargo test -- --test-threads=1`, and finally diffs `nm -D` between the two
objects.

`--test-threads=1` is required: the tests manipulate process-global state
(`environ`, fds 1/2, the cwd) and `fork()` children for the undefined-behaviour
rows.

Result: **89 tests × 4 configurations, all passing; symbol diff empty in both
profiles.**
