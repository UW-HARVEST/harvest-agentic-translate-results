# ERRORS.md — Phase C: error-surface table

Mechanically derived by grepping `c_src/src/driver.c` + `c_src/include/driver.h`
for every rejection construct:

```sh
grep -n 'return\|assert\|NULL\|if\|else\|switch\|RETURN_ERROR\|-1\|MAX\|MIN\|errno' \
    c_src/src/driver.c c_src/include/driver.h
```

Findings — the entire error surface of this library:

* `driver.c:30` — `if (line != NULL)` — the **only** guard/rejection in the library.
* `driver.c:39`, `driver.c:50` — `return charString;` — value returns, not errors.
* `driver.c:60/64` — `if (useGood) / else` — branch selection.
* **No** `assert`, **no** `RETURN_ERROR`, **no** error enum, **no** `errno`, **no**
  `return -1`/`return NULL` at the public boundary, **no** min/max constants,
  **no** length or range checks. Every public function returns `void`, so a
  rejection is observable *only* as "no bytes written to `stdout`".

Because the library's sole rejection signal is *silence*, each row's expected
result is stated as the exact byte sequence C writes to `stdout`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `printLine` | `line == NULL` — the `if (line != NULL)` guard at `driver.c:30` fails | returns normally, writes **0 bytes**, no crash | `err_e1_print_line_null` | [x] |
| E2 | `bad` | no argument; internal `helperBad()` returns NULL (GCC `-Wreturn-local-addr` null substitution), so E1's guard fires inside `bad` | returns normally, writes **0 bytes** | `err_e2_bad_is_silent` | [x] |
| E3 | `driver` | `useGood == 0` — falsy, takes the `else` branch into `bad()`, hence E2 | returns normally, writes **0 bytes** | `err_e3_driver_zero_is_silent` | [x] |
| E4 | `printLine` | non-NULL pointer to a **zero-length** string (`""`), i.e. first byte is the NUL terminator — passes the guard but has no payload; distinct input from E1 | writes exactly `"\n"` (1 byte) | `err_e4_print_line_empty_vs_null` | [x] |

## Generic FFI-boundary boundaries (required even though absent from the table above)

| # | function | trigger | expected C result | test | ✔ |
|---|----------|---------|-------------------|------|---|
| G1 | `printLine` | NULL pointer (see E1) plus repeated NULL calls in a row — guard must be stateless | 0 bytes for every call | `err_g1_repeated_null` | [x] |
| G2 | `printLine` | "oversized length": 1 MiB NUL-terminated string, far past libc's 4096-byte `stdout` buffer | writes all 1 MiB + `"\n"` | `err_g2_oversized_string` | [x] |
| G3 | `printLine` | string whose NUL terminator is the **last byte of the allocation** (no trailing slack) — detects any read-past-terminator | writes the string + `"\n"`, no over-read | `err_g3_no_slack_after_terminator` | [x] |
| G4 | `printLine` | payload containing `printf` format specifiers (`%s %n %d %%`). `driver.c:32` passes `line` as an *argument*, never as a format string, so nothing may be interpreted | the bytes verbatim + `"\n"` | `err_g4_format_specifiers_not_interpreted` | [x] |
| G5 | `printLine` | payload of non-UTF-8 / high bytes `0x80..=0xFF` — a `const char *` is bytes, not text; Rust must not assume UTF-8 | the bytes verbatim + `"\n"` | `err_g5_non_utf8_bytes` | [x] |
| G6 | `driver` | "one step past a documented range" / out-of-range *enum-like* int: `driver` takes a bare `int`, so **every** one of the 2^32 values is a legal input with no valid-variant restriction. Values whose **low byte(s) are zero but which are non-zero overall** (`0x100`, `0x10000`, `0x1000000`, `INT_MIN = 0x80000000`) are the trap: C truthiness is on the full `int`, so all are truthy | takes the `good()` branch → `"helperGood1 string\n"` | `err_g6_driver_truthiness_low_byte_zero` | [x] |
| G7 | `driver` | `INT_MAX`, `INT_MIN`, `-1`, and randomized non-zero `i32`s | all truthy → `"helperGood1 string\n"` | `err_g7_driver_extremes` | [x] |
| G8 | `printLine` | 1-byte payload that is itself `0xFF`, and payload `"\n"` (terminator-adjacent control bytes) | the byte verbatim + `"\n"` | `err_g8_control_and_high_single_byte` | [x] |

All rows are checked off only because a differential test asserts C and Rust
produce the *same* byte stream (and the same silence), not merely that "both
did not crash".
