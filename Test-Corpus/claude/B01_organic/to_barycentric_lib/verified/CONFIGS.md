# CONFIGS.md — Configuration-surface table (Phase A)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from what the C
code branches on / special-cases, not from a guess about what "matters".

## Axis derivation

### Axis 1 — runtime options / modes / flags: **none**

```sh
$ grep -nE '#if|#ifdef|#else|switch|if *\(|\?|enum|static [a-z_]* [a-z_]*_(flag|mode|opt)' c_src/src/lib.c
(no matches)
```

`c_src/src/lib.c` is 29 lines of straight-line code with no conditional of any
kind, no global/static mutable state, no init/config/set function, and no
compile-time `#ifdef`. `c_src/CMakeLists.txt` defines no options,
no `target_compile_definitions`, and no `CMAKE_BUILD_TYPE`, so there is exactly
one C build configuration. `Cargo.toml` originally declared no `[features]`, so
the Rust side likewise has exactly one configuration (`default = []`, i.e. the
sole combination is the empty feature set — see "Feature combinations" below).

### Axis 2 — public entry points: **1 (which is also the lowest level one)**

```sh
$ grep -n '^[a-z_].*(' c_src/src/lib.c        # non-static definitions
16:lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p) {
```

`to_barycentric` is not a convenience wrapper over an exported lower layer —
the only lower layer (`lm_v2`, `lm_sub2`, `lm_dot2`) is `static` and therefore
unreachable from outside the `.so`. So "exercise the low-level entry points
directly, not only the convenience wrappers" is satisfied by *isolating* each
internal helper through crafted inputs (rows C1-C8 pin `lm_sub2`, rows C9-C14
pin `lm_dot2`'s two multiplies and its add, rows C15-C19 pin the
`invDenom`/`u`/`v` block) rather than by calling them directly.

### Axis 3 — input shapes the code's arithmetic distinguishes

The 8 input floats reach 9 `subss`, 18 `mulss`, 5 `addss` and 1 `divss` (counts
from `objdump` on the C `.so`, weighted by call counts of the inlined-in-Rust
`static` helpers). The
shapes that select different IEEE-754 behaviour in that pipeline are:

* **triangle geometry**: non-degenerate vs. collinear vs. coincident vertices;
  winding (positive vs. negative signed area — sets the sign of `dot01` and
  of the numerators, never of `denom`, which is a Gram determinant ≥ 0);
* **edge relationship**: oblique (`dot01 != 0`) vs. orthogonal (`dot01 == 0`,
  which kills the `dot01*dot01` and `dot01*dot12` terms) vs. parallel;
* **query-point position**: interior, exterior, on each vertex (`v2 == 0`), on
  each edge, far away;
* **magnitude class**: normal, large-but-finite (products near `FLT_MAX`),
  subnormal, mixed-magnitude (catastrophic cancellation in `a - b` and in
  `a*b - c*c`);
* **sign class**: all-positive, all-negative, mixed, `+0` vs `-0`;
* **special class**: `±inf`, quiet NaN, signalling NaN, negative NaN, and — the
  case that decides which NaN survives — **two NaNs meeting in one SSE
  instruction**;
* **ambient FP environment**: the MXCSR rounding mode / FTZ+DAZ bits, which the
  caller owns and both libraries inherit. Included because a translation that
  constant-folded any arithmetic at compile time would diverge here while
  passing every default-mode test.

The table is the cross-product of those axes, pruned to combinations the
arithmetic actually treats differently.

## Configuration-surface table

Every row is driven with **many randomized inputs** (fixed seed `0x2B7E1516`,
xorshift64\* PRNG) unless the row is inherently a single exact bit pattern, in
which case it is driven with randomized *surroundings* (the other slots
randomized). Both libraries are called through their `.so` exports and the
returned `lm_vec2` is compared as raw bits (`to_bits()`), so `+0`/`-0` and NaN
payload differences cannot hide.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `to_barycentric` | non-degenerate triangle, `p` **interior**, coordinates uniform in `[-1, 1]` | `cfg_c1_interior_unit` | [x] |
| C2 | `to_barycentric` | non-degenerate triangle, `p` **exterior** (all three barycentric coords out of `[0,1]`), uniform in `[-1, 1]` | `cfg_c2_exterior` | [x] |
| C3 | `to_barycentric` | `p` exactly **on vertex `p1`** → `v2 = (±0, ±0)`, both numerators `±0`, result must be exactly `(0, 0)` with the right zero signs | `cfg_c3_p_on_p1` | [x] |
| C4 | `to_barycentric` | `p` exactly **on vertex `p2`** → expected `(0, 1)` | `cfg_c4_p_on_p2` | [x] |
| C5 | `to_barycentric` | `p` exactly **on vertex `p3`** → expected `(1, 0)` | `cfg_c5_p_on_p3` | [x] |
| C6 | `to_barycentric` | `p` on **edge midpoints** (`(p1+p2)/2`, `(p1+p3)/2`, `(p2+p3)/2`), randomized triangles | `cfg_c6_p_on_edges` | [x] |
| C7 | `to_barycentric` | canonical **unit triangle** `(0,0),(1,0),(0,1)` with randomized `p` — the `dot01 == 0` orthogonal-edge path | `cfg_c7_unit_triangle_orthogonal` | [x] |
| C8 | `to_barycentric` | **axis-aligned right** triangle with randomized leg lengths (`v0`,`v1` orthogonal ⇒ `dot01 = ±0`, so `dot01*dot01 = +0` and `denom = dot00*dot11`) | `cfg_c8_right_triangle` | [x] |
| C9 | `to_barycentric` | **oblique / skewed** triangle, `dot01 != 0`, randomized (exercises the full `dot00*dot11 - dot01*dot01` cancellation) | `cfg_c9_oblique` | [x] |
| C10 | `to_barycentric` | **negative winding** (`p1`,`p2`,`p3` clockwise) vs. positive, randomized — flips the sign of the `u`/`v` numerators | `cfg_c10_both_windings` | [x] |
| C11 | `to_barycentric` | **all-negative** coordinates, randomized in `[-1e3, -1]` | `cfg_c11_all_negative` | [x] |
| C12 | `to_barycentric` | **integral-valued** floats (exact arithmetic, no rounding) in `[-64, 64]` | `cfg_c12_integral` | [x] |
| C13 | `to_barycentric` | **large magnitude** `[-1e18, 1e18]` — dot products up to `~1e36`, still finite, `denom` near `FLT_MAX`, so `invDenom` is subnormal | `cfg_c13_large_magnitude` | [x] |
| C14 | `to_barycentric` | **huge magnitude** `[-1e20, 1e20]` — dot products **overflow to `+inf`**, `denom = inf - inf` | `cfg_c14_overflow_magnitude` | [x] |
| C15 | `to_barycentric` | **tiny magnitude** `[-1e-20, 1e-20]` — dot products underflow toward subnormal/zero, `invDenom` overflows to `+inf` | `cfg_c15_tiny_magnitude` | [x] |
| C16 | `to_barycentric` | **subnormal** coordinates (exponent field `0`, random 23-bit mantissa) | `cfg_c16_subnormal` | [x] |
| C17 | `to_barycentric` | **mixed magnitude** — each of the 8 slots independently drawn from a random binade in `2^[-45, 45]` (catastrophic cancellation, denormal-vs-huge interaction) | `cfg_c17_mixed_binades` | [x] |
| C18 | `to_barycentric` | **log-uniform full-range** finite values, random sign, exponent `[-126, 127]` | `cfg_c18_log_uniform_finite` | [x] |
| C19 | `to_barycentric` | **near-degenerate**: `p3` = `p1 + (1+eps)*(p2-p1)` with random tiny `eps`, i.e. `denom` is a catastrophically-cancelled tiny value (or exactly `0`) | `cfg_c19_near_collinear` | [x] |
| C20 | `to_barycentric` | **signed zeros**: every one of the 8 slots independently `+0.0` or `-0.0` (all 256 combinations, exhaustive) | `cfg_c20_signed_zero_exhaustive` | [x] |
| C21 | `to_barycentric` | **`±inf` in exactly one slot**, other 7 slots randomized finite — all 8 slots × 2 signs | `cfg_c21_inf_single_slot` | [x] |
| C22 | `to_barycentric` | **QNaN in exactly one slot** (`0x7FC00000`), other 7 randomized finite — all 8 slots | `cfg_c22_qnan_single_slot` | [x] |
| C23 | `to_barycentric` | **SNaN in exactly one slot** (`0x7F800001`), other 7 randomized finite — all 8 slots (checks quieting to `0x7FC00001`) | `cfg_c23_snan_single_slot` | [x] |
| C24 | `to_barycentric` | **negative NaN** in exactly one slot (`0xFFC00000`, `0xFFFFFFFF`) — checks the sign bit survives; distinguishes a propagated NaN from the x86 "indefinite" `0xFFC00000` | `cfg_c24_negative_nan_single_slot` | [x] |
| C25 | `to_barycentric` | **two distinct NaN payloads in slots that meet in one `subss`** (`p1.x` and `p3.x`, `p1.y` and `p3.y`, `p1`/`p2`, `p1`/`p`) — the destination-operand-wins case | `cfg_c25_two_nan_operands` | [x] |
| C26 | `to_barycentric` | **NaN × inf mixtures**: random slots get NaN, others `±inf`, others finite (2-3 specials at once) | `cfg_c26_nan_inf_mixture` | [x] |
| C27 | `to_barycentric` | **fully random 32-bit patterns** in all 8 slots — every encoding class simultaneously (NaN, inf, subnormal, huge, `±0`) | `cfg_c27_random_bit_patterns` | [x] |
| C27b | `to_barycentric` | **exhaustive** 8-slot sweep over the 6-value pool `{+0, 1, +inf, QNaN, SNaN, 3}` — 6^8 = 1 679 616 cases, every pairing of those operand classes in every position | `cfg_c27b_pool_exhaustive_positive` | [x] |
| C27c | `to_barycentric` | **exhaustive** 8-slot sweep over `{-0, -1, -inf, -QNaN, min-subnormal, FLT_MAX}` — the negative / subnormal counterpart, another 1 679 616 cases | `cfg_c27c_pool_exhaustive_negative` | [x] |
| C27d | `to_barycentric` | randomized draws from the **full 30-value interesting pool** (all encoding classes, incl. `2^24`, `2^24+1`, `FLT_MAX`, `FLT_MIN`, `±1e30`) in every slot | `cfg_c27d_pool_random` | [x] |
| C28 | `to_barycentric` | **duplicate-vertex families** at random positions: `p1==p2`, `p1==p3`, `p2==p3`, `p1==p2==p3` (valid-input side of E1-E4; the `0 * inf` indefinite-QNaN path) | `cfg_c28_duplicate_vertices` | [x] |
| C29 | `to_barycentric` | **exactly collinear** vertices at random positions and random parameter `t` (`denom` exactly `+0`) | `cfg_c29_exact_collinear` | [x] |
| C30 | `to_barycentric` | ambient **MXCSR rounding mode** = round-down / round-up / round-toward-zero (non-default), with randomized finite inputs — catches any compile-time constant folding in the Rust that would ignore the dynamic rounding mode | `cfg_c30_rounding_modes` | [x] |
| C31 | `to_barycentric` | ambient **MXCSR FTZ + DAZ** enabled, with subnormal-producing inputs | `cfg_c31_ftz_daz` | [x] |
| C32 | `to_barycentric` | **repeat-call / statelessness**: the same input driven 3× interleaved with other inputs, in a different order, and from two threads — confirms there is no hidden state (matches the C, which has none) | `cfg_c32_stateless_interleaved` | [x] |
| C33 | `to_barycentric` | **ABI shape**: `lm_vec2` size/align/offsets, and the register-level calling convention (result read back as a packed 8-byte pair, so a swapped `x`/`y` return would be caught) | `abi_struct_layout` + every row above | [x] |
| C34 | (harness) | both `.so` files really are two **distinct** files, both `dlopen`ed, both resolving `to_barycentric` — guards against accidentally comparing a library with itself | `harness_loads_both_libraries` | [x] |

## Feature combinations

```sh
$ grep -A20 '^\[features\]' Cargo.toml
[features]
default = []
```

The crate declares no optional features and no optional dependencies, so the
complete set of valid feature combinations is:

| # | combination | `cargo check` / `cargo test` invocation |
|---|-------------|------------------------------------------|
| F1 | `default` (empty feature set) | `cargo check --offline` |
| F2 | `--no-default-features` (identical to F1, empty set) | `cargo check --offline --no-default-features` |

Both are verified by `./verify_all.sh`, which additionally re-runs the whole
differential suite against **both** the `debug` (unoptimized) and `release`
(optimized) Rust `.so`, since optimization level is the axis that could
plausibly perturb NaN operand selection even though the feature set cannot.

## Phase B status

All 34 rows above are checked `[x]`: each has a differential test that calls
BOTH `.so` files through `libloading` and compares the returned `lm_vec2`
bit-for-bit, over many randomized inputs (fixed seed `0x2B7E151628AED2A6`) or an
exhaustive sweep where the row is finite.

```
$ ./verify_all.sh
== Phases B + C: differential suite ==
  PASS features='<empty>' vs release .so  (57 tests)
  PASS features='<empty>' vs debug   .so  (57 tests)
  PASS default features vs release .so  (57 tests)
  PASS default features vs debug   .so  (57 tests)

$ cargo test --offline --release --test phase_b_configs
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Soak run (`./verify_all.sh 25`, i.e. 25x the per-row iteration counts, ~100
million input vectors compared bit-for-bit per configuration): **ALL CHECKS
PASSED**, 27 s wall.

### Why these rows and not just "call it once"

The rows that actually distinguish a correct translation from a plausible-looking
one are C25/C26/C27x — two or more NaN operands meeting inside a single SSE
instruction. When `verify_all.sh` substitutes a mutant library whose `lm_dot2`
is written in the naive commuted form `a.y*b.y + a.x*b.x`, exactly these rows
fail (8 rows across both phases) while all the ordinary-geometry rows
(C1-C19) and even the *single*-NaN rows (C21-C24) still pass. Rows driven with
one scalar input, or with only finite values, would have shipped the bug.

### Root cause the rows are pinning down

`c_src` is built with no `CMAKE_BUILD_TYPE`, i.e. `-O0`, and GCC's codegen for
`lm_dot2` does **not** use the left operand as the SSE destination uniformly:

```text
mulss  %xmm0,%xmm1     ; a.x * b.x  -> destination a.x   (LEFT)
mulss  %xmm2,%xmm0     ; b.y * a.y  -> destination b.y   (RIGHT)
addss  %xmm1,%xmm0     ; yy + xx    -> destination yy    (RIGHT)
```

Since `mulss`/`addss`/`subss`/`divss` return the **destination** operand
(quieted) when it is NaN, and only otherwise the source, the destination choice
decides which NaN payload survives. `src/lib.rs` pins each destination with
inline asm, and the Rust release build was disassembled and confirmed to make
the same choice for all 9 `subss`, 18 `mulss`, 5 `addss` and 1 `divss`.

## The one axis outside the library's control: the C build's optimization level

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the prescribed build
command compiles at `-O0`. That is the reference, and `src/lib.rs` matches it
bit-for-bit.

This is worth stating explicitly because the C library's *own* output is not
invariant across optimization levels for NaN inputs. Building the same
`c_src` with `-DCMAKE_BUILD_TYPE=Release` and diffing the two C `.so` files
against each other with this very harness:

```
$ C_SO_PATH=<O0 build> RUST_SO_PATH=<O3 build> cargo test --release --test phase_c_errors err_e14
MISMATCH [E14 p1.x=0x7fc01234, p3.x=0x7fdeadbe]
  inputs : p1=(NaN|0x7fc01234, 0e0|0x00000000)
  C(-O0) : (NaN|0x7fc01234, NaN|0x7fdeadbe)
  C(-O3) : (NaN|0x7fc01234, NaN|0x7fc01234)
```

GCC re-associates the products and picks different SSE destination operands at
`-O3`, so a different NaN payload survives. The two C builds disagree with each
other, hence **no** single Rust binary can be bit-identical to both. The Rust
reproduces the build the prescribed command produces (`-O0`).

The test `reference_c_build_is_unoptimized` asserts this precondition explicitly,
so if the C `.so` is ever built with optimization the suite fails with a
pointed diagnostic ("rebuild the C `.so`") instead of silently blaming the Rust.
