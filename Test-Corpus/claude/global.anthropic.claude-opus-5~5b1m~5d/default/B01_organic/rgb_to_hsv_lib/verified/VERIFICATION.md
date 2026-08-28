# VERIFICATION.md — completion report

Differential verification of the Rust translation in `translation/src/lib.rs`
against the C ground truth in `c_src/`.

Reproduce everything with one command:

```sh
cd translation && ./verify_all.sh
```

## The library under test

`c_src` is a single translation unit exposing a single function:

```c
void rgb_to_hsv(float *dest, const float *src);   /* c_src/include/lib.h */
```

38 lines of C, no options, no state, no error returns, no `#ifdef`, one
early-out `return`. The Rust translation is a literal, line-for-line port.

## Result

| gate | status |
|---|---|
| `cargo check` clean (0 errors, 0 warnings) | **PASS** |
| `SYMBOLS.md` — symbol diff `nm -D` C vs Rust | **EMPTY** (1 export each; 0 missing, 0 extra, 0 undefined non-libc) |
| `CONFIGS.md` — 36/36 rows pass, bit-for-bit | **PASS** |
| `ERRORS.md` — 14/14 rows have a passing test | **PASS** |
| every gate under every feature combination | **PASS** (2 combos — see below) |
| every gate under every build artifact | **PASS** (5 artifact pairs) |
| mutation negative control | **PASS** (12/12 real bugs caught, 1/1 equivalent mutant correctly ignored) |

Totals: **59 tests × 5 artifact configurations**, roughly **230 000 input
vectors per run**, every output compared as raw bits (`f32::to_bits`), so a
wrong zero sign or a wrong NaN payload is a failure.

## Test layout

| file | purpose |
|---|---|
| `tests/common/mod.rs` | loads BOTH `.so`s with `libloading`; PCG32 fixed-seed RNG; bit-exact diff harness |
| `tests/probe.rs` | 3 smoke tests (symbol load, basic colours, signed-zero/NaN) |
| `tests/phase_b_configs.rs` | 36 tests, one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 15 tests, one per `ERRORS.md` row + the child-process worker |
| `tests/phase_d_symbols.rs` | 5 tests enforcing symbol parity and non-stubbiness |
| `mutation_check.sh` | negative control: proves the suite detects real bugs |
| `verify_all.sh` | driver: builds C + Rust, enumerates feature combos, runs everything |

**The Rust code is never called directly.** Every call goes through
`dlopen`/`dlsym` on `librgb_to_hsv_lib.so`, so the `#[no_mangle] extern "C"`
export wrapper is exercised exactly as an external C caller would exercise it.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**. `verify_all.sh`
parses the manifest mechanically (it does not hard-code the list), finds 0
optional features, and therefore verifies the only two meaningful invocations:
`cargo test` and `cargo test --no-default-features`. If a feature is added
later, the script automatically expands to the full subset cross-product.

## Behaviours that required care (and are covered)

These are the places a plausible translation would silently diverge; each has a
dedicated row and each is confirmed caught by the mutation control.

1. **`MIN`/`MAX` are C ternaries, not `f32::min`/`f32::max`.**
   `(((a) < (b)) ? (a) : (b))` returns the **second** operand whenever `<` is
   false — which includes ties *and* unordered (NaN) comparisons. `f32::min`
   suppresses NaN and normalises signed zeros, so using it changes results.
   Observable via `±0.0` (sign bit of `v`) and via NaN placement.
   *Mutant `M01` confirms 11 rows catch this.*

2. **NaN slot position changes the code path.** A NaN in `b` is adopted by the
   last ternary, so `min = max = NaN` and **branch C** runs; a NaN in `r` is
   discarded immediately; a NaN in `g` makes `max = b`. Three different paths
   (rows 25–27), plus payload/quieting checks — the C's `sNaN 0x7F800001`
   quiets to `0x7FC00001` and the Rust matches bit-for-bit.

3. **`delta == 0 || max == 0` short-circuits.** The `||` is what prevents a
   `delta / 0` division; dropping it injects `Inf` into `s`. *Mutant `M02`.*

4. **Tie precedence at the maximum** (`r == max` tested before `g == max`).
   Rows 10–12. Notably, swapping the `r`/`g` precedence is a **provably
   equivalent** change (when `r == g == max`, `min` is `b`, so branch A gives
   `(max-b)/(max-b) = 1.0` and branch B gives `2 + (b-max)/(max-b) = 1.0`,
   both exact) — recorded as `E01_tie_precedence_g_before_r` and asserted *not*
   to be flagged, so the suite is shown not to fail indiscriminately.

5. **Load-before-store ordering.** All three `src` loads precede all `dest`
   stores (verified in the C `objdump`), so `dest == src` and partially
   overlapping buffers are well defined. Rows 29–31. *Mutant `M08` is caught
   only by these three rows* — without them this bug would be invisible.

6. **No clamping anywhere.** Out-of-`[0,1]` input yields `s > 1`, negative `s`,
   unbounded `v`, `Inf` and `NaN`; the Rust must be equally unclamped.

## One divergence was found, diagnosed, and correctly attributed

The null-pointer rows initially failed: C died with `SIGSEGV`, Rust with
`SIGABRT`. Capturing the child's stderr showed
`null pointer dereference occurred` — `rustc`'s `debug_assertions` UB check,
which has no counterpart in the C build (CMake passes only `-fPIC`). This is an
instrumentation difference, not a logic difference. **The translation was not
changed**, because it was not at fault; instead the rows now compare
like-for-like builds and separately require the debug build's abort to be
*proven* to be that specific check. Full write-up in `ERRORS.md`.

No divergence in the translated logic was found. `translation/src/lib.rs` is
byte-for-byte faithful to `c_src/src/lib.c` across every input tested.

## Caveats, stated honestly

* **Rows 4–7 probe undefined behaviour.** A null dereference has no defined C
  semantics; the test pins the *observed* outcome on this platform
  (x86-64 Linux, GCC 11, rustc 1.94) and is not a portable guarantee.
* **Unaligned `float*` is not tested.** It is UB in both languages, and there
  is no defined behaviour to compare.
* **Exhaustive proof is infeasible.** The input domain is 2^96. Coverage is
  branch-directed plus ~230 000 randomised vectors per run, including 50 000
  uniform raw bit patterns and a 24^3 exhaustive special-value cross product;
  the mutation control is the evidence that this sampling is sensitive enough
  to catch realistic bugs.
* **The `-O2` C cross-check uses `cc` defaults**, deliberately *not*
  `-ffast-math`, which would legitimately change float results and is not what
  the graded reference build uses.
