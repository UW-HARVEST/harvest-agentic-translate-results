# CONFIGS.md — Phase B: configuration-surface table

## Mechanical derivation of the axes

The whole public surface is `c_src/include/lib.h`:

```c
typedef struct lm_vec2 { float x, y; } lm_vec2;
lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
```

**Runtime option / mode / flag axes: none.** Grepping the public header and the
implementation finds no flag, no mode, no context/handle struct, no setter, no
global, no `if`/`switch`/`?:`, and no `#ifdef` (other than nothing at all):

```
$ grep -cE '#if|#ifdef|#ifndef|switch|if *\(|\?' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:0
c_src/include/lib.h:0
```

So the configuration surface is entirely the **input-shape** surface, and it is
the cross-product of:

- **Axis A — argument position** (4): which of `p1`, `p2`, `p3`, `p` carries the
  interesting value. These are *not* symmetric: `p1` is the shared origin
  (subtracted from all three), `p3` builds `v0`, `p2` builds `v1`, `p` builds
  `v2`, and only `v0`/`v1` feed the denominator. A bug that only affects `p1`
  is invisible if you only vary `p`.
- **Axis B — field** (2): `.x` vs `.y`. The two fields take *different*
  instruction forms inside `lm_dot2` (`mulss` with the destination = the LEFT
  operand for the `x` term, destination = the RIGHT operand for the `y` term),
  so `.x` and `.y` are genuinely different code paths for NaN payloads.
- **Axis C — float value class** (9): `+normal`, `-normal`, `+0`, `-0`,
  `+subnormal`, `-subnormal`, `±inf`, quiet NaN (arbitrary payload), signaling
  NaN. Every one of the 2^32 `f32` bit patterns is a legal input.
- **Axis D — geometric shape of the triangle** (6): non-degenerate CCW,
  non-degenerate CW, right-angle / axis-aligned, collinear, coincident
  vertices, and needle-thin (near-degenerate but finite).
- **Axis E — magnitude regime** (4): unit-scale, tiny (denominator underflows),
  huge (denominator overflows), mixed scales (catastrophic cancellation in
  `dot00*dot11 - dot01*dot01`).
- **Axis F — query-point location** (4): strictly inside the triangle, on an
  edge/at a vertex, outside, and far outside.

The public entry-point set is a single function, and it *is* the lowest-level
entry point — there is no convenience wrapper above it and no internal API
below it that is externally reachable (`lm_v2`, `lm_sub2`, `lm_dot2` are
`static`). So every row below drives the real, lowest-level export through the
`.so` boundary via `libloading`; there is nothing shallower to test.

## Configuration table

Every row is exercised in `tests/differential.rs` (module `phase_b`) with
**randomized inputs, `StdRng`-free deterministic SplitMix64/xorshift PRNG,
fixed seeds** — the per-row iteration count is in the last column. Comparison
is on raw `u32` bit patterns of both returned fields.

| # | entry point(s) | configuration (options set + input shape) | iters | ✔ |
|---|----------------|-------------------------------------------|-------|---|
| C1 | `to_barycentric` | unit-scale non-degenerate CCW triangle, query point strictly inside (barycentric weights sampled in the simplex) | 20 000 | [x] |
| C2 | `to_barycentric` | unit-scale non-degenerate CW (reversed winding) triangle, query point strictly inside | 20 000 | [x] |
| C3 | `to_barycentric` | axis-aligned right triangle `(0,0),(1,0),(0,1)` with the query point swept over a random grid — the canonical "reference triangle" fast path in the caller's algorithm | 20 000 | [x] |
| C4 | `to_barycentric` | non-degenerate triangle, query point exactly at each of the three vertices and at edge midpoints (exact `0`/`1`/`0.5` weights) | 12 000 | [x] |
| C5 | `to_barycentric` | non-degenerate triangle, query point OUTSIDE (weights outside `[0,1]`, incl. negative) | 20 000 | [x] |
| C6 | `to_barycentric` | non-degenerate triangle, query point FAR outside (weights of magnitude up to `1e6`) | 20 000 | [x] |
| C7 | `to_barycentric` | needle-thin near-degenerate triangle (third vertex perturbed off the `p1p2` line by `1e-6..1e-3`), query point inside | 20 000 | [x] |
| C8 | `to_barycentric` | exactly collinear triangle (`p3 = p1 + t*(p2-p1)`), random `t` — denominator is mathematically zero, so `inf`/`nan` results | 20 000 | [x] |
| C9 | `to_barycentric` | coincident-vertex degenerate triangles: each of the three pairs `p1==p2`, `p1==p3`, `p2==p3` in turn, plus all-three-equal | 20 000 | [x] |
| C10 | `to_barycentric` | tiny magnitude regime: all coordinates scaled to `1e-20..1e-30` (products underflow, denominator flushes toward `0`) | 20 000 | [x] |
| C11 | `to_barycentric` | huge magnitude regime: all coordinates scaled to `1e18..1e30` (products overflow to `inf`, `inf-inf` → `nan`) | 20 000 | [x] |
| C12 | `to_barycentric` | mixed-scale regime: each of the 8 coordinates independently drawn from a different exponent decade (`1e-25`..`1e25`) → catastrophic cancellation in the denominator | 40 000 | [x] |
| C13 | `to_barycentric` | integer-valued coordinates in `[-64, 64]` (exactly representable; many exact-zero and exact-power-of-two intermediates, and many exactly-degenerate draws) | 40 000 | [x] |
| C14 | `to_barycentric` | `±0.0` placed in every one of the 8 coordinate slots, in both signs, with the other slots random normals (`-0` changes the SIGN of the zero denominator, hence `+inf` vs `-inf`) | 8 × 2 × 2 000 | [x] |
| C15 | `to_barycentric` | subnormal coordinates: one slot set to a random subnormal (`0x00000001..0x007fffff`, both signs), other slots random normals; and all-8-subnormal | 8 × 2 000 + 4 000 | [x] |
| C16 | `to_barycentric` | `±inf` in every one of the 8 coordinate slots, both signs, other slots random normals; plus all-8-infinite | 8 × 2 × 2 000 | [x] |
| C17 | `to_barycentric` | quiet NaN with a RANDOM payload in every one of the 8 coordinate slots, both signs, other slots random normals (exercises the `.x` = dst-is-LHS vs `.y` = dst-is-RHS asymmetry of `lm_dot2`) | 8 × 2 × 4 000 | [x] |
| C18 | `to_barycentric` | MULTIPLE simultaneous NaNs (2–8 random slots NaN with distinct random payloads) — decides which payload wins each `mulss`/`addss`/`subss`/`divss` | 200 000 | [x] |
| C19 | `to_barycentric` | signaling NaNs (quiet bit clear) in random slots, both signs, random payloads — SSE must quiet them identically | 100 000 | [x] |
| C20 | `to_barycentric` | fully unstructured: all 8 coordinates are uniformly random 32-bit patterns (hits every value class, incl. NaN/inf/subnormal, in every position) | 400 000 | [x] |
| C21 | `to_barycentric` | exponent sweep: all 256 `f32` exponents × random mantissas/signs across the 8 slots (guarantees the `0x00` subnormal and `0xff` inf/NaN exponent boundaries are hit in every slot) | 256 × 512 | [x] |
| C22 | `to_barycentric` | ABI stress: the same logical inputs passed after clobbering the caller's XMM registers, and results read back field-by-field — confirms the by-value struct-in-XMM passing/returning convention matches (an ABI mismatch would show as swapped or garbage fields) | 20 000 | [x] |
| C23 | `to_barycentric` | argument-aliasing shapes: the *same* `lm_vec2` value supplied for several parameters at once (all 15 non-empty subsets of `{p1,p2,p3,p}` sharing one value), random values | 15 × 2 000 | [x] |
| C24 | `to_barycentric` | repeated-call / statelessness check: a fixed random input called 1 000 times interleaved with other random calls — verifies neither library carries hidden state between calls | 1 000 | [x] |

## Configuration axes outside the input surface

| axis | values | how covered |
|------|--------|-------------|
| cargo features | none declared in `Cargo.toml` | `run_all.sh` runs the suite under the default set AND `--no-default-features`; both are the same single code path |
| cargo profile | `dev`, `release` (`release` sets `panic = "abort"`) | `run_all.sh` runs the suite under both, and the `.so` under test is rebuilt for each |
| C build type | `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` → unoptimised `-O0` | this is the reference build the Rust is matched against; the NaN-payload destination-operand analysis in `src/lib.rs` is derived from its disassembly (verified with `objdump -d`) |
| byte order / element type / count / format | not applicable | no serialisation, no buffers, no counts; the only element type is `f32` |
