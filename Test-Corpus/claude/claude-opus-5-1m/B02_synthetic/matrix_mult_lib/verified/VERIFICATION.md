# Verification report

Differential verification of the Rust translation (`src/`) against the C
original (`c_src/`) of the `driver` shared library.

Everything is driven through `dlopen`/`dlsym` (`libloading`) on **both** shared
libraries — the C `.so` and the Rust `.so` — so only the exported C ABI is
exercised (including the `#[no_mangle] extern "C"` wrappers). No Rust function
of the crate is ever called directly.

## How to reproduce

```
./verify.sh                     # everything: C build, all feature combos,
                                # symbol parity, all differential tests
cargo test                      # just the differential tests (dev profile .so)
DRIVER_RUST_SO=target/release/libdriver.so cargo test    # release profile .so
DRIVER_C_SO=/path/to/libdriver.so cargo test             # a different C build
```

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | all 7 dynamic symbols of the C `.so`, each mapped to its Rust definition; 0 missing, 0 unresolved |
| `ERRORS.md`  | 36 rows: every distinct rejection/error path in the C sources + the generic FFI boundaries |
| `CONFIGS.md` | 46 rows: every configuration/​input-shape combination the C code actually branches on |

Build-time configuration surface: `Cargo.toml` declares **no `[features]`**, and
`c_src/CMakeLists.txt` declares a single unconditional target with no options
(`#if`/`#ifdef` count in the C sources: 0) ⇒ exactly **one** build
configuration, verified with `cargo check --no-default-features --all-targets`.

## Phases B + C — differential tests

| test binary | tests | scope |
|-------------|-------|-------|
| `tests/phase_b.rs` | 40 | `CONFIGS.md` rows C1–C34, C40–C45 (valid paths, seeded-random inputs) |
| `tests/phase_c.rs` | 24 | `ERRORS.md` rows E2–E25, E32, E34, E35 (error paths, exact errno/sentinel + `stderr` bytes) |
| `tests/driver.rs`  | 12 | `CONFIGS.md` C35–C39, C44, C45 and `ERRORS.md` E26–E28, E30 (end-to-end, incl. the written `matrix.txt`) |
| `tests/fuzz.rs`    |  4 | catch-all randomised fuzzing (6000 iterations total) over all 7 entry points |
| `tests/oom_parity.rs` | 6 | `ERRORS.md` rows E1, E6, E9b, E13b, E17b, E29b — allocation-failure and NULL-dereference paths, executed in **child processes** with a capped `RLIMIT_AS`; compares exit code / terminating signal / `stderr` |
| **total**          | **86** | |

Each comparison covers *every* observable of the C API:

* return value (pointer NULL-ness, `int` error code / exit status),
* the full `matrix_t` state (`width`, `height`, every row pointer, every cell),
* the exact bytes of strings returned by `matrix_to_string`,
* the exact bytes written to the target file / `matrix.txt`,
* the exact bytes emitted on `stderr` (`perror` / `fprintf` diagnostics,
  including `strerror(errno)` texts).

Ordering is tested both ways (C-first *and* Rust-first) because several C error
paths `return errno`, a process-global that a C-first ordering could mask.

## Phase D — parity and configurations

* Symbol diff `comm -23 c.syms rust.syms`: **empty** for both the dev and the
  release Rust `.so`.
* `ldd -r`: no unresolved symbols.
* Full suite re-run per configuration: 1 feature combination × 2 Rust profiles
  (dev, release) — **all 86 tests pass in each**.
* Bonus: the suite also passes against an **`-O2` (`CMAKE_BUILD_TYPE=Release`)**
  build of the C library, i.e. the agreement does not depend on the C compiler's
  optimisation level.

## Divergences found

### 1. Real divergence — dev-profile UB instrumentation (FIXED)

On the inputs where the **C itself** dereferences a NULL pointer (large positive
`width` whose row `malloc` fails ⇒ `allocate_matrix` returns NULL, which
`initialize_matrix_from_string` / `multiply_matrices` never check):

| | C | Rust dev (before) | Rust dev (after) | Rust release |
|---|---|---|---|---|
| signal | `SIGSEGV` (11) | **`SIGABRT` (6)** | `SIGSEGV` (11) | `SIGSEGV` (11) |
| extra `stderr` | — | **panic: "null pointer dereference occurred"** | — | — |

`debug-assertions` (default-on in `dev`) makes rustc inject UB checks; the
resulting panic cannot cross the `extern "C"` boundary and aborts. Fixed in
`Cargo.toml` with `[profile.dev] debug-assertions = false` / `overflow-checks =
false` — no change to the translated code, because the C's unchecked dereference
*is* the specified behaviour. Regression-tested by `tests/oom_parity.rs`
(`oom_e9b_…`, `oom_e13b_…`, `oom_driver_null_deref`).

This also corrected a wrong assumption in the first draft of `ERRORS.md`: rows
E9/E13 claimed the unchecked NULL "is never dereferenced because the loop body
never runs" — true for *negative* dimensions, false for large *positive* ones.
Rows E9b/E13b/E29b now cover that case.

### 2. Test-harness bug (FIXED, not a translation issue)

Temporary directories were keyed on the PID only, so a recycled PID could
inherit the `matrix.txt` **directory** planted by the E30 test and make an
unrelated `driver` test fail (~1 run in 6). Temporary roots now include a
high-resolution timestamp and every scratch directory is created fresh. A
stale-`.so` guard was also added (`cargo test` does not rebuild the `cdylib`, so
testing an outdated library is now a hard error instead of a silent false pass).
After the fixes: 40 consecutive parallel runs of the driver suite and 5
consecutive parallel runs of the full suite pass.

## Deliberately untested C behaviour (undefined behaviour in the original)

These are documented in `ERRORS.md` (rows E1, E6, E11, E15, E29) and
`CONFIGS.md` (notes 1 and 2) rather than "fixed" in the Rust:

1. `initialize_matrix_from_string(NULL, …)` and `multiply_matrices(NULL, …)`
   dereference a *caller-supplied* NULL pointer in C — both libraries crash
   identically; verified by source inspection (the Rust performs the same
   unchecked dereference and adds **no** extra guard). The same crash *class*
   arising from the library's own unchecked NULL **is** verified end-to-end in
   `tests/oom_parity.rs` (terminating signal + `stderr` compared between a C and
   a Rust child process).
2. `matrix_to_string` sizes its buffer for an average of ≤ 10 characters per
   cell, so wide matrices full of 11-character values (`INT_MIN` …
   `-1000000000`) overrun the C heap buffer. Tests stay inside that budget
   (`to_string_fits()`), so no test relies on corrupted heap state.
3. `height = INT_MAX` would make the C code perform 2^31 successive row
   allocations (machine OOM). The huge-`size_t` `malloc` path is covered instead
   via negative dimensions and `width = INT_MAX`.
4. Allocation-failure paths (`E1` struct `malloc`, `E6` `strdup`) are **no longer**
   inspection-only: they are exercised for real in child processes with a capped
   `RLIMIT_AS` (`tests/oom_parity.rs`).

## Negative control (mutation testing)

To prove the suite is actually *sensitive* (and not passing vacuously), seven
deliberate mutations were injected into the Rust source one at a time, the
library rebuilt, and the whole suite re-run. Every mutation was detected; the
source was restored afterwards (`diff -r` clean).

| # | mutation in the Rust translation | detected by |
|---|----------------------------------|-------------|
| M1 | `EINVAL` 22 → 21 | `err_e18_write_null_content`, `err_order_write_rust_first` |
| M2 | `matrix_to_string` separator condition `j < width-1` → `j < width` | 7 tests in `phase_b`/`driver` |
| M3 | `"Insufficient columns in row %d."` argument `i + 1` → `i` | 3 tests (`stderr` byte comparison) |
| M4 | `multiply_matrices` checks `mat_b->width` instead of `mat_b->height` | 7 tests |
| M5 | `buffer_size` formula: `+ height` term dropped | suite aborts (`SIGABRT`) — detected |
| M6 | `int → size_t` conversion zero-extends instead of sign-extending | `phase_c` aborts — detected |
| M7 | `OUT_FILE` `"matrix.txt"` → `"matrix.text"` | 8 `driver` tests |
| M8 | an **added** `if mat.is_null() { return null }` guard that the C does not have | `fuzz_matrix_pipeline`, `fuzz_driver` (`stderr` divergence) |

(The battery was re-run after the `[profile.dev]` change: all eight mutations are
still detected.)
