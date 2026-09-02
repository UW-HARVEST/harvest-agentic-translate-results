# CONFIGS.md — configuration-surface table (Phase A) / valid-path tests (Phase B)

## Axes the C code actually branches on

**Runtime options.** The public API takes no option struct and no flags; the two
things a consumer can configure are environment variables read at call time:

| option | read at | effect |
|--------|---------|--------|
| `LOG_FILE` | `logger.c:34` | `getenv("LOG_FILE") ? it : "default.log"`, opened with mode `"a"` (append, created if missing). Chooses the sink for *all* log output and, when it fails to open, flips `initialize_logger` to the `-1` path. |
| `MAX_TASKS` | `task_manager.c:39` | `getenv("MAX_TASKS") ? atoi(it) : 10` → `manager->max_tasks`, which is both the `add_task` capacity check (`task_manager.c:54`) and the multiplicand of the tasks-array `malloc` (`task_manager.c:42`). Parsed with `atoi`, so it accepts leading whitespace/sign/garbage tails and truncates `long`→`int`. |

There are no `#ifdef`s in `c_src`, and the Rust crate declares no `[features]`
(`translation/Cargo.toml`), so the only feature combination that exists is the
default one — see `check_all_features.sh`.

**Entry points.** Ten exported functions, in two layers:

* low level — `initialize_logger`, `log_info`, `log_warning`, `log_error`,
  `finalize_logger`, `create_task_manager`, `add_task`, `print_tasks`,
  `destroy_task_manager`;
* one-shot convenience wrapper — `driver`, which composes all nine.

Both layers are driven directly below; rows 1–24 use the low-level API (including
hand-built `TaskManager` structs, which reach states the wrapper cannot produce),
rows 25–40 drive `driver`.

**Input shapes the code special-cases.**

* `description` length vs the 255-byte `strncpy` bound and the fixed
  `char[256]` field: 0, 1, 2…254, **255**, **256**, **257**, ≫256.
* `description` bytes: pure ASCII, bytes ≥ 0x80 (invalid UTF-8 — fatal for any
  `String`-based translation), and `printf` conversion specifiers (`%s`, `%d`,
  `%n`) which must stay inert because the C passes the text as an *argument*.
* `priority`: any `int`, incl. `0`, `-1`, `INT_MIN`, `INT_MAX`.
* task count vs capacity: 0, 1, `max_tasks-1`, `max_tasks`, `max_tasks+k`.
* `driver` blob shape (`driver.c:45-66`): empty, no trailing `\n`, trailing `\n`,
  only `\n`, leading `\n`, consecutive `\n\n` (empty lines *are* tasks), a single
  line, many lines, more lines than capacity, lines longer than 255.
* Ordering/state: log before/after `initialize_logger`, repeated
  create/destroy, `print_tasks` on a partially filled array.

## Rows

Every row is run with many randomized inputs (`common::Rng`, SplitMix64, fixed
seeds) against both `.so`s, comparing return value + stdout + stderr + log file
byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 1 | `initialize_logger`, `finalize_logger` | `LOG_FILE` unset → `./default.log` created in the cwd | `cfg01_log_default_path` | [x] |
| 2 | `initialize_logger`, `finalize_logger` | `LOG_FILE` = relative path | `cfg02_log_relative_path` | [x] |
| 3 | `initialize_logger`, `finalize_logger` | `LOG_FILE` = absolute path | `cfg03_log_absolute_path` | [x] |
| 4 | `initialize_logger`, `finalize_logger` | `LOG_FILE` names an **existing non-empty** file → mode `"a"` must append, not truncate | `cfg04_log_appends` | [x] |
| 5 | `initialize_logger`, `log_info` | `LOG_FILE` = `/dev/null` (open succeeds, bytes discarded) | `cfg05_log_devnull` | [x] |
| 6 | `log_info` | randomized messages, ASCII only | `cfg06_log_info_ascii` | [x] |
| 7 | `log_warning` | randomized messages, ASCII only | `cfg07_log_warning_ascii` | [x] |
| 8 | `log_error` | randomized messages, ASCII only | `cfg08_log_error_ascii` | [x] |
| 9 | `log_info`/`log_warning`/`log_error` | randomized **non-UTF-8** messages (bytes 0x80–0xFF) | `cfg09_log_non_utf8` | [x] |
| 10 | `log_info`/`log_warning`/`log_error` | messages containing `%s`/`%d`/`%n` — must be inert (`%s` argument, not format) | `cfg10_log_format_specifiers` | [x] |
| 11 | `log_*` | empty message (`""`) and 1-byte message | `cfg11_log_short` | [x] |
| 12 | `log_*` | very long message (4 KiB, > `BUFSIZ`) forcing an intra-message stdio flush | `cfg12_log_long` | [x] |
| 13 | `log_*` | `NULL` message pointer → glibc `%s` prints `(null)` | `cfg13_log_null_message` | [x] |
| 14 | `log_*` | interleaved INFO/WARNING/ERROR sequence, randomized order | `cfg14_log_interleaved` | [x] |
| 15 | `create_task_manager`, `destroy_task_manager` | `MAX_TASKS` unset (default 10), no tasks added | `cfg15_manager_default_capacity` | [x] |
| 16 | `create_task_manager` … `destroy_task_manager` | `MAX_TASKS` ∈ {`1`,`2`,`3`,`10`,`64`,`1000`} × random fill counts 0…capacity | `cfg16_manager_capacities` | [x] |
| 17 | `create_task_manager` | `MAX_TASKS` with `atoi`-quirky text: `"0"`,`" 7"`,`"+7"`,`"7x"`,`"x7"`,`""`,`"0x10"`,`"007"`,`"3.9"` | `cfg17_max_tasks_atoi_quirks` | [x] |
| 18 | `create_task_manager` | `MAX_TASKS` out of `int` range: `"2147483648"`, `"-2147483649"`, `"4294967296"`, `"99999999999999999999"` (`long`→`int` truncation ⇒ `0`/`-1`/`INT_MAX`…) | `cfg18_max_tasks_out_of_int_range` | [x] |
| 19 | `add_task`, `print_tasks` | description length sweep 0,1,2,…,254, **255**, **256**, **257**, 512, 1024 (the `strncpy` 255-byte bound + explicit `[255]=0`) | `cfg19_description_length_sweep` | [x] |
| 20 | `add_task`, `print_tasks` | randomized descriptions × `priority` ∈ {0,1,−1,`INT_MIN`,`INT_MAX`,random} | `cfg20_priority_values` | [x] |
| 21 | `add_task`, `print_tasks` | randomized non-UTF-8 descriptions of random length (incl. > 255) | `cfg21_description_non_utf8` | [x] |
| 22 | `add_task`, `print_tasks` | descriptions consisting of `printf` specifiers | `cfg22_description_format_specifiers` | [x] |
| 23 | `print_tasks` | **hand-built** `TaskManager`: `task_count` ∈ {0, 1, n} independent of `max_tasks`; array pre-filled with non-NUL-terminated/embedded-NUL contents; `task_count < 0` (loop never entered) | `cfg23_print_tasks_handbuilt` | [x] |
| 24 | full low-level pipeline | `create` → random adds → `print` → `destroy`, repeated 3× in one process (state carry-over between managers, log accumulating) | `cfg24_repeated_pipelines` | [x] |
| 25 | `driver` | empty input `""` → header only, one `[INFO] Task added` never logged | `cfg25_driver_empty` | [x] |
| 26 | `driver` | single line, no trailing newline | `cfg26_driver_single_line` | [x] |
| 27 | `driver` | single line **with** trailing newline (must NOT produce a second empty task) | `cfg27_driver_trailing_newline` | [x] |
| 28 | `driver` | input `"\n"` → exactly one empty task | `cfg28_driver_only_newline` | [x] |
| 29 | `driver` | leading newline `"\nx"` → empty task then `x` | `cfg29_driver_leading_newline` | [x] |
| 30 | `driver` | consecutive newlines `"a\n\nb"` → empty middle task | `cfg30_driver_empty_middle_line` | [x] |
| 31 | `driver` | many lines (random 1…30) × random lengths, `MAX_TASKS` unset | `cfg31_driver_many_lines` | [x] |
| 32 | `driver` | line count **exactly** `max_tasks` | `cfg32_driver_exactly_capacity` | [x] |
| 33 | `driver` | line count > `max_tasks` → capacity warnings, priority keeps incrementing for rejected lines | `cfg33_driver_over_capacity` | [x] |
| 34 | `driver` | `MAX_TASKS` ∈ {`1`,`2`,`5`,`10`,`64`} × random blobs | `cfg34_driver_capacities` | [x] |
| 35 | `driver` | lines of length 254/255/256/257/1024 (truncation inside `add_task`) | `cfg35_driver_long_lines` | [x] |
| 36 | `driver` | non-UTF-8 line bytes | `cfg36_driver_non_utf8` | [x] |
| 37 | `driver` | lines made of `printf` specifiers | `cfg37_driver_format_specifiers` | [x] |
| 38 | `driver` | `LOG_FILE` unset (`./default.log`) + random blob | `cfg38_driver_default_log` | [x] |
| 39 | `driver` | `driver` called **twice** in one process (logger re-init, second manager) | `cfg39_driver_twice` | [x] |
| 40 | `driver` | `MAX_TASKS=0` — valid config, every line rejected, header-only stdout | `cfg40_driver_zero_capacity` | [x] |

### Allocator-behaviour axis

Return value + stdout + stderr + log do not reveal *how* the library allocates,
yet the C's `malloc`/`free` pairing is part of its observable contract (a missing
`free` leaks; an extra one double-frees; a `Vec` smuggled into a hot path changes
the heap a caller sees).  `tests/common/mod.rs` interposes `malloc`/`free`
process-wide, so the following rows compare the allocation totals of the two
`.so`s for the same configurations.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 41 | `create_task_manager`, `destroy_task_manager` | default capacity; the pair must be balanced | `alloc01_create_destroy_is_balanced` | [x] |
| 42 | full low-level pipeline | 9 random tasks incl. lengths > 255 | `alloc02_full_low_level_pipeline` | [x] |
| 43 | `driver` | 8 random blobs, 1-14 lines, lengths up to 300 | `alloc03_driver_end_to_end` | [x] |
| 44 | `driver` | 40 lines with `MAX_TASKS=5` — rejected lines are still `malloc`ed and `free`d | `alloc04_driver_over_capacity` | [x] |
| 45 | `create_task_manager` | tasks-array allocation fails (interposed **and** via wrapping `MAX_TASKS=-1`): the C `free`s the manager | `alloc05_tasks_alloc_failure_frees_the_manager` | [x] |
| 46 | `create_task_manager` | manager allocation fails: the C frees **nothing** | `alloc06_manager_alloc_failure_frees_nothing` | [x] |
| 47 | `driver` | per-line copy allocation fails: `destroy_task_manager` + `finalize_logger` still run | `alloc07_driver_task_alloc_failure_cleans_up` | [x] |
| 48 | `driver` | `MAX_TASKS` ∈ {0,1,2,10,64,500} | `alloc08_capacity_sweep` | [x] |

All 48 rows are implemented in `tests/phase_b_logger.rs`,
`tests/phase_b_low_level.rs`, `tests/phase_b_driver.rs` and
`tests/phase_b_alloc_trace.rs`, and all pass.

## Running it

```sh
./check_all_features.sh    # builds both .so's, diffs symbols, runs every feature set
./mutation_check.sh        # proves the suite can actually fail (29/29 mutants caught)
```

`cargo test` on its own is **not** sufficient: it builds only the test
harnesses, never the `cdylib`, so it would happily run against a stale
`target/release/libdriver.so`.  `tests/common/mod.rs` therefore refuses to run
when either `.so` is older than its sources.
