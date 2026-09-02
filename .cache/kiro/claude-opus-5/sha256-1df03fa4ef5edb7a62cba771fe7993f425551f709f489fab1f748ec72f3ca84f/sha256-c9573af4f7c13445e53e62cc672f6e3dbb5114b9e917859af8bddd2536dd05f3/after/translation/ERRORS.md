# ERRORS.md — error-surface table (Phase C gate)

## Mechanical derivation

Every error-shaped construct was grepped out of the whole C tree
(`c_src/include/lib.h`, `c_src/src/lib.c` — the only two source files):

```
grep -nE 'return +-|return +NULL|assert|errno|RETURN_ERROR|ERROR|_MAX|_MIN|if *\(|switch|#if|malloc|free|NULL|enum' -r include src
→ (none)
```

Result: **the C library has no error-reporting surface at all.**

* no `return -1`, no `return NULL`, no error enum, no out-param status code —
  `tritanopia` returns `cb_rgb_255` by value and has no failure representation;
* no `assert`, no `errno` use, no `#ifdef`;
* no pointers in the public API (`cb_rgb_255` is passed and returned **by
  value**), so *null-pointer* rejection cannot be expressed or reached from an
  external caller;
* no `enum` in the API, so there is no out-of-range-enum input to pass across
  the FFI boundary;
* no length/count/size parameter, so there is no zero-length or oversized-length
  input either;
* no heap allocation, so no allocation-failure path;
* the only `if`-equivalents in the whole library are the two `?:` gamma
  thresholds, which are *valid-path* branches and therefore live in
  `CONFIGS.md`, not here.

Because `cb_rgb_255` is three `unsigned char`s, **every one of the 2^24 = 16 777 216
possible argument bit patterns is a valid input** that the C accepts and maps to
a defined-by-the-implementation output. There is no invalid input to construct.

## The real "rejection" surface: implementation-defined / UB conversions

The rows below are what actually stands in for an error surface here: the places
where the C leaves the standard's happy path and its *observed* behaviour must be
reproduced bit-for-bit rather than "fixed". Each row was derived from the C
source plus the disassembly of the reference `.so`, and each has a dedicated
differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `cbDenorm` (`lib.c:28`) via `tritanopia` | `RGB.R*255.f + 0.5f > 255.999…`, i.e. the red row of the matrix overflows 1.0 (`R + 0.1274*G − 0.1274*B > 1`, reachable e.g. at `G≫B`). C converts an out-of-`unsigned char`-range `float` — **UB by the standard**. | `cvttss2si` truncates toward zero into `eax`, then `mov %al` keeps the **low byte** → the value **wraps mod 256** (does *not* saturate to 255). Verified in the disassembly of `cbDenorm` (`cvttss2si %xmm0,%eax; mov %al,…`). | `err_e1_denorm_overflow_wraps_mod_256` | [x] |
| E2 | `cbDenorm` via `tritanopia` | `RGB.* * 255.f + 0.5f < 0`, i.e. a post-matrix channel is negative (red row goes negative when `B ≫ G`, e.g. `RGB = (0,0,255)`). Again out-of-range → UB. | truncation toward zero to a **negative** `i32`, low byte taken → wraps (e.g. `−419 → 0x…FE5D → 0x5D = 93`). Never clamped to 0. | `err_e2_denorm_negative_wraps_mod_256` | [x] |
| E3 | `cbDenorm` via `tritanopia` | value outside the signed-32-bit range, or NaN, reaching `cvttss2si` | `cvttss2si` yields the "integer indefinite" value `0x8000_0000`, whose low byte is `0x00` → result byte `0`. Unreachable from the public API (inputs are bounded bytes) but modelled in `c_float_to_uchar` so the Rust cannot diverge if it ever became reachable. | `err_e3_indefinite_unreachable_but_modelled` | [x] |
| E4 | `pow` in `cbApplyGammaRGB` (`lib.c:35`) | negative base with fractional exponent would be a domain error (`NaN`, `errno = EDOM`) | **cannot be triggered**: the `?:` guard `RGB.x > 0.0031308…` routes every negative/zero channel to the linear `x * 12.92` branch, so `pow` never sees a negative base. Asserted by exhaustively checking no output is produced via a NaN path. | `err_e4_pow_never_sees_negative_base` | [x] |
| E5 | `pow` in `cbRemoveGammaRGB` (`lib.c:11`) | negative base | **cannot be triggered**: input is `byte/255.f ∈ [0,1]`, and the guard `> 0.04045` sends everything else to `x / 12.92`. | `err_e4_pow_never_sees_negative_base` | [x] |
| E6 | `tritanopia` (whole API) | any attempt at an "invalid" argument: all-zero struct, all-`0xFF` struct, and — since the struct is one 8-byte register — **garbage in the padding/high 5 bytes of the argument eightbyte** | C ignores the high bytes (it only reads `%dil`/byte offsets 0..2); output must be identical regardless of what the unused bytes contain. This is the only way an external caller can pass something "malformed" through this ABI. | `err_e6_argument_register_padding_ignored` | [x] |
| E7 | `tritanopia` return value | the returned eightbyte's bytes 3..7 are left partly uninitialised by the C (`cbDenorm` ORs three bytes into `rax`) | only bytes 0..2 are meaningful; a conforming caller must not depend on bytes 3..7. The differential tests therefore compare **exactly the three defined bytes**, mirroring what the C guarantees. | `err_e7_only_three_bytes_are_defined` | [x] |

**All 7 rows are checked off by passing differential tests** (see
`tests/differential.rs`). No row is "both failed somehow" — each asserts the
specific byte-level outcome.

## Generic boundaries required by the task, and how they are covered

| generic boundary | applicability here | covered by |
|---|---|---|
| null pointers | **not expressible** — no pointer parameters in the public ABI | E6 (nearest analogue: junk in the argument register) |
| zero length / oversized length | **not expressible** — no length parameter | — |
| one step past a documented valid range | every `u8` is in range; the *derived* ranges that can be exceeded are the post-matrix float ranges | E1, E2 |
| out-of-range enum across FFI | **not expressible** — no enum in the API | E6 |
| min / max constants | `0` and `255` per channel, and all 8 corners of the RGB cube | `cfg_r16_corners_of_the_cube`, plus the exhaustive sweep |
