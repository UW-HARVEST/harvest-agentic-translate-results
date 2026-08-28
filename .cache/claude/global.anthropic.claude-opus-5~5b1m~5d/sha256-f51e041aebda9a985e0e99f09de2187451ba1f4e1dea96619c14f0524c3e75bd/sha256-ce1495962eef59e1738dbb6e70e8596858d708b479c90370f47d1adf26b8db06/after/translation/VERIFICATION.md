# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C, the
ground truth). Both are loaded as **shared objects** via `libloading` and driven
only through their exported `gaussian_kernel` C-ABI symbol, so the
`#[no_mangle] extern "C"` wrapper is exercised exactly as an external consumer
would exercise it. The Rust functions are never called directly.

## How to reproduce everything

```sh
# 1. C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. everything else (all feature combos x debug/release, symbol diff + suite)
cd translation && ./run_all_tests.sh
```

## Artefacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md` | A / D | every `nm -D` symbol, C vs Rust, diff = empty |
| `ERRORS.md` | A / C | 18 mechanically-derived rejection rows + 6 generic FFI boundary rows |
| `CONFIGS.md` | A / B | 29 configuration rows (size shapes x radius classes x fill x pointer offset) |
| `tests/common/mod.rs` | — | harness: `dlopen` both `.so`s, SplitMix64 PRNG, buffer+guard comparison |
| `tests/phase_b_configs.rs` | B | 29 valid-path tests (one per `CONFIGS.md` row) + audit counter |
| `tests/phase_c_errors.rs` | C | 21 error-path tests (one per `ERRORS.md` row / boundary group) + audit counter |
| `tests/phase_d_symbols.rs` | D | 3 symbol-parity tests driven by `nm -D` |
| `run_all_tests.sh` | D | feature-combination x profile driver |

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      `nm -D --defined-only` is exactly `T gaussian_kernel` on *both* libraries.
      No C source was left untranslated: `CMakeLists.txt` compiles a single
      translation unit (`src/lib.c`, 28 lines) containing a single function, and
      `grep` confirms there are no other functions, `static` helpers, macros or
      types anywhere in `c_src/`. Nothing is stubbed or `unimplemented!()`.
- [x] **Phase B: EVERY row in `CONFIGS.md` passes across randomized inputs.**
      29/29 rows checked off; 36 590 differential comparisons, including a
      20 000-draw and a 2 000-draw property-style fuzz over arbitrary `radius`
      bit patterns with a fixed seed (`0x5EEDC0FFEE000001`).
- [x] **Phase C: EVERY row in `ERRORS.md` has a passing error-path differential
      test.** 18/18 table rows + 6/6 generic boundary rows; 36 817 differential
      comparisons, including a 30 000-draw fuzz crossing arbitrary `radius` bit
      patterns with the degenerate `size` values (`INT_MIN`, `-3`, `-2`, `-1`,
      `0`, `1`, `2`, `3`).
- [x] **All of the above hold under EVERY feature combination.** The crate
      declares no `[features]`, so the complete set of configurations is
      `{default, --no-default-features}`; `run_all_tests.sh` runs each of them in
      both `debug` and `release` (release also flips `panic = "abort"` and full
      optimisation, i.e. genuinely different codegen). 4/4 configurations:
      symbol diff empty, 55/55 tests green (30 Phase B + 22 Phase C + 3 Phase D).

```
=== combo=default    profile=debug   === symbol diff: EMPTY   30 + 22 + 3 tests ok
=== combo=default    profile=release === symbol diff: EMPTY   30 + 22 + 3 tests ok
=== combo=no-default profile=debug   === symbol diff: EMPTY   30 + 22 + 3 tests ok
=== combo=no-default profile=release === symbol diff: EMPTY   30 + 22 + 3 tests ok
ALL feature combinations x profiles: symbol diff empty, all tests passed.
```

## What "identical" means here

Every comparison allocates one scratch buffer per implementation, pre-filled
with an identical, reproducible pattern, and compares **the entire buffer as raw
`u32` bit patterns** afterwards — the slots *before* `dest`, the `size` slots the
caller nominally owns, the one slot the C overruns for even `size`, and 16 `f32`
of trailing guard. Consequences:

* bit-exact float equality (no epsilons), so a 1-ULP divergence fails;
* a divergent *number of stores* fails, even when every stored value matches;
* NaN payloads and the sign of zero are compared, not normalised away.

## Negative controls (mutation testing)

To prove the suite is actually load-bearing rather than vacuously green, 26
deliberate bugs were injected into `translation/src/lib.rs` one at a time,
rebuilt, and the suite re-run. **Every mutant that is not provably equivalent to
the original was killed.**

| mutant | change | outcome |
|--------|--------|---------|
| `hsize_div_euclid` | `size / 2` → `size.div_euclid(2)` | **killed** (B 6 / C 3 failures) |
| `hsize_shift` | `size / 2` → `size >> 1` | **killed** (B 6 / C 3) |
| `size_u32_div` | `size / 2` → `((size as u32) / 2)` | **killed** (harness crashed: 2^31 stores) |
| `kernel_loop_lt_hsize` | `r <= hsize` → `r < hsize` | **killed** (B 26 / C 17) |
| `norm_loop_le_size` | `r < size` → `r <= size` | **killed** (B 21 / C 13) |
| `norm_from_one` | normalise from `r = 1` | **killed** (B 18 / C 10) |
| `skip_center_store` | skip `*k = v` when `r == 0` | **killed** (B 26 / C 17) |
| `store_after_incr` | store after `k` is advanced | **killed** (B 26 / C 17) |
| `clamp_to_neg_zero` | clamp to `-0.0f` instead of `+0.0f` | **killed** (B 22 / C 11) |
| `sum_ge_zero` | `sum > 0.0` → `sum >= 0.0` | **killed** (B 9 / C 7) |
| `divide_instead_of_mul` | `*p *= isum` → `*p /= sum` | **killed** (B 18 / C 8) |
| `sigma_typo` | `1.6f32` → `1.6000001f32` | **killed** (B 15 / C 5) |
| `tetha_real_change` | `2.25f32` → `2.2500005f32` | **killed** (B 15 / C 5) |
| `expf_neg_instead_recip` | `1/expf(x*x)` → `expf(-(x*x))` | **killed** (B 15 / C 6) |
| `s2_recip_via_neg` | `1/expf(arg)` → `expf(-arg)` | **killed** (B 14 / C 2) |
| `rs_reciprocal` | `sigma / radius` → `sigma * (1/radius)` | **killed** (B 13 / C 4) |

Surviving mutants, each **proven** semantically identical (so surviving is
correct, not a coverage hole):

| mutant | why it cannot be distinguished |
|--------|--------------------------------|
| `clamp_v_ge_zero` (`v > 0` → `v >= 0`) | differs only at `v == 0`; `v = a - b` can only be `+0.0` (never `-0.0`) in round-to-nearest, and both branches then yield `+0.0` |
| `clamp_via_f32_max` (`v.max(0.0)`) | `f32::max` returns the non-NaN operand, so NaN → `+0.0`, same as the `>` test; `v` is never `-0.0` |
| `std_exp_not_libm` (`(x*x).exp()`) | Rust's `f32::exp` *is* a call to libm `expf`, the same symbol the C imports |
| `f64_intermediate_x` | `r as f64 * rs as f64` is exact (≤48-bit product), so the single `f64→f32` rounding equals one correctly-rounded `f32` multiply — no double rounding |
| `f64_sum` | same argument for addition: the `f64` sum of two `f32`s is exact, so rounding it to `f32` equals `f32` addition |
| `sum_reverse_order` (`sum = v + sum`) | IEEE-754 addition is commutative (including for NaN payloads, which cannot occur here) |
| `sum_ne_zero` (`sum > 0` → `sum != 0`) | every term is clamped to `>= 0`, so `sum >= 0` always and never NaN ⇒ the predicates coincide |
| `arg_assoc` (`sigma*(sigma*tetha)`) | verified bit-identical: both groupings give `0x40b851ec` |
| `abs_x_before_square` | `|x| * |x| == x * x` exactly for every `f32`, NaN included |
| `early_return_on_null` | only alters `dest == NULL`, and the only null case the C defines is `size <= -2`, where it also does nothing |

## Notable C behaviours deliberately preserved (not "fixed")

1. **Off-by-one out-of-bounds store.** `hsize = size / 2` and the inclusive loop
   `[-hsize, hsize]` write `2*hsize + 1` elements, i.e. `size + 1` for an even
   `size` — one past the caller's buffer. Reproduced, and asserted by
   `e07_even_size_writes_one_element_past` / `c03_…`.
2. **The last store is never normalised** for even `size`, because the second
   loop only covers `r < size`. Reproduced (`dest[size]` keeps the raw tap).
3. **`size == 0` and `size == -1` both store one unnormalised element**, because
   C integer division truncates toward zero (`-1 / 2 == 0`) while the
   normalisation loop `r < size` runs zero times.
4. **`size <= -2` stores nothing at all** and leaves `dest` pristine — which is
   also the only case where a `NULL dest` is safe.
5. **`radius == ±0`, `NaN`, or a subnormal that overflows the division** make
   every tap clamp to `+0.0f`, leaving `sum == 0` so normalisation is skipped
   and the result is an all-zero kernel. A `NaN` is never stored, because
   `NaN > 0` is false and the ternary yields `+0.0f`.
6. **`radius == ±inf`** makes `rs == ±0`, so every tap is `V0` and the kernel is
   flat.
7. **`expf` is imported from the platform libm in both builds** (GCC does not
   constant-fold it at the default CMake build type — the disassembly shows a
   real `call expf@plt`), which is what makes bit-exact equality attainable.
