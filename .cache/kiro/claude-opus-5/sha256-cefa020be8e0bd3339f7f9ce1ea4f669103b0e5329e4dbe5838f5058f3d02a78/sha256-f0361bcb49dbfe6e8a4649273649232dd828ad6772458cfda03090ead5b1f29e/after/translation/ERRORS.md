# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical derivation

Greps run over the whole of `c_src/` (`src/driver.c`, `include/driver.h`):

| grep | result |
|---|---|
| `grep -n 'return' src/*.c include/*.h` | **no matches** (exit 1) |
| `grep -niE 'assert\|NULL\|errno\|error\|-1\|exit\(\|abort' src/*.c include/*.h` | **no matches** (exit 1) |
| `grep -nE '\b(if\|else\|switch\|case)\b\|#if\|#ifdef\|#ifndef' src/*.c include/*.h` | 1 match: `include/driver.h:24:#ifndef DRIVER_H_` (header include guard only) |

Consequences, all verified against the source:

- There is **no error-return macro** (`RETURN_ERROR` or similar), no error enum,
  no `return -1`, no `return NULL`, no `errno` use, no `assert`, no `exit`/`abort`.
- Both functions return `void`, so there is **no error channel at all**:
  `void driver(float x)` and `static void print_hex(unsigned char *, int)`.
- There is **no explicit range check, null check, or min/max constant** anywhere.
- The only conditional in the C is the `for (i = 0; i < len; i++)` loop bound and
  the header's `#ifndef` include guard.

So the error surface of this library is **empty**: the C code rejects nothing.
Every row below therefore records a *would-be* rejection point that the C
deliberately does NOT reject, plus the generic FFI boundary conditions the task
requires be covered anyway. "Expected C result" is what the C actually does, and
the Rust must do exactly the same — including not erroring.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `driver` | `x` = quiet NaN (`0x7fc00000`) — no validity check exists | no error; prints `0000c07f` + `\n`; returns normally | `err_row01_quiet_nan` | [x] |
| 2 | `driver` | `x` = negative quiet NaN (`0xffc00000`) | no error; prints `0000c0ff` + `\n` | `err_row02_negative_quiet_nan` | [x] |
| 3 | `driver` | `x` = signalling NaN (`0x7fa00000`) — passed in xmm0, never dereferenced as a number | no error; prints `0000a07f` + `\n` | `err_row03_signalling_nan` | [x] |
| 4 | `driver` | `x` = NaN with every payload bit set / odd payloads (`0x7fffffff`, `0x7f800001`, `0xffbfffff`) | no error; NaN payload is NOT canonicalised — exact bytes printed (`ffffff7f`, `0100807f`, `ffffbfff`) | `err_row04_nan_payload_preserved` | [x] |
| 5 | `driver` | `x` = `+INFINITY` (`0x7f800000`) | no error; prints `0000807f` + `\n` | `err_row05_pos_infinity` | [x] |
| 6 | `driver` | `x` = `-INFINITY` (`0xff800000`) | no error; prints `000080ff` + `\n` | `err_row06_neg_infinity` | [x] |
| 7 | `driver` | `x` = negative zero (`0x80000000`) — indistinguishable from `+0.0` by value comparison, so a value-based translation would diverge here | no error; prints `00000080` + `\n` | `err_row07_negative_zero` | [x] |
| 8 | `driver` | `x` = smallest subnormal (`0x00000001`) and largest subnormal (`0x007fffff`) | no error; no flush-to-zero; exact bytes printed | `err_row08_subnormals` | [x] |
| 9 | `driver` | `x` = `FLT_MAX` (`0x7f7fffff`), `-FLT_MAX`, `FLT_MIN` (`0x00800000`), `FLT_EPSILON` (`0x34000000`) — one step past / at the representable range boundary | no error; exact bytes printed | `err_row09_flt_limits` | [x] |
| 10 | `driver` | every byte of the representation `< 0x10`, i.e. the `%02x` zero-pad path (`0x01010101`, `0x00000000`, `0x0f0f0f0f`) — a missing pad would silently shorten the output | no error; each byte printed as exactly 2 chars, leading `0` kept | `err_row10_zero_pad_low_bytes` | [x] |
| 11 | `driver` | every byte `>= 0x80`, i.e. the sign-extension trap: `unsigned char` promoted to `int` for the variadic `printf` — a signed promotion would print `ffffffxx` | no error; exactly 2 hex chars per byte, no sign extension | `err_row11_high_bytes_no_sign_extend` | [x] |
| 12 | `driver` | `print_hex`'s `len` parameter: the sole loop bound. It is hard-wired to `sizeof(float)` == 4 by the only caller and is unreachable with any other value from the public API (`print_hex` is `static`, absent from `nm -D` on both `.so`s) | no error; unreachable from outside; output is always exactly 4 bytes → 8 hex chars + `\n` = 9 bytes | `err_row12_output_length_always_9` | [x] |
| 13 | `driver` | null-pointer boundary: the public API takes `float` **by value**, never a pointer, so no null pointer can be supplied. `print_hex`'s `p` is always `&x` on the caller's stack and is never NULL | no error; a null-pointer rejection cannot be triggered — asserted structurally (no pointer parameter in the ABI) | `err_row13_no_pointer_parameter` | [x] |
| 14 | `driver` | out-of-range enum boundary: the C API declares **no enum, no flag, no mode parameter**, so there is no enum value to push out of range. The full `int`-space of the argument is instead covered by feeding all 2^32 bit patterns as `float` | no error; every one of the 2^32 argument bit patterns is a valid input the C accepts | `err_row14_all_bit_patterns_are_valid` | [x] |
| 15 | `driver` | repeated / interleaved calls sharing the process-wide `stdout` FILE stream — the only global state touched | no error; no state carried between calls; N calls emit exactly N 9-byte lines | `err_row15_shared_stdout_no_state` | [x] |

**Rows: 15. Unchecked: 0.**
