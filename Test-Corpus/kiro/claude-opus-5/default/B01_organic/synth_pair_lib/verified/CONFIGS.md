# CONFIGS.md — Configuration / valid-input surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axis derivation (from the source, not from guesswork)

### Public entry points (`c_src/include/lib.h`, complete)

```c
typedef int16_t mp3d_sample_t;
void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);
```

`synth_pair` is the **only** public entry point — there is no convenience
wrapper and no lower-level public function. The lowest-level unit,
`static int16_t mp3d_scale_pcm(float)`, has internal linkage and is therefore
reachable only *through* `synth_pair`. Phase B drives it directly anyway, by
solving for the `z` tap that makes the accumulator take an exact target value
(see `solve_a1` / `solve_a2` in `tests/differential.rs`), so every branch of the
low-level helper is exercised through the real composed pipeline rather than
through a per-wrapper shortcut.

### Axis 1 — runtime options / modes

There are **no flags, modes, `#ifdef`s or `switch` statements** in the C source
(`grep -c 'switch\|#if' c_src/src/lib.c` → 0). The one and only runtime
parameter that changes control/data flow is `int nch`, which selects the store
offset of the second output sample:

```c
pcm[16 * nch] = mp3d_scale_pcm(a);
```

Distinguished `nch` values: `0` (both stores collide on `pcm[0]`), `1` (mono
stride), `2` (stereo stride — real minimp3 usage), other positive strides,
negative (store *below* `pcm`), and `int`-overflowing magnitudes.

### Axis 2 — input shapes the code special-cases

* **`z` is strided**: only indices `{0,64,…,896}` (block 1) and
  `{2,66,…,898}` (block 2) are read. Everything else in the buffer is
  irrelevant — a shape that must be verified, not assumed.
* **Block 1 reads 15 taps** (`0..=14`), block 2 reads only the **8 even taps**
  (`0,2,4,6,8,10,12,14`) after `z += 2`; the odd taps are never read in block 2.
* **Sign pairings differ per term**: block 1 mixes subtraction
  (`z[14]-z[0]`, `z[12]-z[2]`, `z[10]-z[4]`, `z[8]-z[6]`) with addition
  (`z[1]+z[13]`, `z[3]+z[11]`, `z[5]+z[9]`), and block 2 has two negative
  coefficients (`-9975`, `-45`, `-5`). Each pairing is a distinct code path for
  cancellation/rounding.
* **Accumulator magnitude** decides which `mp3d_scale_pcm` branch is taken:
  non-clipping, positive clip, negative clip, the `s < 0` decrement, exact
  half-way rounding, `±0.0`, subnormals, `NaN`, `±Inf`.
* **`float` value classes**: zero, subnormal, normal, huge-but-finite (so the
  products overflow to `Inf`), `NaN` (quiet, and with a payload), signed zeros.
* **Counts**: the API has no count parameter; "empty / one / many" maps onto
  *how many taps are non-zero* (zero, exactly one, all).

## The table

One row per meaningful combination the C actually treats differently. Every row
is driven with **many randomized inputs (fixed seed `0x5EED_1234_ABCD_0001`)**,
not a single hand-picked value, and both `.so`s are compared byte-for-byte on
the full `pcm` buffer.

| #   | entry point(s) | configuration (options set + input shape) | [ ] |
|-----|----------------|-------------------------------------------|-----|
| C1  | `synth_pair` | `nch=2` (stereo), all 900 `z` floats = `+0.0` — the "empty" shape | [x] |
| C2  | `synth_pair` | `nch=2`, all `z` = `-0.0` (signed-zero shape; checks `-0.0*coef` sign and the `s<0` test) | [x] |
| C3  | `synth_pair` | `nch=2`, exactly **one** non-zero tap, swept over **all 15 block-1 taps** × randomized values — isolates each block-1 coefficient and each `+`/`-` pairing | [x] |
| C4  | `synth_pair` | `nch=2`, exactly one non-zero tap, swept over **all 15 block-2 tap slots** (`z+2`) × randomized values — proves the 7 odd slots are ignored and isolates the 8 even coefficients incl. the negative ones | [x] |
| C5  | `synth_pair` | `nch=2`, all taps randomized small (`|v| <= 1e-2`) so **no clipping** — exercises the plain `(int16_t)(a+.5f)` path with `a` near 0 | [x] |
| C6  | `synth_pair` | `nch=2`, all taps randomized mid (`|v| <= 0.5`) — straddles the clip thresholds, mixes clipped and non-clipped across iterations | [x] |
| C7  | `synth_pair` | `nch=2`, all taps randomized large (`|v| <= 1e6`) — accumulator saturates in both directions, and intermediate products stay finite | [x] |
| C8  | `synth_pair` | `nch=2`, taps = huge finite floats (`~1e35`) so the products **overflow to `±Inf`** mid-accumulation | [x] |
| C9  | `synth_pair` | `nch=2`, taps chosen so block 1 and block 2 cancel to values within `1 ULP` of the clip thresholds `32766.5` / `-32767.5` (boundary shape) | [x] |
| C10 | `synth_pair` | `nch=2`, taps = randomized **subnormal** floats (`1e-45 … 1e-38`) — accumulator underflows toward `±0` | [x] |
| C11 | `synth_pair` | `nch=2`, taps randomized over the **full `f32` bit pattern space** (any exponent, incl. `NaN`/`Inf` payloads) — the widest value-class cross-product | [x] |
| C12 | `synth_pair` | `nch=2`, taps randomized **and** the 63 filler floats between every tap filled with adversarial garbage (`NaN`, `Inf`, huge) — proves the stride is respected identically | [x] |
| C13 | `synth_pair` | `nch=1` (mono stride), randomized taps across all magnitude classes | [x] |
| C14 | `synth_pair` | `nch=2` (stereo stride), randomized taps across all magnitude classes | [x] |
| C15 | `synth_pair` | `nch` randomized in `3..=64` (arbitrary positive stride), randomized taps | [x] |
| C16 | `synth_pair` | `nch=0` — both stores target `pcm[0]`; second write must win | [x] |
| C17 | `synth_pair` | `nch` negative (`-1`, `-2`, randomized `-64..=-1`), `pcm` pointing into the middle of a buffer so the backwards store is in bounds | [x] |
| C18 | `synth_pair` | `nch = INT_MAX` / `INT_MIN` / `0x0800_0000` — `16*nch` overflows `int`; the wrapped offset must match | [x] |
| C19 | `synth_pair` | `nch=2`, `pcm` buffer pre-filled with a sentinel pattern — verifies **only** `pcm[0]` and `pcm[32]` are modified and no neighbouring element is clobbered | [x] |
| C20 | `synth_pair` | `nch=2`, `pcm` **aliasing** the `z` buffer (`pcm` points into the same allocation) — the C does all reads before the second store only for block 1, so read/write ordering is observable | [x] |
| C21 | `synth_pair` | `nch=2`, block 1 taps only (block-2 slots zero) and vice-versa — proves the two accumulators are independent | [x] |
| C22 | `synth_pair` (low-level helper path) | `solve_a1`/`solve_a2`-driven exact accumulator targets sweeping every `mp3d_scale_pcm` branch: `>=32766.5`, `<=-32767.5`, exact thresholds, `±1 ULP` inside, `s<0` decrement, `-0.0`, `NaN`, `±Inf` | [x] |
| C23 | `synth_pair` | randomized **fuzz**: 200 000 iterations with `nch`, all 900 `z` floats and the `pcm` prefill all drawn randomly from mixed value classes | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the complete set of
feature combinations is `{ default }` ≡ `{ --no-default-features }`. Both are
run by `run_all.sh`.

## Extra rows added while writing the tests

| #    | entry point(s) | configuration | [ ] |
|------|----------------|---------------|-----|
| C2b  | `synth_pair` | hand-built sign pattern that keeps **every** block-1 term at `-0.0`, so the accumulator itself is `-0.0` (unreachable via a single tap, because the trailing `+ 0.0` terms normalise it) | [x] |
| C24  | `synth_pair` | harness self-validation: `z_for_accumulators(t, t)` really does realise accumulator `t` — checked against the **C** library for 15 384 targets, 100% solved (`harness_solver_hits_the_intended_accumulator`) | [x] |
| C25  | `synth_pair` | `nm -D` symbol parity asserted from inside the test suite (`harness_symbol_parity`) | [x] |
| C26  | `synth_pair` | opt-in wide sweep: 12 769 974 differential calls stepping the full `f32` bit space (stride 1009, coprime with 2^32) at taps 0, 7 and 14 (`tests/exhaustive.rs`) | [x] |

## How the low-level helper is reached exactly

`mp3d_scale_pcm` is `static`, so it can only be driven through `synth_pair`. If
exactly one `z` tap is non-zero, every other term of the accumulation
contributes `+0.0`, and `x + 0.0 == x` is exact, so the accumulator collapses to
`fl(v * coef)` for that tap's signed coefficient. Inverting that gives an exact
handle on the accumulator.

A *single* coefficient is not enough: one ULP of `v` moves `v * coef` by between
1 and 2 ULPs of the product, so one coefficient can only reach part of the
result grid — which is why the first version of the solver failed on
`32766.498046875` (row E7). Searching all 15 block-1 coefficients and all 8
block-2 coefficients (`BLOCK1_TAP_COEFS` / `BLOCK2_TAP_COEFS` in
`tests/common/mod.rs`) reaches **100%** of the targets tried.

## Results

Every row above passes under both the dev and release profiles, driven by
`./run_all.sh`. Divergences found and fixed are recorded in `ERRORS.md`.
`./mutation_check.sh` re-breaks the translation 18 different ways and confirms
this suite catches every non-equivalent one.
