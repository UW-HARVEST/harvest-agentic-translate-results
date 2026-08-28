# Verification report

C ground truth: `c_src/src/lib.c` (273 lines, BTAC1C2 sample predictors).
Rust under test: `translation/src/lib.rs` → `libcall_predict_lib.so`.

## How to reproduce

```sh
# C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# Rust artifacts + suite (crates.io is unreachable here; the registry cache
# already holds libloading 0.8.9, hence --offline)
cd translation
cargo build --offline
cargo build --offline --release
cargo test  --offline

./check_symbols.sh           # nm -D parity (Phase A / D)
./run_all_feature_combos.sh  # whole suite under every feature combination
./mutation_check.sh          # proves the suite can actually fail
```

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D --defined-only` on the C `.so` yields exactly one
      symbol, `call_predict`; the Rust `.so` exports it too. `comm -23` of the
      two sorted symbol lists is **empty**. `nm -D -u` on the Rust `.so` shows
      **0** missing/undefined non-libc symbols (all remaining entries are glibc /
      libgcc-unwind). Verified mechanically by `check_symbols.sh`, which is also
      run inside `run_all_feature_combos.sh`.
      No C module was skipped: `lib.c` is the only C source file and every one
      of its 15 functions is present in `src/lib.rs`. Nothing is stubbed and
      there is no `unimplemented!()`/`todo!()` anywhere.
- [x] **Phase B** — all 37 `CONFIGS.md` rows pass over randomized, fixed-seed
      inputs (`tests/differential.rs` 18 tests, `tests/internal_predictors.rs`
      21 tests).
- [x] **Phase C** — all 14 `ERRORS.md` rows have a passing error-path
      differential test (`tests/errors.rs`), each asserting the *same* sentinel
      (`0`), not merely "both failed".
- [x] **Phase D** — 53 tests × 2 feature combinations (`default` and
      `--no-default-features`; `Cargo.toml` declares no `[features]`, so that is
      the complete set) — all pass, symbol diff empty in both.

## Test inventory (53 tests)

| file | tests | covers |
|------|-------|--------|
| `tests/differential.rs` | 18 | public ABI `call_predict` via `dlsym`, rows C1–C15, C36–C37, plus the `btac1c_idxstate` layout guard |
| `tests/errors.rs` | 14 | rows E1–E14 |
| `tests/internal_predictors.rs` | 21 | rows C16–C35: the 12 specialised predictors, the generic switch (cases 0–15), the FIR rows, `idx` masking, and the `GetPredictFunc` dispatch table |

Both `.so`s are always loaded with `libloading`; the Rust side is never called
as a Rust function. The `static` C predictors are reached through
`runtime_addr(call_predict) − link_addr(call_predict) + link_addr(sym)` on both
libraries, so even the internal entry points are called across the real ABI
boundary. Every comparison also asserts that neither library mutated `psamp` or
`btac1c_idxstate`.

## What the differential testing established

* `call_predict` agrees for the full contiguous band `−1000..=4096`, every
  `±2^k`, `INT_MIN`/`INT_MAX`, and ~45 000 fixed-seed random `i32` selectors.
  The C contract (`1` iff `pfcn ∈ 0..=11`, else `0`) holds in both.
* The `--release` Rust artifact folds the function-pointer comparisons away and
  DCEs the predictors; the `--debug` artifact keeps them as 13 distinct
  functions. Both agree with the C `-O0` build (rows C36/C37), and in both
  builds `GetPredictFunc(0..11)` yields 12 pairwise-distinct pointers.
* Every predictor matches bit-exactly across the `idx` × `psamp` × `firfx`
  matrix, including `i32::MIN`/`i32::MAX` operands where the multiply/add chains
  wrap, and negative operands where truncating `/16`, `/64`, `/256` differ from
  the arithmetic `>>` used by the other cases.
* The three deliberate C quirks are preserved, and a dedicated test
  (`cfg_c28b_generic_vs_specialised_discrepancy_preserved`) asserts the
  generic-vs-specialised *disagreement* pattern is identical in both libraries
  and that the `pfcn` 10/11 divergences are actually observed — so a "helpful"
  correction of the C would be caught rather than silently accepted.

## Harness sensitivity (mutation testing)

`mutation_check.sh` injects 11 real bugs into `src/lib.rs` (one at a time),
rebuilds both artifacts and re-runs the suite. The suite fails for **all 11**,
and correctly still passes for the one deliberately behaviour-preserving
control mutant (`M6c`). No source change is left behind — the script restores
`src/lib.rs` and rebuilds.

## Changes made during verification

* `Cargo.toml`: added `[dev-dependencies] libloading = "0.8"` (and the resulting
  `Cargo.lock` entries).
* Added `tests/common/mod.rs`, `tests/differential.rs`, `tests/errors.rs`,
  `tests/internal_predictors.rs`, `check_symbols.sh`,
  `run_all_feature_combos.sh`, `mutation_check.sh`, `SYMBOLS.md`, `ERRORS.md`,
  `CONFIGS.md`, `VERIFICATION.md`.
* **No changes were needed to `src/lib.rs`** — `cargo check` was clean on the
  first run and no divergence from the C was found. Nothing in `c_src/` was
  modified.
