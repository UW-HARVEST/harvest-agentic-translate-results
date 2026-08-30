# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep results

```
grep -nE 'return|assert|NULL|errno|exit|abort|<|>|==|!=|if|switch' c_src/src/driver.c
```

Findings, exhaustively:

* `return` statements: **0**. All four functions are `void`.
* error-return macros (`RETURN_ERROR`, `goto fail`, ...): **0**.
* `assert` / `static_assert`: **0**.
* error enums / status codes / `errno` use: **0**.
* explicit range checks, min/max constants, size limits: **0**.
* null-pointer checks: **0** — `printIntPtrLine` dereferences `*intNumber`
  unconditionally.
* the ONLY conditional in the whole library: `if (useGood)` in `driver()`.

So this library has **no explicit, in-band error surface**: it cannot report an
error to its caller because there is no return value and no out-parameter. Its
entire rejection surface is therefore *implicit* — the fault behaviour the C
exhibits when handed a pointer it cannot dereference, plus the intentional
CWE-457 defect. Those are the rows below; each one still gets a differential
test, because "what the C actually does" is the specification.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| 1 | `printIntPtrLine` | `intNumber == NULL` | No check exists; `*intNumber` dereferences address 0 → the process dies from `SIGSEGV` (signal 11) with nothing written to stdout. Must NOT return normally, must NOT print. | `err_01_print_int_ptr_line_null` | [x] |
| 2 | `printIntPtrLine` | `intNumber` = non-null but unmapped address (`0x1`) | No check exists; unaligned+unmapped read → `SIGSEGV`. | `err_02_print_int_ptr_line_unmapped_low` | [x] |
| 3 | `printIntPtrLine` | `intNumber` = non-null unmapped high/non-canonical address (`0xdeadbeefdeadbee0`) | No check exists; `SIGSEGV`. | `err_03_print_int_ptr_line_noncanonical` | [x] |
| 4 | `printIntPtrLine` | `intNumber` = misaligned pointer into a valid mapping (`buf + 1`) | x86-64 permits unaligned loads: this is NOT rejected. Reads the 4 bytes at `buf+1` little-endian and prints that value. Exit status 0. | `err_04_print_int_ptr_line_misaligned` | [x] |
| 5 | `printIntPtrLine` | `intNumber` points at the last 4 valid bytes of a mapping (page-end boundary, one step past = fault) | Not rejected; prints the value. Confirms the read width is exactly 4 bytes (`sizeof(int)`), never 8. | `err_05_print_int_ptr_line_page_end` | [x] |
| 6 | `printIntPtrLine` | `intNumber` points 1 byte PAST the end of a mapping (guard page) | `SIGSEGV`. | `err_06_print_int_ptr_line_past_end` | [x] |
| 7 | `bad` | called at all — `int *data;` is used uninitialised (CWE-457) | Indeterminate: reads whatever the stack slot happens to hold and dereferences it. Either `SIGSEGV` or prints a garbage integer. NOT a defined value; the requirement on Rust is that the defect is *preserved*, i.e. Rust must also read an uninitialised slot and dereference it rather than substituting a safe default or refusing to compile. | `err_07_bad_is_undefined_behaviour` | [x] |
| 8 | `driver` | `useGood == 0` | Not an error return — dispatches to `bad()`, inheriting row 7's undefined behaviour. | `err_08_driver_zero_dispatches_to_bad` | [x] |
| 9 | `driver` | out-of-range "enum-like" `int` values passed across FFI: `INT_MIN`, `INT_MAX`, `-1`, `2`, `0x100`, `0x7fff_ff00` | `useGood` is a plain `int` with no valid-range check, so C accepts any `int`. Truthiness is on the FULL 32-bit value: every non-zero value takes the `good()` branch and prints `5\n`; only exactly `0` takes `bad()`. Values whose low byte is zero (`0x100`, `0x7fff_ff00`) are the trap for a byte/bool-truncation bug. | `err_09_driver_out_of_range_int_values` | [x] |
| 10 | `driver` | high 32 bits dirty: caller passes a 64-bit value (`0x1_0000_0000`) where an `int` is expected | The C ABI says `int` args live in the low 32 bits of the register; the callee ignores the upper half. `0x1_0000_0000` therefore truncates to `0` and dispatches to `bad()`. Rust's `c_int` parameter must truncate identically. | `err_10_driver_dirty_high_bits` | [x] |
| 11 | `good` | no invalid input possible (nullary, no parameters, no allocation, cannot fail) | Always prints `5\n`, exit 0. Included to close the table over the full symbol set. | `err_11_good_cannot_fail` | [x] |

## Notes on how the faulting rows are tested

Rows 1, 2, 3, 6, 7 and 8 can terminate the process, so every row is executed in
a **child process** (`examples/runner.rs`) and compared on
`(stdout bytes, exit code, terminating signal)`. That is what makes "the same
error/rejection" a real assertion — a `SIGSEGV` is compared against a `SIGSEGV`,
and a clean exit with output is compared byte-for-byte, rather than merely
observing that "both failed somehow".
