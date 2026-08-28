# CONFIGS.md — Phase A: configuration-surface table

## How the axes were derived

The C library has **no** runtime options, flags, modes, handles or `#ifdef`s —
that was established mechanically in `ERRORS.md` (0 matches for `if`, `switch`,
`enum`, `#if`). So the configuration surface is not "options × options"; it is
**entry point × input shape × float value class**, and the axes below are read
straight off the source.

### Axis 1 — entry points (the FULL public API, lowest level included)

`nm -D --defined-only` on the C `.so` yields exactly one exported entry point,
and the header declares exactly one function, so the "convenience wrapper vs
low-level" distinction collapses: `to_barycentric` *is* the lowest level
reachable from outside. The three `static` helpers (`lm_v2`, `lm_sub2`,
`lm_dot2`) have internal linkage and are unreachable across the ABI, so they are
exercised *through* `to_barycentric` — and, crucially, exercised in a way that
isolates each of them (see axis 3), because a shape that only ever drives the
composed pipeline with well-behaved values cannot see a helper-level bug.

| entry point | reachable helpers driven |
|-------------|--------------------------|
| `to_barycentric` | `lm_sub2` ×3, `lm_dot2` ×5, `lm_v2` ×5, plus 6 `mulss`, 2 `subss`, 1 `divss` inline in the body |

### Axis 2 — geometric input shape (what the *dataflow* special-cases)

`c_src/src/lib.c` builds `v0 = p3-p1`, `v1 = p2-p1`, `v2 = p-p1`, and then
divides by the Gram determinant `dot00*dot11 - dot01*dot01`. The shapes the code
therefore treats differently are:

* non-degenerate triangle, `p` inside / on an edge / on a vertex / outside
* `v1 == 0` (`p2 == p1`), `v0 == 0` (`p3 == p1`), `v2 == 0` (`p == p1`)
* `v0 == v1` (`p2 == p3`), all four coincident
* exactly collinear (`v0 = t·v1`) and *nearly* collinear (determinant rounds)
* orthogonal `v0 ⟂ v1` (`dot01 == 0`, kills the cross terms)
* extreme aspect ratio (needle triangles → cancellation in the determinant)

### Axis 3 — float value class per component (the "element type / width" axis)

`float` is the only type, but its value classes are what the hardware branches
on. Each of the 8 input `float`s can be: `+0.0`, `-0.0`, subnormal, normal,
`FLT_MIN`, `FLT_MAX`, `±inf`, quiet NaN (any of 2^22−1 payloads, either sign),
signalling NaN (either sign). Value classes drive:

* overflow / underflow inside `lm_sub2` and `lm_dot2`
* the `1.0f/0.0` degeneracy on line 25 (no guard)
* **NaN payload selection**, which is the one place where "same arithmetic" is
  not enough for *byte* equality.

### Axis 4 — SSE destination operand (NaN payload tie-break)

Read off the `-O0` reference disassembly (`objdump -d`); this is the axis that
makes the translation non-obvious, so it is enumerated explicitly:

| site | instruction | destination operand ⇒ payload winner |
|------|-------------|--------------------------------------|
| `lm_sub2` x | `subss %xmm1,%xmm0` (`xmm0 = a.x`) | left (minuend) |
| `lm_sub2` y | `subss %xmm1,%xmm2` (`xmm2 = a.y`) | left (minuend) |
| `lm_dot2` x-term | `mulss %xmm0,%xmm1` (`xmm1 = a.x`) | **left** |
| `lm_dot2` y-term | `mulss %xmm2,%xmm0` (`xmm0 = b.y`) | **right** |
| `lm_dot2` sum | `addss %xmm1,%xmm0` (`xmm0 = y-term`) | **right addend** |
| `invDenom` | `mulss -0x10,%xmm1` / `mulss %xmm0,%xmm0` / `subss %xmm0,%xmm1` / `divss %xmm1,%xmm0` | left, left, left, left (`1.0f` is the dividend/dst) |
| `u`, `v` | `mulss`,`mulss`,`subss %xmm1,%xmm0`,`mulss %xmm1,%xmm0` | left every time |

x86 rule encoded: for `op dst, src`, if `dst` is NaN → `Q(dst)`; else if `src` is
NaN → `Q(src)`; else the plain op (so hardware-generated invalid results keep the
default `0xFFC0_0000`).

---

## Configuration table

One row per combination the C actually distinguishes. **Every row is driven with
many randomised inputs from a fixed-seed PRNG** (splitmix64, seed noted in
`tests/common/mod.rs`), not one hand-picked value; the iteration count per row is
in the `iters` column. Both `.so`s are called through identical
`unsafe extern "C" fn(Vec2,Vec2,Vec2,Vec2) -> Vec2` pointers loaded with
`libloading`, and the two returned `lm_vec2`s are compared as raw `u32` bit
pairs.

| # | entry point(s) | configuration (options set + input shape) | iters | [x] |
|---|----------------|-------------------------------------------|-------|-----|
| B1 | `to_barycentric` | canonical unit right triangle `(0,0),(1,0),(0,1)`; `p` = random small-integer/quarter-step coords (inside, on edge, on vertex, outside) | 20 000 | [x] |
| B2 | `to_barycentric` | random *dyadic* coords (mantissa = few bits, exponent 2^-8..2^8) in all 4 points ⇒ every intermediate exact, isolates ordering not rounding | 50 000 | [x] |
| B3 | `to_barycentric` | random full-mantissa **normal** floats, exponent range 2^-20..2^20, all 4 points independent | 200 000 | [x] |
| B4 | `to_barycentric` | `p` **inside** the triangle by construction (random barycentric weights `u+v<=1`), random non-degenerate triangle | 50 000 | [x] |
| B5 | `to_barycentric` | `p` exactly **on a vertex** (`p==p1`, `p==p2`, `p==p3`, cycled) ⇒ `v2 == 0` or `v2 == v0/v1` | 30 000 | [x] |
| B6 | `to_barycentric` | `p` exactly **on an edge** (random `t` on each of the 3 edges) | 30 000 | [x] |
| B7 | `to_barycentric` | **`p2 == p3`** ⇒ `v0 == v1` ⇒ `dot00 == dot01 == dot11` ⇒ determinant cancels to `0` | 20 000 | [x] |
| B8 | `to_barycentric` | **orthogonal** edges: `v1 = (a,0)`, `v0 = (0,b)` ⇒ `dot01 == ±0.0`, tests the `0`-cross-term path and the sign of zero | 20 000 | [x] |
| B9 | `to_barycentric` | **needle / extreme aspect ratio**: `v0` and `v1` nearly parallel with hugely different lengths ⇒ catastrophic cancellation in `dot00*dot11 - dot01*dot01` | 50 000 | [x] |
| B10 | `to_barycentric` | **large magnitudes** `1e18..FLT_MAX` (dot products overflow to `+inf`, differences overflow) | 50 000 | [x] |
| B11 | `to_barycentric` | **small magnitudes** `FLT_TRUE_MIN..1e-20` (subnormal inputs, squares flush to `+0.0`, gradual underflow) | 50 000 | [x] |
| B12 | `to_barycentric` | **mixed magnitude** — each of the 8 floats independently drawn from {tiny, small, one, large, huge} ⇒ mixed overflow/underflow within one call | 100 000 | [x] |
| B13 | `to_barycentric` | **signed zeros**: each of the 8 floats independently `+0.0` or `-0.0` (all 256 combinations, exhaustive) | 256 (exhaustive) | [x] |
| B14 | `to_barycentric` | **special-value cross product**: each of the 8 floats independently drawn from a 24-entry table of `±0`, subnormals, `±FLT_MIN`, `±1`, `±FLT_MAX`, `±inf`, `±QNaN`, `±SNaN` | 200 000 | [x] |
| B15 | `to_barycentric` | **fully random 32-bit patterns** in all 8 floats (≈2 % NaN, all classes, no structure at all) | 300 000 | [x] |
| B16 | `to_barycentric` | **NaN-heavy**: each float 50 % chance of a random-payload NaN (quiet, either sign), else random normal ⇒ exercises the multi-NaN payload race in every op | 300 000 | [x] |
| B17 | `to_barycentric` | **SNaN-heavy**: as B16 but the NaNs are *signalling* (quiet bit clear, random low payload, either sign) ⇒ exercises the SNaN→QNaN quieting at every op | 300 000 | [x] |
| B18 | `to_barycentric` | **one NaN, swept position**: exactly one of the 8 floats is a NaN (8 positions × {quiet,signalling} × {+,−} × random payload), other 7 random normals ⇒ isolates which SSE destination operand wins per site | 8 × 20 000 | [x] |
| B19 | `to_barycentric` | **inf-heavy**: each float 40 % chance of `±inf` ⇒ `inf-inf`, `0*inf`, `inf+(-inf)`, `inf/inf` all reachable | 200 000 | [x] |
| B20 | `to_barycentric` | **argument-slot permutation / ABI shuffle**: the same 4 points passed in all 24 permutations of the parameter order, so a mis-ordered `xmm0..xmm3` mapping cannot hide behind a symmetric input | 24 × 5 000 | [x] |
| B21 | `to_barycentric` | **repeat-call determinism / no hidden state**: each library called twice on the same input, interleaved C→Rust→C→Rust, results must be identical across calls (the C has no globals; this pins that the Rust does not either) | 20 000 | [x] |
| B22 | `to_barycentric` | **exact-integer lattice**: all 8 coords integers in `-64..64` ⇒ every dot product exact, so any divergence is pure ordering / division | 100 000 | [x] |
| B23 | `to_barycentric` | **power-of-two coords** `±2^k`, `k ∈ -60..60`, independently per float ⇒ maximal exponent spread inside one dot product (`x`-term overflows while `y`-term underflows) | 100 000 | [x] |
| B24 | `to_barycentric` | **degenerate-family sweep** (valid inputs that hit the unguarded divide): `p1==p2==p3`, `p2==p1`, `p3==p1`, exactly collinear with random `t`, all with random `p` | 4 × 20 000 | [x] |

Total differential comparisons across Phase B: **≈ 2.6 million**.

## Sensitivity check (why "all rows pass" means something)

A differential suite that never fails is indistinguishable from one that never
checks anything, so `mutation_check.sh` injects 20 deliberate divergences into
`src/lib.rs` one at a time and asserts the suite reacts as predicted:

* **15 mutants that change observable behaviour are all CAUGHT** — including the
  four subtle SSE destination-operand choices in `lm_dot2` and the two
  numerators, the SNaN quiet bit, sign preservation, `f64` intermediates, and
  FMA contraction.
* **5 mutants are provably UNOBSERVABLE through the ABI and correctly survive**;
  each carries a proof in `ERRORS.md` ("Which of these are actually
  observable"). If one of them were ever "caught", the proof would be wrong.

Rows B16, B17 and E15 (multi-NaN) are the ones that catch the payload mutants;
B18 (a *single* NaN) does not, because with one NaN there is no payload race —
which is precisely why enumerating "one NaN per position" would not have been
enough on its own.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so `{}`,
`--no-default-features` and `--all-features` are the same build. All three are
run explicitly by `check_all_features.sh`, and every row above is re-verified
under each.
