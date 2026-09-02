# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

## How this table was derived

Mechanical grep of the complete C source for every rejection construct:

```
$ grep -nE 'return|assert|NULL|ERROR|errno|exit|abort|if|switch|#if' \
      c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_       <- include guard
c_src/src/driver.c:26:#include <stdbool.h>
c_src/src/driver.c:27:#include <stdio.h>
```

Result: the C library contains **zero** `return` statements with a value, zero
`assert`s, zero `NULL` checks, zero explicit range checks, zero error enums,
zero `RETURN_ERROR`-style macros, zero min/max constants, and zero `if`/`switch`
branches. Both public functions return `void`.

This is *not* a library without an error surface, though. It rejects invalid
input **implicitly**, via C bit-field assignment semantics and via undefined
behaviour. Those implicit rejections are exactly the behaviours the Rust must
reproduce bit-for-bit, so each distinct one gets a row below. Each row was read
off the compiled C (`objdump -d c_src/build/libdriver.so`), which is the
authoritative statement of what the C *actually* does:

```
driver:     and $0x3,%eax          ; x  -> 2 bits
            and $0x7,%eax          ; y  -> 3 bits
            and $0x1,%eax ; shl $5 ; b  -> 1 bit
print_foo:  movzbl (%rax); and $0x3            ; x
            movzbl (%rax); shr $0x2; and $0x7  ; y
            movzbl (%rax); shr $0x5; and $0x1  ; b
            mov 0x4(%rax),%esi                 ; z  (no masking)
```

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `driver` | `x > 3` (out of range for `unsigned int x : 2`), e.g. `x = 4` | No error, no diagnostic. Silently truncated to `x & 3`; prints `0` for the first field. |
| 2 | `driver` | `x = UINT_MAX` (maximum out-of-range value) | Truncated to `x & 3 == 3`; prints `3`. No overflow trap. |
| 3 | `driver` | `y > 7` (out of range for `unsigned int y : 3`), e.g. `y = 8` | Silently truncated to `y & 7`; prints `0` for the second field. |
| 4 | `driver` | `y = UINT_MAX` | Truncated to `y & 7 == 7`; prints `7`. |
| 5 | `driver` | `b` is a non-canonical `_Bool` byte, i.e. a value other than 0 or 1 passed across the FFI boundary (C `_Bool` accepts any byte from a foreign caller; only the low bit is architecturally significant here). E.g. `b = 2` | `and $0x1` masks it: `b = 2` behaves as `false` and prints `0`; `b = 3` behaves as `true` and prints `1`. No trap, no normalisation to 1. |
| 6 | `driver` | `b = 0xFF` (all-bits-set byte, the classic non-canonical bool) | `0xFF & 1 == 1`; prints `1`. |
| 7 | `driver` | `b`'s upper 24 bits of the argument register are non-zero garbage (a foreign caller may leave them dirty; the C prologue does `mov %edx,%eax; mov %al,-0x1c(%rbp)`, keeping only the low byte) | Only the low byte, then only its low bit, is used. Upper bits are ignored, never rejected. |
| 8 | `driver` | `z = INT_MIN` (`-2147483648`, one step past the negative end of the documented `int` range) | Stored verbatim into the non-bit-field `int z`; printed as `-2147483648` by `%d`. No clamping. |
| 9 | `driver` | `z = INT_MAX` | Stored and printed verbatim as `2147483647`. |
| 10 | `driver` | `z` bit pattern reinterpreted: any negative `z` (e.g. `-1`) — `z` is a *signed* `int` and is printed with `%d`, unlike the unsigned bit-fields | Prints the signed value (`-1`), not `4294967295`. |
| 11 | `print_foo` | `foo == NULL` — the C dereferences unconditionally with no null check (`mov -0x8(%rbp),%rax; movzbl (%rax),%eax`) | Undefined behaviour; in practice `SIGSEGV` (signal 11) and process death. There is **no** error return, because the function returns `void`. |
| 12 | `print_foo` | `foo` points at a struct whose *padding* bits are garbage: bits 6–7 of byte 0, and bytes 1–3 (the C `driver` itself leaves these uninitialised — its codegen read-modify-writes byte 0 without zeroing it first) | Padding is never read. Output depends only on bits 0–5 of byte 0 and on bytes 4–7. Garbage padding is not rejected and must not change the output. |
| 13 | `print_foo` | `foo` is a validly-typed pointer but *misaligned* for `int` (`alignof(foo_t) == 4`), e.g. offset by 1 byte | Undefined behaviour per the standard; on x86-64 gcc emits a plain `mov 0x4(%rax),%esi`, so it loads successfully and prints the unaligned `z`. Not rejected. |
| 14 | both | Every byte value 0..=255 in byte 0 of the struct — i.e. the exhaustive set of "out-of-range enum-like" inputs for the packed bit-field allocation unit. There is no valid/invalid partition; the C accepts all 256 and decodes them by masking. | For each byte `n`: prints `n&3`, `(n>>2)&7`, `(n>>5)&1`. No value is rejected. |

## Notes on what is deliberately absent

There are no rows for "invalid format string", "output stream failure", or
"allocation failure": the C never allocates, and it ignores `printf`'s return
value, so a short write or `EBADF` on `stdout` is silently discarded by both
implementations identically (the Rust likewise discards `printf`'s `c_int`).

## Row status (checked off in Phase C)

- [x] 1  [x] 2  [x] 3  [x] 4  [x] 5  [x] 6  [x] 7
- [x] 8  [x] 9  [x] 10 [x] 11 [x] 12 [x] 13 [x] 14

## Test mapping

| `ERRORS.md` row | test in `tests/phase_c_error_paths.rs` |
|---|---|
| 1 | `err01_x_out_of_range_is_silently_truncated` |
| 2 | `err02_x_uint_max` |
| 3 | `err03_y_out_of_range_is_silently_truncated` |
| 4 | `err04_y_uint_max` |
| 5 | `err05_noncanonical_bool_byte` (exhaustive over all 256 byte values) |
| 6 | `err06_bool_all_bits_set` |
| 7 | `err07_bool_dirty_upper_argument_bits_ignored` |
| 8 | `err08_z_int_min` |
| 9 | `err09_z_int_max` |
| 10 | `err10_negative_z_is_printed_signed` |
| 11 | `err11_print_foo_null_pointer_same_fatal_signal` (+ `err11b` control) |
| 12 | `err12_garbage_padding_is_ignored_by_both` |
| 13 | `err13_misaligned_pointer_accepted_not_rejected` |
| 14 | `err14_every_byte0_value_accepted_and_decoded_identically` |
| generic boundaries | `generic_bool_slot_pointer_width_garbage`, `generic_print_foo_page_boundary_and_unmapped`, `generic_no_length_parameters_exist` |

Each implicit-rejection test asserts **two** things: that the C output matches
the masking rule derived independently from `objdump`, and that the Rust output
equals the C output. A shared bug in both implementations therefore cannot make
a row pass.
