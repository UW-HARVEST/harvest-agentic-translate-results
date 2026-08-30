# Verification report — C `driver` library vs. its Rust translation

## What was verified

The C library (`c_src/src/driver.c`, the only translation unit in
`c_src/CMakeLists.txt`) exports two functions, `driver` and `printLine`. Both were
compared against the Rust translation **through the FFI boundary only**: every test
`dlopen`s both `libdriver.so` files with `libloading` and calls the exported symbols, so
the `#[no_mangle] extern "C"` wrappers are themselves under test. No Rust function is
ever called directly.

Because both functions return `void` and write through libc's `stdout`, "output" is
defined as *the exact bytes written to file descriptor 1* plus, for the
undefined-behaviour inputs, *the process exit status / terminating signal*.

## How to reproduce

```sh
cd translation && ./run_all.sh       # builds the C .so, then runs every configuration
```

`run_all.sh` builds the C shared library, enumerates the crate's feature combinations
from `Cargo.toml`, and for each of them runs the full suite in both the dev and the
release profile plus a `nm -D` symbol diff.

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md` — `nm -D` diff C vs Rust | **empty** in all 4 configurations; `ldd -r` reports no unresolved symbol |
| Phase B — every `CONFIGS.md` row (C1–C23) | **pass** (21 tests, randomized with a fixed seed + exhaustive sweeps) |
| Phase C — every `ERRORS.md` row (E1–E10 + generic boundary sweep) | **pass** (11 tests) |
| Phase D — feature/profile matrix | **pass**: dev × default, dev × `--no-default-features`, release × default, release × `--no-default-features` (35 tests each) |

No divergence between the C and the Rust implementation was found; `translation/src/lib.rs`
needed no behavioural fix. The only change made to the crate was adding the
`libloading` dev-dependency, the `runner` example, `.cargo/config.toml`
(`RUST_TEST_THREADS=1`, needed because the tests redirect the process-wide fd 1) and the
test/documentation files.

## Layout

```
translation/
  SYMBOLS.md                  Phase A: exported-symbol parity
  ERRORS.md                   Phase A: error/rejection surface table (E1..E10)
  CONFIGS.md                  Phase A: configuration surface table (C1..C23)
  run_all.sh                  runs everything, all configurations
  examples/runner.rs          out-of-process driver (for the crashing UB inputs and
                              for pipe / file / unbuffered stdout modes)
  tests/common/mod.rs         .so loading, fd-1 capture, PRNG, staleness guard
  tests/phase_b_configs.rs    Phase B: valid-path differential tests
  tests/phase_c_errors.rs     Phase C: error-path differential tests
  tests/phase_d_symbols.rs    Phase D: symbol parity / dlsym resolution
```

## Notable findings about the harness itself

1. **`cargo test` does not rebuild the `cdylib`.** No test target links the library
   (it is loaded at runtime), so cargo considers it out of the test graph and a *stale*
   `libdriver.so` would be tested silently — a mutation-testing check exposed this.
   `tests/common/mod.rs::assert_not_stale` now fails loudly when
   `target/<profile>/libdriver.so` is older than any file under `src/`, and
   `run_all.sh` always runs `cargo build` (+ `cargo build --examples`) first.
2. **Tests must not run in parallel.** Capturing what the libraries print requires
   `dup2`-ing the process-wide fd 1; `.cargo/config.toml` sets `RUST_TEST_THREADS=1`,
   and the capture also flushes libtest's own line-buffered stdout before redirecting.
3. **The harness is sensitive**, proven by three reverted mutations (see the mutation
   table at the end of `CONFIGS.md`), including one that "fixed" the C's undefined
   behaviour for negative `data` — which the suite correctly rejects, since the Rust
   must reproduce the C's crash.
