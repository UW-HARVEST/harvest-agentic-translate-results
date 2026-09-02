# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h` and the
`-O0` disassembly. This is the mirror of `ERRORS.md`: the **valid**-input
surface.

## Axes the C actually branches on

There are no runtime option structs, no init/config functions, no global flags,
no `#ifdef`s and no enums — the entire public header is one line:

```c
void tfm(float *dest, const float *src, int count);
```

So the axes are (a) the two real data-dependent branches in the code and (b) the
input shapes the arithmetic distinguishes.

| axis | values | where the C branches / distinguishes |
|---|---|---|
| **A. arm select** | `src[0] < src[1]` → *if*-arm; `>=` or *unordered* → *else*-arm | `lib.c:8`, `comiss %xmm1,%xmm0` + `jbe` at `113c`. Selects both the `dx2`/`dy2` naming **and** which of `dest[0]`/`dest[1]` gets `dxy` |
| **B. discriminant clamp** | `sqd > 0`; `sqd == +0.0`; `sqd < 0` (clamped to `+0.0f`); `sqd == -0.0` (**not** clamped); `sqd == NaN` (**not** clamped); `sqd == +inf` | `lib.c:15,25` `((0)>(sqd))?(0):(sqd)`, `pxor`+`comiss`+`jbe` at `11cd`/`12b5` |
| **C. `count`** | `0`, `1`, `2`, `3`, many (`64`, `1000`), negative | `for (i = 0; i < count; i++)`, signed `jl` at `1324` |
| **D. element value class** | normal, small/large magnitude, subnormal, `+0.0`, `-0.0`, `+inf`, `-inf`, qNaN (both signs, non-trivial payload), sNaN | every `mulss`/`addss`/`subss`/`sqrtf`; NaN operand *role* (dest vs src) is observable, see `src/lib.rs` header |
| **E. `dxy` (`src[2]`)** | `0.0` (⇒ the `4*dxy*dxy` term vanishes), non-zero, `inf` (⇒ `4*inf*inf`), NaN, huge (⇒ `4*dxy*dxy` overflows) | `mulss(4.0f, dxy)` then `mulss(.., dxy)` then `addss(4dxy², acc)` — the final `addss` has `4dxy²` as **dest**, so a NaN `dxy` wins over a NaN accumulator |
| **F. invalid-op producers** | `inf - inf`, `0 * inf`, `inf + -inf` reachable via `src` combos | no guards; hardware yields the x86 "indefinite" qNaN `0xffc00000` |
| **G. buffer relationship** | disjoint; `dest == src` (in-place); `dest` inside `src` at a positive offset; unaligned by 1 byte | no `restrict`, no aliasing/alignment check; `src += 3` vs `dest += 2` per iteration |
| **H. entry points** | `tfm` — the *only* exported symbol; it is simultaneously the lowest-level and highest-level entry point | `nm -D` (see `SYMBOLS.md`) |

`tfm` is the lowest-level entry point available; there is no convenience wrapper
to prefer over it, so every row below drives `tfm` directly through the `.so`
export.

## Table

Every row is exercised with **many randomized inputs** (fixed seed, a small
xorshift PRNG in `tests/common/mod.rs`) plus the row's hand-pinned edge values,
and compared **bit-for-bit** (`to_bits()`) between the C `.so` and the Rust
`.so`. Rows are grouped by the axis combination they pin.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tfm` | A=*if* forced (`src[0] < src[1]`), B=`sqd>0`, C=1, D=normal random, E=non-zero, G=disjoint | [x] |
| 2 | `tfm` | A=*else* forced (`src[0] > src[1]`), B=`sqd>0`, C=1, D=normal random, E=non-zero, G=disjoint | [x] |
| 3 | `tfm` | A=*else* via `src[0] == src[1]` (exact tie, incl. `+0.0`/`-0.0` tie), C=1, D=normal, G=disjoint | [x] |
| 4 | `tfm` | A=random (both arms hit), B=random, C=1, D=**fully random bit patterns** (any of the 2^32 floats per lane), G=disjoint | [x] |
| 5 | `tfm` | A=random, B forced `sqd < 0` (clamp path taken), C=1, D=normal, G=disjoint | [x] |
| 6 | `tfm` | A=random, B forced `sqd == +0.0` exactly (`dxy=0`, `dx2==dy2`), C=1, G=disjoint | [x] |
| 7 | `tfm` | A=random, B forced `sqd == -0.0`(no clamp, `sqrtf(-0.0) = -0.0`), C=1, G=disjoint — **note: `sqd == -0.0` is provably unreachable (see `ERRORS.md`); the row tests the observable signed-zero path instead** | [x] |
| 8 | `tfm` | A=random, B=`sqd == NaN` (clamp skipped), C=1, D includes NaN, G=disjoint | [x] |
| 9 | `tfm` | A=random, B=`sqd == +inf` (overflow), C=1, D=huge finite (`~1e38`), G=disjoint | [x] |
| 10 | `tfm` | A=random, C=**0** (empty), G=disjoint, dest pre-poisoned to detect any write | [x] |
| 11 | `tfm` | A=random, C=**2** and **3** (small multi-element, both arms in one call) | [x] |
| 12 | `tfm` | A=random, C=**64** and **1000** (many), D=normal random, G=disjoint | [x] |
| 13 | `tfm` | A=random, C=many, D=**mixed classes per element** (each element independently picks from the D value set) | [x] |
| 14 | `tfm` | A=random, C=many, D=**`±0.0` only** (all-zero and signed-zero inputs) | [x] |
| 15 | `tfm` | A=random, C=many, D=**subnormal only** (`1e-45`..`1e-38`, incl. gradual-underflow products) | [x] |
| 16 | `tfm` | A=random, C=many, D=**±inf only** (⇒ `inf-inf`, `0*inf` invalid ops, axis F) | [x] |
| 17 | `tfm` | A=random, C=many, D=**qNaN with random payloads and random sign bit** in a random lane (NaN-role / payload propagation) | [x] |
| 18 | `tfm` | A=random, C=many, D=**sNaN** (`0x7fa00000` / `0xffa00000`) in a random lane (quieting) | [x] |
| 19 | `tfm` | A=random, C=many, E=**`dxy` ∈ {`+0.0`,`-0.0`}** (`4*dxy*dxy` term vanishes / signed-zero) | [x] |
| 20 | `tfm` | A=random, C=many, E=**`dxy` huge** so `4*dxy*dxy` overflows to `+inf` while `acc` is finite | [x] |
| 21 | `tfm` | A=random, C=many, E=**`dxy = ±inf`** while `dx2`/`dy2` are finite (⇒ `acc + inf`) | [x] |
| 22 | `tfm` | A=random, C=many, B=`sqd` spanning the full `sqrtf` domain incl. subnormal and `~FLT_MAX` radicands (glibc `sqrtf` vs inline `sqrtss` parity) | [x] |
| 23 | `tfm` | G=**in-place, `dest == src`** (aliasing), C=many, D=normal random | [x] |
| 24 | `tfm` | G=**`dest = src + k` floats** for k ∈ {1,2,3,4} (partial forward overlap), C=many | [x] |
| 25 | `tfm` | G=**both buffers unaligned** (byte-offset 1 inside a larger allocation), C=many, D=normal | [x] |
| 26 | `tfm` | A=random, C=many, D=normal, **repeated back-to-back calls on the same buffers** (no hidden state between calls) | [x] |
| 27 | `tfm` | A=random, C=many, D=**pairs that force `dx2 == dy2`** (⇒ `(dy2-dx2)² == 0`, `sqd == 4dxy²`, `lambda == dx2 + |2dxy|`) | [x] |
| 28 | `tfm` | A=random, C=many, D=**catastrophic-cancellation pairs** (`dx2` and `dy2` within 1 ULP, and within 1e-7 relative) — value-dependent rounding of `sqd` | [x] |

All 28 rows pass. See `tests/valid_paths.rs`.

## Beyond the table

`tests/exhaustive.rs` adds two sweeps that are not per-row but cover the axes
above jointly:

* `nan_full_cross_product` — the **full 36³ = 46 656 cross product** over a
  special-value set (12 distinct qNaN payloads × both signs, 8 sNaNs, ±inf,
  ±0.0, smallest/largest subnormal, smallest normal, `FLT_MAX`, and the C's own
  `4.0f`/`0.5f` literals) in all three lanes. This is what makes axis D's
  operand-role coverage complete rather than sampled.
* `nan_cross_product_batched` — the same cross product as one 46 656-element
  call, and again in-place, so pointer advance and read/write overlap interact
  with every special value.

## Suite sensitivity (negative control)

Passing tests only mean something if they can fail. `scripts/mutation_check.py`
applies 20 single-edit mutations to `src/lib.rs`, rebuilds the `.so`, and re-runs
the suite:

* **14 mutations are caught** — arm-select comparison, loop bound (including
  signed→unsigned, which is exactly the `count < 0` bug), off-by-one, `src`/
  `dest` strides, store order, `subss` operand order, NaN-clamp behaviour, and
  three `sqd`-accumulation operand-role swaps.
* **6 are provably equivalent mutants**, each with its proof recorded inline in
  the script (NaN payloads masked downstream, or a never-NaN constant occupying
  the `dest` role). They are expected to still pass.

Run it with `./scripts/mutation_check.py`; it restores `src/lib.rs` on exit.

## Feature combinations

`Cargo.toml` has no `[features]` table, so the default (empty) feature set is the
only configuration; `--no-default-features` and `--all-features` resolve to it.
`scripts/verify_all.sh` runs the full suite under all three invocations.
