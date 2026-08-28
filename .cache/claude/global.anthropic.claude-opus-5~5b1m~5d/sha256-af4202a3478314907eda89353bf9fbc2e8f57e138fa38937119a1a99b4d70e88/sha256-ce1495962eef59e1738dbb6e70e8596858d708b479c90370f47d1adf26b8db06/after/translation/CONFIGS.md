# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from `c_src/src/lib.c`. Method: enumerate every runtime
option the public API can set, then every branch the C actually takes, then every
input *shape* the code special-cases, then take the pruned cross-product.

## Public entry points (complete)

`c_src/include/lib.h` is one line and declares exactly one function. There are
no convenience wrappers and no lower-level helpers with external linkage — the
lowest-level entry point *is* the only entry point:

| entry point | signature |
|-------------|-----------|
| `tfm` | `void tfm(float *dest, const float *src, int count)` |

## Runtime options / modes / flags

```
$ grep -rn '#if\|#ifdef\|#define\|#pragma' c_src/src/lib.c c_src/include/lib.h
(no matches)
```

**There are none.** No global state, no init/config struct, no flags word, no
`#ifdef`, no environment lookup, no enum selector. The behaviour is a pure
function of the three arguments. So the configuration axes are entirely
*data-shape* axes plus the *buffer geometry* the caller chooses.

## Branch axes the C actually distinguishes

| axis | source line | distinct values |
|------|-------------|-----------------|
| A. element branch | `if (src[0] < src[1])` (line 8) | `if` (strictly less) · `else` (greater, equal, or *unordered*) |
| B. discriminant clamp | `(((0) > (sqd)) ? (0) : (sqd))` (lines 15, 25) | clamp taken (`sqd < 0`) · clamp not taken (`sqd >= 0`, `sqd == -0.0f`, `sqd` NaN) |
| C. loop guard | `for (i = 0; i < count; i++)` (line 7) | `count <= 0` (zero trips) · `1` · `> 1` |
| D. IEEE class per lane | the arithmetic on lines 12–15 / 22–25 | normal · subnormal · `±0` · `±inf` · qNaN · sNaN · overflow-to-inf · invalid-op (`inf-inf`, `0*inf`) |
| E. lane role swap | lines 9–11 vs 19–21 bind `dx2`/`dy2` to **opposite** inputs | which of `src[0]`/`src[1]` is `dx2` |
| F. buffer geometry | `src += 3; dest += 2;` (lines 29–30) — strides differ, so caller-chosen overlap is meaningful and well-defined | disjoint · `dest == src` · `dest == src+k` · `dest` before `src` · float-offset (alignment) |

`[x]` = a differential test exists and passes, across many randomized inputs
(fixed seed), against **both** the C `.so` and the Rust `.so`.

## Table

### Loop-count shapes (axis C) — disjoint buffers, random finite normals

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B1 | `tfm` | `count = 0`; dest pre-filled with canaries; 256 random src buffers | [x] |
| B2 | `tfm` | `count = 1` (tight lower boundary, exactly 3 in / 2 out) | [x] |
| B3 | `tfm` | `count = 2` | [x] |
| B4 | `tfm` | `count = 3` (odd, not a vector multiple) | [x] |
| B5 | `tfm` | `count = 7` (prime, forces scalar remainder if vectorized) | [x] |
| B6 | `tfm` | `count = 8` (exact 4×f32 vector multiple of the 2-wide output) | [x] |
| B7 | `tfm` | `count = 16` | [x] |
| B8 | `tfm` | `count = 17` (vector body + 1 remainder) | [x] |
| B9 | `tfm` | `count = 1000` | [x] |
| B10 | `tfm` | `count = 100_000` (large, single call) | [x] |
| B11 | `tfm` | `count ∈ {-1, -2, -1000, INT_MIN, INT_MIN+1}`, valid non-null buffers | [x] |

### Element-branch patterns (axes A + E)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B12 | `tfm` | every element takes the `if` branch (`src[0] < src[1]` strictly), random magnitudes, `count = 64` | [x] |
| B13 | `tfm` | every element takes the `else` branch via `src[0] > src[1]` | [x] |
| B14 | `tfm` | every element takes the `else` branch via `src[0] == src[1]` (exact equality, incl. `+0.0`/`-0.0` pair which compares equal) | [x] |
| B15 | `tfm` | strictly alternating `if`/`else` down the array, `count = 65` | [x] |
| B16 | `tfm` | randomly mixed `if`/`else` per element (unbiased coin, fixed seed), `count = 257` | [x] |

### Discriminant / clamp regimes (axis B)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B17 | `tfm` | `sqd > 0` strictly — large real discriminant (`dxy` dominant), both branches | [x] |
| B18 | `tfm` | `sqd == +0.0f` exactly (`dx2 == dy2`, `dxy == 0`) → `sqrtf(+0)=+0`, both branches | [x] |
| B19 | `tfm` | `sqd == -0.0f` — clamp **not** taken. **Proved UNREACHABLE** (0 hits over 24³ alphabet + cancellation family + 400 k random triples); the row asserts unreachability instead of asserting a vacuous pass, and the `-0.0` semantics are covered by `e14_*` feeding `-0.0` through every input lane | [x] |
| B20 | `tfm` | `sqd < 0` — clamp **taken**. Reachable only through rounding, so constructed deliberately from near-equal `dx2`/`dy2` (`1 + p·2⁻²³` vs `1 + q·2⁻²³`) with a small `dxy`; 400 hits, both branches | [x] |
| B21 | `tfm` | `sqd == +inf` via `4.0f*dxy*dxy` overflow (`dxy ≈ 1e30`), both branches | [x] |
| B22 | `tfm` | `sqd` NaN via `inf - inf` (`dy2*dy2` and `2*dx2*dy2` both overflow), both branches | [x] |
| B23 | `tfm` | `sqd` NaN via `0 * inf` (`dx2 = ±0`, `dy2 = ±inf`), both branches | [x] |

### IEEE class per lane (axis D)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B24 | `tfm` | all 3 lanes normal, magnitudes in `[-1, 1]`, 4096 random elements | [x] |
| B25 | `tfm` | all 3 lanes huge normals, `|x| ∈ [1e30, FLT_MAX]` (overflow-prone) | [x] |
| B26 | `tfm` | all 3 lanes tiny: subnormals and `|x| ∈ (0, FLT_MIN)` (underflow-prone, FTZ off) | [x] |
| B27 | `tfm` | signed zeros: all 8 combinations of `(±0, ±0, ±0)` | [x] |
| B28 | `tfm` | infinities: full 3-lane cross product over `{-inf, +inf, -1.0, 0.0, +1.0}` (125 triples) | [x] |
| B29 | `tfm` | canonical quiet NaNs: full 3-lane cross product over `{+qNaN, -qNaN, 1.0, -1.0, inf}` | [x] |
| B30 | `tfm` | signalling NaNs `0x7FA0_0000` / `0xFFA0_0000` in each lane position | [x] |
| B31 | `tfm` | NaNs with random non-canonical payloads (random 22-bit payload, random sign) in each lane position, 2048 elements | [x] |
| B32 | `tfm` | **exhaustive** 3-lane cross product over a 24-value special alphabet (`±0`, `±MIN_SUBNORMAL`, `±FLT_MIN`, `±1`, `±FLT_MAX`, `±inf`, `±qNaN`, `±sNaN`, `±0.5`, `±2`, non-canonical NaNs) = 24³ = 13 824 triples | [x] |
| B33 | `tfm` | fully random 32-bit patterns reinterpreted as `f32` in all 3 lanes (hits every class incl. exotic NaNs), 200 000 elements | [x] |

### Buffer geometry / aliasing (axis F)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B34 | `tfm` | disjoint separate allocations for `src` and `dest` (the baseline used by B1–B33) | [x] |
| B35 | `tfm` | fully in-place: `dest == src` (well-defined: writes at `2i`,`2i+1` always trail reads at `3i..3i+2`), `count = 64` | [x] |
| B36 | `tfm` | `dest == src + 1` (overlapping, write index chases read index) | [x] |
| B37 | `tfm` | `dest == src + 2` (iteration *i+1* reads a float written by iteration *i*) | [x] |
| B38 | `tfm` | `dest == src + 3` | [x] |
| B39 | `tfm` | `dest == src + 3*count` (adjacent but disjoint, same allocation) | [x] |
| B40 | `tfm` | `dest` region *before* `src` with overlap: `src == dest + 1` | [x] |
| B41 | `tfm` | `src` and `dest` at float offsets `{0,1,2,3}` inside a larger allocation (all 16 combinations) — `float`-aligned but not 16-byte aligned | [x] |

### Composition / statelessness

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| B42 | `tfm` | 64 successive calls on the same buffers (proves no hidden state / no accumulated FP mode change), random data each call | [x] |
| B43 | `tfm` | one call with `count = N` vs `N` calls with `count = 1` and manually advanced pointers — identical output required for both C and Rust (exercises the pipeline composition, not just one wrapper invocation) | [x] |
| B44 | `tfm` | `count` exceeding the logical element count, buffer over-allocated and pre-seeded so the trailing "garbage" is deterministic — both must process the extra elements identically | [x] |

---

## Verification result

All **44** rows pass across randomized inputs. The mapping to tests is 1:1 and
mechanically checkable:

```
$ diff <(grep -oE '^\| B[0-9]+ ' CONFIGS.md | grep -oE '[0-9]+' | awk '{printf "b%02d\n",$0}' | sort) \
       <(grep -oE '^fn b[0-9]+' tests/phase_b.rs | grep -oE 'b[0-9]+' | sort)
# (no output — 1:1 match)
```

### How the rows are made non-vacuous

A configuration row is worthless if the generator silently fails to reach the
configuration it claims. Each row therefore *proves* it hit its target:

* **Branch rows (B12–B16)** assert, per element, that the generated pair really
  selects the intended branch, before running the diff.
* **`sqd`-regime rows (B17–B23)** are driven by a search over the cancellation
  family + the 24³ alphabet + 400 000 random triples, classified by a mirror of
  the C discriminant. Each row asserts a non-empty hit set **and** that both C
  branches are represented. Observed hit counts:

  | row | regime | triples found | if-branch / else-branch |
  |-----|--------|---------------|--------------------------|
  | B17 | `sqd > 0` | 400 | 186 / 214 |
  | B18 | `sqd == +0.0` | 400 | 200 / 200 |
  | B19 | `sqd == -0.0` | **0 — unreachable** (proved, see ERRORS.md E14) | — |
  | B20 | `sqd < 0`, clamp taken | 400 | 192 / 208 |
  | B21 | `sqd == +inf` | 400 | 211 / 189 |
  | B22 | `sqd` NaN via `inf - inf` | 7 984 (all-finite inputs) | both |
  | B23 | `sqd` NaN via `0 * inf` | 64 / 64 | both |

* **B26/B32/B33** are exhaustive or near-exhaustive rather than sampled:
  13 824 alphabet triples, 200 000 random bit patterns, and a dense sweep of the
  smallest subnormal magnitudes.
* **E20's** counterpart in Phase C additionally asserts a subnormal *output* was
  actually observed, so "FTZ is off" is demonstrated rather than assumed.
* **B43** cross-checks the *composition*: one call with `count = N` must equal
  `N` calls with `count = 1`, for each implementation independently as well as
  across them. That catches pointer-stepping bugs that per-element tests cannot.
* **Guard canaries** surround every `dest` window (`diff_disjoint`,
  `diff_offsets`), and `src` is snapshotted and re-compared, so a stray write
  one element out of range fails the row even when the in-range values match.

### The lowest-level entry point *is* the only entry point

`tfm` is not a convenience wrapper — `c_src` exposes nothing below it. So the
instruction to exercise low-level entry points rather than one-shot wrappers is
satisfied by construction; what stands in for "the composed pipeline" here is
the **loop over elements**, the **caller-chosen buffer overlap** (B35–B41) and
the **call-sequence composition** (B42–B44), all of which are covered
separately from the single-element path.

### Optimization-level robustness (`tests/olevels.rs`)

Not a CONFIGS.md row, but the axis with the most risk. `c_src/CMakeLists.txt`
sets no `CMAKE_BUILD_TYPE`, so the canonical/ground-truth build passes no `-O`
flag. The C library is **not self-consistent** across `-O` levels, so it matters
which build the translation tracks. Measured over a 126 859-triple NaN-free
corpus and a 126 965-triple NaN-bearing corpus:

| C build | NaN-free: Rust vs it | NaN-free: canonical C vs it | NaN-bearing: Rust vs it | NaN-bearing: canonical C vs it |
|---------|---------------------|------------------------------|--------------------------|--------------------------------|
| `gcc` (no `-O`) | **0** | **0** | **0** | **0** |
| `-O0` | **0** | **0** | **0** | **0** |
| `-O1` | **0** | **0** | 2 906 | 2 906 |
| `-O2` | **0** | **0** | 2 906 | 2 906 |
| `-Os` | **0** | **0** | 2 906 | 2 906 |
| `-O3` | **0** | **0** | 1 849 | 1 849 |
| `-Ofast` | 15 415 | 15 415 | 41 830 | 41 830 |

Three conclusions, each asserted by a test:

1. **For every NaN-free input the translation is bit-exact at every conforming
   optimization level** (`nan_free_inputs_agree_at_every_optimization_level`).
   The match is therefore not brittle over the realistic input domain.
2. Where the Rust differs from an `-O1`/`-O2`/`-O3` build, **the canonical C
   differs from it in exactly the same number of places**
   (`nan_input_divergence_is_c_vs_c_instability`). The residual disagreement is
   GCC's own `-O`-dependent NaN-payload operand ordering — C-vs-C instability,
   not a translation defect. `-Ofast` is `-ffast-math` and non-conforming, so it
   is reported informationally.
3. `-O0` and the default build are bit-identical to the cmake build **and** to
   the Rust, over all ~254 000 triples
   (`canonical_build_equals_dash_o0_and_default`).

### Independent ABI cross-check (`probe/abi_probe.c`)

Every test above reaches `tfm` through `libloading`/`dlsym`. `probe/abi_probe.c`
is a plain C program **linked directly** against a `.so` (real dynamic linking
and PLT, calling convention chosen by the C compiler rather than by Rust's
`extern "C"` shim). It is built once against the C `.so` and once against each
Rust `.so`, and the outputs are diffed:

```
native C caller: C .so == Rust debug   .so  (22821 lines identical)
native C caller: C .so == Rust release .so  (22821 lines identical)
```

This is what actually closes the argument: an ordinary external C consumer
cannot distinguish the two libraries.
