# ERRORS.md — Phase A error-surface table

Mechanically derived from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). The greps used, and their results:

| grep over `c_src/src` + `c_src/include`        | hits |
|------------------------------------------------|------|
| `return`                                       | 0    |
| `assert`                                       | 0    |
| `NULL`                                         | 0    |
| `err\|fail\|invalid\|exit\|abort` (-i)         | 0    |
| `if\|switch\|#if\|#ifdef\|?`                   | 1 — only the `#ifndef DRIVER_H_` include guard |
| `MIN\|MAX\|limits`                             | 0    |
| `enum`, `struct`, `typedef`                    | 0    |

So the library contains **no error-return macro, no `return` statement of any
kind (both functions are `void`), no sentinel value, no `assert`, no range
check, no null check, no error enum, and no min/max constant.** The only
conditional in the entire library is the `i < len` loop bound of the
`for` loop in `print_hex`, with `len` hard-wired by the caller to
`sizeof(int)`.

Consequently the error-surface table below has **zero rows derived from the C's
own rejection logic**. The rows that follow are the generic FFI-boundary
boundaries the task mandates be covered even when absent from the table; each
one states what the C *actually* does (never rejects — it always prints
`sizeof(int)` hex bytes and a newline) so the Rust must do the same rather than
panicking, aborting, or returning early.

| #  | function      | trigger (the exact invalid input/condition)                                            | expected C result |
|----|---------------|----------------------------------------------------------------------------------------|-------------------|
| E1 | `driver`      | `x = INT_MIN` (`-2147483648`), i.e. one step past the negative end of the value range   | no rejection: prints `00000080` (LE), newline; returns void |
| E2 | `driver`      | `x = INT_MAX` (`2147483647`), one step past which `int` would overflow                 | no rejection: prints `ffffff7f`, newline |
| E3 | `driver`      | `x = -1` — every object byte is `0xFF`, i.e. all bits set / "all-ones sentinel"         | no rejection: prints `ffffffff`, newline |
| E4 | `driver`      | `x = 0` — the "zero length / zero value" boundary                                       | no rejection: prints `00000000`, newline |
| E5 | `driver`      | object byte `>= 0x80` in any position (e.g. `0x80808080`) — the sign-extension trap: `%02x` receives the byte *promoted from `unsigned char`*, so it must NOT become `ffffff80` | prints exactly two hex digits per byte, e.g. `80808080`, newline |
| E6 | `driver`      | out-of-range value for the parameter's type: an out-of-range *enum-style* int is impossible (the API declares no enum), so the analogous case is a **too-wide argument** — call through the signature `extern "C" fn(u64)` so the 64-bit register carries garbage in bits 32..63 (`0xDEADBEEF_00000001`) | ABI truncation: only the low 32 bits are read; identical output to `driver(1)` |
| E7 | `driver`      | out-of-range value, wider *signed* form: call through `extern "C" fn(i64)` with `i64::MIN` | ABI truncation to `0x00000000`; identical output to `driver(0)` |
| E8 | `driver`      | null pointer / oversized length: **not reachable.** `driver` takes no pointer and no length; `print_hex`'s `p` is always `&x` (never null) and `len` is always `sizeof(int)` (never 0, never oversized). No test can construct this through the public ABI. | N/A — documented as unreachable, not stubbed |
| E9 | `driver`      | repeated invocation without an intervening flush (state-corruption / buffering boundary): 1000 back-to-back calls | 1000 lines, no separator, no truncation, no extra bytes |

Every reachable row (E1–E7, E9) has a differential test in
`translation/tests/differential.rs`; E8 is proven unreachable from the public
ABI by inspection of the C source above rather than by a test.

## Row status

- [x] E1 — `err_e1_int_min`
- [x] E2 — `err_e2_int_max`
- [x] E3 — `err_e3_minus_one_all_ones`
- [x] E4 — `err_e4_zero`
- [x] E5 — `err_e5_high_bit_bytes_no_sign_extension`
- [x] E6 — `err_e6_oversized_unsigned_arg_abi_truncation`
- [x] E7 — `err_e7_oversized_signed_arg_abi_truncation`
- [x] E8 — unreachable by construction (no pointer/length in the public ABI)
- [x] E9 — `err_e9_repeated_calls_no_state_corruption`
