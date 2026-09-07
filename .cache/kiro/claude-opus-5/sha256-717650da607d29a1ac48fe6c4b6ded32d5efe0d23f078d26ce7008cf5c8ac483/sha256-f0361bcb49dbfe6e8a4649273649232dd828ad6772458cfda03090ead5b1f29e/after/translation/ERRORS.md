# ERRORS.md — Phase C error-surface table

Derived mechanically from the whole of `c_src` (`include/lib.h`, 3 lines;
`src/lib.c`, 118 lines — the entire library).

## Mechanical grep audit of every rejection construct

Every one of these greps over `c_src/**` returns **zero** matches:

| construct searched | matches |
|--------------------|---------|
| `return -1` / `return NULL` / `return 0;` as an error path | 0 |
| `RETURN_ERROR` / `*_ERROR` / `*_ERR` macro | 0 |
| `assert` / `static_assert` / `abort` / `exit` | 0 |
| `errno` | 0 |
| `if` / `else` / `switch` / `case` / `?:` (any branch at all) | 0 |
| `enum` (so: no enum parameter can receive an out-of-range int) | 0 |
| `*` pointer parameter anywhere in the public API | 0 |
| length / size / count parameter anywhere in the public API | 0 |
| `#ifdef` / `#if` / `#ifndef` conditional compilation | 0 |
| `goto` | 0 |
| named min/max range constants | 0 |

`float2half` is a **total function**: signature `uint16_t float2half(float)`.
It is straight-line code with no branches, returns `uint16_t`, and **every one
of the 65536 possible return values is a legitimate result**, so the API has no
error code and no sentinel value. There is no input it rejects.

Consequently the error surface consists entirely of *implicit* safety
invariants (the checks the C author encoded in masks and table sizes rather
than in `if` statements) plus the generic FFI boundaries. Each row below is a
condition that in the Rust translation *could* panic / trap / UB where the C
does not, i.e. a place where Rust could diverge by aborting instead of
returning. Each row asserts C and Rust return the **same** `uint16_t`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `float2half` | Table index overrun: any input whose raw exponent field would index past `m__base[512]` / `m__shift[512]`. The C guards this **only** with `j = (n >> 23) & 0x1ff`, which caps `j` at 511 for every one of the 2^32 bit patterns. Adversarial probe: all-ones bits `0xFFFFFFFF` (`j == 511`, the last element) and `0x7FFFFFFF` (`j == 255`). | no rejection: returns `m__base[j] + ((n & 0x7fffff) >> m__shift[j])`; `0xFFFFFFFF` -> `0xffff`, `0x7FFFFFFF` -> `0x7fff`. Rust must index in bounds, not panic. | [x] |
| 2 | `float2half` | Shift-width overflow: `(n & 0x007fffff) >> m__shift[j]` where `m__shift[j]` is the widest value in the table (`0x18` = 24). C never range-checks the shift; it relies on the table only ever holding 13..=24, all `< 32`. Probe: every `j` in the two `shift == 0x18` runs (`j` 0..102, 143..254, 256..358, 399..510) with mantissa `0x7fffff`. | no rejection: the shift is well defined; addend is `0`, result is exactly `m__base[j]`. Rust must not panic with "attempt to shift right with overflow". | [x] |
| 3 | `float2half` | `uint16_t` truncation of the `uint32_t` sum: `(uint16_t)((uint32_t)m__base[j] + addend)`. C silently truncates. Worst case is `j == 511`, mantissa `0x7fffff`: `0xfc00 + 0x3ff = 0xffff`. | no rejection: `0xffff`. (Exhaustively confirmed the sum never exceeds `0xffff`, so truncation is never observable — but Rust must use wrapping, not checked, arithmetic so it cannot panic in debug builds either.) | [x] |
| 4 | `float2half` | Signalling NaN input (`0x7FBFFFFF`, `0xFFBFFFFF`) — the payload must survive the FFI float argument unaltered; a quieting or canonicalisation on either side changes the result. | `0x7c00 + (0x3fffff >> 13)` = `0x7dff` / `0xfdff`. | [x] |
| 5 | `float2half` | Quiet NaN with all distinct payloads, including payloads whose low 13 bits are non-zero (dropped) and payloads that shift down to `0` (`0x7F800001`, which is a NaN that maps onto the **infinity** encoding `0x7c00` — a value-dependent aliasing the C performs and must be reproduced, not "fixed"). | `0x7c00` for `0x7F800001`; `0xfc00` for `0xFF800001`. | [x] |
| 6 | `float2half` | `+Inf` / `-Inf` (`0x7F800000`, `0xFF800000`). | `0x7c00` / `0xfc00`. | [x] |
| 7 | `float2half` | Overflow to infinity: finite float too large for half, one step past the last representable exponent (`j == 143`, i.e. bits `0x47800000`, and the negative mirror `0xC7800000`). This is where the C's table saturates instead of erroring. | `0x7c00` / `0xfc00` (silent saturation, no error). | [x] |
| 8 | `float2half` | Underflow to zero: finite non-zero float too small for a half subnormal, one step below the first representable (`j == 102`, bits `0x33000000` .. `0x337FFFFF`, and the negative mirror). | `0x0000` / `0x8000` — the sign is preserved for negatives, so `-tiny` becomes `-0.0`, not `+0.0`. | [x] |
| 9 | `float2half` | Signed zero (`0x00000000`, `0x80000000`) and float subnormals (`0x00000001`, `0x807FFFFF`). | `0x0000` / `0x8000` for all of them (`j <= 102` / `256..358` -> base only). | [x] |
| 10 | `float2half` | Exact boundary between the `shift == 0x18` flush-to-zero run and the first graduated-underflow entry: `j == 102` vs `j == 103` (and `j == 358` vs `j == 359`), across all 2^23 mantissas of each. | `j == 102` -> `0x0000`; `j == 103` -> `0x0001 + (mantissa >> 23)`, i.e. `0x0001` or `0x0002`. | [x] |
| 11 | `float2half` | Exact boundary between graduated underflow and normal halves: `j == 112` (`shift 0x0e`) vs `j == 113` (`shift 0x0d`), and the negative mirror `j == 368` / `369`. | `0x0200 + (m >> 14)` vs `0x0400 + (m >> 13)`. | [x] |
| 12 | `float2half` | Exact boundary between the last finite normal and saturation: `j == 142` (`shift 0x0d`, base `0x7800`) vs `j == 143` (`shift 0x18`, base `0x7c00`), and mirror `j == 398` / `399`. | `0x7800 + (m >> 13)` vs `0x7c00`. | [x] |
| 13 | `float2half` | Exact boundary between the saturation run and the Inf/NaN entry: `j == 254` (`shift 0x18`) vs `j == 255` (`shift 0x0d`), and mirror `j == 510` / `511`. This single-element discontinuity at the very end of each half of the table is the easiest element to mis-transcribe. | `0x7c00` (mantissa ignored) vs `0x7c00 + (m >> 13)` (mantissa honoured). | [x] |
| 14 | `float2half` | **Null pointer**: not applicable — `float2half` takes no pointer parameter (grep: 0 `*` in the public API). Documented per the generic-boundary requirement; there is no pointer input to pass `NULL` for. | n/a | [x] |
| 15 | `float2half` | **Zero / oversized length**: not applicable — `float2half` takes no length, size, or count parameter. Documented per the generic-boundary requirement. | n/a | [x] |
| 16 | `float2half` | **Out-of-range enum across the FFI boundary**: not applicable — the C API declares no `enum` and takes no integer mode/flag parameter (grep: 0 `enum`). The nearest analogue is "an argument bit pattern with no valid interpretation", i.e. a non-canonical / NaN `float`; that is covered exhaustively by rows 1–13 and by the exhaustive 2^32 sweep, which passes **every** representable argument bit pattern across the FFI boundary. | n/a (subsumed by the exhaustive sweep) | [x] |

## How these rows are discharged

`translation/tests/differential.rs`:

* `phase_c_error_surface_rows` — one explicit, named assertion per row 1–13,
  comparing the C `.so` and the Rust `.so` return values for that exact probe.
* `phase_b_exhaustive_all_bit_patterns` — sweeps **all 2^32 `f32` bit
  patterns** through both `.so`s and asserts equality. This is a strict
  superset of every row above (and of every conceivable invalid input), because
  the function's entire input domain is the 32-bit argument.

## Result

All 16 rows pass, under every feature combination and under both the debug and
release cdylib builds. Driver: `translation/verify_all.sh`.

Rows 1–3 are the rows that matter most for a C-to-Rust port, because they are
the places where C's silent behaviour could become a Rust panic:

* row 1 — indexing `M__BASE`/`M__SHIFT` with `j`. Verified in bounds for **all
  2^32** inputs, including with `debug_assertions` on.
* row 2 — `>> shift` with the table's widest shift (24). Verified never to
  trigger `attempt to shift right with overflow`, with `overflow-checks` on.
* row 3 — the `uint32_t` -> `uint16_t` narrowing. Verified never to trigger
  `attempt to add with overflow`; the maximum sum is exactly `0xffff`.

Because the whole input domain of `float2half` is its single 32-bit argument,
`phase_b_exhaustive.rs` (`CONFIGS.md` row 34) passes **every** input the ABI
admits — valid and invalid alike — through both `.so`s. That makes the error
surface above provably complete: there is no rejectable input it omits.
