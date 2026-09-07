# Verification record — completion gate

Artifacts: `SYMBOLS.md` (Phase A/D), `ERRORS.md` (Phase A → gates C),
`CONFIGS.md` (Phase A → gates B).
Tests: `tests/harness/mod.rs`, `tests/differential.rs` (23), `tests/error_paths.rs` (15).

Both libraries are reached **only** through `dlopen` + their exported C symbols
(`libloading`); no Rust function is ever called directly, so the
`#[no_mangle] extern "C"` wrappers are themselves under test.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff between the C `.so` and the Rust `.so` is
      empty (`static_sum`, `driver`); 0 missing symbols, 0 undefined non-libc
      symbols. Asserted by `symbols::rust_so_exports_every_c_symbol` and
      `symbols::rust_so_has_no_undefined_non_libc_symbols`.
- [x] **Phase B** — all 21 `CONFIGS.md` rows pass, each driven with many
      seeded-random inputs (`Rng` = SplitMix64, fixed seeds) rather than one
      hand-picked value. Both entry points are exercised directly, including
      the low-level `static_sum`, and in randomized interleavings on shared
      hidden state (row 20).
- [x] **Binary stdout** — N/A and asserted as such: `c_src/CMakeLists.txt` has
      no `add_executable` and the crate has no `[[bin]]`
      (`configs::row21_no_binary_target`). `driver`'s `printf` output is still
      compared byte-for-byte via fd-1 capture.
- [x] **Phase C** — all 15 `ERRORS.md` rows have a passing differential test
      (`tests/error_paths.rs`).
- [x] **All feature combinations** — the crate declares no `[features]`, so the
      complete set is `default` and `--no-default-features`. `check_all_features.sh`
      enumerates them mechanically and runs `cargo check` + `cargo build` +
      `cargo test` for each, in **both** the `release` and `dev` profiles
      (the latter has overflow checks on, which is a genuinely different code
      path for the `wrapping_*` arithmetic). Result: **ALL PASSED**
      (4 configurations × 38 tests).

## Anti-vacuity evidence (mutation testing)

A green suite is only meaningful if it can go red. Six deliberate bugs were
injected into `src/lib.rs`, rebuilt, and re-run; every one was caught:

| injected bug | tests failed |
|--------------|--------------|
| `wrapping_add` → `saturating_add` in `static_sum` | 10 |
| `driver` loops 9 times instead of 10 | 9 |
| `printf("%d\n")` → `printf("%d ")` | 9 |
| `i * stride` → `i + stride` | 9 |
| `#[no_mangle]` removed from `static_sum` (symbol unexported) | 14, incl. the symbol-parity test |
| off-by-one triggered *only* when `update == 7` (value-dependent) | 3 |

`src/lib.rs` was restored to its original byte content afterwards.

## Two harness bugs found and fixed during verification

Both produced misleading results and are worth recording, since either would
have invalidated the whole exercise:

1. **libtest progress text leaking into the `driver` stdout capture.** fd 1 is
   process-global; with parallel test threads, libtest's own
   `"test foo ... ok"` output landed inside the captured bytes and was reported
   as a C/Rust divergence (the numbers themselves matched). Fixed by
   serialising captures behind a mutex, flushing Rust's line-buffered
   `io::stdout()` before stealing fd 1, pinning `RUST_TEST_THREADS=1` in
   `.cargo/config.toml`, and adding an explicit "harness error" assertion so
   this can never again masquerade as a translation bug.
2. **`cargo test` does not rebuild a `crate-type = ["cdylib"]` artifact.**
   Integration tests never link the cdylib, so `cargo test` left
   `target/*/libStaticLoop.so` stale — the first mutation run reported all six
   injected bugs as *passing*. Fixed by `assert_not_stale()` in the harness
   (the suite now refuses to run against a `.so` older than `src/lib.rs`) and
   by building the cdylib explicitly in `check_all_features.sh`.

## Additional robustness checks

- **Per-test fresh hidden state.** `static_sum` owns a function-scope
  `static int sum`, so the *sequence* of calls is the real input. `Pair::fresh()`
  copies each `.so` to a unique temp path before `dlopen`, giving glibc a
  distinct image with `sum` re-initialised to 0. Row 1 asserts this directly
  (across 64 fresh pairs, the first `static_sum(v)` returns exactly `v`).
- **C optimization invariance.** Signed overflow is UB in C, so the C ground
  truth was rebuilt out-of-tree at `-O0`, `-O2`, `-O3 -fstrict-overflow` and
  `-Os` and the full suite re-run against each. The Rust `wrapping_*`
  arithmetic matched the C at every level (0 failures each time), confirming
  the translation is not tied to one particular codegen of the UB.

## Reproducing

```sh
cd translation && ./check_all_features.sh          # everything, all combos
# or, manually — note the mandatory explicit cdylib build:
cd c_src && cmake -S . -B build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build build
cd ../translation && cargo build --release && cargo test --release
```

`CARGO_FLAGS` (default `--offline`) is forwarded to every cargo invocation.
