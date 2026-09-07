# VERIFICATION.md — completion gate

Library surface: **one** C translation unit (`c_src/src/lib.c`, 62 lines) exporting
**one** function, `int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit)`.

## How to reproduce

```sh
# 1. build the C shared object
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. run the differential suite (it rebuilds the Rust cdylib itself and
#    dlopen()s BOTH .so files via libloading)
cd ../../translation && cargo test --release -- --test-threads=1 --nocapture

# 3. every feature combination
./run_all_features.sh
```

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust.**
      C exports `{encode_quant}`; Rust exports `{encode_quant}`. Symbol diff is
      empty. All Rust undefined symbols are glibc / `_Unwind_*` imports.
      Enforced in-suite by `phase_d_symbol_parity`, which fails on any missing
      C symbol *or* any non-libc undefined symbol.
      No C module was untranslated, so no Phase A "translate the missing source"
      work was needed, and no stubs / `unimplemented!()` exist anywhere.

- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      Rows 1-144 (`lsbit mode x uni class x step class`) x 512 seeded random
      tuples each in `phase_b_config_surface`; rows 145-156 (extreme /
      overflow shapes and forced selection outcomes) in
      `phase_b_extreme_shapes`. Backed by `phase_b_exhaustive_low_bits`,
      `phase_b_random_fuzz`, `phase_b_dense_exhaustive_grid` and
      `phase_b_dense_exhaustive_grid_negative_uni`.
      **156 / 156 rows checked.**

- [x] **Binary stdout comparison: NOT APPLICABLE.**
      `c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)`;
      there is no `add_executable` and no `main()` in the C sources, so the
      project builds no driver binary and there is no stdout to diff.

- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential
      test.** 24 / 24 rows, `phase_c_row01..row24`. The C has an *empty*
      explicit-rejection surface (1 unconditional `return`, 0 asserts, 0 error
      codes, 0 pointer or length parameters), so the table enumerates the real
      boundary surface instead: out-of-range `lsbit` "enum" values (including
      negatives, `INT_MIN`, `INT_MAX`, and one step either side of the only
      special variant `4`) and every signed-overflow / truncating-division
      boundary. Row 23 is the full `7^6 = 117 649` boundary cross-product.

- [x] **All of the above hold under every feature combination.**
      `translation/Cargo.toml` declares no `[features]`, so the combination set
      is exactly `{default}` == `{--no-default-features}`. `run_all_features.sh`
      derives this mechanically from `Cargo.toml` and runs `cargo check` plus
      the full suite for each; both configurations pass 22/22.

## Test totals

`cargo test --release`: **22 tests, 22 passed, 0 failed**, ~1.5 s.
Roughly **50 million** C-vs-Rust differential calls, all byte-identical.

| test | differential calls |
|---|---|
| `phase_b_dense_exhaustive_grid` | 38 738 960 |
| `phase_b_dense_exhaustive_grid_negative_uni` | 9 391 200 |
| `phase_b_random_fuzz` | 300 000 |
| `phase_b_extreme_shapes` | 159 876 |
| `phase_c_row23_boundary_cross_product` | 117 649 |
| `phase_b_exhaustive_low_bits` | 102 168 |
| `phase_b_config_surface` | 73 728 (144 rows x 512) |
| `phase_c_row01..row24` (the rest) | ~1.9 M |

Branch-coverage evidence asserted inside the tests (they fail if a branch is
never reached): both selection outcomes occur in `phase_b_config_surface`
(`result==uni` 40 401 / `result!=uni` 33 327), and all three
`uni` / `uni1` / `uni2` selections occur in `phase_b_extreme_shapes`
(15 430 / 12 318 / 27 548).

## Harness properties

* The Rust side is **never** called directly. `rust_so_path()` runs
  `cargo build --release` and then `libloading::Library::new()` +
  `dlsym("encode_quant")`, so the `#[no_mangle] extern "C"` export wrapper is
  itself under test. The C side is loaded the same way.
* The harness rebuilds the cdylib on every run, so a stale artifact cannot
  silently pass.
* Randomness is `SplitMix64` with a fixed per-row seed, so failures reproduce.

## Adversarial validation of the harness (mutation testing)

A passing suite proves nothing unless it can fail. 23 deliberate bugs were
injected into `translation/src/lib.rs`, one at a time, the cdylib rebuilt, and
the suite re-run:

**20 / 23 killed.** The 3 survivors are *provably semantically equivalent* to
the original, i.e. not test-coverage gaps:

| survivor | why it cannot change behaviour |
|---|---|
| `lsbit_order` — test `lsbit & 1` before `lsbit == 4` | `4 & 1 == 0`, so the two dispatch orders agree. Verified **exhaustively over all 2^32 `lsbit` values**: 0 differ. |
| `neg_saturating` — `saturating_neg()` for `wrapping_neg()` | They differ only at `INT_MIN`. `diff` is always `P / 8` for some `i32 P`, so `diff` is bounded to `[-268435456, 268435455]`. Verified **exhaustively over all 2^32 numerators**: `diff == INT_MIN` occurs 0 times. |
| `uni2_clamp_uni1` — `uni2 = uni1` instead of `uni2 = uni` | The `uni2` clamp fires only when `uni & 7 == 0`, in which case the `uni1` clamp does *not* fire, so `uni1 == uni + 1`. The mutant then makes `d2 == d1`, and the pair `if (d1<d0) uni=uni1; if (d2<d0) uni=uni2;` yields the same final value as the original (where `uni2 == uni` forces `d2 == d0`, so the second `if` never fires). Confirmed empirically: 0 mismatches over an independent 2 M-call fuzz. |

Killed mutants included: floor-instead-of-truncating division (`div_euclid(8)`),
`>> 3` instead of `/ 8`, logical instead of arithmetic `>> 31`, `<=` for `<`,
`d3 >> 4` for `d3 >> 5`, `uni & 15` for `uni & 7`, `& !15` for `& !7` in the
clamp test, `& 4` for `& 8` on the sign bit, `|` for `&` in the `lsbit == 4`
bit-0 recovery, wrong shift amounts there, `uni + 2` for `uni + 1`, dropped
`+ 1` in `2*(uni&7)+1`, `pred - diff` for `pred + diff`, `tgt` for `tgt2`,
`p1` for `p0`, swapped `uni1`/`uni2` selection, comparing against the updated
best instead of `d0`, `lsbit > 0` for `lsbit != 0`, and set-instead-of-clear in
the even-`lsbit` branch.

## Robustness beyond the graded build

`c_src` compiles with `C_FLAGS = -fPIC` and an empty `CMAKE_BUILD_TYPE`, i.e.
**no optimization** (gcc 11.5.0). Because the C relies on signed-integer
overflow (`uni + 1` at `INT_MAX`, `(2*(uni&7)+1)*step`, `pred + diff`,
`tgt - p0`, `-diff`) — which is UB, not guaranteed wrapping — the same `lib.c`
was additionally compiled at `-O0`, `-O1`, `-O2`, `-O3` and `-O2 -fwrapv` and
all six builds compared against the Rust `.so` over 2 M spicy random inputs plus
an exhaustive low-bit sweep:

```
reference: c_src/build/libharvest-work-cHZhAC.so
  translation/target/release/libencode_quant_lib.so   mismatches=0  identical
  gcc -O0   mismatches=0  identical
  gcc -O1   mismatches=0  identical
  gcc -O2   mismatches=0  identical
  gcc -O3   mismatches=0  identical
  gcc -O2 -fwrapv  mismatches=0  identical
```

So the Rust's `wrapping_*` model matches the C regardless of optimization level.

The Rust `.so` was also built in the **debug** profile (`overflow-checks = on`)
and re-compared: `mismatches=0`, no panics — confirming every arithmetic site is
explicitly wrapping rather than accidentally relying on release-mode wraparound.

## Changes made to `translation/`

The pre-existing `src/lib.rs` translation was found **correct**; no behavioural
fix was required. Changes were additive only:

* `Cargo.toml`: added `[dev-dependencies] libloading = "0.8"`.
* `tests/differential.rs`: new (the whole harness).
* `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`, `VERIFICATION.md`: new (Phase A-D artifacts).
* `run_all_features.sh`: new (Phase D feature-combination driver).

Nothing in `c_src/` was modified.
