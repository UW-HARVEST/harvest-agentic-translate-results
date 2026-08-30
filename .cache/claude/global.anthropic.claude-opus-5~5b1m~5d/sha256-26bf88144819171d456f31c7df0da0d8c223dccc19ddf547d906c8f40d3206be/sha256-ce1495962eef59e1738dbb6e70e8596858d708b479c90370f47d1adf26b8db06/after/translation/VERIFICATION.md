# Verification report — C `Sieve` vs. Rust `Sieve`

## What is under test

| | |
|---|---|
| C ground truth | `c_src/src/sieve.c` (40 lines, 18 of them licence header) → `c_src/build/libSieve.so` |
| Rust translation | `translation/src/lib.rs` → `target/<profile>/libSieve.so` (`crate-type = ["cdylib"]`) |
| Public API | `void sieve(int val)` — the only exported symbol of either library |
| Observable behaviour | the stdout byte stream produced by `printf("%d\n", val)` in a loop |

Both libraries are exercised **only** through `dlopen`/`dlsym` (`libloading`)
on their `.so` files. No Rust function is ever called directly, so the
`#[unsafe(no_mangle)] extern "C"` export wrapper is part of what is tested.

## How to reproduce

```bash
# 1. build the C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. run the differential suite (the harness rebuilds the Rust cdylib itself)
cd translation && cargo test

# 3. all build configurations (feature combos x debug/release)
./run_all_configs.sh

# 4. prove the suite is not vacuous
./mutation_check.sh
```

## Test layout

| file | phase | contents |
|------|-------|----------|
| `tests/common/mod.rs` | harness | child-process capture, FIFO capture, bounded-prefix capture, closed-stdout runs, threaded runs, PCG32 RNG, reference model |
| `tests/valid_paths.rs` | **B** | 22 tests, one per `CONFIGS.md` row |
| `tests/error_paths.rs` | **C** | 12 tests, one per `ERRORS.md` row |
| `tests/symbols.rs` | **D** | `nm -D` parity, unresolved-import check, feature-axis and C-project-shape guards |

### Why every measurement runs in a child process

The only observable of this library is fd 1. Redirecting fd 1 inside a
multi-threaded libtest process would race with the harness's own output, so
each measurement re-executes the test binary (`--exact common::child_worker`),
and the child `dup2`s a private file/FIFO onto fd 1 before calling `sieve`.
This also makes the two libraries impossible to confuse: each child `dlopen`s
exactly one `.so`, even though both export the same symbol name.

### A real harness bug that was found and fixed

The first version of the harness loaded `target/debug/libSieve.so` as produced
by an earlier `cargo build`. **`cargo test` does not rebuild a cdylib-only
library** (integration tests cannot link one), so the suite was comparing the C
library against a *stale* artifact. A deliberately broken `src/lib.rs` still
passed all 41 tests. `rust_lib()` now builds the cdylib from current sources
into a dedicated `CARGO_TARGET_DIR` (`target/difftest`, which has its own lock
and therefore cannot deadlock against the outer `cargo test`) and asserts the
artifact is newer than `src/lib.rs` and `Cargo.toml`.

## Behaviours of the C code that the translation must (and does) reproduce

1. **Truncated modulo.** `val % 10` in C truncates toward zero, so for negative
   `val` the remainder is in `-9..=0` and **never** equals 9. `sieve(-9)` does
   *not* stop at `-9`; it counts all the way up to `+9`. Rust's `%` has the
   same truncating semantics, so `val % 10 == 9` is a correct transcription
   (`rem_euclid` would **not** be — that mutation is caught by 11 tests).
2. **Print-then-test.** The value is printed *before* the exit test, so the
   starting value is always emitted, even when it already ends in 9.
3. **Signed overflow wraps.** For `val ≥ 2147483640` no reachable value ends in
   9, so `val++` overflows — UB in C. The shipped build (`-O0`) wraps to
   `INT_MIN` and keeps going; GCC 11.5 wraps at `-O2`/`-O3` too. Rust uses
   `wrapping_add(1)`, which matches (`saturating_add` is caught).
4. **`printf` errors are ignored.** The C code never checks `printf`'s return
   value, so with fd 1 closed the loop still runs to completion and returns
   normally. The Rust version likewise discards the result.
5. **The platform `printf` is reused**, so `%d` formatting and stdio buffering
   are byte-identical rather than reimplemented.
6. **No state, no configuration.** No globals, no statics, no init, no flags —
   so calls are independent and the function is reentrant.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports exactly one defined
      symbol, `sieve`; the Rust `.so` exports it under the identical name. Diff
      C→Rust is **empty**. No hard-undefined symbol of the Rust `.so` is
      unprovided by the libraries the loader binds. Re-checked mechanically by
      `tests/symbols.rs` on every run, so it cannot go stale.
- [x] **Phase B** — all **22** `CONFIGS.md` rows pass, with fixed-seed
      randomized inputs per row (≈2 000 distinct inputs and ≈12 million
      compared output lines in total), including exhaustive sweeps of
      `[-300,-1]`, `[10,99]`, `[-64,64]`, `[2147483630, 2147483639]`, a
      10⁶-iteration run, FIFO output, and 2/4/8/16-thread concurrent callers.
- [x] **Phase C** — all **12** `ERRORS.md` rows have a passing differential
      test. The C library has a provably empty explicit error surface (0
      `return`s, 0 asserts, 0 range checks, 0 null checks, 0 enums — verified
      by grep and re-asserted by a test), so each row asserts identical
      observable behaviour and identical process termination for the exact
      triggering input. Covered: all-negative inputs, `INT_MIN`, `INT_MAX`, the
      whole 8-value overflow region, hostile bit patterns, garbage in the upper
      half of the argument register, and a closed stdout.
- [x] **Every build configuration** — default / `--no-default-features` /
      `--all-features` × debug / release: 6 configurations, all green
      (`run_all_configs.sh`). `Cargo.toml` declares no `[features]`, and a test
      fails if one is ever added without revisiting these phases.
- [x] **Suite is not vacuous** — `mutation_check.sh` injects 9 plausible
      mistranslations (wrong terminator digit, `rem_euclid`, extra `-9` case,
      `+2` increment, `saturating_add`, test-before-print, CRLF, `%u`, `%ld`)
      and every one is detected.

## Result

**No divergence between the C and Rust implementations was found.**
`translation/src/lib.rs` needed no behavioural changes; the only code changes
made were to the test harness (plus `libloading` in `[dev-dependencies]`).
Nothing in `c_src/` was modified.
