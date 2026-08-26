# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or conditional definitions. The complete feature set therefore has
one member:

| # | Cargo feature combination | C configuration | Compile check |
|---|---------------------------|-----------------|---------------|
| 1 | Empty set: `--no-default-features --features ""` | Default/unconditional | [x] |

## Mechanically Derived Runtime Axes

- Public entry points: `tfm` only.
- Loop shape: `count <= 0` executes no iterations; positive counts execute one
  iteration per three source floats and two destination floats.
- Arithmetic selector: `src[0] < src[1]` selects the first branch; equality and
  unordered comparisons select the second.
- Clamp selector: `0 > sqd` passes `0` to `sqrtf`; false, including unordered
  `sqd`, passes `sqd`.
- Data classes affecting those selectors or IEEE-754 results: finite normal,
  signed zero, subnormal, NaN, infinity, and finite values whose intermediates
  overflow.
- Pointer shape: disjoint arrays and legal aliasing/overlap. No `restrict`
  qualifier appears in the public declaration.

## Valid-Path Matrix

Each checked row must pass byte-for-byte differential comparisons over many
fixed-seed randomized inputs unless the row is a zero-iteration boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tfm` | Disjoint pointers; `count = 1`; finite normal values; `src[0] < src[1]`; `sqd >= 0` | [x] |
| 2 | `tfm` | Disjoint pointers; `count = 1`; finite normal values; `src[0] > src[1]`; `sqd >= 0` | [x] |
| 3 | `tfm` | Disjoint pointers; `count = 1`; finite equality `src[0] == src[1]` selects the second branch | [x] |
| 4 | `tfm` | Disjoint pointers; `count = 1`; first branch; rounded `sqd < 0` selects the zero clamp | [x] |
| 5 | `tfm` | Disjoint pointers; `count = 1`; second branch; rounded `sqd < 0` selects the zero clamp | [x] |
| 6 | `tfm` | Disjoint pointers; `count = 1`; NaN in `src[0]` makes the comparison unordered and selects the second branch | [x] |
| 7 | `tfm` | Disjoint pointers; `count = 1`; NaN in `src[1]` makes the comparison unordered and selects the second branch | [x] |
| 8 | `tfm` | Disjoint pointers; `count = 1`; NaN in `src[2]` makes `sqd` unordered and bypasses the zero clamp | [x] |
| 9 | `tfm` | Disjoint pointers; `count = 1`; signed-zero and subnormal elements | [x] |
| 10 | `tfm` | Disjoint pointers; `count = 1`; infinities and finite values with overflowing intermediates | [x] |
| 11 | `tfm` | Disjoint pointers; `count > 1`; every item selects the first branch | [x] |
| 12 | `tfm` | Disjoint pointers; `count > 1`; every item selects the second branch | [x] |
| 13 | `tfm` | Disjoint pointers; `count > 1`; mixed branch, clamp, and IEEE-754 data classes | [x] |
| 14 | `tfm` | Exact alias `dest == src`; `count > 1` | [x] |
| 15 | `tfm` | Forward overlap with `dest = src + 2`; stores alter source consumed by later iterations | [x] |
| 16 | `tfm` | Backward overlap with `src = dest + 1`; `count > 1` | [x] |
