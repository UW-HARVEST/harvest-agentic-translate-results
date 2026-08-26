# CONFIGS.md — configuration-surface table (Phase A.3)

## Build-time configuration (feature combinations)

`Cargo.toml` declares **no** features (the `[features]` table is empty), and
`c_src/` contains **no** `#ifdef`/`#if` build switch other than the
`SILLYMAIN_H_` include guard, and `c_src/CMakeLists.txt` defines no `option()`,
no `target_compile_definitions` and no build types. Therefore the complete
enumeration of valid feature combinations is:

| # | feature combination | command |
|---|---------------------|---------|
| 1 | *(empty — the only one)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`--all-features` and the default build are the same configuration as #1; all
three are run by `./verify.sh` for completeness.

## Runtime configuration axes, derived from the C source

The public API is `int helloworld();` (`sillymain.h`) plus the `main` symbol the
shared object exports. Neither takes a parameter, neither reads a global, an
environment variable or a file, and the code contains no `if`/`switch` at all —
so **the C code branches on nothing**. The axes below are therefore not invented
options: they are the only pieces of program state the two functions can
observe or affect, i.e. what `printf` does with `FILE *stdout`:

* **A. entry point**: `helloworld` (lowest-level public function, called
  directly, not only through the wrapper) · `main` (the composed entry point
  that returns `helloworld()`'s value) · the whole `driver` program end to end
  (CMake executable vs. `cargo build` binary).
* **B. fd 1 destination**, which is what glibc inspects on first use of the
  stream to pick a buffering mode: regular file · pipe · `/dev/null` ·
  character device · closed / non-writable (see `ERRORS.md`).
* **C. stream buffering mode** actually in force: fully buffered (`_IOFBF`,
  chosen for files/pipes) · line buffered (`_IOLBF`) · unbuffered (`_IONBF`),
  set via `setvbuf` by the caller before the call — the caller can put the
  shared `stdout` into any of the three states and the emitted bytes must be
  identical in all of them.
* **D. call count / shape**: 0 calls (empty) · 1 call (one) · many calls
  (2…64, randomized) — the "empty / one / many" boundary for the only
  quantity in the API.
* **E. interleaving with a foreign writer of fd 1**: none · raw `write(2)`
  marker bytes between calls · `printf` by the caller between calls · mixed C
  `.so` and Rust `.so` calls in one stream. This is the axis that makes
  buffering observable and is where a naive translation diverges.
* **F. concurrency**: single-threaded · N threads calling concurrently.
* **G. flush discipline**: explicit `fflush(stdout)` between calls · no flush
  until the end of the capture · `fflush(NULL)`.
* **H. process lifetime**: normal `exit()` (buffers flushed) · `_exit()` /
  `SIGKILL` before exit (buffered bytes lost identically).

## Configuration table

One row per meaningful combination the C actually distinguishes (the pruned
cross-product of A×B×C×D×E×F×G×H). Every row is driven through the `.so`
exports of **both** libraries and compared byte-for-byte, with many randomized
inputs (call counts, marker payloads, orderings, argument values) from a fixed
seed.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `helloworld` | fd 1 → regular file, default buffering, exactly **one** call | `cfg01_single_call_to_file` | [x] |
| 2 | `helloworld` | fd 1 → regular file, default buffering, **zero** calls (empty baseline: both must emit nothing) | `cfg02_zero_calls_emit_nothing` | [x] |
| 3 | `helloworld` | fd 1 → regular file, **many** calls, randomized count 2…64 | `cfg03_many_calls_to_file` | [x] |
| 4 | `helloworld` | fd 1 → **pipe** (fully buffered by glibc), randomized count, drained after flush | `cfg04_many_calls_to_pipe` | [x] |
| 5 | `helloworld` | fd 1 → **`/dev/null`**, randomized count (return value only) | `cfg05_calls_to_dev_null` | [x] |
| 6 | `helloworld` | fd 1 → regular file, `setvbuf` **`_IONBF`** (unbuffered) | `cfg06_unbuffered_stream` | [x] |
| 7 | `helloworld` | fd 1 → regular file, `setvbuf` **`_IOLBF`** (line buffered) | `cfg07_line_buffered_stream` | [x] |
| 8 | `helloworld` | fd 1 → regular file, `setvbuf` **`_IOFBF`** with a caller-supplied buffer, many calls, single flush at the end | `cfg08_fully_buffered_stream` | [x] |
| 9 | `helloworld` | fd 1 → regular file, **raw `write(2)` marker bytes interleaved** between calls (exposes buffering/flush timing) | `cfg09_interleaved_raw_write_markers` | [x] |
| 10 | `helloworld` | fd 1 → regular file, caller's own **`printf` interleaved** between calls (same stream, same buffer) | `cfg10_interleaved_caller_printf` | [x] |
| 11 | `helloworld` | fd 1 → regular file, **explicit `fflush` after every call** | `cfg11_flush_after_every_call` | [x] |
| 12 | `helloworld` | fd 1 → regular file, **no flush until the very end** (`fflush(NULL)` once) | `cfg12_no_intermediate_flush` | [x] |
| 13 | `main` | fd 1 → regular file, one call (checks the composed `main → helloworld` path and its return value) | `cfg13_main_single_call` | [x] |
| 14 | `main` | fd 1 → regular file, many randomized calls | `cfg14_main_many_calls` | [x] |
| 15 | `main` + `helloworld` | fd 1 → regular file, **randomized mix** of both entry points in one stream | `cfg15_mixed_entry_points` | [x] |
| 16 | both, both libraries | fd 1 → regular file, randomized **A/B interleaving of the C `.so` and the Rust `.so`** with marker bytes, so the two libraries share one `FILE *stdout` (strongest ordering/buffering check) | `cfg16_cross_library_interleaving` | [x] |
| 17 | `helloworld` | fd 1 → regular file, **N threads × M calls** concurrently (line atomicity under the stdio lock) | `cfg17_concurrent_threads` | [x] |
| 18 | `helloworld` | library **`dlclose`d and re-`dlopen`ed** between batches (fresh relocations, shared stream state) | `cfg18_reload_between_batches` | [x] |
| 19 | whole program | `driver` executable (CMake) vs `driver` binary (cargo): stdout → **pipe**, exit status compared | `cfg19_program_pipe` | [x] |
| 20 | whole program | `driver` executable vs binary: stdout → **regular file**, exit status compared | `cfg20_program_file` | [x] |
| 21 | whole program | `driver` executable vs binary invoked with **extra argv / empty environment** (both ignore `argc`/`argv`/`envp`) | `cfg21_program_argv_env` | [x] |
| 22 | whole program | `driver` executable vs binary with stdout → **closed fd** (write failure at process level) — exit status must still be 0 | `cfg22_program_closed_stdout` | [x] |
| 23 | `helloworld` | fd 1 → file **and** pipe, default (full) buffering, **never flushed** (`_exit` instead of `exit`, i.e. killed before the buffers drain): both must lose exactly the same bytes | `cfg23_unflushed_output_is_lost_identically` | [x] |
| 24 | `helloworld` | same as row 23 but `_IONBF`: everything has already reached fd 1, so **nothing** is lost | `cfg24_unflushed_but_unbuffered_loses_nothing` | [x] |
| 25 | whole program | `driver` executable vs binary with stdout → **`/dev/full`**: the exit-time flush fails with `ENOSPC`, which `exit()` ignores → status 0, no output | `cfg25_program_stdout_enospc` | [x] |

All 25 rows pass. Verified for every configuration by `./verify.sh`:

| axis | values covered |
|---|---|
| feature combination | `''` (the only one), plus the default and `--all-features` builds |
| Rust profile | `debug`, `release` (`panic = "abort"`) |
| C optimization | default (no `-O`, what `c_src/CMakeLists.txt` uses) and `-O2` |
| test runner | `--test-threads=1` and the default parallel runner (5/5 runs green) |

## Finding: what this table caught

Rows 9, 10, 16 and 23 exist because of an axis that a per-function happy-path
test cannot see: **when** the output reaches fd 1.

The original translation wrote through `std::io::stdout()` (a `LineWriter` that
flushes on every `\n`), whereas the C code writes through `FILE *stdout`, which
glibc fully buffers on a file or pipe. Same bytes, different timing — and the
timing is observable:

* interleaved with any other writer of fd 1 (raw `write(2)`, or the caller's own
  `printf`), the lines come out in a different order (rows 9, 10, 16);
* if the process is killed instead of exiting, C loses the buffered line while
  the eager version has already emitted it (row 23).

`src/sillymain.rs` therefore calls the platform's `printf` — the same stream, the
same buffering mode, the same flush-at-`exit()` semantics — and all four rows
pass. `negative_control.sh` re-creates the old behaviour as a mutant and
confirms these rows reject it.
