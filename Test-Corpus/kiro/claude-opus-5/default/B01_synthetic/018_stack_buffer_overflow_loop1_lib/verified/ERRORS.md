# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h` by
grepping for every `return`, `assert`, `NULL`, comparison operator, error macro,
`errno`, `exit`, and every min/max constant:

```
src/driver.c:32:    if(line != NULL)
src/driver.c:50:        for (i = 0; i < 10; i++)
src/driver.c:61:        data = NULL;
src/driver.c:66:        for (i = 0; i < 10; i++)
```

Findings that shape the table:

* Every public function returns `void`. There is **no** error code, no sentinel
  return, no `errno` write, no `assert`, no error enum, no `RETURN_ERROR` macro,
  and no `exit`/`abort` path anywhere in the library.
* Therefore the library's entire *rejection* surface is the single guarded
  branch at `driver.c:32` — `printLine` silently does nothing when handed a
  null pointer. "Same error/rejection" for this API means **same observable
  effect**, i.e. identical bytes on `stdout` (and no crash).
* The remaining rows below are the generic FFI boundaries the task requires:
  null pointers, zero/oversized lengths, and values one past a valid range.
  `driver`'s `int useGood` is the only enum-like parameter crossing the FFI
  boundary; C accepts any `int` there, so out-of-range "enum" values
  (`INT_MIN`, `-1`, `2`, `INT_MAX`, …) are real inputs and each must select the
  same branch in Rust as in C (`!= 0` → `good`, `== 0` → `bad`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` (the `if (line != NULL)` guard at `driver.c:32` fails) | no output at all; returns normally, no crash | `err_row01_print_line_null` | [x] |
| 2 | `printLine` | `line` points at an immediately-terminating string `""` (zero length — the degenerate/"zero length" boundary; guard passes) | prints just `"\n"` | `err_row02_print_line_empty` | [x] |
| 3 | `printLine` | `line` points at a huge (64 KiB) string — "oversized length" boundary | prints all 65536 bytes then `"\n"` | `err_row03_print_line_oversized` | [x] |
| 4 | `printLine` | `line` contains `printf` format metacharacters (`%s %n %d %%`) — data must be treated as data, never as a format string | prints the literal bytes then `"\n"` | `err_row04_print_line_format_metachars` | [x] |
| 5 | `printLine` | `line` contains high/non-UTF-8 bytes (`0x80..0xFF`) — invalid Unicode is not an error for a C `char*` | prints the raw bytes verbatim then `"\n"` | `err_row05_print_line_non_utf8` | [x] |
| 6 | `printIntLine` | `intNumber == INT_MIN` (`-2147483648`) — one step past the negative end of the range | prints `-2147483648\n` | `err_row06_print_int_line_int_min` | [x] |
| 7 | `printIntLine` | `intNumber == INT_MAX` (`2147483647`) — the positive extreme | prints `2147483647\n` | `err_row07_print_int_line_int_max` | [x] |
| 8 | `driver` | `useGood == 0` (the false branch of `if (useGood)`) → calls the *defective* `bad()` whose `alloca(10)` under-allocates | prints `0\n`; must not crash or corrupt | `err_row08_driver_zero_selects_bad` | [x] |
| 9 | `driver` | out-of-range enum-ish value `useGood == -1` (no "valid variant"; C truncates nothing, any non-zero is true) | prints `0\n` via `good()` | `err_row09_driver_out_of_range_enum` | [x] |
| 10 | `driver` | out-of-range enum-ish value `useGood == 2` (one past the documented `{0,1}` range) | prints `0\n` via `good()` | `err_row10_driver_two` | [x] |
| 11 | `driver` | `useGood == INT_MIN` — non-zero, so the true branch; also checks no sign/`!= 0` mistranslation | prints `0\n` via `good()` | `err_row11_driver_int_min` | [x] |
| 12 | `driver` | `useGood == INT_MAX` | prints `0\n` via `good()` | `err_row12_driver_int_max` | [x] |
| 13 | `driver` | `useGood == 0x100000000` truncated to `int` by the ABI, i.e. the low 32 bits are `0` → must take the **`bad`** branch even though the wider value is non-zero | prints `0\n` via `bad()` | `err_row13_driver_truncating_value` | [x] |
| 14 | `bad` | the intrinsic defect: `alloca(10)` gives 10 **bytes** but the loop writes 10 `int`s (40 bytes). Calling `bad()` repeatedly must stay survivable and observably identical | prints `0\n` each call, no crash | `err_row14_bad_repeated_overrun` | [x] |
| 15 | `good` | no invalid input is expressible (`void` parameters); called repeatedly as the control for row 14 | prints `0\n` each call | `err_row15_good_repeated` | [x] |
