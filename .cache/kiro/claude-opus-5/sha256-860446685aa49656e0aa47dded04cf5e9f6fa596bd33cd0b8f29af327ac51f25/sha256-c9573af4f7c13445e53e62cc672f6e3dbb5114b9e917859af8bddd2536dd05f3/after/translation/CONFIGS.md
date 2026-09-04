# CONFIGS.md — configuration surface table (Phase B gate)

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from what
`c_src` actually branches on, not from what looks important.

## Axes the C code distinguishes

**Runtime options.** The library has no flag/mode parameters at all; its only
configuration channel is the environment, read with `getenv`:

| axis | read at | states the C distinguishes |
|------|---------|-----------------------------|
| `$LOG_FILE` | `logger.c:34` | unset ⇒ path `"default.log"`; set ⇒ that exact string used verbatim (**no** emptiness check — `""` is a "set" value, see `ERRORS.md` E2) |
| `$MAX_TASKS` | `task_manager.c:39` | unset ⇒ `10`; set ⇒ `atoi(value)`, so also: `"0"`, plain digits, leading spaces/`+`/`-`, non-numeric ⇒ `0`, over/underflowing digit strings |
| logger static `log_file` | `logger.c:31,48,54,60,66` | `NULL` (pre-init / failed init) vs open handle — toggles whether **every** log line in the whole library is emitted; also re-init overwrites the handle without closing, and `finalize_logger` closes without resetting |
| log-file pre-state | `fopen(...,"a")` | file absent (created) vs file already containing bytes (appended after them) |

**Input shapes.**

| axis | branch site | shapes |
|------|-------------|--------|
| `add_task` `description` length | `strncpy(..., 255)` + forced NUL at `[255]` | 0, 1, 254, 255, 256, 257, ≫256 bytes |
| `description` bytes | passed through `%s` later | ASCII, embedded `printf` specifiers (`%s %n %d`), high/non-UTF-8 bytes (`0x80..0xFF`), `\t`/`\r` |
| `priority` | stored + `%d` | `INT_MIN`, `-1`, `0`, `1`, `INT_MAX`, random `i32` |
| manager occupancy | `task_count >= max_tasks` | empty (0), 1, mid, exactly `max_tasks`, over |
| `print_tasks` count | `for (i=0; i<task_count; i++)` | 0 rows, 1 row, many rows |
| `driver` `tasks` string | `strchr(start,'\n')`, `*end=='\n'` | `""`; one line, no trailing `\n`; one line + `\n`; many lines; consecutive `\n` (⇒ empty tasks); leading `\n`; trailing `\n`; only `\n`s; line ≥256 bytes; more lines than `max_tasks`; exactly `max_tasks` lines |
| call sequencing | static `log_file` | ops before any `initialize_logger`; after success; after failure; after `finalize_logger`; `driver` called twice |

**Full set of public entry points**, low-level first (all are exercised
directly, not only through the `driver` one-shot wrapper): `initialize_logger`,
`log_info`, `log_warning`, `log_error`, `finalize_logger`,
`create_task_manager`, `add_task`, `print_tasks`, `destroy_task_manager`,
`driver`.

**Observables compared byte-for-byte** for every row: return value; captured
`stdout` bytes; captured `stderr` bytes; log-file bytes; and, for the
`TaskManager` pipeline, the struct fields read back through the returned pointer
(`max_tasks`, `task_count`, and every `Task`'s 256 `description` bytes +
`priority`).

## Table

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `log_info`, `log_warning`, `log_error`, `finalize_logger` | `log_file == NULL` (fresh library, nothing initialised); random messages | [x] |
| C2  | `initialize_logger` | `$LOG_FILE` unset ⇒ `default.log` created in CWD; check the `[INFO] Logger initialized.` line and rc 0 | [x] |
| C3  | `initialize_logger` | `$LOG_FILE` = fresh writable path; rc 0 + first line | [x] |
| C4  | `initialize_logger` | `$LOG_FILE` = path that already contains bytes ⇒ `"a"` append mode, prior bytes preserved | [x] |
| C5  | `initialize_logger` ×2 (re-init, no finalize in between) | same `$LOG_FILE`; handle overwritten without `fclose` (leak quirk); two `Logger initialized.` lines | [x] |
| C6  | `initialize_logger` → `initialize_logger` with a *different* `$LOG_FILE` | second path gets subsequent lines, first file keeps only its own | [x] |
| C7  | `initialize_logger` → `log_info`/`log_warning`/`log_error` interleaved → `finalize_logger` | randomized message set, each level exercised; exact `[INFO]`/`[WARNING]`/`[ERROR]` prefixes and ordering | [x] |
| C8  | `initialize_logger` → `log_*` with boundary messages | empty message `""`, 1 byte, 4 KiB message, message containing `\n`, `%s`, `%d`, `%n`, bytes `0x01..0xFF` | [x] |
| C9  | `initialize_logger` → `finalize_logger` | `[INFO] Logger finalized.` appended, then `fclose` | [x] |
| C10 | `create_task_manager`, `destroy_task_manager` | `$MAX_TASKS` unset ⇒ `max_tasks == 10`, `task_count == 0`; logger uninitialised (⇒ no log output) | [x] |
| C11 | `create_task_manager`, `destroy_task_manager` | logger initialised first ⇒ `[INFO] TaskManager created successfully.` / `... destroyed successfully.` lines present | [x] |
| C12 | `create_task_manager` | `$MAX_TASKS` = `"1"`, `"2"`, `"7"`, `"10"`, `"64"`, `"1000"`, random 1..4096 ⇒ `max_tasks` echoes `atoi` | [x] |
| C13 | `create_task_manager` | `$MAX_TASKS` = `"0"` ⇒ `malloc(0)` still non-NULL ⇒ manager returned with `max_tasks == 0` | [x] |
| C14 | `create_task_manager` | `$MAX_TASKS` non-numeric / partially numeric: `"abc"`, `""`, `"  12"`, `"+5"`, `"3x"`, `"0x10"`, `"1e3"`, `"99999999999999999999"` ⇒ whatever `atoi` yields (`0`, `12`, `5`, `3`, `0`, `1`, clamp) | [x] |
| C15 | `create_task_manager` + `add_task` ×n + `print_tasks` + `destroy_task_manager` | happy pipeline, `n` random in 1..max, random ASCII descriptions, random `i32` priorities; compare stdout, log, and every struct field | [x] |
| C16 | `add_task` | description length sweep 0,1,2,127,254,**255**,**256**,257,300,1024 — the `strncpy` 255-byte truncation boundary; verify all 256 `description` bytes incl. padding NULs | [x] |
| C17 | `add_task` | description bytes: random non-UTF-8 (`0x80..0xFF`), `printf` specifiers (`%s`, `%n`, `%%`, `%1000d`), tabs/`\r`, then `print_tasks` renders them through `%s` | [x] |
| C18 | `add_task` | `priority` = `INT_MIN`, `-1`, `0`, `1`, `INT_MAX`, plus random `i32`; stored verbatim and rendered by `%d` | [x] |
| C19 | `add_task` | fill exactly to `max_tasks` (boundary `task_count == max_tasks - 1` accepted) for `max_tasks` = 1, 2, 10, random | [x] |
| C20 | `print_tasks` | `task_count == 0` ⇒ only `Tasks:\n`; and 1 / 2 / max rows ⇒ `  [i] desc (Priority: p)\n`, 1-based index | [x] |
| C21 | `print_tasks` | called twice in a row (idempotent, no state change) | [x] |
| C22 | `print_tasks` | manager mutated by hand: `task_count` set below the number of populated slots ⇒ prints only the first `task_count` rows | [x] |
| C23 | `driver` | `tasks == ""` (empty string) ⇒ loop body never runs, `Tasks:\n` only, rc 0, full log sequence incl. `Logger finalized.` | [x] |
| C24 | `driver` | single line without trailing newline (`"only task"`) | [x] |
| C25 | `driver` | single line with trailing newline (`"only task\n"`) | [x] |
| C26 | `driver` | many lines, no trailing newline; priorities 1..n | [x] |
| C27 | `driver` | many lines with trailing newline | [x] |
| C28 | `driver` | consecutive newlines `"a\n\nb"`, `"a\n\n\n"` ⇒ zero-length tasks added (empty `description`) | [x] |
| C29 | `driver` | leading newline `"\nx"`; string of only newlines `"\n"`, `"\n\n\n"` | [x] |
| C30 | `driver` | a line ≥256 bytes ⇒ per-task truncation inside `add_task` | [x] |
| C31 | `driver` | line count **exactly** `$MAX_TASKS` (boundary, no warning) | [x] |
| C32 | `driver` | line count **greater than** `$MAX_TASKS` ⇒ first n printed, remainder produce `[WARNING]` lines but priority still increments | [x] |
| C33 | `driver` | `$MAX_TASKS = "1"` with multi-line input (tightest limit) | [x] |
| C34 | `driver` | `$MAX_TASKS = "0"` with non-empty input ⇒ every line warns, `Tasks:\n` only | [x] |
| C35 | `driver` | `$LOG_FILE` unset ⇒ `default.log`; verify rc 0 and log content | [x] |
| C36 | `driver` | `$LOG_FILE` already non-empty ⇒ appended | [x] |
| C37 | `driver` ×2 in the same process (same `$LOG_FILE`) | second call re-`fopen`s and appends; the dangling post-`finalize` handle is overwritten before use | [x] |
| C38 | `driver` then manual `initialize_logger` + `log_*` | logger reusable after `driver` finalized it | [x] |
| C39 | `driver` | randomized line sets: 0..20 lines, each 0..300 random printable bytes, with random `$MAX_TASKS` in 0..25 (fixed-seed property sweep, 200 iterations) | [x] |
| C40 | full manual pipeline `initialize_logger` → `create_task_manager` → `add_task`* → `print_tasks` → `destroy_task_manager` → `finalize_logger` | randomized: `$MAX_TASKS` 0..16, 0..20 adds with random lengths/bytes/priorities (fixed-seed property sweep, 200 iterations); compares stdout + stderr + log + all struct fields | [x] |

Every row is checked off only after its `phase_b_*` test in
`tests/differential.rs` passed for **both** `.so`s across all randomized inputs
of that row.

## Additional randomised rows

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C41 | `driver` | fuzz over **arbitrary** non-NUL bytes (`0x01..0xFF`, not just printable) 0..900 bytes long with `'\n'` sprinkled at random positions, random `$MAX_TASKS` 0..12 (250 iterations, fixed seed) | [x] |
| C42 | random *sequences* of `initialize_logger` / `log_info` / `log_warning` / `log_error` / `finalize_logger` / `create_task_manager` / `add_task` / `print_tasks` / `destroy_task_manager` | 1..30 ops per iteration over multiple live managers, random `$MAX_TASKS` 0..8, random descriptions 0..320 arbitrary bytes and random `i32` priorities; snapshots every manager after every mutating op (150 iterations, fixed seed). Sequences respect the library's one genuine precondition — after `finalize_logger` the static handle is closed but not NULLed, so it is re-initialised before any further logging call, which would be UB in *both* builds | [x] |
