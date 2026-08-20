# Verification report — C ↔ Rust differential testing

Library under test: `hex2bin` (single translation unit, `c_src/src/lib.c`).

## Completion gate

| gate | status | evidence |
|---|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`, 0 non-libc undefined | **PASS** | `comm -23 c_syms rust_syms` empty; see `SYMBOLS.md`; automated by `tests/harness.rs::symbol_parity` |
| Phase B: every `CONFIGS.md` row (1–33) passes across randomized/exhaustive inputs | **PASS** | `tests/valid_paths.rs` (30 tests), `tests/exhaustive.rs` (3 tests) |
| Phase C: every `ERRORS.md` row (1–20) has a passing error-path differential test | **PASS** | `tests/error_paths.rs` (21 tests) |
| All of the above under EVERY feature combination | **PASS** | `Cargo.toml` has no `[features]` → exactly one combination (`--no-default-features`); enumerated by `scripts/feature_combos.sh`, run by `scripts/run_diff_tests.sh` |

Total: **57 differential tests**, ≈ 35 million C-vs-Rust comparisons, all
matching byte-for-byte (return value, whole `bin` buffer incl. slack,
`*hex_end_p` offset, `hex` buffer after the call).

## Changes made to the Rust translation

**None were required** — the translation in `src/hex2bin.rs` already reproduced
the C behaviour exactly, including:

* the `unsigned int` promotion / truncation chain of the classifier
  (`c_num0`, `c_alpha0`, `c_alpha`),
* `state = ~state` toggling between `0` and `0xFF`,
* the `strchr(ignore, 0)` quirk (an embedded NUL is treated as an ignorable
  character whenever `ignore != NULL` and the parser is byte-aligned),
* the `hex_pos--` rewind on an odd digit count,
* keeping the bytes already written to `bin` while still returning `-1`,
* the strict-mode (`hex_end_p == NULL`) "unconsumed input" rejection.

Work added by this task: `libloading` dev-dependency, the test harness
(`tests/`), the Phase A artifacts (`SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`),
and `scripts/`. Nothing in `c_src/` was modified (only `c_src/build/`, the
CMake output directory, was created).

## How to reproduce

```sh
# everything: C build + cargo check/build/test for every feature combination
./scripts/run_diff_tests.sh

# harness self-validation (mutation testing)
python3 scripts/mutation_check.py

# cross-check optimized builds against each other
cmake -S c_src -B "$TMPDIR/cbuild_o2" -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build "$TMPDIR/cbuild_o2"
cargo build --release
HEX2BIN_C_SO="$TMPDIR/cbuild_o2/libtranslated_rust.so" \
HEX2BIN_RUST_SO="$PWD/target/release/libhex2bin_lib.so" cargo test
```

## Two traps found while verifying (and closed)

1. **`cargo test` does not rebuild a `cdylib`-only lib target.** A bare
   `cargo test` loads a *stale* `.so`, which produced false PASSes early on
   (mutated Rust code appeared to "match" the C). Closed by (a) always running
   `cargo build` first in `scripts/run_diff_tests.sh`, and (b) a freshness guard
   in the harness that panics with `STALE SHARED OBJECT` if any `.so` is older
   than its sources.
2. **A crashing implementation never prints libtest's summary line.** Detection
   logic that greps for `test result: FAILED` silently misses mutants that
   segfault/abort; `scripts/mutation_check.py` keys off the process exit status
   instead.

## Harness power (mutation testing)

`scripts/mutation_check.py` injects 29 single-edit mutations into
`src/hex2bin.rs`, rebuilds, and re-runs the whole suite:

* **26 behaviour-changing mutants → all 26 detected** (nibble accumulator,
  classifier constants `48`/`55`/`10`/`16`, the `~32` case-folding mask, the
  `>= bin_maxlen` bound, the `state == 0` and `ignore != NULL` guards, the
  `hex_pos--` rewind, the strict-mode check, the `hex_end_p` NULL check, the
  reported `hex_end`, the stored byte, the error value `-1` → `-2`, …).
* **3 provably equivalent mutants → correctly not detected** (control group):
  `state = state ^ 1` (state is only compared against `0`), `bin_pos = 1`
  instead of `0` on the error path (dead store — `ret` is returned), and
  `as u16 as u8` truncation (identical low byte).

Result: `29/29 mutants behaved as expected (0 unexpected)`.
