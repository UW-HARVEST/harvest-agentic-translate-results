# CONFIGS.md — Configuration-surface table (valid inputs)

## Mechanical derivation of the axes

### Public entry points

`c_src/include/lib.h` is one line and declares the entire public API:

```c
void tfm(float *dest, const float *src, int count);
```

`nm -D` on the C `.so` confirms exactly one exported symbol (`tfm`). There are
no convenience wrappers vs. low-level functions to distinguish — `tfm` *is* the
lowest-level entry point, so every row below drives it directly.

### Runtime options / modes / flags

Grepped for: `#define`, `#if`, `#ifdef`, `enum`, `static` globals, setter
functions, environment reads. **None exist.** The library is stateless and
takes no options; there are no `#ifdef`-gated code paths, and the Rust crate
declares no `[features]` in `Cargo.toml`, so there is exactly one build
configuration.

### Branches the C actually takes

From `c_src/src/lib.c`, the complete set of control-flow / value-dependent
decisions:

| source | branch |
|--------|--------|
| line 7 | `for (i = 0; i < count; i++)` — iteration count axis |
| line 8 | `if (src[0] < src[1])` — the **branch axis**: `if` (ordered ascending) vs `else` (`>=`, equal, or unordered/NaN). The two branches bind `dx2`/`dy2` to *opposite* inputs **and** assign the outputs in swapped slots, so they are genuinely different code. |
| lines 15, 25 | `(((0) > (sqd)) ? (0) : (sqd))` — the **clamp axis**: `sqd > 0`, `sqd == +0.0`, `sqd < 0`, `sqd == NaN`. (`sqd == -0.0` is a *fifth* nominal case but is unreachable: the final addend `4.0f*dxy*dxy` is `+0.0f` for every finite `dxy`, and `x + (+0.0)` is never `-0.0`. See ERRORS.md row 10.) |
| lines 15, 25 | `sqrtf(...)` — exact vs. inexact square roots |
| lines 12–13, 22–23 | FP arithmetic on the input **magnitudes**: overflow to `±inf`, `inf - inf` and `0 * inf` invalid ops, underflow to subnormal/zero, cancellation |
| lines 29–30 | `src += 3; dest += 2;` — mismatched strides ⇒ the **aliasing axis** (no `restrict` in the signature) |

### Input-shape axes

* `count`: `1`, `2`, `3`, small, large (many), non-multiple-of-vector-width
  (to catch autovectorisation tail bugs: 1,2,3,4,5,7,8,15,16,17,31,33,1000).
* element value classes: normal, subnormal, `±0.0`, `±FLT_MIN`,
  `±FLT_TRUE_MIN`, `±FLT_MAX`, `±inf`, quiet `NaN` (canonical and
  non-canonical payloads, both signs), signalling-`NaN` bit patterns, and
  fully-random 32-bit bit patterns.
* `dest`/`src` relationship: disjoint, exactly aliased, partially overlapping
  in both directions.

## Table

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (not a single hand-picked value), and asserted **byte-for-byte**
(`to_bits()` comparison, so `-0.0 != 0.0` and NaN payloads are compared).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tfm` | `count == 1`, `if`-branch forced (`src[0] < src[1]`), random finite normals | [x] |
| 2 | `tfm` | `count == 1`, `else`-branch forced (`src[0] > src[1]`), random finite normals | [x] |
| 3 | `tfm` | `count == 1`, `else`-branch via **exact equality** `src[0] == src[1]` (boundary of the `<` test), random values | [x] |
| 4 | `tfm` | `count == 1`, unconstrained random finite normals — branch chosen by the data (mixes both branches) | [x] |
| 5 | `tfm` | `count == 1`, clamp axis: inputs constructed so `sqd > 0` (positive discriminant ⇒ real `sqrtf`) | [x] |
| 6 | `tfm` | `count == 1`, clamp axis: inputs constructed so `sqd == +0.0` exactly (`dxy == 0`, `dx2 == dy2` ⇒ discriminant exactly zero) | [x] |
| 7 | `tfm` | `count == 1`, clamp axis: `sqd < 0` (clamped to `0.0f`, `sqrtf(0)`) | [x] |
| 8 | `tfm` | `count == 1`, `dxy == 0` so the `4*dxy*dxy` term vanishes (`±0.0` both signs) | [x] |
| 9 | `tfm` | `count == 1`, perfect-square discriminants (exact `sqrtf` results) | [x] |
| 10 | `tfm` | `count == 1`, all three inputs drawn from the **special-value set** (`±0.0`, `±FLT_MIN`, `±FLT_TRUE_MIN` subnormals, `±1.0`, `±FLT_MAX`, `±inf`, `NaN`) — full 3-way cross product, exhaustive | [x] |
| 11 | `tfm` | `count == 1`, huge magnitudes near `FLT_MAX` so `dy2*dy2`/`dx2*dx2` **overflow to `inf`** and `inf - inf` yields the invalid-op NaN | [x] |
| 12 | `tfm` | `count == 1`, tiny magnitudes near `FLT_TRUE_MIN` so products **underflow** to subnormal/zero | [x] |
| 13 | `tfm` | `count == 1`, fully random 32-bit **bit patterns** reinterpreted as `f32` (covers every class incl. sNaN and non-canonical NaN payloads simultaneously) | [x] |
| 14 | `tfm` | `count == 1`, mixed-magnitude triples (one huge, one tiny, one normal — all 6 permutations) to hit catastrophic cancellation in `sqd` | [x] |
| 15 | `tfm` | `count == 2` (smallest multi-iteration; verifies pointer advance `src+=3`/`dest+=2`), random finite | [x] |
| 16 | `tfm` | `count` ∈ {3,4,5,7,8,15,16,17,31,33} — vector-width boundaries and odd tails, random finite | [x] |
| 17 | `tfm` | `count == 1000` (large, many iterations), random finite normals | [x] |
| 18 | `tfm` | `count == 1000` with random **bit-pattern** elements (both branches and every value class interleaved across iterations) | [x] |
| 19 | `tfm` | `count` large, elements drawn from the special-value set so consecutive iterations alternate branches and clamp outcomes | [x] |
| 20 | `tfm` | Aliasing: `dest == src` (exact aliasing), `count` 1..64, random finite — mismatched strides make writes clobber unread source | [x] |
| 21 | `tfm` | Aliasing: `dest = src + k` for `k` ∈ 1..6 (forward partial overlap), random finite | [x] |
| 22 | `tfm` | Aliasing: `src = dest + k` for `k` ∈ 1..6 (backward partial overlap), random finite | [x] |
| 23 | `tfm` | Aliasing with random **bit patterns** and `dest == src` | [x] |
| 24 | `tfm` | Non-zero pre-filled `dest` with guard elements around both buffers, `count` 1..8 — verifies exactly `2*count` writes and `3*count` reads and no over-run | [x] |
| 25 | `tfm` | Unaligned-relative buffers (`src`/`dest` offset by 1..3 floats inside larger allocations) to defeat any alignment assumption in vectorised code | [x] |
| 26 | `tfm` | Repeated back-to-back calls on the same buffers (statelessness: no residual state between invocations) | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains only `add_library(... SHARED src/lib.c)` — no
`add_executable`. The Rust crate is `crate-type = ["cdylib"]` with no `[[bin]]`.
**There is no driver binary**, so the stdout-comparison requirement is
vacuously satisfied.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies, so the only build configurations are
`--no-default-features` and default — which are byte-identical. Both are run.

---

## Verification result

Every row above is checked `[x]`: implemented as the correspondingly-named test
in `tests/phase_b_valid.rs` (`row01_…` … `row26_…`), each driving BOTH `.so`s
through `dlopen`/`dlsym` over many fixed-seed randomized inputs and comparing
`f32::to_bits()` (so `-0.0 != 0.0` and NaN sign/payload are compared).

```
$ cargo test --offline --test phase_b_valid
test result: ok. 26 passed; 0 failed
```

Additional cross-cutting rows in `tests/phase_d_parity.rs`:

| entry point | configuration | [x] |
|-------------|---------------|-----|
| `tfm` | `soak_all_shapes_mixed` — 20 000 calls, `count` 1..64, value class chosen per element from {finite, arbitrary bits, specials/NaNs, full-exponent-range}, each run both disjoint and self-aliased | [x] |
| `tfm` | `chunked_equals_whole` — one `count == n` call must equal `n` separate `count == 1` calls, asserted for BOTH implementations (catches loop-carried state) | [x] |
| `tfm` | `split_calls_compose` — `count == n` must equal `count == k` then `count == n-k` at the right offsets, for BOTH implementations | [x] |
| `tfm` | `rust_matches_cmake_build_and_all_opt_levels_agree_off_nan` — 400 000 inputs against the CMake C build *and* against `gcc -O0/-O1/-O2/-O3/-Os` rebuilds of the same C | [x] |

### Cross-`-O`-level finding (recorded, not a defect)

`src/lib.rs` documents that GCC chooses different FP operand orders at different
`-O` levels and that this is observable through x86 SSE NaN-payload selection.
That was confirmed empirically. Comparing the CMake build (`CMAKE_BUILD_TYPE`
unset ⇒ no `-O` flag ⇒ `-O0`) against rebuilds of the *same* C source, over
60 000 000 random triples (59 649 217 with no NaN operand, 350 783 with one):

| implementation | diverges from CMake build | on NaN-free input | on NaN-operand input |
|---|---|---|---|
| `gcc -O0` | 0 | 0 | 0 |
| `gcc -O1` | >0 | **0** | all of them |
| `gcc -O2` | 7 573 | **0** | 7 573 |
| `gcc -O3` | 4 242 | **0** | 4 242 |
| `gcc -Os` | >0 | **0** | all of them |
| **Rust cdylib (release)** | **0** | **0** | **0** |
| **Rust cdylib (debug)** | **0** | **0** | **0** |

So: the C disagrees with *itself* across `-O` levels, but *only* when an input
operand is already a NaN — never for NaN-free input. The Rust matches the build
`c_src/CMakeLists.txt` actually produces on **every** input, NaN operands
included, in both the debug and the release profile. Both halves of this are
pinned by `rust_matches_cmake_build_and_all_opt_levels_agree_off_nan`.

## Feature-combination result

`verify.sh` enumerates the configurations from `Cargo.toml` and runs the whole
suite (Phases A–D) against each, in both the debug and the release profile:

| config | profile | symbol diff | Phases A–D |
|--------|---------|-------------|-----------|
| default | debug | empty | 52/52 pass |
| default | release | empty | 52/52 pass |
| `--no-default-features` | debug | empty | 52/52 pass |
| `--no-default-features` | release | empty | 52/52 pass |
