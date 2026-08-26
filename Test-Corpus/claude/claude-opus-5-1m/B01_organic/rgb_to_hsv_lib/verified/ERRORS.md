# ERRORS.md — Error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` (38 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep evidence

```
$ grep -nE "return|assert|NULL|errno|-1|enum|#if" c_src/src/lib.c c_src/include/lib.h
src/lib.c:23:        return;        <- bare early return (degenerate/achromatic path)
```

* `RETURN_ERROR`-style macros: **none**
* `return <error code>` / `return NULL`: **none** (function returns `void`)
* `assert` / `abort` / `errno`: **none**
* explicit range checks, null checks, min/max constants: **none**
* enums / flags / lengths crossing the FFI boundary: **none**
  (the only parameters are two `float*`; there is no count/length argument and
  no enum, so "out-of-range enum value" and "oversized length" are not
  reachable inputs for this API)

The library therefore has **no explicit error-return surface**. What it *does*
have is (a) one degenerate-input early-exit branch, (b) inputs that make the
arithmetic produce non-finite / signed-zero results, and (c) pointer
preconditions that are undefined behaviour when violated. Every one of those is
enumerated below, one row per distinct rejection/degenerate condition, and each
row has a differential test in `tests/errors.rs`.

## Table

| #  | function | trigger (the exact invalid/degenerate input or condition) | expected C result | [ ] |
|----|----------|-----------------------------------------------------------|-------------------|-----|
| 1  | `rgb_to_hsv` | `delta == 0` because `r == g == b`, value `> 0` (e.g. `0.5,0.5,0.5`) | early return at line 23: `dest = {0.0, 0.0, max}` (h and s left at their `0` initialisers) | [x] |
| 2  | `rgb_to_hsv` | `max == 0` because `r == g == b == +0.0` | early return: `dest = {0.0, 0.0, +0.0}` | [x] |
| 3  | `rgb_to_hsv` | `max == 0` reached with `delta != 0`: some components negative, largest is `0` (e.g. `0.0,-1.0,-2.0`) | early return **before** `s = delta/max` (so no `x/0` division): `dest = {0.0, 0.0, 0.0}` | [x] |
| 4  | `rgb_to_hsv` | all components `== -0.0` (`max == -0.0`, `-0.0 == 0` is true in C) | early return: `dest = {0.0, 0.0, -0.0}` — v keeps the **negative zero bit pattern** | [x] |
| 5  | `rgb_to_hsv` | mixed `+0.0` / `-0.0` (e.g. `+0.0,-0.0,+0.0`): `delta = 0.0 - (-0.0) = +0.0`, so `delta == 0` | early return: `dest = {0.0, 0.0, v}` where `v` is the max as selected by the `>` ternaries (`+0.0` here) | [x] |
| 6  | `rgb_to_hsv` | all components negative and distinct (`max < 0`, `delta > 0`): no early return; `s = delta/max` is **negative** | no error: negative saturation is returned as-is; h from the matching sector, `h += 360` applied if `h < 0` | [x] |
| 7  | `rgb_to_hsv` | `NaN` in `r` (`src[0]`) | no error/branch guard: every `<`/`>`/`==` with NaN is false, so `min = b`-chain and `max = b`-chain skip r; `r == max` is false, so the g/b sectors are used; result may be NaN | [x] |
| 8  | `rgb_to_hsv` | `NaN` in `g` (`src[1]`) | as above, with NaN excluded from the min/max ternaries; result bit pattern must match C exactly | [x] |
| 9  | `rgb_to_hsv` | `NaN` in `b` (`src[2]`) | as above | [x] |
| 10 | `rgb_to_hsv` | all three components `NaN` | `min = max = NaN` (ternaries fall through to the last operand), `delta = NaN`, `delta == 0` false, `max == 0` false, `r == max` false, `g == max` false → `h = 4 + (r-g)/delta = NaN`, `h < 0` false → `dest = {NaN, NaN, NaN}` | [x] |
| 11 | `rgb_to_hsv` | `+inf` present (e.g. `inf,0,0`) | `max = inf`, `delta = inf`, `s = inf/inf = NaN`, `h = (g-b)/inf = 0` → finite h with NaN s | [x] |
| 12 | `rgb_to_hsv` | `-inf` present (e.g. `-inf,1,2`) | `min = -inf`, `delta = inf`, `s = inf/max`, sector value `(x-y)/inf` | [x] |
| 13 | `rgb_to_hsv` | both `+inf` and `-inf` (`inf,-inf,0`) | `delta = inf - (-inf) = inf`, `s = inf/inf = NaN`, sector arithmetic yields `-inf/inf = NaN` etc. — must match bit-for-bit | [x] |
| 14 | `rgb_to_hsv` | overflow in `max - min`: `FLT_MAX, -FLT_MAX, 0` | `delta = +inf` (overflow, no error reported), `s = inf/FLT_MAX = inf` | [x] |
| 15 | `rgb_to_hsv` | underflow in `max - min`: two adjacent subnormals (`1e-45, 0, 0`) | `delta` subnormal, `delta == 0` false, `s = delta/max = 1.0` | [x] |
| 16 | `rgb_to_hsv` | subnormal max with `delta == 0` (`1e-45, 1e-45, 1e-45`) | early return, `v` = subnormal | [x] |
| 17 | `rgb_to_hsv` | `h < 0` correction path (`r` is max and `b > g`, e.g. `1.0, 0.0, 0.5`) | `h = -30 + 360 = 330` | [x] |
| 18 | `rgb_to_hsv` | `h` exactly `-0.0` before the correction (`r` max, `g == b`, e.g. `1,0,0` gives `+0.0`; `r` max with `g-b == -0.0`) | `h < 0` is **false** for `-0.0`, so **no** `+360`; `-0.0 * 60 = -0.0` is stored | [x] |
| 19 | `rgb_to_hsv` | `dest == src` (fully aliasing/in-place call) | all three inputs are copied into locals before any store, so in-place is well defined and produces the same result as a non-aliasing call | [x] |
| 20 | `rgb_to_hsv` | writes past `dest[2]` / reads past `src[2]` | C touches exactly 3 floats of each: canary words around a 3-float `dest` buffer must be untouched, and `src` bytes beyond index 2 must not affect the result | [x] |
| 21 | `rgb_to_hsv` | `src == NULL` (undefined behaviour; C has no null check) | the very first statement `src[0]` faults: process dies with `SIGSEGV` | [x] |
| 22 | `rgb_to_hsv` | `dest == NULL`, `src` valid (undefined behaviour; no null check) | the first store `dest[0]` faults: process dies with `SIGSEGV` | [x] |
| 23 | `rgb_to_hsv` | both pointers `NULL` | faults on `src[0]`: `SIGSEGV` | [x] |
| 24 | `rgb_to_hsv` | misaligned / unmapped-but-non-null `src` (e.g. `0x1`) | unmapped read faults: `SIGSEGV` | [x] |

Rows 21–24 are verified by re-executing the test binary as a child process and
comparing the *termination signal* of the C call and the Rust call (a crash in
the parent would abort the suite). All other rows compare the 3 output floats
**bit-for-bit** (`to_bits()`), which also pins down `+0.0` vs `-0.0` and NaN
payloads.

Each row maps 1:1 to a test named `errNN_*` in `tests/errors.rs`
(`err01_delta_zero_positive` … `err24_unmapped_pointers`); rows 7–9 share the
helper `nan_slot_test`. All 24 pass.

## Note on rows 21–24 and the cargo profile

* **release profile** (`cargo test --release`, i.e. what an external consumer
  links against): the Rust `.so` faults exactly like the C one — both children
  die with `SIGSEGV` (signal 11), asserted for equality.
* **dev profile** (`cargo test`): rustc enables `-C ub-checks` together with
  `-C debug-assertions`, so the Rust standard library detects the *same*
  undefined behaviour one instruction earlier and reports
  `null pointer dereference occurred` as a non-unwinding panic, which becomes
  `SIGABRT` (signal 6). This is a Rust-toolchain safety net for the identical UB,
  not a behavioural translation difference: the source performs the same
  unchecked load the C does. The tests assert this exact outcome (signal 6 *and*
  the specific UB-check message) in the dev profile, and strict signal equality
  in the release profile.

No valid (defined-behaviour) input diverges in any profile.
