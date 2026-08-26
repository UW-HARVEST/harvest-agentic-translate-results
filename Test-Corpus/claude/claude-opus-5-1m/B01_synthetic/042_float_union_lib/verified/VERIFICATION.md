# VERIFICATION.md — summary of the C-to-Rust differential verification

Library under test: `driver` — a single exported function

```c
void driver(double f);   // prints "%llx %a %.4f\n" of f's bits, f, f
```

Run everything with:

```sh
./verify.sh          # enumerates feature combos, builds C + Rust, tests, diffs symbols
cargo test           # just the differential suite
```

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | 1 exported C symbol (`driver`); 1 exported Rust symbol (`driver`). 0 missing. |
| `ERRORS.md` | 15 rows. The C error surface is provably **empty** (no `return`/`assert`/`NULL`/branch); rows cover the degenerate & boundary inputs instead. |
| `CONFIGS.md` | 28 rows across IEEE-754 class, magnitude regime, sign, plus the FPU-rounding and locale process-state axes. |

## Build configurations

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` has no
`option()` / `target_compile_definitions` and no conditional sources. The only
preprocessor conditional in the C is the `DRIVER_H_` include guard. So the
feature powerset has exactly **one** member, exercised explicitly as
`--no-default-features`. `verify.sh` computes the powerset mechanically, so if a
feature is ever added the loop picks it up automatically.

Both the `dev` and `release` profiles were run (release also sets
`panic = "abort"`); all tests pass in both, and both export `driver`.

## Test inventory (53 tests, all passing)

| file | tests | purpose |
|------|-------|---------|
| `tests/common/mod.rs` | — | harness: loads both `.so`s with `libloading`, captures fd 1, compares bytes, SplitMix64 PRNG |
| `tests/harness_selfcheck.rs` | 6 | proves the harness is not vacuous (see below) |
| `tests/phase_b_valid.rs` | 24 | one test per `CONFIGS.md` row C1–C24 |
| `tests/phase_b_env.rs` | 8 | `CONFIGS.md` rows C26–C28 (rounding direction, locale, and their cross-product) |
| `tests/phase_c_errors.rs` | 15 | one test per `ERRORS.md` row E1–E15 |

Both libraries are always reached **through their exported `driver` symbol**
resolved by `dlopen`/`dlsym` via `libloading` — no Rust function is ever called
directly, so the `#[unsafe(no_mangle)] pub extern "C"` wrapper is itself under
test. Randomized rows use a fixed seed (`0x9E3779B97F4A7C15`) so any failure
reproduces; roughly 100k values in total flow through both implementations.

## Two vacuity traps that were found and closed

Both of these would have produced a fully green suite that verified nothing.

1. **`cargo test` never relinks the `cdylib`.** Nothing links a `crate-type =
   ["cdylib"]` target, so `target/debug/libdriver.so` was stale: deliberately
   changing `%.4f` to `%.5f` in `src/lib.rs` still gave 45/45 passes. The
   harness now shells out to `cargo build --lib` before `dlopen`, and asserts
   the `.so` is not older than `src/lib.rs`. It also builds the C `.so` via
   cmake if missing or stale.
2. **Parallel test threads corrupt an fd-1 capture.** libtest's own progress
   output ("test x ... ok") was being flushed into a capture window by another
   thread, producing bogus diffs and off-by-one line counts. Fixed by forcing
   `RUST_TEST_THREADS = "1"` in `.cargo/config.toml`, serializing captures
   behind a mutex, and asserting one output line per call so any residual
   contamination fails loudly instead of silently.

The self-check tests pin these down permanently: they assert the two `driver`
pointers differ, that the captured bytes for `1.0` are exactly
`3ff0000000000000 0x1p+0 1.0000\n`, that different inputs yield different
captures (so the comparison can discriminate), and that a >320-byte `f64::MAX`
record is captured whole.

## Mutation study — evidence the tests can detect divergence

Each mutation was applied to `src/lib.rs`, the full suite was run, and the
source restored (final `md5sum` matches the original). A test suite that cannot
fail is worthless, so every mutation must be **caught**:

| mutation to the Rust translation | result |
|----------------------------------|--------|
| `%a` -> `%A` | CAUGHT |
| `%.4f` -> `%.4g` | CAUGHT |
| `%.4f` -> `%.4e` | CAUGHT |
| `%.4f` -> `%.3f` | CAUGHT |
| `%.4f` -> `%.5f` | CAUGHT |
| `%llx` -> `%llX` | CAUGHT |
| `%llx` -> `%llu` | CAUGHT |
| drop the trailing `\n` | CAUGHT |
| `to_bits() ^ 1` (one wrong bit) | CAUGHT |
| bits taken from `f.abs()` (sign bit lost) | CAUGHT |
| NaN quieted before `to_bits` (payload lost) | CAUGHT |
| `f32` round-trip (precision lost) | CAUGHT |
| value printed as `f.abs()` | CAUGHT |
| format `%.4f` in Rust (`format!`) instead of delegating to `printf` | CAUGHT |

**13/13 caught, 0 survivors.**

Two notes on the study:

* `let x = (f * 1.0).to_bits()` did *not* fail — but that is not a coverage gap:
  LLVM folds `fmul x, 1.0` to `x`, so the mutant is semantically identical to
  the original. The explicit "NaN quieted" mutation above tests the same
  property unambiguously and **is** caught, by `error_e6_signaling_nan`,
  `error_e7_nan_payload_sweep`, `config_c5`, `config_c20` and `config_c21`.
* Dropping the trailing newline first appeared to survive, but that run was a
  *compile* error (the `&[u8; 14]` length annotation); re-applied correctly with
  the length fixed, it is caught.

## Notable behaviours confirmed identical

* **Signaling NaNs are not quieted.** The C union type-pun reproduces the raw
  payload, so `driver(sNaN(payload=1))` prints `7ff0000000000001 nan nan` — the
  Rust `f64::to_bits()` matches bit-for-bit across a randomized payload sweep.
* **Sign of NaN/zero is preserved:** `-nan`, `-0.0000`, `-0x0p+0`.
* **Subnormals** render with a `0x0.` leading hex digit
  (`1 0x0.0000000000001p-1022 0.0000`).
* **`f64::MAX`** expands to a ~320-byte `%.4f` record; captured and compared whole.
* **`%llx` prints no leading zeros** (bits `1` prints as `1`, not `0000…0001`).
* **Rounding direction and locale are honoured identically**, because the Rust
  translation forwards to the same `printf@GLIBC_2.2.5` the C `.so` imports
  rather than formatting the value itself.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing exported symbols and 0 undefined
      non-libc symbols in the Rust `.so`.
- [x] Phase B: every row in `CONFIGS.md` (C1–C28) passes across randomized inputs.
- [x] Phase C: every row in `ERRORS.md` (E1–E15) has a passing differential test.
- [x] All of the above hold under every feature combination — the powerset has
      one member, verified by `verify.sh`, and additionally re-run under the
      `release` profile.
- [x] Nothing in `c_src/` was modified.
- [x] The suite is proven able to fail (13/13 mutations caught).
