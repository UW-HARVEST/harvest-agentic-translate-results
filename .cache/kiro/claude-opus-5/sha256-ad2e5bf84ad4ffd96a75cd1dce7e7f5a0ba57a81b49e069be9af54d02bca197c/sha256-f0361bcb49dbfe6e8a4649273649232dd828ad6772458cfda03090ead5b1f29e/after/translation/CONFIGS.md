# CONFIGS.md — Phase B configuration-surface table

## Axes, derived from the C source

`c_src/CMakeLists.txt` defines exactly one target and no options; there is no
`#ifdef`, no `#if`, no compile-time or runtime flag, no mode/option struct, and
no global state in `c_src/src/lib.c`. `translation/Cargo.toml` declares **no
`[features]` table**, so the only feature combination that exists is the default
(empty) one. There is also no `[[bin]]`/`add_executable`, so there is no driver
binary whose stdout could be diffed.

The axes the C code actually branches on are therefore:

* **A1 — arm selection**: `if (src[0] < src[1])` — then arm (`dx2=src[0]`,
  `dy2=src[1]`, stores `dx2-lambda, dxy`) vs else arm (`dy2=src[0]`,
  `dx2=src[1]`, stores `dxy, dx2-lambda`). Includes the unordered (NaN) and
  equal cases, which both fall to the else arm.
* **A2 — discriminant clamp**: `((0) > (sqd)) ? (0) : (sqd)` — clamped
  (`sqd < 0`), passed through (`sqd >= 0`), unordered (`sqd` NaN).
* **A3 — element count**: `0`, `1`, `2`, many, and negative (loop never runs).
  Also exercises pointer advance `src += 3` / `dest += 2` (different strides,
  the classic off-by-one site).
* **A4 — value class of each of the 3 input lanes** (`src[0]`, `src[1]`,
  `src[2]`): normal, `±0.0`, subnormal, `±FLT_MAX`/`±FLT_MIN`, `±inf`, quiet
  NaN, signalling NaN. These select which SSE operand wins a NaN and whether the
  discriminant overflows/underflows.
* **A5 — buffer relationship**: disjoint, fully aliased (`dest == src`),
  partially overlapping. C has no `restrict` and no aliasing check.

`tfm` is simultaneously the lowest-level and the only public entry point, so
"exercise the low-level entry points, not just the convenience wrapper" is
satisfied by calling `tfm` directly through `dlsym` — there is no wrapper layer.

## Rows (pruned cross-product of A1–A5)

Every row is driven with **many randomized inputs** from a fixed-seed PRNG
(`SEED = 0x5EED_1234_ABCD_F00D`), not one hand-picked value, and the two `.so`s
are compared **bit-for-bit** (`to_bits()`) on the whole output buffer plus the
canary bytes past its end.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `tfm` | `count = 0`, disjoint buffers, random garbage in `dest` — nothing written | `cfg_row01_count_zero` | [x] |
| 2 | `tfm` | `count = 1`, disjoint, A1=then arm forced (`src[0] < src[1]`), normal finite randoms, A2=pass-through | `cfg_row02_count1_then_arm` | [x] |
| 3 | `tfm` | `count = 1`, disjoint, A1=else arm forced (`src[0] >= src[1]`), normal finite randoms | `cfg_row03_count1_else_arm` | [x] |
| 4 | `tfm` | `count = 1`, A1=else via `src[0] == src[1]` exactly (incl. `+0.0`/`-0.0` pair) | `cfg_row04_count1_equal_arm` | [x] |
| 5 | `tfm` | `count = 2`, disjoint, arms mixed within one call (element 0 then arm, element 1 else arm) — validates the `+=3`/`+=2` strides | `cfg_row05_count2_mixed_arms` | [x] |
| 6 | `tfm` | `count` random in `3..64`, disjoint, arm chosen randomly per element, fully random `f32` bit patterns (all value classes of A4 reachable) | `cfg_row06_many_random_bitpatterns` | [x] |
| 7 | `tfm` | `count` random in `3..64`, disjoint, all lanes drawn from a curated special-value pool (`±0`, subnormals, `±FLT_MIN/MAX`, `±inf`, qNaN, sNaN, `±1`, `±2`, `±0.5`) | `cfg_row07_many_special_values` | [x] |
| 8 | `tfm` | A2 = clamped: inputs constructed so `sqd < 0` (small `dxy`, `dx2 ≈ dy2` ⇒ discriminant negative by rounding), both arms | `cfg_row08_sqd_negative_clamped` | [x] |
| 9 | `tfm` | A2 = pass-through with `sqd` very large (near-overflow `dxy`), both arms | `cfg_row09_sqd_huge` | [x] |
| 10 | `tfm` | A2 = unordered: `sqd` forced NaN via `inf`/`inf` cancellation, both arms | `cfg_row10_sqd_nan` | [x] |
| 11 | `tfm` | A4 = one NaN lane at a time (lane 0, 1, or 2), random NaN payloads and signs, both arms — pins down which SSE operand's payload survives | `cfg_row11_single_nan_lane` | [x] |
| 12 | `tfm` | A4 = subnormal-only lanes (discriminant underflows to `±0`/subnormal), both arms | `cfg_row12_subnormals` | [x] |
| 13 | `tfm` | A4 = `±inf` lanes in every position (produces `inf-inf`, `0*inf` invalid ops), both arms | `cfg_row13_infinities` | [x] |
| 14 | `tfm` | A5 = fully aliased `dest == src`, `count` random in `1..32`, random finite data — in-place clobbering with mismatched strides | `cfg_row14_aliased_in_place` | [x] |
| 15 | `tfm` | A5 = partially overlapping (`dest = src + 1`, and `dest = src - 1`), `count` random in `1..32` | `cfg_row15_partial_overlap` | [x] |
| 16 | `tfm` | A3 = large `count` (1024–4096), disjoint, random bit patterns — long-run drift / off-by-one | `cfg_row16_large_count` | [x] |
| 17 | `tfm` | mixed sweep: `count` random `0..48`, every lane independently chosen from {random bits, special pool, near-duplicate of the other lane}, disjoint — the pruned cross-product of A1×A2×A4 in bulk, 20 000 calls | `cfg_row17_bulk_cross_product` | [x] |
| 18 | `tfm` | `count = 1`, `src[0]`/`src[1]` differing by 1 ULP in both directions (the exact boundary of A1), random magnitudes incl. subnormal and huge | `cfg_row18_one_ulp_branch_boundary` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` section, so the complete set of
feature combinations is `{default}` = `{}`. `scripts/all_features.sh` enumerates
them from `Cargo.toml` and runs `cargo test` for each; it finds exactly one
combination and additionally re-runs with `--no-default-features` to prove the
default set is empty.

## Harness validation (mutation testing)

A green suite is only meaningful if the harness can actually see a divergence,
so the differential harness was validated by injecting deliberate bugs into
`src/lib.rs` (each reverted afterwards; `diff -q` confirmed `lib.rs` was restored
byte-identically every time) and confirming the suite goes red:

| mutant | injected bug | detected by |
|--------|--------------|-------------|
| A | reassociate `2.0f*dx2*dy2` as `dx2*(2.0f*dy2)` | 11 rows |
| B | reverse the final `addss` operand roles (NaN-payload-visible only) | 13 rows |
| C | clamp a NaN discriminant to `0.0f` instead of letting it through | 25 rows |
| D | arm predicate `<` → `<=` | 15 rows |
| E | loop bound `i < count` → `i <= count` | 19 rows + SIGSEGV in the NULL row |
| F | `count` compared as `u32` instead of `i32` | SIGSEGV in the negative-count / NULL rows |
| G | `src` stride 3 → 2 | 10 rows |
| H | `dest[1] = dxy` routed through a subtraction (loses `-0.0`) | 12 rows |
| I | `sqrtf` without explicit NaN quieting | 0 — **semantically equivalent**, `SQRTSS` quiets in hardware anyway |
| K | `sqrtf(+inf)` → `FLT_MAX` | 7 rows |
| L | NaN predicate takes the *then* arm instead of the *else* arm | 13 rows |

**31 of 32 rows** detect at least one mutant in a whole-suite run; the 32nd
(`cfg_row01_count_zero`) detects mutant E when run in isolation — in the
whole-suite run its report was masked by the concurrent `phase_c_errors`
SIGSEGV. Verified individually:

```
$ cargo test --release --test phase_b_configs cfg_row01_count_zero -- --exact   # with mutant E
test cfg_row01_count_zero ... FAILED
row01 count=0 [iter 0]: output mismatch at dest slot 0 (element 0, lane 0)
```

So every row is non-vacuous. Mutant I is the one non-detection and it is
correct: it is not a behavioural change.

Two further negative controls confirm the harness cannot pass by accident:

* Deleting the Rust cdylib for the profile under test makes every test panic
  with "refusing to fall back to another profile's .so" rather than silently
  loading the other profile's artifact (a real false-green this harness hit and
  then fixed).
* Deleting the C `.so` makes every test panic with "no .so found under
  c_src/build".
