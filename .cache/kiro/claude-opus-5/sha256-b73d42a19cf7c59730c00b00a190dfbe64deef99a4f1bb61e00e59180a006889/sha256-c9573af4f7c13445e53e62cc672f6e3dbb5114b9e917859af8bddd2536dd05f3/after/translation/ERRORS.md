# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Every error-signalling construct was grepped out of the complete C source
(`c_src/src/driver.c`, 40 lines; `c_src/include/driver.h`, 29 lines):

```sh
grep -nE 'return|RETURN_ERROR|assert|NULL|errno|exit\(|abort|if *\(|switch|#ifdef|#if |enum' \
    c_src/src/driver.c c_src/include/driver.h
```

Result, with license-comment lines removed, is a **single** line — and it is a
loop guard, not an error path:

```
c_src/src/driver.c:30:    for (int i = 0; i < len; i++) {
```

Therefore, mechanically:

| construct searched for | occurrences in C source |
|------------------------|-------------------------|
| `return` (any value or bare) | 0 |
| `RETURN_ERROR` / error macro | 0 |
| `assert` | 0 |
| `NULL` / null check | 0 |
| error `enum` / status code type | 0 |
| explicit range check (`if`, `switch`) | 0 |
| `errno`, `exit()`, `abort()` | 0 |
| min/max constant | 0 |
| `#ifdef` / `#if` compile-time branch | 0 (other than the `DRIVER_H_` include guard) |
| conditional of any kind | 1 — the `i < len` loop guard on line 30 |

`driver` returns `void`, takes one by-value `int`, dereferences no
caller-supplied pointer, and allocates nothing. It has **no reject path and no
error channel**: there is no input value of `int` for which it fails, and no
sentinel or error code it can return. `print_hex` has internal linkage and its
only call site passes `(&raw[0], 4)`, both always valid.

Consequently the error-surface table below has no rows derived from C reject
logic (there are none to derive). What it does contain is the set of generic
ABI boundary conditions the prompt requires be covered anyway, each mapped to
the concrete form it takes for *this* ABI. Each row's "expected C result" is
established by observing the C `.so`, and the differential test asserts the
Rust `.so` produces the identical observable result — same stdout bytes, same
(void) return, same non-crash.

## Error / boundary surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | `x = 0` — the zero/empty-value boundary; every byte is `0x00`, exercising `%02x` zero-padding on both nibbles | no error; prints `00000000\n`, returns `void` | `err_e1_zero` | [x] |
| E2 | `driver` | `x = INT_MAX` (`2147483647`) — largest valid `int` | no error; prints `ffffff7f\n` | `err_e2_int_max` | [x] |
| E3 | `driver` | `x = INT_MIN` (`-2147483648`) — smallest valid `int`; sign bit set, one step past `INT_MAX` in wrapping terms | no error; prints `00000080\n` | `err_e3_int_min` | [x] |
| E4 | `driver` | `x = -1` — all four bytes `0xff`, i.e. every byte is negative *as `signed char`*. The C stores into `char raw[]` (signed on x86-64) but prints through an `unsigned char *`, so the values passed to `%02x` are 255, not sign-extended `-1`. A translation that sign-extends emits `ffffffff` per byte instead of `ff` | no error; prints `ffffffff\n` (8 hex digits, NOT 32) | `err_e4_all_high_bytes` | [x] |
| E5 | `driver` | every `x` whose bytes are individually `>= 0x80` (high-bit-set byte in each of the 4 positions, e.g. `0x80808080`, `0xff000000`, `0x008000ff`) — the sign-extension class of E4, swept per byte position | no error; each byte printed as exactly 2 hex digits | `err_e5_high_bit_per_byte_position` | [x] |
| E6 | `driver` | `x` whose bytes are individually `< 0x10` (e.g. `0x01020304`, `0x0f000000`) — omitted `0` flag / width would print 1 digit and desynchronise the whole string | no error; each byte printed as exactly 2 zero-padded hex digits | `err_e6_zero_padding_per_byte_position` | [x] |
| E7 | `driver` | **out-of-range argument bit pattern across the FFI boundary.** This is the analog of "out-of-range enum value": `driver` takes an `int`, but C's ABI accepts whatever bits the caller places in the argument register. The symbol is called through a mis-declared `extern "C" fn(i64)` so the upper 32 bits of `rdi` are garbage that no valid `int` could produce (`0x7fff_ffff_dead_beef`, `-1i64`, `i64::MIN`, …). The callee must observe only the low 32 bits | no error; both libraries truncate to the low 32 bits and print those 4 bytes; C and Rust agree bit-for-bit | `err_e7_oversized_argument_truncation` | [x] |
| E8 | `driver` | **`int` value with no "valid variant".** Restated for completeness: unlike a C `enum`, `int` has no invalid variant — the full `[INT_MIN, INT_MAX]` range is valid input and none of it is rejected. Verified by sweeping the boundary values and randomized values rather than assuming | no error for any of the 2^32 inputs; no rejection path exists | `err_e8_no_value_is_rejected` | [x] |
| E9 | `driver` | **no pointer parameter exists**, so a null-pointer row is not constructible for the public API. The only pointer in the C is `print_hex`'s `p`, which has internal linkage and a single call site passing `&raw[0]`; it is unreachable with `NULL` from any external caller. Asserted structurally: `nm -D` shows `print_hex` is not exported by *either* `.so`, so neither library can be made to null-deref through its public surface | not constructible; `print_hex` absent from both dynamic symbol tables | `err_e9_no_null_pointer_surface` | [x] |
| E10 | `driver` | **no length parameter exists**, so zero-length and oversized-length rows are not constructible for the public API. `print_hex`'s `len` is always the compile-time constant `sizeof(int)` (= 4, verified on this target). The `len <= 0` branch of the `i < len` guard — the source's only conditional — is therefore dead code in the C; both libraries must still emit the trailing `\n` unconditionally, which is what the 4-byte path already proves | not constructible; `len` is always 4; trailing newline always emitted | `err_e10_no_length_surface` | [x] |
| E11 | `driver` | repeated / interleaved invocation — calling the C and Rust symbols alternately in one process, so both write into the *same* libc `stdout` `FILE`. A translation that used Rust's own buffered `std::io::stdout` instead of libc `printf` would interleave or lose bytes here even though each call looks correct in isolation | no error; output is strictly the concatenation of each call's 9 bytes, in call order, with no reordering or loss | `err_e11_interleaved_c_and_rust_calls` | [x] |

**All 11 rows have a passing differential test. 0 rows unchecked.**
