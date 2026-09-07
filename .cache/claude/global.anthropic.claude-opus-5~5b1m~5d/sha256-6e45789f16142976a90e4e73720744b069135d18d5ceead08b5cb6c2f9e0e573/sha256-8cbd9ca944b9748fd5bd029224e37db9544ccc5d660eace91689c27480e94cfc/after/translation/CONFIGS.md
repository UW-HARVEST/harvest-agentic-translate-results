# CONFIGS.md — Phase B configuration-surface table

Axes the C code actually branches on (derived from the source, not guessed):

**Runtime options.** The library takes no flag/mode parameters; its only runtime
configuration is the **environment**, read with `getenv`:

| option | read in | branch it drives |
|--------|---------|------------------|
| `LOG_FILE`   | `logger.c:34` | `log_file_env ? log_file_env : "default.log"`, then `fopen` success/failure |
| `MAX_TASKS`  | `task_manager.c:39` | `max_tasks_env ? atoi(max_tasks_env) : 10` → `malloc` size, and the `task_count >= max_tasks` guard in `add_task` |

**Stateful axis.** `log_file` is a file-scope `static`, so every function's
behaviour depends on whether the logger is currently *uninitialised*, *open*, or
*failed-to-open*. Every `log_*` call site is guarded by `if (log_file)`.

**Input shapes the code special-cases.**
* `description` length vs. the 256-byte field: `< 255`, `== 255`, `> 255` (`strncpy` + forced NUL).
* `task_count` vs. `max_tasks`: below / exactly at / above capacity; `max_tasks` of 0, 1, many.
* `print_tasks` loop trip count: 0 tasks vs. 1 vs. many.
* `driver`'s tokeniser: `strchr(start,'\n')` found vs. `NULL` (last line, no trailing
  newline); `*end == '\n'` → skip separator vs. stop; empty tokens from `\n\n`;
  leading `\n`; the `""` early exit; `priority++` monotonic numbering.
* `atoi` input shape: decimal, leading whitespace, explicit `+`/`-`, trailing
  garbage, non-numeric, out-of-`int` range.

**Public entry points — the FULL set, low-level included.** `driver()` is the
one-shot convenience wrapper; the rows below drive the low-level
`initialize_logger` / `log_info` / `log_warning` / `log_error` / `finalize_logger`
and `create_task_manager` / `add_task` / `print_tasks` / `destroy_task_manager`
entry points **directly**, in hand-composed sequences, as well as through `driver`.

Every row is run against **both** `.so`s via `libloading` with many randomized
inputs (`SplitMix64`, fixed seed `0x5EED_1234_ABCD_0001`), comparing:
`stdout` bytes, `stderr`-relevant return codes, the log-file bytes, and — for the
`TaskManager` rows — the raw struct fields (`max_tasks`, `task_count`) and the
full 260-byte `Task` array contents read back through the pointer.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `initialize_logger` | `LOG_FILE` = fresh writable path; assert return `0` and that the file now contains `[INFO] Logger initialized.\n` | [x] |
| C2 | `initialize_logger` | `LOG_FILE` **unset** → default path `default.log` created relative to CWD; return `0`, file contents compared | [x] |
| C3 | `initialize_logger` | `LOG_FILE` = path that **already exists with content** → `"a"` append mode must preserve the prefix | [x] |
| C4 | `initialize_logger` ×2 | called twice in a row (2nd `fopen` replaces the static, 1st stream leaked); both return `0`, both `Logger initialized.` lines present | [x] |
| C5 | `log_info`/`log_warning`/`log_error` | logger **uninitialised** (`log_file == NULL`); 32 random messages → no output, no crash | [x] |
| C6 | `log_info` | logger open; 200 random ASCII messages (len 0..200) → exact `[INFO] <m>\n` bytes | [x] |
| C7 | `log_warning` | logger open; 200 random ASCII messages → `[WARNING] <m>\n` | [x] |
| C8 | `log_error` | logger open; 200 random ASCII messages → `[ERROR] <m>\n` | [x] |
| C9 | `log_*` interleaved | logger open; random sequence of 300 calls across all three severities → whole-file byte compare (ordering + buffering) | [x] |
| C10 | `log_info` | message = `""` (empty), and message containing `%s %d %%` conversion specifiers (must appear literally, since it is a `%s` argument) | [x] |
| C11 | `log_info` | message of 4096 and 65536 bytes (crosses glibc's stdio buffer) | [x] |
| C12 | `log_info` | message with high-bit / non-UTF-8 bytes `0x80..0xFF` and embedded `\t`, `\r` | [x] |
| C13 | `initialize_logger` + `finalize_logger` | full open→finalize cycle; file must end with `[INFO] Logger finalized.\n` and be flushed by `fclose` | [x] |
| C14 | `create_task_manager` | `MAX_TASKS` **unset** → `max_tasks == 10`, `task_count == 0`, `tasks != NULL`; logger open so `TaskManager created successfully.` is logged | [x] |
| C15 | `create_task_manager` | `MAX_TASKS` = random `1..=64` (16 draws) → `max_tasks` echoes the value exactly | [x] |
| C16 | `create_task_manager` | `MAX_TASKS` = `"0"` → `max_tasks == 0`, `malloc(0)` still non-NULL, creation **succeeds** | [x] |
| C17 | `create_task_manager` | `MAX_TASKS` `atoi` shapes: `" 7"`, `"+3"`, `"12abc"`, `"abc"`, `"0012"`, `"3.9"`, `"2147483647"` → `max_tasks` must match C's `atoi` result | [x] |
| C18 | `create_task_manager` (+`destroy_task_manager`) | logger **uninitialised** → creation still succeeds, nothing logged | [x] |
| C19 | `add_task` | `MAX_TASKS=10`, add `k` tasks for `k` ∈ `0..=10` with random descriptions/priorities; compare `task_count` and the full `Task` array bytes | [x] |
| C20 | `add_task` | description length exactly `254`, `255`, `256`, `257`, `1024` → 255-byte truncation + NUL at index 255; compare all 260 bytes of the `Task` | [x] |
| C21 | `add_task` | description = `""`; description with embedded `%d`; description with bytes `0x80..0xFF` | [x] |
| C22 | `add_task` | `priority` ∈ {`INT_MIN`, `-1`, `0`, `1`, `INT_MAX`} + 32 random `i32` → stored verbatim, and rendered by `print_tasks` with `%d` | [x] |
| C23 | `add_task` | fill to exactly `max_tasks` (capacity boundary, `MAX_TASKS` ∈ {1,2,10,64}) → last accepted task is index `max_tasks-1` | [x] |
| C24 | `print_tasks` | `task_count == 0` → stdout is exactly `Tasks:\n` | [x] |
| C25 | `print_tasks` | `task_count` ∈ {1, 2, 10, 64} with random contents → stdout byte-compare of the `  [%d] %s (Priority: %d)\n` lines (1-based index) | [x] |
| C26 | `print_tasks` | tasks whose descriptions were truncated at 255 bytes → printed form must also be truncated | [x] |
| C27 | `destroy_task_manager` | logger open → `[INFO] TaskManager destroyed successfully.\n` appended; logger closed → nothing | [x] |
| C28 | full low-level pipeline | `initialize_logger` → `create_task_manager` → N× `add_task` → `print_tasks` → `destroy_task_manager` → `finalize_logger`, hand-composed (NOT via `driver`), randomized N/descriptions/priorities/`MAX_TASKS`, 64 iterations → stdout **and** whole log file byte-compared | [x] |
| C29 | `driver` | `tasks` = `""` → returns `0`, stdout `Tasks:\n` | [x] |
| C30 | `driver` | `tasks` = one line, **no** trailing `\n` (`strchr` returns NULL branch) | [x] |
| C31 | `driver` | `tasks` = one line **with** trailing `\n` (`*end=='\n'` → `end+1`, then `*start=='\0'` ends loop) | [x] |
| C32 | `driver` | `tasks` = several lines, no trailing `\n`; and several lines with trailing `\n` | [x] |
| C33 | `driver` | `tasks` containing consecutive `\n\n` → empty tasks that still consume a priority | [x] |
| C34 | `driver` | `tasks` = `"\n"` only; `tasks` = `"\n\n\n"`; `tasks` starting with `\n` | [x] |
| C35 | `driver` | more lines than capacity: `MAX_TASKS` ∈ {0,1,3} with 8 lines → extras rejected with the warning, `priority` still increments | [x] |
| C36 | `driver` | a line longer than 255 bytes (300 / 1024 bytes) → truncated in the printed output | [x] |
| C37 | `driver` | `MAX_TASKS` unset (default 10) with exactly 9 / 10 / 11 lines (capacity boundary through the wrapper) | [x] |
| C38 | `driver` | randomized: 64 iterations of random line counts `0..=20`, random line lengths `0..=300` incl. empty lines, random `MAX_TASKS` ∈ {unset,0,1,5,10,64} → stdout **and** log file byte-compared, plus return code | [x] |
| C39 | `driver` | `LOG_FILE` unset (default.log) end-to-end | [x] |
| C40 | `driver` then `driver` again | two calls in one process (second `initialize_logger` reopens; log file accumulates) → return codes + appended log bytes | [x] |

## Result

All 40 rows pass. `tests/phase_b.rs`: **40 tests, 40 passing** (test `cNN_…`
corresponds to row `CNN`), in every feature combination and in both the `debug`
and `release` profiles. Randomized rows use `common::Rng` (SplitMix64) seeded
from the fixed constant `0x5EED_1234_ABCD_0001`, so every run is reproducible.

What each row compares, on every iteration:

* the `obs` string — return values plus the raw `TaskManager` header
  (`max_tasks`, `task_count`, `tasks == NULL`) and a full **260-byte hex dump of
  every initialised `Task`** read back through the C-allocated pointer;
* `stdout` bytes (captured by `dup2`-ing fd 1 to a file, with `fflush(NULL)`
  before and after, so glibc buffering is included in the comparison);
* `stderr` bytes;
* the complete log-file bytes (each side gets its own sandbox directory and its
  own `LOG_FILE`, so their logs never interleave).

### No binary executable to compare

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED ...)` and the
Rust crate is `crate-type = ["cdylib"]` — neither project builds a driver
executable. The "compare the two binaries' stdout" requirement is therefore met
by driving the exported `driver()` entry point through the FFI boundary and
comparing captured `stdout` byte-for-byte (rows C29–C40).

### Note on the `max_tasks * sizeof(Task)` conversion

Measured: no valid `MAX_TASKS` value can distinguish C's sign-extending
`int → size_t` conversion from a zero-extending one, because across the whole
negative `int32` range both products exceed available address space and `malloc`
returns `NULL` either way. The row is therefore covered differentially over all
boundary values *and* pinned at the source level — see `ERRORS.md` row E23.

### Suite sensitivity (mutation testing)

To prove these rows are not vacuously green, 19 mutants were injected into
`translation/src/` one at a time (wrong log severity tag, `[WARNING]` → `[WARN]`,
`strncpy` length off by one, dropped forced NUL, `>=` → `>` in the capacity
guard, 1-based → 0-based print index, pre- vs post-increment of `priority`,
`EXIT_FAILURE` 1 → 2, `-1` → `-2` from `initialize_logger`, `"a"` → `"w"` fopen
mode, `"default.log"` renamed, default capacity 10 → 11, `strchr('\n')` →
`strchr('\r')`, inverted newline-skip, corrupted task terminator, dropped
`log_info`/`log_warning` calls, altered `Tasks:` header, zero-extending size
conversion). **All 19 were killed.** `src/` was afterwards verified byte-identical
to its pristine state.
