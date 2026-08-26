# ERRORS.md — Phase C: error-surface table

Derived mechanically from `c_src/src/lib.c` (48 lines, one function) and
`c_src/include/lib.h`.

## Mechanical grep of every rejection-ish construct

```
$ grep -nE 'return|assert|NULL|errno|goto|exit|abort|ERROR|-1|if *\(|else|#if|#def' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:10:    if (s == 0) {
c_src/src/lib.c:14:        return;            <-- the ONLY return statement
c_src/src/lib.c:19:    if (h >= 0.0f && h < 60.0f) {
c_src/src/lib.c:23:    } else if (h >= 60.0f && h < 120.0f) {
c_src/src/lib.c:27:    } else if (h < 120.0f && h < 180.0f) {
c_src/src/lib.c:31:    } else if (h >= 180.0f && h < 240.0f) {
c_src/src/lib.c:35:    } else if (h >= 240.0f && h < 300.0f) {
c_src/src/lib.c:39:    } else if (h >= 300.0f && h < 360.0f) {
c_src/src/lib.c:43:    } else {
```

Facts established by that grep:

* `hsl_to_rgb` returns **`void`** — there is no error code, no sentinel, no
  `errno` use, no output "status" parameter.
* There is **no** `assert`, no `NULL` check, no explicit range/bounds check, no
  `RETURN_ERROR`-style macro, no error enum, and no `MIN`/`MAX` constant.
* The function unconditionally reads `src[0..3]` and unconditionally writes
  `dest[0..3]` (the `s == 0` path writes all three too, then returns early).

So the library has **no explicit error surface**. What it does have is a set of
*implicit rejection paths*: input classes for which the arithmetic pipeline is
bypassed or falls through to a fallback. Those are the rows below — each is a
distinct way the C "rejects" (refuses to compute a real conversion for) an
input, and each must be reproduced bit-for-bit. `bits(v)` denotes the raw
`u32` bit pattern; comparison in every test is on bit patterns, so `-0.0` is
distinguished from `+0.0` and NaN payload/sign is significant.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `hsl_to_rgb` | `src[1] == 0.0f` (`s = +0.0`) — line 10 early-out, achromatic bypass | `dest[0..3] = l` (bit-exact copy of `src[2]`), `c/m/x` never computed, `fmodf` never called; returns before the hue chain |
| 2  | `hsl_to_rgb` | `src[1] == -0.0f` — `-0.0f == 0` is **true** in C, so `-0.0` also takes the line-10 early-out | identical to row 1: `dest[0..3] = l` |
| 3  | `hsl_to_rgb` | `src[1] = NaN` (any payload) — `s == 0` is **false** for NaN, so the early-out is *not* taken | full chain runs with `c = (1-|2l-1|) * NaN` → NaN; branch selected by `h` alone |
| 4  | `hsl_to_rgb` | `src[0] = NaN` (any payload/sign, quiet or signalling) — every `comiss` against `h` is unordered, so **all six** `if`/`else if` guards are false | falls into the final `else` (line 43): `dest[0] = dest[1] = dest[2] = m` |
| 5  | `hsl_to_rgb` | `src[0]` in `[120.0f, 180.0f)` — the third guard is `h < 120.0f && h < 180.0f` (**not** `h >= 120.0f`), so 120..180 matches *no* guard | falls into the final `else`: `dest[0..3] = m` (the cyan/green sector is silently unreachable — a real C bug that must be replicated) |
| 6  | `hsl_to_rgb` | `src[0] >= 360.0f` (e.g. `360.0`, `720.0`, `+inf`, `f32::MAX`) — out of the documented hue range | final `else`: `dest[0..3] = m` |
| 7  | `hsl_to_rgb` | `src[0] < 0.0f` (e.g. `-1.0`, `-0.0`? no — `-0.0 >= 0.0` is true, so strictly negative such as `-1e-45`, `-30.0`, `-400.0`, `-inf`) — first two guards fail, third guard `h < 120 && h < 180` is **true** | takes the *third* branch: `dest[0] = m`, `dest[1] = c + m`, `dest[2] = x + m` — negative hue is **not** rejected, it silently aliases the 120..180 sector body |
| 8  | `hsl_to_rgb` | `src[0] = -inf` (with `s != 0`) — row 7's branch, and `h/60 = -inf` so `fmodf(-inf, 2.0f)` hits libm's *domain-error* path | glibc `fmodf` returns `(x*y)/(x*y)` for infinite `x` → NaN; `x = (1 - \|NaN-1\|) * c` = NaN; `dest[2] = x + m` = NaN with glibc's exact bit pattern |
| 9  | `hsl_to_rgb` | `src[0] = +inf` — `+inf` fails all guards | final `else`: `dest[0..3] = m` (the `fmodf(+inf,2)` NaN is computed into `x` but discarded) |
| 10 | `hsl_to_rgb` | `src[2] = NaN` (`l` NaN, `s != 0`) — no check on `l` | `c`, `m` both NaN; result is NaN in every component the selected branch writes |
| 11 | `hsl_to_rgb` | `src[2] = ±inf` (`l` out of `[0,1]`, `s != 0`) — `2*l - 1 = ±inf`, `1 - inf = -inf`, `c = -inf * s` | `c = ∓inf·s`, `m = l - 0.5c` may be `inf - inf` = NaN; propagates through the selected branch |
| 12 | `hsl_to_rgb` | `src[2]` outside `[0,1]` but finite (e.g. `-5.0`, `2.0`) — no clamping/validation | negative/`>1` RGB components are produced and written verbatim; **no** rejection |
| 13 | `hsl_to_rgb` | `src[1]` outside `[0,1]` but finite and non-zero (e.g. `-1.0`, `7.0`) — no clamping/validation | out-of-gamut components written verbatim; **no** rejection |
| 14 | `hsl_to_rgb` | `src[1] = ±inf` (`s` infinite) | `c = (1-\|2l-1\|)·±inf` → `±inf` or NaN (when `1-\|2l-1\|` is `±0`, `0·inf` = NaN); propagates |
| 15 | `hsl_to_rgb` | `src[0]` exactly on a guard boundary: `0.0`, `60.0`, `120.0`, `180.0`, `240.0`, `300.0`, `360.0` — half-open `>=`/`<` comparisons | `0`→sector 1, `60`→sector 2, `120`→**else** (row 5), `180`→sector 4, `240`→sector 5, `300`→sector 6, `360`→**else** (row 6) |
| 16 | `hsl_to_rgb` | one step past a boundary: `nextafter(60,0)`, `nextafter(120,0)`, `nextafter(180,0)`, `nextafter(360,0)`, `nextafter(0,-inf)` = `-1e-45` | `<60`→sector 1, `<120`→sector 2, `<180`→**else**, `<360`→sector 6, `-1e-45`→sector-3 body (row 7) |
| 17 | `hsl_to_rgb` | denormal / subnormal `h`, `s`, `l` (e.g. `1e-45`, `f32::MIN_POSITIVE/2`); `s` subnormal is `!= 0` so the early-out is **not** taken | computed with gradual underflow (no FTZ/DAZ is set by either library); bit-exact match required |
| 18 | `hsl_to_rgb` | `dest == src` (in-place conversion) — the C reads `h`, `s`, `l` into locals *before* any store, so aliasing is well-defined and **not** rejected | same result as the non-aliased call; the caller's buffer is overwritten with `[r,g,b]` |
| 19 | `hsl_to_rgb` | partially overlapping `dest`/`src` (e.g. `dest = src + 1`, `dest = src - 1`) — all three loads precede all three stores | result equals the non-aliased result; the overlapped source words are clobbered identically by both libraries |
| 20 | `hsl_to_rgb` | `dest == NULL` and/or `src == NULL` | **undefined behaviour / SIGSEGV in both** — the C has no null check (`grep NULL` finds nothing) and dereferences immediately. Verified out-of-process (`fork`) that C and Rust *both* die on the same signal rather than one of them silently succeeding; see `test_row_20_null_pointers_both_fault`. |
| 21 | `hsl_to_rgb` | `src` shorter than 3 floats / `dest` shorter than 3 floats (undersized buffer) | UB in both — the C reads/writes exactly 3 `float`s with no length parameter to validate. Covered indirectly: the differential tests use guard words around 3-float buffers and assert neither library touches `dest[3]` or reads past `src[2]`. |

## Notes on rows that are *not* differentially testable

Rows 20 and 21 are undefined behaviour with no defined C result. Row 20 is
covered by a fork-based test asserting both libraries fault identically (i.e.
the Rust translation did **not** "helpfully" add a null check the C lacks —
adding one would be a behavioural divergence). Row 21 is covered by
out-of-bounds guard-word assertions (neither library may write a 4th float).

There are no other enums, no other integer/flag parameters, and therefore no
"out-of-range enum value across the FFI boundary" case: the only parameters are
two pointers, and every one of the 2^32 possible `float` bit patterns for each
of `h`, `s`, `l` is an accepted input. The randomized tests therefore sample
*raw `u32` bit patterns* (not just "reasonable" floats) so that NaNs,
subnormals, infinities and negative zeros occur naturally.

## Checklist (all rows have a passing differential test)

- [x] 1  `s == +0.0` early-out
- [x] 2  `s == -0.0` early-out
- [x] 3  `s = NaN` skips the early-out
- [x] 4  `h = NaN` → final `else`
- [x] 5  `h ∈ [120,180)` → final `else` (replicated C bug)
- [x] 6  `h >= 360` → final `else`
- [x] 7  `h < 0` → third branch (replicated C bug)
- [x] 8  `h = -inf` → third branch, glibc `fmodf` NaN bit pattern
- [x] 9  `h = +inf` → final `else`
- [x] 10 `l = NaN`
- [x] 11 `l = ±inf`
- [x] 12 `l` outside `[0,1]`, finite
- [x] 13 `s` outside `[0,1]`, finite, non-zero
- [x] 14 `s = ±inf`
- [x] 15 exact boundary hues `0/60/120/180/240/300/360`
- [x] 16 one step past each boundary
- [x] 17 subnormal `h`/`s`/`l`
- [x] 18 `dest == src` aliasing
- [x] 19 partial `dest`/`src` overlap
- [x] 20 null pointers — both fault identically (no added null check)
- [x] 21 no out-of-bounds write past `dest[2]` / read past `src[2]`

## Note on the two undefined-behaviour rows (20, 21) and build instrumentation

The C reference is compiled by CMake with no `CMAKE_BUILD_TYPE`, i.e. plain
`-O0` with **no** instrumentation. Rust's `dev` profile enables
`debug-assertions`, which injects `assert_unsafe_precondition!` null/alignment
checks into `ptr::read`/`write`/`read_unaligned`/`write_unaligned` *and* into raw
pointer dereferences. Those checks turn the C's `SIGSEGV` (row 20) and the C's
"works, because `movss` does not require alignment" (row 31 of `CONFIGS.md`) into
a `panic` → `abort`, which is an observable divergence in the `.so`.

Two consequences, both settled in favour of matching the C:

1. `src/lib.rs` uses plain `*ptr` / `*ptr = v` dereferences (the literal
   translation of `src[0]` / `dest[0] = ...`) rather than
   `ptr::read_unaligned` / `ptr::write_unaligned`. The `_unaligned` helpers carry
   a null-pointer precondition check that made the debug build `abort` on row 20
   where the C `SIGSEGV`s.
2. `tests/common/mod.rs` builds the unoptimized Rust `cdylib` with
   `RUSTFLAGS=-Cdebug-assertions=off`, so it is an apples-to-apples counterpart
   of the unoptimized C build. Optimisation level remains the axis under test.

With those two decisions, C, `rust-debug` and `rust-release` all terminate with
the **same** signal (`SIGSEGV`, 11) for every null-pointer variant
(`dest`, `src`, both), and all agree bit-for-bit on mis-aligned buffers.

## Test evidence

| file | tests | covers |
|------|-------|--------|
| `tests/errors.rs` | 23 | ERRORS.md rows 1-21 + the generic FFI-boundary sweep |
| `tests/differential.rs` | 37 | CONFIGS.md rows 1-34 + Phase D symbol parity |

Every assertion compares raw `u32` bit patterns of all three output components,
so `+0.0` vs `-0.0` and NaN sign/payload differences fail the test. Every call
is made through `dlopen`/`dlsym` on the two `.so` files — the Rust crate is never
linked into the test binary, so the `#[no_mangle] extern "C"` wrapper is on the
tested path. `interesting_floats()` contains **57** values (zeros of both signs,
±1, ±0.5, all seven sector boundaries and their two 1-ULP neighbours, ±inf,
`f32::MAX`/`MIN`, subnormals, and nine NaNs covering quiet/signalling × both
signs × extreme payloads); rows that cross-product it therefore cover 57² =
3249 `(s, l)` pairs per hue.
