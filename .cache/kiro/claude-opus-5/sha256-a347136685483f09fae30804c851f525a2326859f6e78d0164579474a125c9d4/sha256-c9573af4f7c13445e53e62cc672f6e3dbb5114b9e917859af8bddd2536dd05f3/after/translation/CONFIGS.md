# CONFIGS.md — configuration-surface table (Phase A, gate for Phase B)

## Mechanical derivation of the axes

### Public entry points (full set, from `c_src/include/lib.h`)

```c
typedef struct lm_vec2 { float x, y; } lm_vec2;
lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
```

One exported function. `nm -D --defined-only` on the C `.so` confirms
`to_barycentric` is the only global symbol, so there is no lower-level entry
point hidden behind it — `lm_v2` / `lm_sub2` / `lm_dot2` are `static` and
therefore unreachable for an external caller. There is no convenience-wrapper /
low-level split to exercise separately: `to_barycentric` *is* the lowest level
the ABI exposes. It is called directly through `dlsym` in every row below.

### Runtime options / modes / flags

```sh
grep -nE 'if|switch|#if|#ifdef|enum|static [a-z_]* [a-z_]*flag|setenv|getenv' c_src/src/lib.c
# (no matches for any branch or flag construct)
```

Zero. The function is branch-free and stateless: no globals, no
initialisation, no option struct, no mode enum, no `#ifdef`. There is no state
to set up and no option to apply, so the option axis has exactly one value
("none"). Likewise there is no Cargo feature axis — `translation/Cargo.toml`
declares **no** `[features]` table, so the only feature combination that exists
is the default (empty) one; see the note at the bottom.

### Input-shape axes the arithmetic actually distinguishes

`to_barycentric` has a fixed shape (exactly four `lm_vec2` = eight `float`s;
no counts, sizes, widths, element types, formats or byte-order parameters).
The axes it *does* distinguish are therefore the numeric classes of those eight
floats and the geometric relationships between them, because those are what
decide the magnitude/sign/exceptional status of the intermediates
(`v0`, `v1`, `v2`, `dot00`, `dot01`, `dot02`, `dot11`, `dot12`, `denom`,
`invDenom`):

* **A. geometric relation of `p1,p2,p3`** — non-degenerate (positive area),
  right-angled at `p1` (`dot01 == 0`), collinear (`denom == 0`), two vertices
  coincident, all three coincident, mirrored/negative winding.
* **B. position of `p`** — at `p1` (`v2 == 0`, `u = v = 0`), at `p2`, at `p3`,
  interior centroid, on an edge, outside, far outside.
* **C. magnitude class** — 1, tiny (~1e-30, denom underflow), huge (~1e20,
  denom overflow), `FLT_MAX`, mixed tiny/huge in the same call.
* **D. IEEE class of individual components** — normal, `±0.0`, subnormal,
  `±inf`, quiet NaN, signalling NaN, arbitrary random bit pattern.
* **E. sign pattern** — all positive, all negative, mixed, `-0.0` vs `+0.0`.
* **F. exact-vs-rounded arithmetic** — small integer coordinates (all
  intermediates exact in binary32) vs values whose products round.

`v0`/`v1`/`v2` are only ever consumed componentwise by `lm_dot2`, so the
cross-product of A×B×C×D×E×F pruned to combinations the code treats
differently is the row list below. Every row is driven with **many** randomized
inputs from a fixed seed (`SplitMix64`, seed `0x5EED_1234_ABCD_EF01`), not a
single hand-picked value, and compared bit-for-bit (`f32::to_bits`) between the
C `.so` and the Rust `.so`, both loaded via `libloading`.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C1 | `to_barycentric` | options: none (stateless). Non-degenerate triangle, small **integer** coordinates in `[-16,16]`, `p` random integer — every intermediate exact in binary32 | [x] |
| C2 | `to_barycentric` | Non-degenerate triangle, random normals in `[-1,1)`, `p` interior (random convex combination) — the canonical happy path, rounded arithmetic | [x] |
| C3 | `to_barycentric` | Non-degenerate triangle, `p` exactly at `p1` / `p2` / `p3` (each sub-case), random vertices | [x] |
| C4 | `to_barycentric` | Non-degenerate triangle, `p` on an edge (random convex combination of two vertices) | [x] |
| C5 | `to_barycentric` | Non-degenerate triangle, `p` outside (barycentric coords in `[-8,8]`, extrapolated) | [x] |
| C6 | `to_barycentric` | Right-angled at `p1` (`v0 ⟂ v1` ⇒ `dot01 == 0`), random leg lengths — kills the `dot01*dot01` term | [x] |
| C7 | `to_barycentric` | Negative winding / mirrored triangle (vertices swapped ⇒ `denom` computed from reversed edges) | [x] |
| C8 | `to_barycentric` | Collinear vertices, non-coincident (`v1 = k*v0`, random `k`) ⇒ `denom == 0` with non-zero numerators ⇒ `±inf` | [x] |
| C9 | `to_barycentric` | Coincident vertices: `p2 == p1`, then `p3 == p1`, then `p3 == p2`, then all three equal (four sub-cases), random `p` | [x] |
| C10| `to_barycentric` | Tiny magnitudes (all components scaled to ~`1e-25`…`1e-30`) ⇒ dots underflow / flush to `0` / go subnormal | [x] |
| C11| `to_barycentric` | Huge magnitudes (~`1e18`…`1e20`) ⇒ dots overflow to `+inf`, `invDenom → 0` | [x] |
| C12| `to_barycentric` | Mixed magnitude classes within one call (one vertex tiny, one huge, one normal) — catastrophic cancellation in `denom` | [x] |
| C13| `to_barycentric` | Sign-pattern sweep: all-`+`, all-`-`, mixed, and `±0.0` components (incl. `-0.0` where the C yields `-0.0`) | [x] |
| C14| `to_barycentric` | Subnormal components (`0x00000001`…`0x007FFFFF`, random) | [x] |
| C15| `to_barycentric` | `±inf` components in random positions (1–8 of the 8 floats) | [x] |
| C16| `to_barycentric` | Quiet-NaN components with **distinct payloads** in random positions — exercises SSE destination-operand NaN selection, i.e. GCC's exact operand order | [x] |
| C17| `to_barycentric` | Signalling-NaN components (`0x7F80_0001`-class) in random positions ⇒ quieted intermediates | [x] |
| C18| `to_barycentric` | `±FLT_MAX` / `±FLT_MIN` boundary components in random positions | [x] |
| C19| `to_barycentric` | Wide-exponent random normals: uniform random sign + exponent in `[1,254]` + random mantissa (spans the whole normal range, not just `[-1,1]`) | [x] |
| C20| `to_barycentric` | Unrestricted fuzz: all eight floats are uniformly random 32-bit words (every IEEE class, every combination, incl. NaN×NaN in both operand positions of every op) | [x] |
| C21| `to_barycentric` | Argument-aliasing shapes: the same `lm_vec2` value passed in 2, 3 or all 4 argument slots (random value), covering `p == p1 == p2 == p3` etc. | [x] |
| C22| `to_barycentric` | Repeated-call / statelessness check: the same input replayed 64 times interleaved with other random inputs, asserting C and Rust are both pure (identical every time) | [x] |

## Cargo feature combinations

`translation/Cargo.toml` contains no `[features]` section and the crate has no
optional dependencies:

```sh
grep -n '\[features\]\|^\[dependencies\]\|optional' translation/Cargo.toml
```

so the complete set of feature combinations is exactly one — the default. It
is nevertheless verified explicitly by the driver script
`translation/run_all_features.sh`, which enumerates the (empty) feature list
from `Cargo.toml` and runs the whole suite under `--no-default-features` as
well as with default features, proving both configurations pass.

## C build configuration — the one axis that is NOT free to choose

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the documented build

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

compiles at `-O0` with no inlining. **That artifact is the reference**, and it
is the one every test in `tests/` loads.

This is not a cosmetic detail. `mulss`/`addss` return their *destination*
operand when both operands are NaN, and GCC chooses different destination
operands at `-O0` than at `-O2`/`-O3`:

| operation | `-O0` destination | optimized destination |
|-----------|-------------------|-----------------------|
| `lm_dot2`: `a.y * b.y` | `b.y` | `a.y` (after inlining) |
| `lm_dot2`: `x_prod + y_prod` | `y_prod` | `x_prod` |
| `dot00 * dot11` | `dot00` | `dot11` |
| `dot01 * dot12` (u numerator) | `dot01` | `dot12` |
| `dot00 * dot12` (v numerator) | `dot00` | `dot12` |
| `dot01 * dot02` (v numerator) | `dot01` | `dot02` |

So the two C builds are **not bit-identical to each other**: measured by row
C20, they disagree on 135 of 200 000 uniformly random 32-bit argument patterns
— precisely the inputs where two *different* NaNs meet as the two operands of
one commutative instruction. No single implementation can match both, so this
crate matches the documented `-O0` build.

`translation/check_optimized_c.sh` reproduces the whole measurement: it builds
the unmodified `c_src` a second time out-of-tree with
`-DCMAKE_BUILD_TYPE=Release`, runs the fuzz row against both artifacts (0
divergences against `-O0`, 135 against the optimized build), and dumps the
`mulss`/`addss`/`subss`/`divss` sequence of each for inspection. All tests
accept a `C_SO=<path>` override so any other C artifact can be substituted.

Everything outside NaN-payload provenance — every finite, zero, subnormal and
infinite result — is identical under both C builds and under both Rust
optimisation levels (row-level proof: `phase_d_debug_and_release_agree`).

## Result

All 22 rows pass under every feature configuration. Reproduce with:

```sh
cd translation && ./run_all_features.sh
```
