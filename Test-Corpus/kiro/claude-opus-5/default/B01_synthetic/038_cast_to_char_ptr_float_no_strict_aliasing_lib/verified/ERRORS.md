# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source. The complete code body of the library
(`c_src/src/driver.c`, lines 24–40, after the licence comment) is:

```c
#include "driver.h"
#include <stdio.h>
#include <string.h>

static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) {
        printf("%02x", p[i]);
    }
    printf("\n");
}

void driver(float x) {
    char raw[sizeof(x)];
    memcpy(raw, &x, sizeof(x));
    print_hex((unsigned char *)raw, sizeof(raw));
}
```

## Mechanical grep for rejection constructs

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|if|switch|#ifdef|[<>]=|==|!=' src/driver.c include/driver.h
```

Matches: only `#ifndef DRIVER_H_` / `#endif` (the header include guard) in
`include/driver.h`. **Zero** matches in `src/driver.c`.

Consequently the C library has:

* no error-return macro (`RETURN_ERROR` or similar) — none exists,
* no `return` statement of any kind (both functions are `void`),
* no error enum, no status code, no sentinel value,
* no `assert`, no `abort`, no `exit`,
* no explicit range check, no null check,
* no min/max constant,
* no `errno` use.

`driver` is a `void`-returning function that takes a `float` by value. Every
32-bit pattern is a valid `float` object representation, so **there is no input
the C rejects**: the function has no reachable rejection path. The error surface
is empty by construction.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | — | *(no rejection paths exist — see the grep above)* | — |

## Boundary conditions covered anyway (Phase C tests)

The task requires covering the generic boundaries every C API has even when
absent from the table. Each of these is exercised by a differential test in
`tests/differential.rs`, asserting C and Rust behave **identically** (same
stdout bytes, no crash, no diagnostic):

| # | boundary class | concrete input | expected C result | test |
|---|----------------|----------------|-------------------|------|
| E1 | null pointer | N/A — `driver` takes a `float` by value, there is no pointer parameter anywhere in the public API, so a null pointer is not representable. Documented as not-applicable rather than invented. | n/a | — |
| E2 | zero length | N/A — no length/count parameter in the public API. The only length in the code is `sizeof(x)`, a compile-time constant `4` fixed by `driver`; `print_hex` is `static` and unreachable from outside. | n/a | — |
| E3 | oversized length | N/A — same reason as E2. `len` is never caller-controlled. | n/a | — |
| E4 | out-of-range enum across FFI | N/A — the public API declares no enum, no mode and no flag parameter. | n/a | — |
| E5 | zero | `+0.0f` | prints `00000000` | `err_zero_and_negative_zero` |
| E6 | negative zero (distinct bit pattern from `+0.0`, must not be normalised) | `-0.0f` | prints `00000080` | `err_zero_and_negative_zero` |
| E7 | one step past the largest finite value | `f32::MAX` then `+inf` (`0x7f7fffff` → `0x7f800000`) | prints the raw bytes, no clamping | `err_past_finite_range` |
| E8 | one step past the most negative finite value | `f32::MIN` then `-inf` (`0xff7fffff` → `0xff800000`) | prints the raw bytes, no clamping | `err_past_finite_range` |
| E9 | infinities | `+inf`, `-inf` | `0000807f`, `000080ff` | `err_infinities` |
| E10 | quiet NaN | `0x7fc00000` | `0000c07f` | `err_nan_payloads` |
| E11 | signalling NaN (must not be quieted in transit) | `0x7fa00000`, `0xffa00000` | `0000a07f`, `0000a0ff` | `err_nan_payloads` |
| E12 | NaN with a non-canonical payload one step past the canonical one | `0x7fc00001`, `0x7f800001` (smallest sNaN), `0x7fffffff` (max payload) | raw bytes, payload preserved | `err_nan_payloads` |
| E13 | smallest subnormal / one step past zero | `0x00000001`, `0x80000001` | `01000000`, `01000080` | `err_subnormal_boundaries` |
| E14 | largest subnormal and the subnormal→normal step | `0x007fffff` → `0x00800000` | raw bytes | `err_subnormal_boundaries` |
| E15 | byte values that stress `%02x` zero padding (bytes `< 0x10`) | patterns like `0x01020304`, `0x00000000`, `0x0f0f0f0f` | two hex digits per byte, zero padded | `err_hex_padding` |
| E16 | byte values `>= 0x80` (the sign-extension trap: C casts `char raw[]` to `unsigned char *`, so bytes must print unsigned, e.g. `ff` not `ffffffff`) | `0xffffffff`, `0x80808080`, patterns with mixed high-bit bytes | each byte as exactly 2 unsigned hex digits | `err_high_bit_bytes` |
| E17 | every possible individual byte value in every byte position | `0x000000NN`, `0x0000NN00`, `0x00NN0000`, `0xNN000000` for all `NN` in `0..=255` | raw bytes | `err_all_byte_values_all_positions` |
| E18 | repeated / interleaved calls (shared `stdout` FILE state across both `.so`s) | alternating C and Rust calls without an intervening flush | identical, independently correct lines | `err_interleaved_calls` |
