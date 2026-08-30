# ERRORS.md — Phase A error-surface table

Derived **mechanically** from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep evidence

```sh
$ grep -nE 'return|assert|NULL|errno|exit|abort|if|switch|#if|==|!=|MAX|MIN|error|ERROR' -r c_src/src c_src/include
c_src/include/driver.h:7:// modify, merge, publish, distribute, sublicense,   # comment
c_src/include/driver.h:24:#ifndef DRIVER_H_                                   # header guard
c_src/include/driver.h:29:#endif //DRIVER_H_                                  # header guard
c_src/src/driver.c:7:// modify, merge, publish, distribute, sublicense,       # comment
c_src/src/driver.c:26:#include <stdio.h>                                      # include
```

Every hit is a comment, a header guard, or an `#include`. Therefore, in the
entire library:

* **0** `return` statements of any kind (both functions are `void`; neither has
  an explicit `return`).
* **0** error-return macros / enums / sentinels (`RETURN_ERROR`, `return -1`,
  `return NULL`, …).
* **0** `assert` / `abort` / `exit`.
* **0** `if` / `switch` / conditional expressions — no explicit range checks.
* **0** pointer parameters anywhere in the public API ⇒ **no null checks and no
  null-pointer-dereference path**.
* **0** length / size / count parameters ⇒ no zero-length or oversized-length
  path.
* **0** enum types in the API ⇒ no named-variant validation. (See row 6 for the
  out-of-range *integer* analogue, which does exist at the ABI level.)
* **0** `MIN`/`MAX` constants.
* The one library call, `printf`, has its return value **ignored** — so even a
  write failure is not turned into an error.

**The C library has an empty explicit error surface: no input is ever
rejected.** Consequently every row below is a row about *implicit* /
ABI-level rejection-equivalents — the generic C-API boundaries Phase C
mandates covering even when they are absent from the source. The "expected C
result" for each is therefore *"no rejection; specific defined output"*, and
the differential requirement is that Rust produce the **same** non-rejection
and the same bytes.

## Error / rejection surface

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `printHexCharLine` | no validation exists — every one of the 256 `char` bit patterns is accepted | never returns an error (`void`); prints `printf("%02x\n", (int)charHex)`. For `charHex >= 0`: two-or-more hex digits, zero-padded to 2. For `charHex < 0` (signed `char` on x86-64): the value sign-extends to a negative `int` which `%x` reinterprets as `unsigned`, printing **8** hex digits, e.g. `-1` → `ffffffff` |
| 2 | `driver` | no validation exists — every one of the 256 `char` bit patterns is accepted | never returns an error (`void`); computes `char result = data + 1` and forwards to row 1 |
| 3 | `driver` | signed-overflow boundary `data == 0x7f` (127): `data + 1` is computed in `int` as `128`, then narrowed to `char`, which is *implementation-defined*, not an error | gcc/clang narrow by two's-complement truncation ⇒ `result == -128` ⇒ prints `ffffff80\n`. **No trap, no error.** (Disassembly confirms `movzbl` → `add $1` → `mov %al`.) |
| 4 | `driver` | wrap-to-zero boundary `data == 0xff` (`-1`): `data + 1 == 0` | prints `00\n` — the only case where `%02x` zero-padding is observable at both digits. No error. |
| 5 | `printHexCharLine` | one step past the *positive* `char` range as seen by `%02x` single-digit padding: `charHex == 0x00 .. 0x0f` | prints a leading `0` then one hex digit (`00`…`0f`). No error. |
| 6 | `driver`, `printHexCharLine` | **out-of-range value passed across the FFI boundary**: the C prototype takes `char`, but C ABI-wise the caller passes a full 32-bit register. A caller declaring the symbol as `void f(int)` and passing e.g. `0x1234_5678`, `-1`, `256`, `INT_MIN`, `INT_MAX` supplies a value with no valid `char` representation. This is the enum-out-of-range analogue for this API. | **not rejected.** The callee ignores the upper 24 bits (`mov %edi,%eax; mov %al,…`), i.e. it truncates to the low byte and proceeds as rows 1–5. So `driver(0x1234_5678)` behaves exactly as `driver(0x78)`. Rust must truncate identically. |
| 7 | `printHexCharLine` | stdout is closed / unwritable, so `printf` fails and returns negative | return value is **ignored**; function still returns `void` normally, no error propagated, no `errno` check |

### Row 6 found a real bug

Row 6 is the row that paid off. The Rust translation originally declared its
exports as `extern "C" fn(c_char)`, which lets LLVM assume the caller
sign-extended and, in `--release`, drop gcc's re-truncation of the argument
register: `printHexCharLine` called as `fn(int)` with `255` printed `ff` in Rust
but `ffffffff` in C. See "Divergence found and fixed: argument-register
truncation" in `SYMBOLS.md`. The bug was invisible in debug builds, so the
suite is run against both profiles.

## Row → test mapping (Phase C)

| row | test in `translation/tests/phase_c_errors.rs` |
|-----|--------------------------------------------|
| 1 | `err_row1_print_all_256_bit_patterns_accepted` |
| 2 | `err_row2_driver_all_256_bit_patterns_accepted` |
| 3 | `err_row3_driver_signed_overflow_boundary_0x7f` |
| 4 | `err_row4_driver_wrap_to_zero_0xff` |
| 5 | `err_row5_print_single_hex_digit_padding` |
| 6 | `err_row6_out_of_range_int_across_ffi_truncates` |
| 7 | `err_row7_printf_failure_is_ignored` |
