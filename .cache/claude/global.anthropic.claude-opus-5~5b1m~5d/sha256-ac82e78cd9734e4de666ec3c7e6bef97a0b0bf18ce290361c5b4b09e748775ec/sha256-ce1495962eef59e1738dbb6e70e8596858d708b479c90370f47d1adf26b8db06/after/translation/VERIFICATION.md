# VERIFICATION.md — completion gate

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `../c_src`.

Both libraries are always loaded as shared objects with `libloading`
(`dlopen`/`dlsym`) and called through the exported `synth_pair` symbol. **No Rust
function is ever called directly**, so the `#[no_mangle] extern "C"` wrapper and
the C ABI are part of what is under test.

## The library under test

`c_src` is a single translation unit (`src/lib.c`, 34 lines) exposing one
function:

```c
typedef int16_t mp3d_sample_t;
void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
```

plus one `static` helper, `mp3d_scale_pcm`. This is the final
sub-band-synthesis pair-output stage of a minimal MP3 decoder. There are no
options, no state, no `#ifdef`s and no Cargo features, so the verification
surface is entirely *input shapes and value ranges*.

## Gate

| requirement | status | evidence |
|---|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust | **PASS** | C exports exactly `{synth_pair}`; Rust exports exactly `{synth_pair}`; symbol diff empty. `tests/symbols.rs` (5 tests) + the `comm -23` diff inside `run_all_feature_combos.sh`, re-checked per feature combo and per profile. |
| Phase B: EVERY `CONFIGS.md` row passes across randomized inputs | **PASS** | 20 rows `C1`–`C20` (`tests/configs.rs`) + 6 exhaustive rows `X1`–`X6` (`tests/exhaustive.rs`) + `C21` rounding modes (`tests/rounding_mode.rs`) = **27 rows**. |
| Phase C: EVERY `ERRORS.md` row has a passing error-path differential test | **PASS** | **25 rows** `E1`–`E25`, 20 tests in `tests/errors.rs`. |
| All of the above under EVERY feature combination | **PASS** | `./run_all_feature_combos.sh` → `default` and `--no-default-features` (Cargo.toml declares no `[features]`, so that is the complete matrix), each in **both** the `dev` and `release` profiles → 4 configurations, 57 tests each. |

```
$ ./run_all_feature_combos.sh
...
ALL FEATURE COMBINATIONS x BOTH PROFILES: PASS
```

## Scale of the differential evidence

| kind | count |
|---|---|
| `#[test]` functions | **57** (configs 20, errors 20, symbols 5, exhaustive 5, equivalence 6, rounding_mode 1) |
| **Exhaustive** full-domain `f32` sweeps (all 2^32 bit patterns, C vs Rust) | 2 (`z[448]`, `z[514]`) = **8 589 934 592** comparisons |
| Exhaustive sweep validating the reference model against the real C `.so` | **4 294 967 296** comparisons |
| Exhaustive `f32` equivalence proofs for surviving mutants | 4 x **4 294 967 296** |
| Strided full-domain sweeps over all 23 read taps / all 16 weights | ~30 M |
| Difference-pair and sum-pair sweeps against 13 pinned special values | ~9 M |
| Randomized property rows (`CONFIGS.md`) | ~430 K calls |
| Error/boundary rows (`ERRORS.md`) | ~40 K calls |

## Findings — things that were actually fixed

1. **`16 * nch` overflow semantics.** GCC emits `shl $0x4,%eax; cltq`, i.e. the
   product wraps in 32 bits *before* being sign-extended to a pointer offset.
   The original Rust computed `16isize * nch as isize`, which does not wrap and
   would index wildly out of bounds for `nch >= 2^27`. Fixed to
   `16i32.wrapping_mul(nch) as isize` + `wrapping_offset`. Covered by
   `ERRORS.md` E19/E20; the mutant "nch index without int wraparound" is killed.
2. **`s -= (s < 0)` overflow trapping.** The C evaluates this in `int` and
   truncates back to 16 bits, so it wraps. Switched to `wrapping_sub` so a
   debug-profile overflow panic can never diverge from C.
3. **Null-pointer behaviour under the `dev` profile.** rustc's debug assertions
   insert a language-UB null check, turning a null `pcm` into a Rust panic and
   **SIGABRT**, where the C faults with **SIGSEGV**. `[profile.dev]` now sets
   `debug-assertions = false` / `overflow-checks = false`, documented in
   `Cargo.toml`. Caught by `ERRORS.md` E21.

## Why the passing tests are trustworthy

Passing tests only mean something if they *can* fail. `./mutation_check.sh`
injects 45 deliberate bugs into `src/lib.rs` and requires the suite to fail:

```
killed by fast suite       : 40
killed only by exhaustive  : 0
survived everything        : 5
```

The 5 survivors are each **proven observationally equivalent** to the C by an
exhaustive 2^32 enumeration in `tests/equivalence.rs` — see
`EQUIVALENT_MUTANTS.md`. In particular the four *precision-changing* mutants
(f64 excess precision, and FMA contraction as `-ffp-contract=fast` would
produce) are all **killed**, which is the evidence that the suite pins the exact
single-precision, non-contracted evaluation order the C compiles to.

Two further guardrails:

* **Anti-vacuity assertions.** Rows assert they actually reached the branch they
  claim to cover (e.g. `cfg_c4_near_clamp_thresholds` requires >100 high clamps,
  >100 low clamps *and* >100 conversion-path results; `cfg_c9` requires >100 NaN
  taps; `differential_under_every_rounding_mode` requires the non-default modes
  to actually change results).
* **Stale-artifact guard.** `cargo test` does not rebuild a `cdylib`-only lib
  target, so an old `.so` could silently be tested. `common::rust_so_path()`
  compares mtimes against `src/lib.rs` and panics rather than testing stale
  code. (This guard fired for real during development.)

## Independent structural check

The Rust arithmetic was also mechanically diffed against the C — every term
extracted from `c_src/src/lib.c` as `(z-index tuple, weight, is_subtraction)`
and compared to the same extraction from `src/lib.rs`:

```
lane0: C terms == Rust terms -> True (8 terms)
lane1: C terms == Rust terms -> True (8 terms)
```

and the 16 `f32` weight constants in the C `.so`'s `.rodata`
(`29, 213, 459, 2037, 5153, 6574, 37489, 75038, 104, 1567, 9727, 64019, -9975,
-45, 146, -5`, plus the guards `32766.5 / -32767.5 / 0.5`) match the Rust
literals exactly.

## Reproducing

```sh
cd translation
./run_all_feature_combos.sh     # the gate: all combos x both profiles  (~6 min)
./mutation_check.sh             # suite-sensitivity check               (~10 min)
```

Individual phases:

```sh
cargo build --release && cargo test --release --test symbols      # Phase A/D
cargo build --release && cargo test --release --test configs      # Phase B
cargo build --release && cargo test --release --test exhaustive   # Phase B (exhaustive)
cargo build --release && cargo test --release --test rounding_mode
cargo build --release && cargo test --release --test errors       # Phase C
cargo build --release && cargo test --release --test equivalence  # mutant proofs
```

`cargo build` must precede `cargo test` because the crate is `cdylib`-only;
`build.rs` also compiles the C library out-of-tree (into `OUT_DIR`, so nothing
under `c_src/` is modified) and exports its path to the tests.
