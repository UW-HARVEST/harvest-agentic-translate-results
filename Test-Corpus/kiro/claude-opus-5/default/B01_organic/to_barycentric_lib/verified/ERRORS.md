# ERRORS.md — error-surface table (Phase A, gate for Phase C)

## Mechanical derivation

Every rejection mechanism was grepped for across the whole of `c_src`
(`src/lib.c`, `include/lib.h` — the only files):

```sh
grep -nE 'return|assert|NULL|-1|error|if|switch|#if|enum|malloc|free' -i \
    c_src/src/lib.c c_src/include/lib.h
```

Complete output:

```
src/lib.c:5:    return v;
src/lib.c:9:    return lm_v2(a.x - b.x, a.y - b.y);
src/lib.c:13:    return a.x * b.x + a.y * b.y;
src/lib.c:28:    return lm_v2(u, v);
```

All four hits are unconditional value returns. Therefore, mechanically:

| mechanism | count in `c_src` |
|-----------|------------------|
| `RETURN_ERROR`-style macros | 0 |
| `return -1` / negative sentinel | 0 |
| `return NULL` / pointer sentinel | 0 |
| error enums / status codes | 0 |
| `assert` / `static_assert` | 0 |
| explicit range checks (`if`, `switch`, ternary) | 0 |
| null-pointer checks | 0 |
| `#if` / `#ifdef` compile-time branches | 0 |
| min/max clamping constants | 0 |
| allocation (`malloc`/`free`) that can fail | 0 |
| pointer parameters anywhere in the public API | 0 |
| integer or enum parameters anywhere in the public API | 0 |

**The C library has no error surface.** `to_barycentric` takes four
by-value `lm_vec2` structs (eight `float`s), is branch-free, and returns a
by-value `lm_vec2` for *every* input in the domain. There is no input it can
reject: any of the 2^256 argument bit patterns is accepted and produces a
result. Consequently there is no error code or sentinel to compare — the
"same rejection" requirement degenerates into "same returned bit pattern".

The generic C-API boundaries the task lists are structurally inapplicable
here and this is a property of the signature, not an assumption:

* **null pointers** — impossible: no parameter or return value is a pointer.
* **zero / oversized lengths** — impossible: no length, size or count parameter.
* **out-of-range enum values across FFI** — impossible: no `enum`, `int` or
  other discrete-domain parameter; `float` has no invalid bit patterns (every
  32-bit word is a valid `float`, including all NaN encodings).

## Rows

What remains are the *degenerate numeric conditions* — the places where the
arithmetic itself produces a non-finite result instead of a normal one. These
are the only "abnormal input" rows the C actually distinguishes, so they are
the Phase C rows. `expected C result` is the bit-exact value the compiled
`c_src` `.so` returns; each row is asserted bit-for-bit (`to_bits()`) against
the C library, not merely "both non-finite".

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| E1 | `to_barycentric` | fully degenerate triangle: `p1 == p2 == p3` ⇒ `v0 = v1 = 0` ⇒ all dots `0` ⇒ `denom == 0`, numerators `0` ⇒ `invDenom = 1/0 = +inf`, `u = v = 0 * inf` | `(NaN, NaN)` — no rejection, IEEE default result; bit pattern compared exactly | [x] |
| E2 | `to_barycentric` | collinear/degenerate triangle with `p2 == p1` (`v1 == 0`) but `p3 != p1` ⇒ `dot11 = dot01 = dot12 = 0`, `denom = 0`, `u`-numerator `0`, `v`-numerator `0` | `(NaN, NaN)`, bit-exact | [x] |
| E3 | `to_barycentric` | degenerate with `p3 == p1` (`v0 == 0`) ⇒ `dot00 = dot01 = dot02 = 0`, `denom = 0` | `(NaN, NaN)`, bit-exact | [x] |
| E4 | `to_barycentric` | collinear non-coincident vertices (`v0`, `v1` parallel, both non-zero) ⇒ `dot00*dot11 - dot01*dot01 == 0` exactly ⇒ `invDenom = ±inf` with a *non-zero* numerator | `(±inf, ±inf)` per IEEE sign rules, bit-exact incl. sign of zero/inf | [x] |
| E5 | `to_barycentric` | near-degenerate: `denom` underflows to `0` (or to a subnormal) although the vertices are distinct — e.g. coordinates of magnitude ~1e-25 so the squared dots flush to zero | `inf`/`NaN`/subnormal exactly as C, bit-exact | [x] |
| E6 | `to_barycentric` | `denom` overflows to `+inf` (coordinates ~1e20 so `dot00*dot11` overflows binary32) ⇒ `invDenom = 1/inf = 0`, numerator may be `inf - inf = NaN` | bit-exact `NaN`/`±0.0` mix as C | [x] |
| E7 | `to_barycentric` | any input component is `+inf` or `-inf` ⇒ `inf - inf = NaN` propagation through the dots | bit-exact `NaN` (incl. sign/payload) as C | [x] |
| E8 | `to_barycentric` | any input component is a **quiet** NaN (`0x7FC00000`), incl. distinct payloads in several components simultaneously | bit-exact NaN propagation: on SSE a binary op with two NaN operands returns the *destination* operand, so the exact payload depends on GCC's operand order — asserted bit-for-bit | [x] |
| E9 | `to_barycentric` | any input component is a **signalling** NaN (`0x7F800001`) — quieted by the first arithmetic op | bit-exact quieted NaN (`0x7FC00001`-class) as C | [x] |
| E10| `to_barycentric` | negative zero inputs (`-0.0`) producing `-0.0` intermediates ⇒ sign of zero in the result | bit-exact, `-0.0 != +0.0` under `to_bits()` | [x] |
| E11| `to_barycentric` | subnormal input components (`1e-45`, `0x00000001`) ⇒ subnormal/flush-to-zero intermediates | bit-exact as C (no FTZ: default MXCSR) | [x] |
| E12| `to_barycentric` | maximum-magnitude finite inputs (`±FLT_MAX`, `±3.4028235e38`) ⇒ overflow to `±inf` inside the dots | bit-exact `inf`/`NaN` as C | [x] |
| E13| `to_barycentric` | `p` far outside the triangle (barycentric coords ≫ 1 or ≪ 0) — "out of range" for the *conceptual* contract; the C does **not** check or clamp it | normal finite `(u, v)` outside `[0,1]`, bit-exact — confirms C performs **no** range rejection | [x] |
| E14| `to_barycentric` | every argument is an arbitrary uniformly-random 32-bit word per component (unrestricted bit-pattern fuzz, incl. all NaN/inf/subnormal classes mixed) | whatever C returns, bit-exact; asserts the absence of any rejected input | [x] |

All rows are covered by `translation/tests/differential.rs`
(`phase_c_*` tests) and pass. `E14` additionally fuzzes 200 000 fully random
bit patterns, which subsumes the "one step past a valid range" requirement
because the `float` domain has no invalid range to step past.
