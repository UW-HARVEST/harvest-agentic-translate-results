# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -nE 'return +(-1|NULL|0)|RETURN_ERROR|assert|errno|ERROR|_MIN|_MAX|if *\(|switch|#ifdef|#if |exit|abort' -r src include
(no matches)

$ grep -n 'return' -r src include
src/lib.c:19:    return Result;
src/lib.c:25:    return Result;
src/lib.c:32:    return Result;
src/lib.c:45:    return Result;
src/lib.c:59:    return Result;
```

All five `return`s return a fully-formed value struct. There is:

* no error-return macro,
* no `return -1` / `return NULL` / error enum / status code,
* no `assert`, `abort`, `exit`, or `errno` use,
* no explicit range check, null check, or min/max constant,
* no pointer parameter in the public API (so no null-pointer rejection path).

**The library has an empty rejection surface.** The single public entry point
`cb_rgb_255 tritanopia(cb_rgb_255 RGB)` is total: it takes a by-value 3-byte
struct, so *every* one of the 2^24 possible inputs is valid and produces a
value. There is no input the C rejects, and therefore no error code or sentinel
for the Rust to match.

Because "no rejection exists" is itself a claim that must be verified rather
than assumed, the rows below cover (a) the *implicit* undefined-behaviour
conversion points, which are the only places the C can produce a
surprising/out-of-band result, and (b) the generic FFI boundaries the task
mandates. "Expected C result" is the *observed* behaviour of the compiled
reference `.so`, which is the ground truth.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `cbDenorm` (via `tritanopia`) | Post-gamma channel is **negative**, so `v*255+0.5 < 0` and the `(unsigned char)` conversion is out of the destination range (C UB). Reachable: pure blue `(0,0,255)` drives red to about `-1.65`, then the `<= 0.0031308` linear arm gives about `-21.3`, so `-5434` reaches the cast. | `cvttss2si` truncates toward zero into `%eax`, then `mov %al` keeps the low byte: the value **wraps modulo 256** (e.g. `-5434 & 0xff == 198`). No trap, no clamp. | `e1_negative_channel_wraps` | PASS |
| E2 | `cbDenorm` (via `tritanopia`) | Post-gamma channel **exceeds 1.0**, so `v*255+0.5 > 255` and the conversion overflows `unsigned char` (C UB). Reachable: `(255,255,0)` drives red to `1.127`, post-gamma about `1.053`, so `269` reaches the cast. | Same lowering: truncate then keep low byte, so the value **wraps modulo 256** (`269 & 0xff == 13`). Not clamped to 255. | `e2_over_one_channel_wraps` | PASS |
| E3 | `cbDenorm` | Value not representable in `int` at all (`\|v\| >= 2^31`) or NaN — the x86 "integer indefinite" case. **Unreachable through the public API** (post-gamma magnitudes stay near 1, so at most a few hundred reaches the cast), but it is the documented semantics the Rust helper `f32_to_u8_c_cast` emulates. | `cvttss2si` yields `0x8000_0000`, low byte `0x00`. | asserted unreachable by the exhaustive sweep (row C9); helper semantics covered by `e3_indefinite_helper_semantics` | PASS |
| E4 | `cbRemoveGammaRGB` | `pow` receives a negative base. **Unreachable**: the `> 0.04045` guard means `pow` only ever sees a base `>= (0.04045+0.055)/1.055 > 0`. Verified no `NaN` ever leaves the C. | no `NaN`/domain error; `errno` untouched | `e4_no_nan_ever_escapes` | PASS |
| E5 | `cbApplyGammaRGB` | `pow` receives a negative base. **Unreachable** for the same reason (`> 0.0031308` guard sends negatives to the `*12.92` linear arm). This is why negative channels survive as large negative numbers into E1 instead of becoming `NaN`. | no `NaN`; negative input takes linear arm | `e4_no_nan_ever_escapes` | PASS |
| E6 | both gamma helpers | Channel is exactly **on** the branch threshold (`0.04045`, and `0.00313080495356037151702786377709`). `>` is strict, so equality takes the `else` arm. One step past the range in each direction must also agree. | strict `>`: equal value takes the linear arm | `e6_threshold_exact_and_one_step` | PASS |
| E7 | `tritanopia` (FFI boundary) | **Out-of-range "enum"/padding bits**: the 3-byte struct travels in a 64-bit register, so a caller may leave arbitrary garbage in the upper 5 bytes (and there is no enum in this API whose variants could be exceeded — every `unsigned char` bit pattern `0x00..0xff` is a valid variant, so the "no valid variant" case does not exist here). | C reads only the low 3 bytes (`movzbl -0x38..`, `-0x37..`, `-0x36..`); upper bits are ignored. Rust must ignore them identically. | `e7_garbage_in_upper_register_bytes` | PASS |
| E8 | `tritanopia` (FFI boundary) | **Every byte-triple boundary value**: all channels `0x00`, all `0xff`, and each single channel at `0x00`/`0x01`/`0xfe`/`0xff` with others varied — "zero and oversized lengths" analogue for a fixed-width value API. | a defined 3-byte result for every one of them | subsumed by the exhaustive sweep (row C9) + `e8_boundary_triples` | PASS |
| E9 | `tritanopia` | Null pointer. **Not applicable**: the public API takes and returns the struct *by value* and exposes no pointer, array, length, or handle parameter, so there is no null-pointer or zero-length input to construct. Recorded explicitly so the omission is deliberate rather than an oversight. | n/a | n/a (documented) | N/A |

All applicable rows are covered by a passing differential test; E3/E4/E5 are
additionally proven unreachable rather than merely untested.
