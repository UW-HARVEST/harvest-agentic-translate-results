# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`
(43 + 30 lines, the library's *entire* source), by grepping for every
rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR|\bif\b|\bswitch\b|<|>|\?' \
    src/driver.c include/driver.h
grep -nE '#\s*(if|ifdef|ifndef|elif|else|endif|define)' src/driver.c include/driver.h
```

## Result of the grep

* **0** `return` statements (both functions are `void`; `driver` and
  `print_foo` fall off the end).
* **0** `assert` / `abort` / `exit` calls.
* **0** `NULL` checks — `print_foo` dereferences `foo` unconditionally.
* **0** `if` / `switch` / ternary / loop — the code is straight-line.
* **0** error enums, error codes, sentinel returns, or `errno` use.
* **0** configuration macros (only the `DRIVER_H_` include guard).
* **0** explicit range checks and **0** named min/max constants.

**There is no explicit error surface: neither function can report an error.**
The library therefore has no error *codes* to compare. What it does have are
*implicit* input constraints that silently transform out-of-range input, plus
one unchecked pointer dereference. Those are the rejection paths, so each gets
a row below. "Expected C result" is stated as the exact observable behaviour
(the bytes printed to `stdout`, or the fault), since that is this library's
only output channel.

The bit-field widths (`x : 2`, `y : 3`, `b : 1`) are the de-facto range checks:
they are the only constants in the file that constrain input, so every one gets
a row at and one step past its boundary. Truncation semantics were confirmed
against the C machine code (`and $0x3` / `and $0x7` / `and $0x1`), not guessed.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `print_foo` | `foo == NULL` — no null check, `foo->x` dereferences it | fatal `SIGSEGV` (signal 11), no output; **not** an error return | `err01_print_foo_null_segv_both` | [x] |
| 2  | `driver` | `x = 4` — one past the `x : 2` field max (3) | silently truncated to `x & 3 == 0`; prints `0 …` | `err_driver_truncation_boundaries` | [x] |
| 3  | `driver` | `x = UINT_MAX` — maximally out of range for `x : 2` | truncated to `3` | `err_driver_truncation_boundaries` | [x] |
| 4  | `driver` | `y = 8` — one past the `y : 3` field max (7) | silently truncated to `y & 7 == 0` | `err_driver_truncation_boundaries` | [x] |
| 5  | `driver` | `y = UINT_MAX` — maximally out of range for `y : 3` | truncated to `7` | `err_driver_truncation_boundaries` | [x] |
| 6  | `driver` | `b = 2` — a non-`0`/`1` `_Bool`, i.e. an out-of-range enum-like value crossing the FFI boundary | GCC masks bit 0 only (`and $0x1`); it does **not** test for non-zero, so the field becomes **`0`**, printing `0` | `err_driver_bool_out_of_range_all_ints` | [x] |
| 7  | `driver` | `b` = every other out-of-range byte `3..=255` (incl. `0xFF`) | field = `b & 1`, i.e. odd byte → `1`, even byte → `0` | `err_driver_bool_out_of_range_all_ints` | [x] |
| 8  | `driver` | `b` passed as a full 32-bit int whose low byte is `0` but which is non-zero (`0x100`, `0x1FE`, …) | callee reads only the low byte, then bit 0 → field `0` | `err_driver_bool_out_of_range_all_ints` | [x] |
| 9  | `driver` | `z = INT_MIN` — boundary of the `int` member | printed verbatim by `%d`: `-2147483648` | `err_driver_z_extremes` | [x] |
| 10 | `driver` | `z = INT_MAX` / `z = -1` — remaining `int` boundaries | printed verbatim: `2147483647` / `-1` | `err_driver_z_extremes` | [x] |
| 11 | `print_foo` | storage byte with the *padding* bits 6..7 set (`0xC0`), i.e. a struct holding bits no field owns | padding is never read; output depends only on bits 0..5 | `err_print_foo_padding_bits_ignored` | [x] |
| 12 | `print_foo` | misaligned `foo_t*` (`_Alignof(foo_t) == 4`, pointer at offset 1/2/3) | x86-64 tolerates it; same output as the aligned case | `err_print_foo_misaligned_pointer` | [x] |

Rows 2–10 are "invalid input" in the sense that the values cannot be
represented in the target field/type; the C's response is silent truncation
rather than rejection, and the Rust must reproduce that truncation **bit for
bit** — including row 6, the classic case a happy-path test misses.

## Generic FFI boundaries also covered (beyond the table)

| boundary | how it is exercised | test |
|---|---|---|
| null pointer | `print_foo(NULL)` in a forked child, comparing wait-status | `err01_print_foo_null_segv_both` |
| out-of-range "enum"/`_Bool` value | **all** 256 byte values + non-zero-low-byte 32-bit values for `b`, via a common `extern "C" fn(u32,u32,u32,i32)` prototype so the raw ABI (not a Rust `bool`) is what is tested | `err_driver_bool_out_of_range_all_ints` |
| values one step past a valid range | `x = 4`, `y = 8`, `b = 2` (rows 2/4/6) | `err_driver_truncation_boundaries` |
| oversized / max values | `x`/`y`/`b` = `UINT_MAX`, `z` = `INT_MIN`/`INT_MAX` | `err_driver_truncation_boundaries`, `err_driver_z_extremes` |
| zero values | `x = y = b = z = 0` | `cfg01_driver_all_zero` |
| exhaustive small domain | all 2^32-representable storage bytes: all 256 values of byte 0 | `err_print_foo_padding_bits_ignored` |

There is no length/size parameter anywhere in this API, so "zero and oversized
lengths" does not apply; the pointer-and-struct equivalents are covered above.

## Status

All 12 rows plus every generic boundary have a passing differential test.
Run: `cargo test --test phase_c_errors` (9 tests, all green under both the
debug and the release cdylib).

## Bug found by this phase

**Row 1 (`print_foo(NULL)`) exposed a real divergence, now fixed.**

The original translation read the struct through Rust pointer operations. Rust
1.94 inserts debug-only soundness checks that C does not have, so the Rust
`.so` terminated with `SIGABRT` (signal 6) where the C `.so` raises `SIGSEGV`
(signal 11):

* `&*foo` → "misaligned pointer dereference" abort,
* `ptr::read_unaligned` → "unsafe precondition violated: … non-null" abort,
* plain `*ptr` → "null pointer dereference occurred" abort.

`print_foo` now copies the 8-byte image with libc `memcpy`, an opaque
`extern "C"` call that carries none of those checks, so it faults exactly where
and how the C does. Note this was a *debug-build* divergence: it would have
been invisible had the suite only exercised the optimized artifact, which is
why Phase D runs every row against both profiles.
