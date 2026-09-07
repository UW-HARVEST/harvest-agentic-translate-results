# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Mechanically derived by grepping `c_src/src/driver.c` and
`c_src/include/driver.h` for every rejection / error path:

```
grep -n 'return\|assert\|NULL\|if\s*(\|errno\|exit\|abort\|<\|>' c_src/src/driver.c
```

Findings:

* `return` statements: **none** (every function is `void`; no `return -1`,
  no `return NULL`, no error enum, no error macro such as `RETURN_ERROR`).
* `assert(...)`: **none** (`<assert.h>` is not even included).
* explicit range / min / max constant checks: **none**.
* `exit` / `abort` / `errno` use: **none**.
* null checks: **exactly one** — `if (line != NULL)` in `printLine`.
* `if` statements in the whole file: **exactly one** (the null check above).

So the library's entire rejection surface is a single row. Everything else in
the table below is the mandatory generic-boundary coverage required by
Phase C (null pointers, zero/oversized lengths, one-step-past-range values,
out-of-range enum values), annotated with what the C actually does.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `printLine` | `line == NULL` | `if (line != NULL)` is false → **nothing is printed at all**, no newline, function returns normally (void). No crash, no error code. | `err_e1_print_line_null` | [x] |
| E2 | `printLine` | `line` points at a lone NUL byte (`""`), i.e. zero-length string | pointer is non-NULL so the check passes → prints exactly one byte, `"\n"` (`printf("%s\n","")`). NOT treated as an error. | `err_e2_print_line_empty_is_not_an_error` | [x] |
| E3 | `printLine` | `line` contains printf conversion specifiers (`%s`, `%d`, `%n`, `%%`) | passed as the *argument*, never as the format → the specifiers are emitted literally. No format-string error, no extra varargs consumed. | `err_e3_print_line_format_specifiers` | [x] |
| E4 | `printLine` | `line` contains non-UTF-8 / arbitrary high bytes `0x80..=0xFF` | C `printf`/`puts` are byte-oriented → bytes are copied verbatim then `\n`. Must NOT be rejected or replaced (a Rust `CStr::to_str()` translation would error here). | `err_e4_print_line_non_utf8_bytes` | [x] |
| E5 | `printLine` | very long string (oversized length: 1 byte, 4095, 4096, 65537 bytes) | no length limit exists in the C → whole string plus `\n` is printed. | `err_e5_print_line_oversized` | [x] |
| E6 | `printIntLine` | `intNumber == INT_MIN` (`-2147483648`), i.e. the value whose negation overflows | `printf("%d\n", INT_MIN)` prints `-2147483648\n`. No check, no rejection. | `err_e6_print_int_line_int_min` | [x] |
| E7 | `printIntLine` | `intNumber == INT_MAX` (`2147483647`) and `INT_MAX`±1 patterns passed as 32-bit `int` | prints the decimal value and `\n`; a 64-bit value passed in the register is truncated to 32 bits identically on both sides. | `err_e7_print_int_line_int_max_and_past_range` | [x] |
| E8 | `printIntLine` | out-of-range "enum-like" `int` values crossing FFI (there is no enum in the C, so *every* `int` is in range; verified with values that would be invalid discriminants: `-1`, `0`, `1`, `999999`, `INT_MIN`, `INT_MAX`) | all accepted; printed as plain decimal. Neither side may panic or map to a default variant. | `err_e8_print_int_line_out_of_range_enum_values` | [x] |
| E9 | `bad` / `good` / `driver` | take no arguments → no invalid input is constructible; the only "defect" (CWE-482 discarded expression `intOne + intTwo;` in `bad`) is not an error path but observable behaviour | `bad` prints `0\n0\n`; it must NOT be "fixed" to print `0\n2\n`. | `err_e9_bad_defect_is_preserved` | [x] |
| E10 | all five | called repeatedly / interleaved, and called with the `.so` loaded twice | functions are stateless (all locals) → output depends only on the current arguments; no cross-call state. | `err_e10_stateless_across_repeated_calls` | [x] |

## Not applicable

* No function returns a status, so "same error code" is asserted as *same
  emitted byte stream (including the empty byte stream) and normal return*.
* No allocation is performed, so there is no out-of-memory path.
* No enums, structs, or pointers-to-output exist in the ABI, so there are no
  invalid-handle / uninitialised-struct paths.
