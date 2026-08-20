# VERIFICATION.md — how the translation was verified, and what it found

The C code in `c_src/` is the ground truth and was not modified.  Everything
below compares the **built artefacts**: the C shared library / executable
against the Rust `cdylib` / binary.

## Layout

| file | role |
|------|------|
| `SYMBOLS.md` | Phase A/D: every `nm -D` symbol of the C `.so` and its Rust counterpart, struct layouts, feature combinations |
| `ERRORS.md` | Phase A/C: the error-surface table (every rejection the C code performs) with the test that proves parity |
| `CONFIGS.md` | Phase A/B: the configuration-surface table (option × input-shape combinations) with the test that proves parity |
| `check_features.sh` | Phase D: enumerates the feature combinations and runs `check` + `build` + `test` (dev **and** release) for each |
| `tests/common/mod.rs` | the differential harness: `dlopen`s both libraries with `libloading`, serialises access to their global state, captures fd 1/2, deterministic RNG, random-text generators |
| `examples/ffi_runner.rs` | replays a script of C-API calls against **one** `dlopen`ed library in a fresh process (needed for stdin-consuming entry points, virgin-state scenarios and flush-at-exit) |
| `tests/*.rs` | 103 differential tests (see the row → file map at the end of `CONFIGS.md`) |

## Rules the harness follows

* **Nothing is called directly.**  Every C-API call in every test goes through a
  `dlsym`ed symbol of `c_src/build/libtextanalyzer_c.so` or of
  `target/<profile>/libtext_analyzer.so`, so the `#[no_mangle]` wrappers,
  the struct ABI (`token_t` is 280 bytes and returned through memory) and the
  function-pointer dispatch are all part of what is tested.
* **Both libraries always get the same operation sequence.**  Both keep global
  state and some of it (`total_*_processed`) can never be reset, so every test
  holds one global lock for its whole body and applies each operation to both
  libraries; assertions only ever compare C against Rust.
* **Freshly built artefacts.**  `cargo test --test <name>` does *not* rebuild a
  `cdylib` that nothing links against, so the harness runs `cargo build --lib`
  / `--bin driver` / `--example ffi_runner` (for the profile the tests were built
  with) before `dlopen`ing anything.  It also rebuilds the C `.so`/executable
  from `c_src/src/*.c` when they are older than the sources.
* **Only observable bytes are compared.**  `token_t.value` is compared as a C
  string, because everything past the NUL (and the struct padding) is
  uninitialised in the C build.
* **The harness is self-checked.**  `tests/harness_selfcheck.rs` asserts
  hand-derived absolute values (token types, values, columns, the analysis
  result of a known text, the menu text) so that a harness which compared two
  empty buffers could not pass silently.  Deliberate mutations of the Rust code
  were also confirmed to fail the suite (see below).

## Divergences found and fixed

| # | symptom | root cause | fix |
|---|---------|------------|-----|
| 1 | `analyzer_init` with an all-`NULL` `tokenizer_ops_t`, then `analyze_text`/`find_patterns`/`interactive_tokenizer`: the C build died from `SIGSEGV` with no output, the Rust build printed a panic message plus an abort backtrace and died from `SIGABRT` | the FFI wrappers used `Option::expect` on the ops members, while the C code calls the member unconditionally | `src/ffi.rs::null_ops_member` reproduces the fault instead of panicking; asserted by `tests/runner_scenarios.rs::null_ops_dispatch_dies_identically` (same signal, same empty output) |
| 2 | `driver … \| head -c 64`: the C build died from `SIGPIPE` (status 141), the Rust build exited 0 | the Rust runtime sets `SIGPIPE` to `SIG_IGN` at startup, a C program starts with the default disposition | `src/main.rs::restore_default_sigpipe`; asserted by `tests/driver_e2e.rs::c50c_stdout_closed_early` |
| 3 | fully interactive run (stdin *and* stdout on one terminal): C printed `"Choice: Enter filename: "` **before** the `stderr` message of a failing `read_file`, the Rust build printed it after | glibc's `_IO_new_file_underflow` calls `_IO_flush_all_linebuffered()` before reading from a line-buffered `stdin`, which flushes the unterminated prompt held by the line-buffered `stdout` | `cio::In::fill` now flushes the emulated `stdout` when `stdin` is a terminal (`cio::Out::flush_if_line_buffered`), and `cio::Out`/`cio::In` take their buffer size from `st_blksize` like `_IO_file_doallocate`; asserted by `tests/driver_e2e.rs::c50e_interactive_terminal` |

Two harness bugs were also found and fixed, both of which had been *hiding*
failures: a stale `cdylib` was being compared (see "freshly built artefacts"),
and the test harness' own `test … ok` progress lines leaked into the captured
`stdout` of another thread's test (fixed by holding this binary's `Stdout`/
`Stderr` lock across the redirection — the `.so`s have their own `std`, so their
output is unaffected).

Mutation checks used to prove the suite has teeth (each was reverted):

| mutation | caught by |
|----------|-----------|
| `Complexity: Low` → `Complexity: low` in `driver.rs` | 6 of the `driver_e2e` tests |
| `column = current_column - length` → `… + 1` in `tokenizer.rs` | all 25 `tokenizer_valid` tests, 10 `errors` tests, 4 `analyzer_valid`, 5 `runner_scenarios`, 5 `driver_e2e`, `harness_selfcheck` |
| removing the `SIGPIPE` fix | `driver_e2e::c50c_stdout_closed_early` |
| disabling the flush-before-read of divergence 3 | `driver_e2e::c50e_interactive_terminal` |

## Structural changes to the Rust crate (no behavioural change)

* `src/main.rs` shrank to the entry point; the program logic moved to
  `src/driver.rs` **unchanged**, so the binary and the shared library run the
  same code.
* `src/lib.rs` + `src/ffi.rs` are new: the C ABI surface and the process-global
  singletons that stand in for the C translation units' `static` variables.
  `src/ffi.rs` dispatches through the `tokenizer_ops_t` function pointers
  exactly like `analyzer.c`/`main.c` do.
* `src/analyzer.rs` gained `account_token`/`is_initialized` so that the
  `analyze_text` loop body is shared between the direct (`driver.rs`) and the
  function-pointer (`ffi.rs`) paths instead of being written twice.
* `Cargo.toml` gained the `[lib]` `cdylib` target (with `test = false`, because
  the library exports a C `main`) and `libloading` as a dev-dependency.
  `libloading` (with `cfg-if`/`windows-link`) is only ever compiled for
  `cargo test`; `cargo build`/`cargo build --release` of the `driver` binary does
  not build it, but cargo still has to *resolve* it, so the pinned versions in
  `Cargo.lock` must be present in the registry cache (or downloadable) even for a
  plain `cargo build --offline`.
  There are still **no `[features]`**, so no `#[cfg(feature = …)]` gating is
  needed; `check_features.sh` derives the combination list from `Cargo.toml`
  and verifies both of them.

## Completion gate

* [x] `SYMBOLS.md`: `nm -D` shows **0** of the C `.so`'s 16 symbols missing from
      the Rust `.so`, and 0 unresolved non-libc imports
      (`tests/layout.rs::rust_so_exports_every_c_symbol`,
      `…::rust_so_has_no_unresolved_non_libc_imports`).
* [x] Phase B: every row of `CONFIGS.md` (C1-C50e) passes, with randomized
      inputs (fixed seeds) wherever the row describes a shape rather than a
      boundary.
* [x] Phase C: every row of `ERRORS.md` (E1-E42, G1-G7) has a passing
      differential test; the only unchecked row is E33 (`malloc` failure), which
      cannot be provoked without an allocator fault injector and is documented
      as such.
* [x] Both feature combinations × `dev` and `release` profiles:
      `./check_features.sh` → `ALL FEATURE COMBINATIONS PASSED`
      (11 test binaries × 4 configurations, 412 test-case executions).

Reproduce with:

```sh
cd translated_rust
./check_features.sh          # everything, all configurations
cargo test                   # the default configuration only
```
