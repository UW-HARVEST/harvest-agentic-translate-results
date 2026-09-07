# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c`. The file contains:

* **0** `assert` / `RETURN_ERROR` macros / error enums
* **0** NULL checks
* **1** boolean-returning validator, `parse_val`, with **one** `if` whose
  condition is a 4-term conjunction — so **4 distinct rejection triggers**
* **1** consumer of that result, `driver`, with **1** error output path
* **2** `<limits.h>` constants used as range bounds: `INT_MIN`, `INT_MAX`
* **1** `errno` interaction: `errno = 0` before `strtol`, `errno == 0` after

The exact C source of the rejection logic:

```c
static bool parse_val(const char *str, int *val) {
    errno = 0;
    char *endp = (char *)str;
    long tmp = strtol(str, &endp, 10);
    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
        *val = tmp;
        return true;
    } else {
        return false;
    }
}

void driver(const char *in) {
    int x;
    if (parse_val(in, &x)) { ... } else { printf("An error occurred\n"); }
}
```

Every rejection is observable from outside the `.so` only as the exact stdout
bytes `"An error occurred\n"` (and the *absence* of any `The house has …`
lines). That, not "both failed somehow", is what each test asserts.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `parse_val` → `driver` | `endp == str`: `strtol` consumed **zero** characters — empty string `""` | `false` → stdout exactly `An error occurred\n` | `err_01_empty_string` | [x] |
| 2 | `parse_val` → `driver` | `endp == str`: whitespace only (`" "`, `"\t"`, `"\n"`, `" \t\r\n\v\f "`) — `strtol` skips it, then finds no digits | `false` → `An error occurred\n` | `err_02_whitespace_only` | [x] |
| 3 | `parse_val` → `driver` | `endp == str`: sign with no digits (`"+"`, `"-"`, `"++"`, `"--"`, `"+-3"`, `"- 3"`) | `false` → `An error occurred\n` | `err_03_sign_without_digits` | [x] |
| 4 | `parse_val` → `driver` | `endp == str`: non-numeric leading text (`"abc"`, `"x1"`, `".5"`, `"e5"`, `"/9"`, `":9"`, `"٣"`, `"NaN"`, `"inf"`) | `false` → `An error occurred\n` | `err_04_non_numeric_prefix` | [x] |
| 5 | `parse_val` → `driver` | `endp == str`: base-10 rejects hex digits with no `0` prefix (`"ABC"`, `"deadbeef"`, `"xFF"`) | `false` → `An error occurred\n` | `err_05_hex_digits_base10` | [x] |
| 6 | `parse_val` → `driver` | `errno != 0`: `strtol` sets `ERANGE` on **positive** overflow past `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999999999"`, 400-digit number) | `false` → `An error occurred\n` (note: fails the `errno` term *before* the range terms) | `err_06_erange_overflow` | [x] |
| 7 | `parse_val` → `driver` | `errno != 0`: `strtol` sets `ERANGE` on **negative** overflow past `LONG_MIN` (`"-9223372036854775809"`, `"-1" + 30 zeros`) | `false` → `An error occurred\n` | `err_07_erange_underflow` | [x] |
| 8 | `parse_val` → `driver` | `tmp < INT_MIN`, `errno == 0`: value parses fine as `long` but is below `INT_MIN` (`"-2147483649"` = `INT_MIN-1`, `"-3000000000"`, `"-9223372036854775808"` = exact `LONG_MIN`) | `false` → `An error occurred\n` | `err_08_below_int_min` | [x] |
| 9 | `parse_val` → `driver` | `tmp > INT_MAX`, `errno == 0`: value parses fine as `long` but is above `INT_MAX` (`"2147483648"` = `INT_MAX+1`, `"4294967296"`, `"9223372036854775807"` = exact `LONG_MAX`) | `false` → `An error occurred\n` | `err_09_above_int_max` | [x] |
| 10 | `parse_val` → `driver` | **out-of-range value with trailing garbage**: the range check runs on the parsed prefix, so `"2147483648abc"` is rejected while `"2147483647abc"` is accepted | `false` / `true` respectively | `err_10_range_check_with_trailing_garbage` | [x] |
| 11 | `parse_val` → `driver` | boundary **one step inside** the valid range must NOT be rejected: `"2147483647"`, `"-2147483648"` | `true` → four `The house has …` lines ×2 | `err_11_boundaries_accepted` | [x] |
| 12 | `driver` | **no NULL check exists.** `in == NULL` is passed straight to `strtol`, which dereferences it | `SIGSEGV` (crash), *not* an error return | `err_12_null_pointer_crashes_identically` (runs each `.so` in a **separate child process** and compares the termination signal) | [x] |
| 13 | `driver` | zero-length buffer / lone NUL byte with garbage after the terminator (`"\0abc"`) — only the empty prefix is visible | `false` → `An error occurred\n` | `err_13_nul_terminator_boundary` | [x] |
| 14 | `run` | **no NULL check exists.** `the_house == NULL` is dereferenced by `print_house` | `SIGSEGV` (crash) | `err_14_run_null_house_crashes_identically` (separate child process, signal compared) | [x] |
| 15 | `run` | `extra_bedrooms` at the extremes of `int` — `INT_MIN` / `INT_MAX` — is **not** validated; `bedrooms += extra_bedrooms` is allowed to overflow (signed-overflow UB; gcc at `-O0` wraps) | no error; wrapped `%d` output | `err_15_int_extremes_not_rejected` | [x] |
| 16 | `run` | `floors == INT_MAX`: `house->floors++` is **not** range-checked and overflows | no error; wrapped `%d` output | `err_16_floors_overflow_not_rejected` | [x] |
| 17 | `run` / `driver` | there are **no enum parameters** in the ABI. The only non-pointer parameter is `int extra_bedrooms`; every one of the 2³² bit patterns is a legal input with no invalid variant. Verified by feeding raw bit patterns (including `0x80000000`, `0x7FFFFFFF`, `0xFFFFFFFF`, and randomized values) across the FFI boundary | no rejection; identical output | `err_17_no_enum_domain_all_int_bitpatterns_valid` | [x] |

## Deliberately-absent checks (documented so they are not "fixed")

* `parse_val` **accepts trailing garbage** (`"12abc"` → `12`). There is no
  `*endp == '\0'` check. Row 10 pins this.
* `parse_val` **accepts leading whitespace** (`strtol` skips it).
* `driver` leaves `int x;` uninitialized and only reads it on the `true` path.
  The Rust translation initialises it to `0`, which is unobservable.
* No length limit, no NULL guard, no overflow guard anywhere.
