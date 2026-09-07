# ERRORS.md — Error / rejection surface table (Phase A)

## Mechanical derivation

Greps run over the complete C source (`c_src/src/lib.c`, `c_src/include/lib.h`):

```
grep -n  "return"                                            -> src/lib.c:23: "return;"   (1 hit)
grep -niE "assert|errno|NULL|RETURN_ERROR|error|_MAX|_MIN|goto|exit|abort"  -> (none)
grep -nE "^#"                                                -> src/lib.c:1: #include "lib.h"
```

Result: the C library contains **zero** error-return statements, zero error
enums or sentinels, zero `assert`s, zero null checks, zero range checks and zero
min/max constants. `rgb_to_hsv` returns `void`, so it has no channel through
which to report an error at all. The single `return;` at line 23 is the
early-out of a **valid** achromatic path (`delta == 0 || max == 0`), not a
rejection — it is covered in `CONFIGS.md` (rows C1–C3, C15–C16), not here.

Consequently every row below is a *generic C-API boundary* mandated by Phase C
rather than an explicit rejection the C performs. Each row states the concrete,
observable expected behaviour so it can be asserted differentially — none is
"both failed somehow".

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| E1 | `rgb_to_hsv` | `src == NULL`, `dest` valid — unchecked `src[0]` load at line 4 | no validation: process dies on the load. Fatal signal (SIGSEGV). Rust `.so` must die with the **same signal** | [x] |
| E2 | `rgb_to_hsv` | `dest == NULL`, `src` valid — reaches an unchecked `dest[0]` store (line 20 or 35) | no validation: process dies on the store. Fatal signal (SIGSEGV). Rust must match the signal | [x] |
| E3 | `rgb_to_hsv` | `dest == NULL && src == NULL` | dies on the `src[0]` load first (loads precede all stores). Fatal SIGSEGV, same as E1 | [x] |
| E4 | `rgb_to_hsv` | zero/undersized length: caller supplies fewer than 3 floats. C has no length parameter and no check — it reads **exactly** `src[0..=2]` and writes **exactly** `dest[0..=2]` | out-of-range accesses are pure UB with no diagnostic. Observable, assertable contract: with a 3-float buffer plus guard canaries at index 3.., the canaries beyond index 2 are **never** read or written. Rust must touch the identical 3-element window | [x] |
| E5 | `rgb_to_hsv` | oversized length: caller supplies more than 3 floats | elements at index >= 3 are ignored entirely; result depends only on `src[0..=2]`. Rust must ignore them too (differential on padded buffers) | [x] |
| E6 | `rgb_to_hsv` | `dest == src` (full aliasing) — no `restrict`, no overlap check | well-defined: all three loads happen before any store, so output equals the non-aliased result. Rust must match bit-for-bit | [x] |
| E7 | `rgb_to_hsv` | partial overlap `dest == src + 1` and `dest == src - 1` | same as E6: loads precede stores, so result equals the non-aliased result; the untouched neighbour slot is preserved. Rust must match | [x] |
| E8 | `rgb_to_hsv` | misaligned `float*` (pointer offset by 1..3 bytes) — C requires 4-byte alignment, provides no check | no check; on x86-64 the unaligned scalar load/store simply succeeds and produces the ordinary result. Rust must produce the identical bits | [x] |
| E9 | `rgb_to_hsv` | out-of-domain component values one step past any "documented valid range": RGB outside `[0,1]`, i.e. negative, `> 1.0`, `nextafter(0,-1)`, `nextafter(1,+inf)` | **no range check exists** — the value is used arithmetically as-is, so `s` may exceed 1 or be negative and `v` may be negative. No error. Rust must reproduce the same out-of-range outputs bit-for-bit | [x] |
| E10 | `rgb_to_hsv` | non-finite components: `NaN` (quiet, and signalling bit pattern) in any subset of r/g/b | no check. `NaN` fails every `<`/`>`/`==` comparison, so the ternary min/max expansions keep the *other* operand; the exact result is dictated by the literal ternary order in the C. Rust must reproduce identical output bits, including NaN sign/payload | [x] |
| E11 | `rgb_to_hsv` | non-finite components: `+INFINITY` / `-INFINITY`, including `+inf` and `-inf` together (`delta = inf`, `x/inf = 0` or `inf/inf = NaN`) | no check; IEEE-754 arithmetic propagates. Rust must reproduce identical output bits | [x] |
| E12 | `rgb_to_hsv` | the enum-with-no-valid-variant analogue: this API declares **no enum and no flag/mode parameter** (header is 1 line, `float*` only), so there is no integer parameter whose out-of-range value could be passed across FFI. The corresponding "bit pattern with no valid meaning" input is an arbitrary `u32` reinterpreted as `f32` (covers all NaN payloads, denormals, infinities) | no check, no rejection: all 2^32 bit patterns are accepted and processed arithmetically. Verified differentially over randomized raw-bit triples (fixed seed) | [x] |
| E13 | `rgb_to_hsv` | denormal / smallest-magnitude inputs where `delta` underflows to a subnormal or `0`, and huge inputs where `max - min` overflows to `+inf` | no check; `delta == 0` after underflow silently takes the achromatic early-return, `delta == inf` yields `s == inf`-ratio results. Rust must match | [x] |

All 13 rows are exercised by `translation/tests/errors.rs`
(E1–E3 via a re-exec'd child process so the fatal signal can be compared).

## Divergences found and fixed

Both were found by the error-path rows, not by the happy path, and both were
invisible in the release profile:

1. **E8 (misaligned pointer) — Rust aborted where C succeeded.**
   The translation read/wrote through plain raw-pointer dereferences
   (`*src.add(i)` / `*dest.add(i) = X`). With a `float*` skewed by 1–3 bytes the
   C performed the unaligned access and returned normally, while the Rust `.so`
   tripped rustc's inserted misaligned-pointer check and died with a
   non-unwinding panic → **SIGABRT** instead of producing a result.

2. **E1–E3 (NULL pointer) — Rust aborted with the wrong signal.**
   The first fix attempt (`std::ptr::read_unaligned` / `write_unaligned`) cured
   E8 but not this: those functions carry a debug-assertions null check, so a
   NULL argument killed the Rust `.so` with **SIGABRT (6)** whereas the C dies
   with **SIGSEGV (11)**. Plain `*ptr` and `#[repr(C, packed)]` derefs were both
   measured and behave the same way, so neither was an acceptable fix.

**Fix (in `translation/src/lib.rs` only):** all three loads and all six stores now
go through two helpers that perform a *volatile 4-byte-array* access,

```rust
#[inline(always)] unsafe fn load_f32(p: *const c_float) -> f32 {
    f32::from_ne_bytes(std::ptr::read_volatile(p as *const [u8; 4]))
}
#[inline(always)] unsafe fn store_f32(p: *mut c_float, v: f32) {
    std::ptr::write_volatile(p as *mut [u8; 4], v.to_ne_bytes())
}
```

which is alignment-agnostic and carries no inserted checks, so it faults exactly
like the C. Measured at `opt-level` 0 and 3 with debug-assertions on: misaligned
access succeeds, NULL yields SIGSEGV. The arithmetic and control flow were not
touched — verified by normalizing the pointer-access syntax and diffing against
the pre-fix source (identical).

## Test-suite sensitivity (mutation testing)

To prove the differential suite is not vacuously green, the Rust source was
deliberately mutated and the suite re-run. Every semantically meaningful mutation
is caught:

| mutation | result |
|----------|--------|
| ternary `min` → `f32::min` (NaN semantics) | 2 tests FAIL |
| ternary `max` → `f32::max` (NaN semantics) | 3 tests FAIL |
| drop the `\|\| max == 0.0` disjunct | 3 tests FAIL |
| `if h < 0.0` → `if h <= 0.0` | 6 tests FAIL |
| `2 + (b-r)/delta` → `2 + (r-b)/delta` | 11 tests FAIL |
| `4 + (r-g)/delta` → `4 + (g-r)/delta` | 11 tests FAIL |
| `h *= 60.0` → `h *= 60.000004` (1 ULP) | 17 tests FAIL |
| `h += 360.0` → `h += 359.99998` | 10 tests FAIL |
| `s = delta / max` → `delta / max.abs()` | 7 tests FAIL |
| `r` → `r + 0.0` (perturbs `-0.0`) | 3 tests FAIL |
| early-return `v` → `-v` | 10 tests FAIL |
| read `g` from the wrong index | 23 tests FAIL |

Four mutations correctly did **not** fail, because they are provably equivalent
rewrites rather than blind spots: `r == max` → `r >= max` and `g == max` →
`g >= max` (`max` is produced by a ternary chain seeded with `r`, so `max >= r`
always holds and NaN makes both comparisons false), `s = delta / max` →
`delta / v` (`v` is assigned `max`), and `h *= 60.000001` (which rounds to the
identical `f32` bit pattern as `60.0`, confirmed as `0x42700000` for both).

Additionally, `errors.rs::e00_negative_control_harness_detects_divergence`
asserts at runtime that the two `dlopen`'d `rgb_to_hsv` symbols resolve to
*different* addresses (so two real libraries are loaded), that both actually
overwrite the `0xDEADBEEF` canary in `dest`, and that the bitwise comparator
rejects a deliberately flipped bit.
