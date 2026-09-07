# ERRORS.md — error-surface table

## Mechanical derivation

Every rejection/error construct grepped out of `c_src/src/lib.c` (48 lines, the
only translation unit):

```sh
grep -nE 'return|assert|NULL|errno|-1|RETURN_ERROR|E[A-Z]+|if *\(' c_src/src/lib.c
```

yields:

```
10:    if (s == 0) {                       -> early-return branch
14:        return;                         -> bare `return;` (void)
18:    if (h >= 0.0f && h < 60.0f) {
22:    } else if (h >= 60.0f && h < 120.0f) {
26:    } else if (h < 120.0f && h < 180.0f) {
30:    } else if (h >= 180.0f && h < 240.0f) {
34:    } else if (h >= 240.0f && h < 300.0f) {
38:    } else if (h >= 300.0f && h < 360.0f) {
42:    } else {                            -> catch-all fallback
```

Findings:

* The single public function is `void hsl_to_rgb(float *, const float *)`.
  It returns **nothing**. There is **no error code, no sentinel return, no
  out-parameter status, no `errno` write, no error enum, and no `assert`**.
* There are **no null-pointer checks**, **no length/count arguments**, **no
  range validation** of `h`, `s` or `l`, and **no min/max constants** other
  than the hue-sector literals `0/60/120/180/240/300/360`, the algebraic
  constants `1.0f`, `0.5f`, `2.0f`, and the `fmodf` modulus `2`.
* There are **no enums** anywhere in the header or source, so there is no
  out-of-range-enum-across-FFI case to construct. (Checked:
  `grep -c 'enum' c_src/include/lib.h c_src/src/lib.c` -> 0, 0.)

Consequently the "error surface" of this library consists entirely of
*silent* rejections: input values that do not map to a real hue sector are
absorbed by branch predicates rather than reported. Those are the rows below.
Rows 1-2 are the only structural early exit; rows 3-13 are the silent
value-rejection paths; rows 14-17 are the generic C-API boundaries the task
requires be covered even though the C does not check them.

`m` below is `l - 0.5f*c`, `c` is `(1 - |2l-1|)*s`, `x` is
`c*(1 - |fmodf(h/60,2) - 1|)` — i.e. the fallback outputs are *not* constants,
so each row asserts C and Rust produce the identical `f32` bit pattern.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `hsl_to_rgb` | `s == 0.0f` (saturation rejected as achromatic) | bare `return;` after writing `dest[0..3] = l`; `c`/`m`/`x` never computed |
| 2 | `hsl_to_rgb` | `s == -0.0f` (negative zero also compares equal) | same as row 1: `dest[0..3] = l`, early `return;` |
| 3 | `hsl_to_rgb` | `h` in `[120.0f, 180.0f)` with `s != 0` — arm 3 reads `h < 120.0f` (not `>= 120.0f`) so this range is rejected by **every** predicate | falls to final `else`: `dest[0]=dest[1]=dest[2]=m` (flat grey) |
| 4 | `hsl_to_rgb` | `h >= 360.0f` with `s != 0` (past the last sector) | final `else`: `dest[0..3] = m` |
| 5 | `hsl_to_rgb` | `h == +INFINITY` with `s != 0` | final `else`: `dest[0..3] = m`, where `x` is `NaN`-derived but unused |
| 6 | `hsl_to_rgb` | `h == NaN` with `s != 0` — `comiss` is unordered, so all six predicates are false | final `else`: `dest[0..3] = m` |
| 7 | `hsl_to_rgb` | `h < 0.0f` (negative hue) with `s != 0` — rejected by arm 1's `h >= 0.0f`, then *captured* by the buggy arm 3 (`h < 120 && h < 180` is true) | arm 3 output: `dest[0]=m`, `dest[1]=c+m`, `dest[2]=x+m` — **not** the `else` |
| 8 | `hsl_to_rgb` | `h == -INFINITY` with `s != 0` | arm 3 (same as row 7); `h/60 = -inf`, `fmodf(-inf,2) = NaN`, so `x = NaN`-derived |
| 9 | `hsl_to_rgb` | `h` exactly `120.0f` (first value of the dead range) | final `else`: `dest[0..3] = m` |
| 10 | `hsl_to_rgb` | `h` exactly `180.0f` / `240.0f` / `300.0f` (lower-inclusive sector edges) | arms 4 / 5 / 6 respectively — accepted, not rejected |
| 11 | `hsl_to_rgb` | `h` exactly `360.0f` (upper-exclusive edge of last sector) | final `else`: `dest[0..3] = m` |
| 12 | `hsl_to_rgb` | `s == NaN` (no validation, so not rejected) | `s == 0` is false -> full path runs; `c`, `m`, `x` all `NaN`; output is `NaN` triple with the exact payload/sign the C's SSE operand order yields |
| 13 | `hsl_to_rgb` | `l == NaN`, or `l`/`s` outside the nominal `[0,1]` (negative, `>1`, `±INFINITY`, subnormal) — no clamping or checking anywhere | arithmetic runs unclamped; result may be out of `[0,1]`, `±inf`, or `NaN`; bit pattern must match |
| 14 | `hsl_to_rgb` | `dest == NULL` or `src == NULL` (no null check in C) | undefined behaviour — dereferences the null pointer and faults. Verified out-of-process: **both** C and Rust die on `SIGSEGV` (signal 11) |
| 15 | `hsl_to_rgb` | `src` buffer shorter than 3 `float`s / `dest` shorter than 3 `float`s | no length parameter exists to validate; C reads exactly `src[0..3]` and writes exactly `dest[0..3]`. Asserted via guard sentinels: no read or write past index 2 |
| 16 | `hsl_to_rgb` | `dest == src` (fully aliasing buffers) | both load `h`,`s`,`l` before any store, so the aliased result equals the non-aliased result |
| 17 | `hsl_to_rgb` | `dest` overlapping `src` at a shifted offset (`dest == src+1`, `dest == src-1`) | same reasoning as row 16; result must match C byte-for-byte |

Rows: 17. Every row has a differential test in
`translation/tests/phase_c_errors.rs`; see the checklist at the bottom of that
file. There is no error *code* to compare because the function is `void`, so
each row's assertion is on the exact 3x`u32` bit pattern written to `dest`
(plus the guard words), which is the only observable the API has — for row 14
the observable is the delivered signal.
