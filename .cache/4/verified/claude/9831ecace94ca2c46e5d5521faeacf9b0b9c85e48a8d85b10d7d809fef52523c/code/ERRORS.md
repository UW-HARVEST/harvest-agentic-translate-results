# ERRORS.md — Phase C: the ERROR-SURFACE TABLE

Derived **mechanically** from the C source, not from docs or assumptions.

## Mechanical grep of every rejection construct in `c_src/`

```sh
grep -nE 'RETURN_ERROR|return[[:space:]]+-|return[[:space:]]+NULL|assert|errno|exit\(|abort|\
_MAX|_MIN|if[[:space:]]*\(|switch|goto|\?|enum' c_src/src/driver.c c_src/include/driver.h
```

Result: **no matches.** The complete non-comment C source is:

```c
#include "driver.h"
#include <stdio.h>

static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) {
        printf("%02x", p[i]);
    }
    printf("\n");
}

void driver(float x) {
    print_hex((unsigned char *)&x, sizeof(x));
}
```

Therefore, factually:

* `driver` returns `void` — there is **no** error code, no sentinel, no
  out-parameter status. It cannot report failure.
* There is **no** `assert`, no `NULL` check, no explicit range check, no
  `_MIN`/`_MAX` constant, no `errno` use, no `exit`/`abort`.
* There is **no** `enum` and no `int`-typed mode/flag parameter, so there is no
  "out-of-range enum value across FFI" variant to reject.
* The public API takes **no pointers**, so there is no null-pointer path
  (`float` is passed by value; the only pointer, `&x`, is produced internally
  and is never null).
* The single conditional in the whole library is the `for` loop guard
  `i < len`, and `len` is always the compile-time constant `sizeof(float) == 4`
  because `print_hex` is `static` and has exactly one call site.

So the library's "error surface" consists only of (a) the degenerate loop-guard
condition and (b) failures of the underlying `stdout` stream, which the C
**deliberately ignores** (it discards `printf`'s return value). Every row below
is one of those, plus the generic FFI boundaries the prompt requires be covered
even when they are not in the source. "Expected C result" is the behaviour the C
**actually** has — for this library that is almost always *"no error: emit the 4
object-representation bytes as `%02x` then `\n`"*, and the Rust must match that
non-rejection exactly.

## The table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| E1 | `driver` | *No* error return exists anywhere: `void driver(float)` and `static void print_hex(...)` both return `void`; `printf`'s `int` return is discarded at both call sites. | Nothing is ever rejected. Every call emits exactly 9 bytes (`8` hex digits + `\n`) and returns normally. Rust must also return normally and emit 9 bytes. |
| E2 | `print_hex` (internal) | Loop guard `i < len` with `len <= 0`. Unreachable from the public ABI (`len` is hard-coded `sizeof(float)`), so the only *observable* consequence is that the hex-digit count can never be anything but 8. | 8 hex digits always; the `len <= 0` branch (bare `\n`) is dead code and must be dead in Rust too — i.e. `driver` must never print a lone newline. |
| E3 | `driver` | Quiet NaN, `f32::NAN` (bits `0x7FC00000`). Not rejected — but a naive translation could canonicalise or fold it. | No error; prints the exact object representation `0000c07f` (little-endian). |
| E4 | `driver` | **Signalling** NaN (e.g. bits `0x7F800001`, `0xFF800001`). C never inspects the value, so the sNaN bits must survive the `xmm0` parameter pass and the `&x` spill unchanged. | No error; prints the exact bits, e.g. `0100807f`. Must NOT be quieted to `…c07f`. |
| E5 | `driver` | Arbitrary NaN **payloads** (`0x7F800001..0x7FFFFFFF`, `0xFF800001..0xFFFFFFFF`) — bit patterns with no unique numeric value. | No error; the payload bits are printed verbatim, byte for byte. |
| E6 | `driver` | `+INFINITY` / `-INFINITY` (bits `0x7F800000` / `0xFF800000`) — values outside the finite range. | No error; prints `0000807f` / `000080ff`. |
| E7 | `driver` | `-0.0` (bits `0x80000000`) — numerically equal to `+0.0`, so any value-based (rather than bit-based) translation diverges here. | No error; prints `00000080`, distinct from `+0.0`'s `00000000`. |
| E8 | `driver` | `+0.0` (bits `0x00000000`) — all-zero bytes exercise `%02x`'s zero padding; a `%x`-style translation would print `0` instead of `00`. | No error; prints exactly `00000000` (8 characters). |
| E9 | `driver` | Subnormal / denormal magnitudes below `FLT_MIN` (bits `0x00000001` = `1e-45`, up to `0x007FFFFF`), i.e. one step past the normal-range boundary. | No error; exact bits printed; no flush-to-zero. |
| E10 | `driver` | Range boundaries one step past the extremes: `FLT_MAX` `0x7F7FFFFF`, next step `0x7F800000` (=inf), `FLT_MIN` `0x00800000`, next step down `0x007FFFFF` (subnormal), and `f32::EPSILON`. | No error; exact bits printed for each. |
| E11 | `driver` | Bytes with the high bit set (any byte `>= 0x80`, e.g. bits `0xFFFFFFFF`, `0x80808080`). In C, `p[i]` is `unsigned char` promoted to `int`; a translation using a *signed* byte would print `ffffff80`-style sign-extended garbage. | No error; each byte prints as exactly 2 lowercase hex digits, e.g. `ffffffff`, `80808080`. |
| E12 | `driver` | Every one of the 256 possible byte values, in every one of the 4 byte positions (the `%02x` formatting domain, exhaustively). | No error; lowercase, zero-padded, no `0x` prefix, no separators, in ascending address order. |
| E13 | `driver` | **`stdout` write failure — `ENOSPC`.** `fd 1` redirected to `/dev/full`. `printf`/`putchar` fail, and the C **ignores** the failure. | No crash, no abort, `driver` returns normally; the failure is silently swallowed and surfaces only later via `fflush` returning `EOF` / `ferror(stdout)` being set. Rust must swallow it identically (same `fflush` return, same `ferror` flag). |
| E14 | `driver` | **`stdout` write failure — `EBADF`.** `fd 1` made a *read-only* descriptor (`open("/dev/null", O_RDONLY)` dup2'd onto 1) underneath the still-open `FILE*`, so every `write` is rejected with `EBADF`. | Same as E13: silently ignored, returns normally, error state observable only through `fflush`/`ferror` (`errno == EBADF`). |
| E15 | `driver` | `stdout` in its *error* state already (sticky `ferror` set) before the call. | No error; the call is still made and still returns normally; C does not clear or check the flag. |
| E16 | `driver` | Called with `fd 1` pointing at `/dev/null` (output discarded) and with `stdout` **unbuffered** (`setvbuf(_IONBF)`) — the write path changes from buffered to one syscall per `printf`. | No error; identical byte stream reaches the fd in both buffering modes. |
| E17 | `driver` | Out-of-range *enum* value passed across FFI. **Not applicable by construction**: the public ABI has no `enum`, no `int`, and no flag parameter — the sole parameter is `float`, whose every 32-bit pattern is a legal input already covered by E3–E12. | N/A — the whole 2^32 input domain is valid; rows E3–E12 + the randomised full-bit-space sweep cover it, so there is no "no valid variant" value to diverge on. |
| E18 | `driver` | Null / oversized *pointer or length* arguments. **Not applicable by construction**: `driver` takes no pointer and no length. The internal `&x` can never be null and `len` is a constant. | N/A — no pointer/length parameter exists to abuse. |
| E19 | `driver` | Many consecutive calls (thousands) with `stdout` fully buffered so the 4 KiB/`BUFSIZ` buffer boundary is crossed mid-`driver`, splitting one line's bytes across two `write` syscalls. | No error; the concatenated output is exactly the per-call outputs in call order, with no lost, duplicated or reordered bytes. |

## Status

Every row above has a differential test in `tests/differential.rs` that
constructs that exact condition, calls **both** the C `.so` and the Rust `.so`
through `libloading`, and asserts identical observable results (identical
captured bytes, and for E13–E15 identical `fflush` return value and identical
`ferror(stdout)` flag — not merely "both failed somehow").

| row | test | status |
|-----|------|--------|
| E1 | `e1_no_error_return_ever` | [x] |
| E2 | `e2_never_emits_bare_newline` | [x] |
| E3 | `e3_quiet_nan` | [x] |
| E4 | `e4_signalling_nan_bits_preserved` | [x] |
| E5 | `e5_nan_payloads_random` | [x] |
| E6 | `e6_infinities` | [x] |
| E7 | `e7_negative_zero` | [x] |
| E8 | `e8_positive_zero_padding` | [x] |
| E9 | `e9_subnormals` | [x] |
| E10 | `e10_range_boundaries_one_step_past` | [x] |
| E11 | `e11_high_bit_bytes` | [x] |
| E12 | `e12_all_256_byte_values_in_all_4_positions` | [x] |
| E13 | `e13_stdout_enospc_dev_full` | [x] |
| E14 | `e14_stdout_ebadf_read_only_fd` | [x] |
| E15 | `e15_preexisting_error_state` | [x] |
| E16 | `e16_unbuffered_and_devnull` | [x] |
| E17 | `e17_full_32bit_input_domain_is_valid` | [x] |
| E18 | `e18_no_pointer_or_length_parameters` | [x] |
| E19 | `e19_buffer_boundary_many_calls` | [x] |
