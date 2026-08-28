# EQUIVALENT_MUTANTS.md

`./mutation_check.sh` injects deliberate bugs into `src/lib.rs` and requires the
differential suite to fail. Of the 45 mutants, **40 are killed** by the fast
suite (`configs` + `errors` + `symbols`, ~5 s) and **5 survive everything,
including the exhaustive 2^32 sweeps**.

A survivor is only acceptable if it is *provably observationally equivalent* to
the C — otherwise it is a hole in the tests. Each survivor below is discharged by
a test in `tests/equivalence.rs` that enumerates **all 2^32 `f32` bit patterns**
(`sample` is the only input to `mp3d_scale_pcm`, so 2^32 is genuinely
exhaustive for that function).

| # | mutant | why it cannot change behaviour | proof test |
|---|--------|--------------------------------|-----------|
| 1 | high guard `>=` → `>` | The guard only differs at `a == 32766.5` exactly. There the conversion path computes `(int16_t)(32766.5f + 0.5f) == (int16_t)32767.0f == 32767`, and `32767` is not negative so the `s -= (s < 0)` correction does not fire — the same `32767` the guard returns. | `survivor_hi_guard_ge_vs_gt_is_equivalent` |
| 2 | low guard `<=` → `<` | Differs only at `a == -32767.5` exactly. There the conversion path computes `(int16_t)(-32767.5f + 0.5f) == (int16_t)(-32767.0f) == -32767`, which *is* negative, so `s -= 1` gives `-32768` — the same value the guard returns. | `survivor_lo_guard_le_vs_lt_is_equivalent` |
| 3 | high threshold `32766.5` → `32767.5` | For every `f32` in `[32766.5, 32767.5)` the sum `a + 0.5f` lies in `[32767.0, 32768.0)` and truncates to `32767` — again the clamp value. (The largest `f32` below `32767.5` is `32767.498046875`; `+ 0.5f` is exactly `32767.998046875`, still `< 32768`, so no rounding can push it to `32768.0`.) Above `32767.5` the mutated guard fires. | `survivor_hi_threshold_32766_5_vs_32767_5_is_equivalent` |
| 4 | narrow `f32 -> i16` directly instead of via `i32` | The two guards bound `a + 0.5f` inside `(-32767.0, 32768.0)`, so the value always fits `i16`; Rust's saturating `as i16` and the C's `cvttss2si` + 16-bit store therefore agree, and both map NaN to `0`. | `survivor_direct_f32_to_i16_narrowing_is_equivalent` |
| 5 | `a += t` → `a = t + a` | IEEE-754 addition is commutative for all operands; the only implementation-chosen part is *which* NaN payload propagates when both operands are NaN. `mp3d_scale_pcm` maps **every** one of the `2 * (2^23 - 1) = 16 777 214` `f32` NaN encodings to `0`, so the choice is unobservable. Note the C assembly literally computes `product + a` (`mulss` into `%xmm0`, then `addss %xmm1,%xmm0`), i.e. this mutant matches GCC's own order. The same argument covers `z * w` vs `w * x` for the multiplications, which is also checked. | `survivor_accumulation_order_is_equivalent` |

## The mutants that ARE killed (selected, to show sensitivity)

All 16 weight constants, both clamp return values, the low threshold constant,
the `s < 0` predicate, the `+ .5f` rounding offset, NaN handling, every tap
index, the `z += 2` pointer bump, the `16 * nch` stride and its `int`
wrap-around, and — importantly for a floating-point port — **all four
precision-changing mutants**:

* `term 4 accumulated in f64 (excess precision)` → KILLED
* `term 8 fused into an FMA (-ffp-contract=fast)` → KILLED
* `lane1 term 4 fused into an FMA` → KILLED
* `lane0 term 2 computed in f64 throughout` → KILLED

That last group is the important evidence that the suite pins down the exact
single-precision, non-contracted evaluation order that
`c_src/src/lib.c` compiles to, and not merely "approximately the right answer".

## Reproducing

```sh
./mutation_check.sh          # ~10 min; must report "survived everything : 5"
cargo test --release --test equivalence   # discharges all 5 survivors
```
