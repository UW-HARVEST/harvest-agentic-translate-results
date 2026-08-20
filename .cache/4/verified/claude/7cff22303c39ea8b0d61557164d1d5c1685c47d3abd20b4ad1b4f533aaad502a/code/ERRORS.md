# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep of every rejection construct

```
$ grep -nE "return|assert|NULL|errno|ERROR|goto|exit|abort" c_src/src/lib.c c_src/include/lib.h
(no matches)

$ grep -nE "if *\(|\?|<|>|==|!=" c_src/src/lib.c
8:        if (src[0] < src[1]) {
15:            0.5f * (dy2 + dx2 + sqrtf((((0) > (sqd)) ? (0) : (sqd))));
7:    for (i = 0; i < count; i++) {
25:            ... same ternary as line 15 ...
```

So `tfm` has **no error return, no error enum, no sentinel, no `assert`, no
null check, and no `errno` use**. It returns `void`. Every "rejection" it can
perform is therefore one of:

* a *control-flow* rejection — a guard that steers execution away from a
  computation (`i < count`, `src[0] < src[1]`, `0 > sqd`), or
* an *IEEE-754* rejection — an invalid/inexact operation whose "error result"
  is a specific NaN / infinity bit pattern that the caller observes in `dest`.

Both classes are enumerated exhaustively below. The "expected C result" column
is what the reference build (`c_src/CMakeLists.txt`, i.e. `gcc` with no
`CMAKE_BUILD_TYPE` and no optimization flags, glibc `sqrtf`) actually produces,
verified by differential test.

The only min/max constant in the file is the inlined `MAX(0, sqd)` on lines 15
and 25; the only literal constants are `2.0f`, `4.0f`, `0.5f`, `0`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `tfm` | `count == 0` (loop guard `i < count` fails immediately) | returns without reading `src` or writing `dest`; `dest` untouched |
| 2 | `tfm` | `count < 0` (e.g. `-1`) | same as #1 — loop body never runs, `dest` untouched (no wraparound to a huge unsigned count) |
| 3 | `tfm` | `count == INT_MIN` (most negative `int`) | same as #1 — `dest` untouched |
| 4 | `tfm` | `count <= 0` **and** `dest == NULL` **and** `src == NULL` | no dereference at all; returns cleanly, no crash |
| 5 | `tfm` | `count <= 0` and only `src == NULL` (`dest` valid) | no dereference; `dest` untouched |
| 6 | `tfm` | `count <= 0` and only `dest == NULL` (`src` valid) | no dereference; returns cleanly |
| 7 | `tfm` | branch guard `src[0] < src[1]` **false because `src[0] > src[1]`** | takes the `else` arm: `dy2=src[0]`, `dx2=src[1]`, `dest[0]=dxy`, `dest[1]=dx2-lambda` |
| 8 | `tfm` | branch guard false **because `src[0] == src[1]`** (`<` is strict) | takes the `else` arm |
| 9 | `tfm` | branch guard false **because `src[0]` is NaN** (`comiss` unordered ⇒ `jbe` taken) | takes the `else` arm |
| 10 | `tfm` | branch guard false **because `src[1]` is NaN** (unordered) | takes the `else` arm |
| 11 | `tfm` | branch guard false **because both are NaN** (unordered) | takes the `else` arm |
| 12 | `tfm` | branch guard with `src[0] == -0.0f`, `src[1] == +0.0f` (`-0.0 < +0.0` is false) | takes the `else` arm |
| 13 | `tfm` | range check `(0 > sqd) ? 0 : sqd` **rejects** `sqd`: `sqd < 0` (ordered) | `sqrtf` is called with `+0.0f` (`.LC1 == 0x00000000`), never with a negative; `lambda == 0.5f*(dy2+dx2)` |
| 14 | `tfm` | range check **does not reject** `sqd == NaN` (`0 > NaN` is false, unlike `fmaxf`) | the NaN is passed to `sqrtf`, which returns it quieted; NaN propagates into `dest` |
| 15 | `tfm` | range check **does not reject** `sqd == -0.0f` (`0 > -0.0` is false, so no normalization — unlike `fmaxf`) | **Proven unreachable** through the public API: the final addend `(4.0f*dxy)*dxy` always has a non-negative sign (`sign(4*dxy) == sign(dxy)`), so `+0.0f + x` can never yield `-0.0f`. Test asserts 0 hits over 2M randomized inputs + the exhaustive specials sweep, and differentially checks that whole space |
| 16 | `tfm` | range check boundary `sqd == +0.0f` (`0 > 0` false) | `sqrtf(+0.0f) == +0.0f` |
| 17 | `tfm` | range check boundary `sqd == -FLT_MIN` (smallest magnitude negative subnormal, `0x80000001`) | clamped to `+0.0f` — one step past the accepted range |
| 18 | `tfm` | range check boundary `sqd == +FLT_MIN` subnormal (`0x00000001`) | accepted, `sqrtf` of a subnormal |
| 19 | `tfm` | invalid op: `dy2*dy2` overflows (`\|dy2\| > ~1.8e19`) | `+inf` (overflow, not an error return) |
| 20 | `tfm` | invalid op: `inf - inf` inside `sqd` (both the `dy2*dy2` and the `2*dx2*dy2` term overflow to `+inf`) | x86 "real indefinite" QNaN `0xffc00000` (**sign bit set**), *not* `0x7fc00000` |
| 21a | `tfm` | invalid op: `0.0f * inf` inside `2.0f*dx2*dy2` (`dx2 == ±0`, `dy2 == ±inf`, or the reverse) | `0xffc00000` (x86 indefinite) |
| 21b | `tfm` | invalid op: `0.0f * inf` inside `4.0f*dxy*dxy` | **Proven unreachable**: the two factors of `(4.0f*dxy)*dxy` satisfy `\|4*dxy\| >= \|dxy\|`, so they cannot be `0` and `inf` at once. 0 hits over the whole search |
| 22 | `tfm` | invalid op: `inf + (-inf)` in `dy2 + dx2` | `0xffc00000` |
| 23 | `tfm` | `sqrtf` domain error: a negative argument | **Proven unreachable** — the clamp (#13) forbids strictly negative arguments and `-0.0f` is unreachable (#15), so the only non-`>= 0` argument `sqrtf` ever receives is NaN (#14). glibc's `__math_invalidf` path is dead code here; 0 hits over the whole search |
| 24 | `tfm` | signaling NaN input (`0x7fbfffff` / `0xffbfffff`, quiet bit clear) reaches `mulss`/`addss`/`subss`/`sqrtf` | operand is **quieted** (`\| 0x00400000`), sign and remaining payload preserved |
| 25 | `tfm` | two NaN operands reach one SSE op | the **destination** operand's payload wins (SSE rule), quieted; see `nan_result()` |
| 26 | `tfm` | `lambda` infinite ⇒ `dx2 - lambda` where `dx2` is infinite with the same sign | **Proven unreachable**: `lambda == -inf` requires `dy2 + dx2 == -inf` with a finite `root`, but an infinite `dx2` only occurs on the else arm and always forces `sqd` to `NaN` or `+inf`, hence `lambda` to `NaN`; and `dx2 == +inf` forces `dy2 == +inf`, hence `sqd == inf - inf == NaN`. 0 hits over the whole search |
| 27 | `tfm` | `dest` aliases `src` (same buffer) — no restrict, no overlap check | all three `src[i]` are read into locals **before** either `dest[i]` write, per iteration; later iterations read the already-overwritten prefix |
| 28 | `tfm` | out-of-range `int` values for `count` passed across FFI (`count` is `int`, so any 32-bit pattern is a legal input) | `count = 0x7fffffff` etc. is *not* rejected; only the sign matters for a no-op. Tested with `count <= 0` patterns only, since positive huge counts would read out of bounds |

**All 28 rows have a passing differential test** in `tests/phase_c_errors.rs`
(one `#[test] fn rowNN_...` per row; row 21 is covered by the single
`row21_zero_times_inf` test, which handles both sub-rows 21a and 21b).

Rows 15, 21b, 23 and 26 describe conditions the C source *writes* but that
cannot be reached through the public API. Their tests use
`assert_unreachable()`, which

1. asserts 0 hits over 2M+ randomized inputs plus the exhaustive 24x24x24
   specials cross-product, and
2. differentially compares C vs Rust on that entire search space anyway,

so the row is proven, not assumed. The full reachability map is regenerated and
re-asserted by `tests/reachability.rs`, which fails if any row's
reachable/unreachable classification ever goes stale.

Rows deliberately **not** listed, with justification:

* *unaligned `dest`/`src`* — not a rejection: `movss` has no alignment
  requirement, and the C never checks. Covered as a valid configuration in
  `CONFIGS.md` row 24 instead.
* *out-of-range enum values* — the API declares no `enum` and no flag
  parameter; the only non-pointer parameter is `int count`, covered by rows
  1–3 and 28.
* *oversized lengths* — a `count` larger than the buffers is out-of-bounds
  access, which the C does not check and which is UB for both languages; it is
  not a rejection the C performs, so it has no defined result to compare.
