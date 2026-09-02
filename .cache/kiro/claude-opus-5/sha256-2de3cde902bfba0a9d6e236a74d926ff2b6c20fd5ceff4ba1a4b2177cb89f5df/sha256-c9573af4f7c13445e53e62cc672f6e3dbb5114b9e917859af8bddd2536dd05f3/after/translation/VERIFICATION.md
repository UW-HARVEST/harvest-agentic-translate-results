# VERIFICATION.md — completion record

Reproduce everything with:

```sh
cd translation && ./run_all.sh --exhaustive
```

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff between the C `.so` and the Rust `.so`
      is **empty** in both directions; **0** unresolved non-libc symbols in
      the Rust `.so`. Verified for all four cdylib build configurations.
- [x] **Phase B** — all **24** rows of `CONFIGS.md` pass, across randomized
      inputs (fixed-seed SplitMix64) *and* an exhaustive enumeration of the
      complete input domain.
- [x] **Phase C** — all **11** rows of `ERRORS.md` have a passing
      differential test asserting the *same* sentinel value (`0`), plus 3
      extra generic-boundary tests.
- [x] **Every configuration** — `translation/Cargo.toml` declares no
      `[features]`, so the feature space is `{default}` ≡
      `{--no-default-features}`; both are run. In addition the cdylib itself
      is rebuilt and retested under 4 build configurations, because pointer
      identity (what `get_predict_func` is built on) is precisely what
      optimisation level, LTO and identical-code-folding can perturb.

## What was tested and how

The library's only exported symbol is `int get_predict_func(int pfcn)`. Both
implementations are loaded as shared objects with `libloading` and driven
through `dlsym`; the Rust code is **never** called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrapper is itself under test.

| test file | tests | covers |
|---|---|---|
| `tests/common/mod.rs` | (harness) | dual `dlopen`, byte-for-byte compare, fixed-seed PRNG, source-derived oracle |
| `tests/phase_b_valid_paths.rs` | 23 | `CONFIGS.md` rows 1–23 |
| `tests/phase_c_error_paths.rs` | 14 | `ERRORS.md` rows 1–11 + generic boundaries |
| `tests/phase_d_symbols.rs` | 4 | symbol parity, no leaked internal-linkage symbols, no unresolved imports |
| `tests/exhaustive.rs` | 1 (`#[ignore]`) | `CONFIGS.md` row 24 — all 2³² inputs |

## Exhaustive result

The exported input domain is a single 32-bit `int`, so it is finite and was
enumerated **completely**:

```
release-default          OK  (4294967296 values checked)
dev-unopt                OK  (4294967296 values checked)
release-lto-fat-cgu1     OK  (4294967296 values checked)
release-opt-z            OK  (4294967296 values checked)
```

Zero divergences. For every one of the 4 294 967 296 possible arguments, the
C `.so` and the Rust `.so` return an identical `int`, and both agree with the
oracle derived from the C source (`1` for `pfcn ∈ 0..=11`, `0` otherwise).
This is a complete equivalence result for this ABI surface, not a sample.

## Negative control (proof the harness is not vacuous)

To confirm the tests can actually fail, one arm of the Rust
`get_predict_func` switch was temporarily changed to compare against the
wrong predictor (`case 11` → `_Pfn10`). The suite immediately failed with:

```
assertion `left == right` failed: divergence at pfcn = 11 (0x0000000b): C = 1, Rust = 0
...
test result: FAILED. 7 passed; 16 failed
```

`src/lib.rs` was then restored (`diff` against the pre-change backup: clean)
and the suite returned to green.

## Translation notes — C oddities deliberately preserved

The C source contains two internal inconsistencies. Both are reproduced
verbatim in the Rust and are **not** "fixed":

| C location | `BTAC1C2_PredictSample` arm | standalone `_PfnNN` |
|---|---|---|
| `case 10` vs `_Pfn10` | `(5*p0 - p1) >> 4` | `(5*p0 - p1) >> 3` |
| `case 11` vs `_Pfn11` | `(p0 + p1) >> 3` | `(p0 + p1) >> 1` |

These are unobservable through the exported ABI (the predictor pointers are
only ever *compared*, never called, and all the predictors have internal
linkage), but the Rust mirrors the C expression-for-expression anyway so the
translation stays faithful if the surface is ever widened.

Other fidelity details:

- All arithmetic uses `wrapping_*` / `wrapping_div`, matching C's two's
  complement `int` behaviour on this target rather than panicking.
- `psamp[(i - n) & 7]` keeps the C masking semantics (`& 7` on a negative
  two's-complement value yields the low three bits), so the index is always
  in `0..=7`.
- `struct btac1c_idxstate_s` is translated `#[repr(C)]` with identical field
  order and types.
- The C `static` helpers stay private in Rust. Exporting them would be a
  *divergence* from the C's dynamic symbol table, and `phase_d_symbols.rs`
  asserts they are not leaked.

## Files created / changed

Under `translation/` only — `c_src/` was read but never modified:

```
translation/SYMBOLS.md                  (new)
translation/ERRORS.md                   (new)
translation/CONFIGS.md                  (new)
translation/VERIFICATION.md             (new, this file)
translation/run_all.sh                  (new)
translation/tests/common/mod.rs         (new)
translation/tests/phase_b_valid_paths.rs(new)
translation/tests/phase_c_error_paths.rs(new)
translation/tests/phase_d_symbols.rs    (new)
translation/tests/exhaustive.rs         (new)
translation/Cargo.toml                  (added libloading 0.8 to dev-dependencies)
translation/src/lib.rs                  (unchanged — no divergence was found)
```
