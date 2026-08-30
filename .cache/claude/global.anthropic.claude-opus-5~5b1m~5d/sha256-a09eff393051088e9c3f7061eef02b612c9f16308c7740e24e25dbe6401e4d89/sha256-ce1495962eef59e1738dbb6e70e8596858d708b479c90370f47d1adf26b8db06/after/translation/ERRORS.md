# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.
Greps run over the whole of `c_src/{src,include}`:

| grep pattern | hits (excluding license comments) |
|--------------|-----------------------------------|
| `return` | **0** |
| `assert` | **0** |
| `NULL` / `nullptr` | **0** |
| `errno` / `error` / `fail` / `invalid` | **0** |
| `#define` | 1 — `DRIVER_H_` (header guard only) |
| `enum` | **0** |
| `MAX` / `MIN` | **0** |
| `if` / `switch` / `?:` / `&&` / `||` | **0** |
| `for` / `while` | 1 — `for (int i = 0; i < len; i++)` in `print_hex` |

**Conclusion: the C library has NO error-return surface.** Both functions are
`void`, there are no sentinels, no error enums, no range checks, no null checks,
no asserts, and no early returns. Every input is *accepted*; the only
"rejection-like" behaviour is the loop guard `i < len`, which decides how many
bytes get printed.

The table below therefore enumerates every distinct condition the C code
actually *evaluates or tests*, plus the generic FFI boundary classes the prompt
requires, with an explicit statement of what the C does (which the Rust must
match bit-for-bit).

## Error / boundary-condition table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `print_hex` (internal) | `len > 0` — loop guard true (only reachable case; `driver` always passes `sizeof(float)` = 4) | loop body runs exactly `len` times, then `"\n"`; no error | [x] |
| 2 | `print_hex` (internal) | `len == 0` — loop guard false on entry | prints only `"\n"`; **unreachable** from the public API (`sizeof(float)` is a compile-time 4), so it is not part of the exported behaviour. Rust `print_hex` is likewise `while i < len`, so it agrees by construction | [x] |
| 3 | `print_hex` (internal) | `len < 0` | `0 < len` false ⇒ same as row 2, prints only `"\n"` (no crash, no error). **Unreachable** from the public API; Rust's `while i < len` on `c_int` matches (signed compare, not `usize`) | [x] |
| 4 | `driver` | *null pointer* argument | **N/A — impossible.** `driver` takes `float` by value; the public API accepts no pointer at all. There is nothing to null-check, which is why the C has no null check. Verified: `driver.h` declares exactly `void driver(float x);` | [x] |
| 5 | `driver` | *zero length / oversized length* argument | **N/A — impossible.** No length/count/size parameter exists in the public API; the length is the internal compile-time constant `sizeof(x)` | [x] |
| 6 | `driver` | *out-of-range enum value* across the FFI boundary | **N/A — impossible.** The public API declares no `enum`, `int` flag, or mode parameter (`grep enum` ⇒ 0 hits) | [x] |
| 7 | `driver` | `x` = `+0.0f` (all-zero bit pattern; the "empty" boundary value) | prints `00000000\n` — no special-casing, raw object representation | [x] |
| 8 | `driver` | `x` = `-0.0f` (sign bit set, zero payload — a value `printf("%f")` would flatten but the hex dump must not) | prints `00000080\n` | [x] |
| 9 | `driver` | `x` = `+INFINITY` | prints `0000807f\n` (no error, no check) | [x] |
| 10 | `driver` | `x` = `-INFINITY` | prints `000080ff\n` | [x] |
| 11 | `driver` | `x` = quiet NaN (`0x7fc00000`) | prints `0000c07f\n`; the bit pattern must survive the call unmodified | [x] |
| 12 | `driver` | `x` = **signalling** NaN (`0x7f800001`) — the classic case where a translation silently quietens the payload | prints `0100807f\n`; sNaN payload must NOT be canonicalised by the Rust `extern "C"` wrapper | [x] |
| 13 | `driver` | `x` = NaN with arbitrary non-canonical payloads (both signs, every mantissa/sign combination) | raw bytes printed verbatim; no NaN canonicalisation anywhere | [x] |
| 14 | `driver` | `x` = smallest positive subnormal (`0x00000001`) and largest subnormal (`0x007fffff`) — one step *past* the normal range | prints the literal bytes; no FTZ/denormal flush | [x] |
| 15 | `driver` | `x` = `FLT_MIN` (`0x00800000`) / `FLT_MAX` (`0x7f7fffff`) — the documented range endpoints | literal bytes printed | [x] |
| 16 | `driver` | `x` = value *one step past* each endpoint: `nextafter(FLT_MAX, INF)` ⇒ `+inf`, `nextafter(0, 1)` ⇒ smallest subnormal, `nextafter(FLT_MIN, 0)` ⇒ largest subnormal | literal bytes printed, no clamping or error | [x] |
| 17 | `driver` | `x` = **arbitrary/garbage 32-bit pattern** reinterpreted as `float` (the true "invalid input" for this API — every one of the 2^32 patterns is legal input the C accepts) | the 4 bytes printed low-address-first (little-endian ⇒ LSB first), then `"\n"`; never an error | [x] |
| 18 | `driver` | repeated / interleaved calls (no state to corrupt, no init required, no "not initialised" error path) | each call emits exactly 9 bytes (`8` hex digits + `\n`); output simply concatenates on the shared libc `stdout` | [x] |

## Notes on how "same error/rejection" is asserted

Because there is no error channel, "identical rejection behaviour" is asserted
as: **identical stdout byte stream, identical byte count, and identical
non-crashing (no abort/panic) completion** for both `.so`s, for every trigger
above. A Rust `panic!` (the crate is built with `panic = "abort"` in release)
where C returns normally would abort the test process — so "no crash" is
checked implicitly and by comparing captured output lengths.

Covered by `tests/phase_c_errors.rs`.
