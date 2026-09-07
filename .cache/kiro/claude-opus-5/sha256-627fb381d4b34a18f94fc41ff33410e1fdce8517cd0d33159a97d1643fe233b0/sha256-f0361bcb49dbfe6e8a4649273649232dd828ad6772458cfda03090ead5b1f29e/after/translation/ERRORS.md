# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Mechanically derived from `c_src/src/driver.c`. The whole library contains
exactly the following rejection machinery — there are no `assert`s, no
`RETURN_ERROR` macros, no error enums, no NULL checks and no other range
checks anywhere in the translation unit:

```
$ grep -n 'return\|assert\|NULL\|INT_MIN\|INT_MAX\|errno\|error' c_src/src/driver.c
  errno = 0;                                  # parse_val
  long tmp = strtol(str, &endp, 10);          # sets errno = ERANGE on overflow
  if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
      *val = tmp; return true;
  } else {
      return false;                           # the ONLY failure return
  }
  ...
  } else {
      printf("An error occurred\n");          # the ONLY error output
  }
```

So the observable error surface is: `parse_val` returns `false`, and `driver`
therefore prints exactly `An error occurred\n` and performs **no** `run()`
calls and **no** mutation of `the_house`. There is no error code / return
value: `driver` is `void`. The differential assertion for every row below is
therefore *(a)* stdout is byte-identical (`An error occurred\n`) **and**
*(b)* the global state is provably untouched — verified by a subsequent
`run(0)` on both libraries, whose printed `floors`/`bedrooms`/`bathrooms`
must match.

Each of the four conjuncts in the `if` is a distinct rejection trigger; the
`errno` conjunct has two distinct causes (over- and underflow), giving 5
in-source rows, plus the generic FFI boundary rows required by the task.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` → `parse_val` | conjunct 1 fails: `endp == str`, empty string `""` — `strtol` consumes nothing | `parse_val` → `false`; stdout `An error occurred\n`; `the_house` unchanged |
| 2 | `driver` → `parse_val` | conjunct 1 fails: non-numeric text, no digits at all (`"abc"`, `"!"`, `"++1"`, `"--1"`, `"e5"`, `".5"`, `"NaN"`, `"inf"`) | same as #1 |
| 3 | `driver` → `parse_val` | conjunct 1 fails: whitespace-only / sign-only, i.e. `strtol` skips its prefix but finds no digit (`" "`, `"\t\n"`, `"+"`, `"-"`, `"   +"`) | same as #1 |
| 4 | `driver` → `parse_val` | conjunct 2 fails: `errno == ERANGE` on **positive** overflow past `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`, 400-digit number) | same as #1 (rejected by `errno`, *before* the `INT_MAX` test can matter) |
| 5 | `driver` → `parse_val` | conjunct 2 fails: `errno == ERANGE` on **negative** overflow past `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`) | same as #1 |
| 6 | `driver` → `parse_val` | conjunct 4 fails: value in `(INT_MAX, LONG_MAX]`, no `errno` (`"2147483648"`, `"4294967296"`, `"9223372036854775807"`) | same as #1 |
| 7 | `driver` → `parse_val` | conjunct 3 fails: value in `[LONG_MIN, INT_MIN)` (`"-2147483649"`, `"-4294967296"`, `"-9223372036854775808"`) | same as #1 |
| 8 | `driver` | one step past the valid range on **both** sides of the accepted window — `INT_MAX` = `2147483647` accepted vs `2147483648` rejected; `INT_MIN` = `-2147483648` accepted vs `-2147483649` rejected | boundary values succeed (4×`run` output), boundary+1 rejected as #6/#7 |
| 9 | `driver` | NULL pointer `in` (no NULL check exists in the C: the pointer is handed straight to `strtol`) | both libraries fault identically — child process dies on the same signal (`SIGSEGV`), no output |
| 10 | `run` | out-of-range "enum-like" / extreme `int` arguments crossing the FFI boundary: `INT_MAX`, `INT_MIN`, `-1`, values that make `bedrooms += extra` overflow `int` | no rejection path exists — `run` accepts every `int`; the two-complement wrapped `bedrooms` must be printed identically. (There are no enums in this API, so the enum-abuse class degenerates to arbitrary `int`.) |
| 11 | `driver` | trailing garbage after digits (`"12abc"`, `"5 5"`, `"0x10"`, `"0b101"`, `"1.9"`) — **NOT** an error in the C: only `endp != str` is checked, never `*endp == '\0'` | ACCEPTED: parses the leading integer (`12`, `5`, `0`, `0`, `1`) and runs normally. Row kept in the error table because it is the obvious place a "fixed" translation would wrongly reject; confirmed live by a negative control that added `*endp == 0` to the Rust condition and made this row fail. |
| 12 | `driver` | oversized input: a 4096-byte digit string, and a 4096-byte non-digit string | oversized digits → ERANGE → as #4; oversized garbage → as #2. No length limit and no buffer exists in the C. |

## Checklist (Phase C)

- [x] 1 — `errors_row01_empty_string`
- [x] 2 — `errors_row02_non_numeric`
- [x] 3 — `errors_row03_whitespace_or_sign_only`
- [x] 4 — `errors_row04_erange_positive`
- [x] 5 — `errors_row05_erange_negative`
- [x] 6 — `errors_row06_above_int_max_within_long`
- [x] 7 — `errors_row07_below_int_min_within_long`
- [x] 8 — `errors_row08_one_past_valid_range_both_sides`
- [x] 9 — `errors_row09_null_pointer_same_signal`
- [x] 10 — `errors_row10_run_extreme_ints`
- [x] 11 — `errors_row11_trailing_garbage_accepted`
- [x] 12 — `errors_row12_oversized_inputs`
