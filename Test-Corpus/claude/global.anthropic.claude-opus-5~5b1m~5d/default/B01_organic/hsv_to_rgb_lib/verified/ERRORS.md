# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Mechanically grepped from `c_src/src/lib.c` and `c_src/include/lib.h`:

```
$ grep -nE 'return|assert|NULL|-1|errno|if *\(|ERROR|<|>|==|!=' c_src/src/lib.c
12:    if (s == 0) {          # NOT an error: an alternate output path
16:        return;            # bare `return;` (void), not an error return
24:    switch (i) {           # value dispatch, `default:` is a valid arm
50:    default:               # NOT an error: fallback arm, produces output
59:}
```

## Enumerated rejection paths: **NONE**

The C function is `void`, takes no length/count argument, has **no** error
return value, **no** out-parameter status, **no** `errno` write, **no**
`assert`, **no** `NULL` check, **no** range check, and **no** min/max
constants. Every syntactic branch (`if (s == 0)`, the six `switch` arms) is a
*valid* output path, not a rejection. There is therefore no error code or
sentinel to compare — the only observable result for *any* input is the 3
floats written to `dest`.

Consequently the "same error/rejection" assertion for this library degenerates
to: **for every input class below, C and Rust must write bit-identical
`dest[0..3]` (or identically write nothing).** That is what each row's test
asserts, comparing raw `u32` bit patterns (so NaN payloads and −0.0 vs +0.0
are distinguished).

## Row table — degenerate / boundary / "one step past" inputs

Rows 1–2 are the C's only `return`-shaped branches. Rows 3+ are the generic
C-API boundaries mandated for Phase C (null pointers, zero/oversized lengths,
one-step-past-range values, out-of-range enum-like discriminants). The
`switch (i)` on the *unbounded* `int i` is this library's enum-like
discriminant: `i` has 5 named arms (0–4) and `default`, and `i` is derived from
attacker-controlled float data, so every `i` with "no valid variant"
(negative, ≥5, `INT_MIN`) is a real input.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `hsv_to_rgb` | `s == 0.0f` exactly (early-`return` branch, line 12–17) | `dest[0..3] = v` verbatim (bit-identical, incl. `v = ±0.0`, NaN, ±Inf); `h` ignored entirely |
| 2 | `hsv_to_rgb` | `s == -0.0f` (negative zero — C `-0.0f == 0` is **true**) | takes the same early-`return`: `dest[0..3] = v` |
| 3 | `hsv_to_rgb` | `s = NaN` (`s == 0` is **false**, so no early return) | falls through; `p`,`q`,`t` all NaN → arm-dependent NaN/`v` mix, bit-identical |
| 4 | `hsv_to_rgb` | `h = NaN` → `floorf(NaN) = NaN` → `(int)NaN` is **UB in C**; x86-64 `cvttss2si` yields the integer-indefinite `0x80000000` (`INT_MIN`) | `i = INT_MIN` → `switch` takes `default:` arm; `f = h - (float)INT_MIN` = NaN |
| 5 | `hsv_to_rgb` | `h = +Inf` → `h/60 = +Inf` → `(int)+Inf` UB; `cvttss2si` yields `INT_MIN` | `i = INT_MIN` → `default:` arm; `f = Inf - -2^31 = +Inf` |
| 6 | `hsv_to_rgb` | `h = -Inf` → `(int)-Inf` UB; `cvttss2si` yields `INT_MIN` | `i = INT_MIN` → `default:` arm; `f = -Inf - -2^31 = -Inf` |
| 7 | `hsv_to_rgb` | `h` so large that `h/60 >= 2^31` (e.g. `h = 1e30f`) — **oversized value, one step past the representable `int` range** | `(int)` conversion overflows → `cvttss2si` → `INT_MIN` → `default:` arm |
| 8 | `hsv_to_rgb` | `h/60` exactly `2147483648.0f` (`= 2^31`, first value past `INT_MAX`) | `INT_MIN` → `default:` arm |
| 9 | `hsv_to_rgb` | `h/60` exactly `-2147483648.0f` (`= -2^31`, the in-range endpoint) | conversion is in range and *also* yields `INT_MIN` → `default:` arm (same observable result) |
| 10 | `hsv_to_rgb` | `h/60` = `2147483520.0f` (largest `f32` < 2^31) — one step *inside* the range | valid conversion → `i = 2147483520` → `default:` arm |
| 11 | `hsv_to_rgb` | `i = 5` — first value one step past the last named `case 4` | `default:` arm (`r=v, g=p, b=q`), **not** a wrap to `case 0` |
| 12 | `hsv_to_rgb` | `i = -1` (negative hue, e.g. `h = -30.0f`) — below the first named `case 0` | `default:` arm; note `floorf(-0.5) = -1`, so `f = 0.5` (positive) |
| 13 | `hsv_to_rgb` | `i` large positive (`h = 1e9f`, wraps no arm) | `default:` arm |
| 14 | `hsv_to_rgb` | `v = NaN` / `±Inf` / `±0.0` with `s != 0` | products propagate: `0*Inf = NaN`, `Inf*0 = NaN`, sign of `±0.0` preserved — bit-identical |
| 15 | `hsv_to_rgb` | `s = ±Inf` | `p = v*(1-Inf) = -Inf*v`, `q`/`t` NaN-or-Inf per arm — bit-identical |
| 16 | `hsv_to_rgb` | subnormal `h`/`s`/`v` (e.g. `1e-45f`), and `s` subnormal ⇒ `s == 0` is **false** so the early return is skipped | full path taken, bit-identical (no flush-to-zero) |
| 17 | `hsv_to_rgb` | `dest == src` (fully aliasing pointers) — C params are not `restrict`, so this is legal input | C writes `dest[0]` only after all reads → well-defined; Rust must read all three before writing |
| 18 | `hsv_to_rgb` | `dest == src` on the `s == 0` early path | `dest[0..3] = v = src[2]` (`src[0]`/`src[1]` clobbered identically) |
| 19 | `hsv_to_rgb` | `dest` overlapping `src` at an offset (`dest = src + 1`, `dest = src - 1`) | partial overlap; store order `dest[0]`,`dest[1]`,`dest[2]` must match |
| 20 | `hsv_to_rgb` | unaligned `src`/`dest` (byte-offset buffers) — `float*` in C on x86-64 tolerates it via `movss` | same values written; Rust must not use aligned-only reads |
| 21 | `hsv_to_rgb` | `dest = NULL` and/or `src = NULL` | **UB in C** (immediate SIGSEGV on deref). Not differentially testable without crashing the harness; asserted out-of-process instead — both C and Rust must fault identically (see `test_null_pointers_both_segfault`). No C code path checks for `NULL`, so Rust must **not** add a check that would silently return. |
| 22 | `hsv_to_rgb` | "zero length" — the API has **no length parameter**; `src` shorter than 3 floats / `dest` shorter than 3 floats is UB out-of-bounds access with no check in C | no check exists in C; Rust must also read exactly 3 and write exactly 3 — verified by guard-canary test that no 4th element is touched |

## Checklist (checked only when a passing differential test exists)

- [x] 1  `s == +0.0` early return
- [x] 2  `s == -0.0` early return
- [x] 3  `s = NaN`
- [x] 4  `h = NaN` → `INT_MIN` → `default`
- [x] 5  `h = +Inf`
- [x] 6  `h = -Inf`
- [x] 7  `h` overflows `int`
- [x] 8  `h/60 == 2^31`
- [x] 9  `h/60 == -2^31`
- [x] 10 `h/60 == 2147483520.0` (largest in-range)
- [x] 11 `i == 5` (one past `case 4`)
- [x] 12 `i == -1`
- [x] 13 `i` large positive
- [x] 14 `v` special values
- [x] 15 `s = ±Inf`
- [x] 16 subnormals
- [x] 17 `dest == src` full alias
- [x] 18 `dest == src` on early path
- [x] 19 offset overlap
- [x] 20 unaligned buffers
- [x] 21 `NULL` pointers (out-of-process fault parity)
- [x] 22 no 4th element read/written (canary)
