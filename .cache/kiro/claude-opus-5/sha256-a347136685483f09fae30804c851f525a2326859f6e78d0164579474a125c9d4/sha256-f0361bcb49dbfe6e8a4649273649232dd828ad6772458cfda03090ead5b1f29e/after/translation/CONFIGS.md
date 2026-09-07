# Phase A.3 — Configuration-surface table

## Mechanical derivation of the axes

**Runtime options / modes / flags:** none. `c_src/include/lib.h` declares exactly
one function and one struct; there is no context object, no init call, no
setter, no global. `grep` for `if`/`switch`/`#ifdef` over `c_src` returns
nothing (see `ERRORS.md`), so the C takes **no** data-dependent branch — there
is literally one code path.

**Cargo feature axes:** none. `translation/Cargo.toml` has no `[features]`
section, so the only feature combination that exists is the default (empty) one.
Verified with:

```sh
grep -n '^\[features\]' translation/Cargo.toml   # no output
cargo metadata --no-deps --format-version 1 | python3 -c \
  'import json,sys; print(json.load(sys.stdin)["packages"][0]["features"])'   # {}
```

**Binary executable axis:** none. `c_src/CMakeLists.txt` contains only
`add_library(... SHARED src/lib.c)` — no `add_executable` — and
`translation/Cargo.toml` declares only `crate-type = ["cdylib"]` with no
`[[bin]]` and no `src/main.rs`. There is therefore no driver whose stdout could
be compared; that completion-gate item is not applicable.

**Public entry points (the FULL set, lowest-level included):** the dynamic symbol
table exposes exactly one, `to_barycentric` (see `SYMBOLS.md`). The three
lower-level primitives `lm_v2`, `lm_sub2`, `lm_dot2` are `static`, hence not
callable across the `.so` boundary in either implementation — they can only be
exercised *through* `to_barycentric`. The rows below are constructed so that
each lower-level primitive is driven into its interesting states via the one
reachable entry point: rows that zero a component exercise `lm_sub2`'s
cancellation, rows that null out a vector exercise `lm_dot2`'s zero/sign
handling, and rows 26–33 exercise the reciprocal-and-multiply tail.

**Input-shape axes that remain** (the cross-product below is pruned to the
combinations the arithmetic actually distinguishes):

* *A — geometry*: non-degenerate CCW / non-degenerate CW / right-angled /
  needle-thin (near-degenerate) / exactly collinear / two vertices coincident /
  all three coincident.
* *B — position of `p`*: strictly inside / outside / on a vertex / on an edge /
  far outside / exactly at `p1`.
* *C — magnitude regime*: unit-scale / large (≈1e18, dot products overflow) /
  tiny (≈1e-20, dot products underflow to subnormal or zero) / mixed
  large-and-tiny in the same triangle / maximal finite / minimal subnormal.
* *D — float class of components*: normal / zero (`+0.0`, `-0.0`) / subnormal /
  infinity (`+inf`, `-inf`) / quiet NaN / signalling NaN / negative NaN /
  multiple *distinct* NaN payloads in one call.
* *E — exactness*: coordinates exactly representable in binary32 (integers,
  powers of two) vs. requiring rounding at every step, since operand order and
  rounding interact.

## Rows

Each row is exercised with **many** randomized inputs (fixed seed `0x5EED_1234`
plus per-row salt) unless the row's definition pins the inputs exactly.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `to_barycentric` | A=non-degenerate CCW, B=inside, C=unit, D=normal, E=rounding — random unit-scale triangles, `p` a random convex combination | [x] |
| 2 | `to_barycentric` | A=non-degenerate CW (reversed winding, negative denominator sign path), B=inside, C=unit, D=normal | [x] |
| 3 | `to_barycentric` | A=non-degenerate, B=outside (random `p` far from the triangle, `u`/`v` out of `[0,1]`) | [x] |
| 4 | `to_barycentric` | A=non-degenerate, B=on a vertex (`p == p2`, `p == p3`) — exact `0`/`1` barycentrics | [x] |
| 5 | `to_barycentric` | A=non-degenerate, B=on an edge (`p` = midpoint of each of the three edges) | [x] |
| 6 | `to_barycentric` | A=right-angled (`dot01 == 0` exactly, orthogonal `v0`/`v1`), B=inside | [x] |
| 7 | `to_barycentric` | A=needle-thin near-degenerate (`v1 = v0 * (1 ± 1e-6)`), B=inside — catastrophic cancellation in `denom` | [x] |
| 8 | `to_barycentric` | A=exactly collinear, B=inside — `denom` exactly `±0.0`, division by zero | [x] |
| 9 | `to_barycentric` | A=two vertices coincident (`p1==p2`, `p1==p3`, `p2==p3` — all three variants) | [x] |
| 10 | `to_barycentric` | A=all three vertices coincident | [x] |
| 11 | `to_barycentric` | E=exactly representable: all components small integers in `[-16,16]` — every intermediate exact, no rounding anywhere | [x] |
| 12 | `to_barycentric` | E=powers of two only (`±2^k`, `k ∈ [-30,30]`) — exact products, exponent-range stress | [x] |
| 13 | `to_barycentric` | C=large ≈`1e18`, D=normal — `dot00*dot11` overflows to `+inf`, `inf-inf` NaN path | [x] |
| 14 | `to_barycentric` | C=tiny ≈`1e-20`, D=normal — products underflow to subnormal then to `+0.0` | [x] |
| 15 | `to_barycentric` | C=mixed: one vertex at `1e20`, another at `1e-20` in the same call | [x] |
| 16 | `to_barycentric` | C=maximal finite: components drawn from `{±f32::MAX, ±f32::MIN_POSITIVE}` | [x] |
| 17 | `to_barycentric` | C=minimal subnormal: components drawn from `{±1e-45 (0x1), ±0x7fffff}` (DAZ/FTZ detector) | [x] |
| 18 | `to_barycentric` | D=zero: every component drawn from `{+0.0, -0.0}` — full 2^8 = 256 exhaustive sign combinations | [x] |
| 19 | `to_barycentric` | D=infinity: one random component replaced by `+inf` or `-inf`, rest random normals — all 8 positions × 2 signs | [x] |
| 20 | `to_barycentric` | D=infinity, several components infinite simultaneously (random mask over the 8 slots) | [x] |
| 21 | `to_barycentric` | D=quiet NaN in one random position, rest random normals — all 8 positions | [x] |
| 22 | `to_barycentric` | D=signalling NaN (`0x7f800001`) in one random position — quieting behaviour | [x] |
| 23 | `to_barycentric` | D=negative NaN (`0xffc00000`, `0xff800001`) in one random position — NaN sign propagation | [x] |
| 24 | `to_barycentric` | D=**multiple distinct NaN payloads** across different components (e.g. `p3.x=0x7fc0_00aa`, `p2.x=0x7fc0_00bb`) — operand-order / first-operand-wins discriminator for every commutative `mulss`/`addss` in the call graph | [x] |
| 25 | `to_barycentric` | D=fully unstructured: all 32 bits of all 8 components uniformly random (covers every float class jointly, including NaN×NaN×inf mixtures) | [x] |
| 26 | `to_barycentric` | `lm_sub2` cancellation path: `p2`, `p3`, `p` chosen so that each of `v0`,`v1`,`v2` has an exactly-cancelling component (`a.x == b.x`) | [x] |
| 27 | `to_barycentric` | `lm_dot2` sign-cancellation path: `v0`/`v1` chosen so `x`- and `y`-products are exact negatives, `dot01` cancels to `±0.0` | [x] |
| 28 | `to_barycentric` | `lm_dot2` rounding path: components with 24-bit-full mantissas so each product needs rounding and the add is inexact | [x] |
| 29 | `to_barycentric` | reciprocal tail: `denom` an exact power of two (`invDenom` exact) vs. non-power-of-two (`invDenom` inexact) — proves `1.0f/d` then multiply, not a direct divide | [x] |
| 30 | `to_barycentric` | reciprocal tail: `denom` so small that `1.0f/denom` overflows to `+inf` (`denom ≈ 1e-40`, subnormal) | [x] |
| 31 | `to_barycentric` | reciprocal tail: `denom` so large that `1.0f/denom` underflows to `+0.0`/subnormal (`denom ≈ 1e38`) | [x] |
| 32 | `to_barycentric` | numerator exactly `±0.0` with non-zero `invDenom` — signed-zero result | [x] |
| 33 | `to_barycentric` | numerator `±inf` with `invDenom == ±0.0` — `inf * 0 = NaN` in the tail multiply | [x] |
| 34 | `to_barycentric` | argument-slot permutation sweep: the same four vectors passed in all 24 orderings, confirming no argument mix-up between `p1`,`p2`,`p3`,`p` | [x] |
| 35 | `to_barycentric` | struct/ABI shape: `size_of`/`align_of` parity, and calling with the XMM upper 64 bits dirtied | [x] |
| 36 | `to_barycentric` | purity/reentrancy: the same input called repeatedly and from multiple threads returns identical bits (no hidden state) | [x] |
| 37 | `to_barycentric` | default feature set (the only feature combination that exists — no `[features]` in `Cargo.toml`); asserted programmatically so a future feature addition fails the test | [x] |

## Gate

- [x] All 37 rows pass across randomized inputs (`cargo test --release`, and
      again with `--no-default-features`, which is identical here).

## Appendix A — the C build's optimisation level is part of the configuration

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the documented build
command produces an unoptimised (`-O0`) reference library. That matters, because
GCC's choice of *destination operand* for the commutative `mulss`/`addss`
instructions differs between `-O0` (no inlining, one instruction per C operator)
and `-O2` (everything inlined and operands re-canonicalised), and on x86 an SSE
binary operation returns its destination operand when both operands are NaN. The
translation matches the `-O0` stream, which is what the specified build command
produces.

The blast radius of that choice was measured rather than assumed. Compiling the
unmodified `c_src/src/lib.c` at both `-O0` and `-O2` in a scratch directory and
comparing 5,000,000 uniformly random inputs through `dlopen`:

```
n=5000000  -O0 vs -O2 differ=3649  of which ZERO-NaN-inputs=0
n=5000000  -O0 vs -O2 differ=3649  of which NOT-pure-NaN-payload-differences=0
```

So the two optimisation levels agree bit-for-bit on **every** input that contains
no NaN, and all 3,649 divergences are differences in the payload/sign of a NaN
*result*. The translation is therefore bit-identical to the C for the specified
build at every input, and bit-identical to an `-O2` build at every NaN-free
input.

Struct layout was confirmed against a real C compile of the unmodified header
(`sizeof(lm_vec2) == 8`, `_Alignof(lm_vec2) == 4`, `offsetof(x) == 0`,
`offsetof(y) == 4`), matching the Rust `size_of`/`align_of` assertions in rows 35
and `ERRORS.md` row 19.

## Appendix B — coverage was validated by mutation testing

Passing tests only prove coverage if the tests can fail. Every commutative
arithmetic instruction in the assembly core was mutated in turn (its destination
and source operands swapped, via a `movaps`/op/`movaps` triple that preserves
register allocation), the library rebuilt, and the whole suite re-run. 23
instructions were mutated:

* **4** are literal squarings (`v0.x*v0.x`, `v0.y*v0.y`, `v1.x*v1.x`,
  `v1.y*v1.y`, `dot01*dot01`) where operand order is semantically meaningless —
  the mutation is a no-op, and correctly produced no failure.
* **13** are observable, and **all 13 were caught** by the suite (rows
  `cfg21`–`cfg25`, `err12`–`err15` and the heavy fuzz are what catch them).
* **6** were not caught: the three inside `dot01`, `dot00*dot11`,
  `dot01*dot12` and `dot01*dot02`. Each of these was then fuzzed against the C
  for 50,000,000 further cases and found **indistinguishable**, because
  `dot01`'s NaN payload and `invDenom`'s NaN payload are structurally always
  masked: whenever `dot01` is a propagated NaN so is `dot00` or `dot11`, and both
  then reach the subtraction as its *destination* operand, whose payload wins.
  These are unobservable through the public API, not test blind spots. The
  translation reproduces the C's operand order for them regardless.

## Appendix C — how to run the whole matrix

```sh
cd translation
../c_src/build/... # build the C .so first (see SYMBOLS.md)
./run_matrix.sh    # every feature combination x {dev, release}
FUZZ_ITERS=25000000 cargo test --release --test symbol_parity   # deeper fuzz
```
