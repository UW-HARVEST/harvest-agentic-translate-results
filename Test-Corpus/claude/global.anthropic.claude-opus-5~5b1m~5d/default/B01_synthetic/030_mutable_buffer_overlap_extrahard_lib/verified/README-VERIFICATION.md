# Differential verification of the Rust translation

This crate is a Rust translation of the C library in `../c_src`. Everything here
exists to prove the two produce **byte-identical** results through the C ABI.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | every public symbol of the C `.so`, and its counterpart in the Rust `.so` |
| `ERRORS.md` | the error/rejection surface: one row per distinct way the C rejects or faults on input |
| `CONFIGS.md` | the configuration surface: one row per meaningful combination of options and input shapes the C treats differently |

## How the tests work

Both libraries are loaded with `libloading` and called **only** through their
exported C symbols — the Rust crate is never linked or called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrappers are themselves under test.

| test target | phase | covers |
|-------------|-------|--------|
| `tests/phase_b_configs.rs` | B | every row of `CONFIGS.md`, with many seeded-random inputs per row |
| `tests/phase_c_errors.rs` | C | every row of `ERRORS.md`, including NULL pointers, out-of-range lengths and the C's undefined-behaviour faults |
| `tests/phase_d_symbols.rs` | D | `nm`-based symbol parity between the two `.so`s |
| `tests/common/mod.rs` | — | the shared harness: library loading, PRNG, aliasing schemes, stdout capture, crash isolation |

Two details of the harness are worth knowing:

* **stdout capture runs in a subprocess.** `driver` reports results only via
  libc `printf`, so comparison must be on the raw bytes of fd 1. Capturing
  in-process is unsound: libtest writes its own progress lines to fd 1 from the
  main thread while worker threads run, and those bytes corrupt the capture. The
  harness re-execs the test binary as a single-threaded worker
  (`run_driver_batch`) that redirects fd 1 per case, one subprocess per test.
* **UB cases run on a pinned stack.** The C's `int out[len]` is a VLA on the
  caller's stack, so where it overflows depends on the caller. The crash worker
  performs its call on a thread with an explicit 8 MiB stack
  (`WORKER_STACK_BYTES`) so the threshold is reproducible: `len <= 2^20` is fine,
  `len >= 2^21` kills the C.

## Running everything

```sh
# build the C shared library
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . )

# run one configuration
cargo test --release

# run every feature combination x profile, logging to verify_all.log
./scripts/verify_all.sh
```

`scripts/verify_all.sh` reads the feature list out of `Cargo.toml` and iterates
the full powerset; since this crate declares no `[features]`, that reduces to the
default and `--no-default-features`, each run under both `debug` and `release`.

## Known, UB-only differences

`fma_array` matches the C on every input tested. `driver` matches the C on every
input where the C's behaviour is defined. The only residual differences are on
inputs where the C invokes undefined behaviour and dies without returning a
value — negative `len`, and a `len` whose VLA overflows the stack. These are
measured, asserted and explained in the "Residual, UB-only divergences" section
of `ERRORS.md`.

One further difference is a *toolchain* artifact rather than a translation
defect: in a **debug** build Rust inserts null/alignment preconditions into raw
pointer dereferences, so passing NULL aborts (SIGABRT) where the C segfaults
(SIGSEGV). In the shipped **release** `cdylib` the two agree signal-for-signal.
The tests encode exactly this via `common::assert_same_fault`.
