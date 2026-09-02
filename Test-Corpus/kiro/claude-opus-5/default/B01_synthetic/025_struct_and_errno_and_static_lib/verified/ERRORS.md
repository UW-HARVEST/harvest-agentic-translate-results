# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/driver.c`. Every rejection site in the
translation unit was found by grepping for `return`, `assert`, `errno`, `NULL`,
`INT_MIN`/`INT_MAX` and every `if`/`else`:

```
67:    errno = 0;
70:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
72:        return true;
73:    } else {
74:        return false;
80:    if (parse_val(in, &x)) {
83:    } else {
84:        printf("An error occurred\n");
```

There are **no** `assert`s, no error enums, no `RETURN_ERROR`-style macros, no
`return NULL` and no allocation failure paths in this library. The entire
rejection surface is the single 4-conjunct guard on line 70 plus the single
`else` on line 83. Each conjunct of line 70 is an independent trigger, so it
contributes one row per conjunct.

Constants involved: `INT_MIN` = -2147483648, `INT_MAX` = 2147483647,
`strtol` base = 10, `errno` sentinel = 0. On this platform `long` is 64-bit
(LP64), which is what makes rows 4 and 5 reachable at all.

The externally observable "error" result of the library is always the same
side effect — `driver` prints exactly the 18 bytes `An error occurred\n` to
stdout and performs **no** `run` calls, i.e. `the_house` is left untouched.
The distinction between rows is *which* internal condition produced it, so
each row is tested with its own dedicated input.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `parse_val` / `driver` | **`endp == str`** — `strtol` performed no conversion because the string has no valid base-10 prefix. Inputs: `""`, `"abc"`, `"   "`, `"+"`, `"-"`, `"."`, `"x10"`, `" \t\n"`, `"++1"`, `"--1"`, `"e5"`, `"0x"`→(no, see CONFIGS row) | `parse_val` → `false`; `driver` prints `An error occurred\n`, `the_house` unmodified |
| 2 | `parse_val` / `driver` | **`errno != 0`** — `strtol` sets `ERANGE` because the magnitude exceeds `LONG_MAX`. Inputs: `"99999999999999999999"`, `"9223372036854775808"` (`LONG_MAX`+1) | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 3 | `parse_val` / `driver` | **`errno != 0`** — `strtol` sets `ERANGE` because the magnitude is below `LONG_MIN`. Inputs: `"-99999999999999999999"`, `"-9223372036854775809"` (`LONG_MIN`-1) | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 4 | `parse_val` / `driver` | **`tmp < INT_MIN`** — conversion succeeded (`errno == 0`) but the `long` value is below `INT_MIN`. Inputs: `"-2147483649"` (`INT_MIN`-1), `"-3000000000"`, `"-9223372036854775808"` (`LONG_MIN`) | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 5 | `parse_val` / `driver` | **`tmp > INT_MAX`** — conversion succeeded (`errno == 0`) but the `long` value is above `INT_MAX`. Inputs: `"2147483648"` (`INT_MAX`+1), `"3000000000"`, `"9223372036854775807"` (`LONG_MAX`) | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 6 | `driver` | **`parse_val` returned `false`** (the `else` on line 83) — the only error *output* path. Asserted for every one of rows 1–5: stdout is exactly `An error occurred\n` and **neither** `run` call happens (verified by checking that a following `run(0)` still reports the pre-`driver` floors/bedrooms/bathrooms) | prints `An error occurred\n`; `the_house` state unchanged |
| 7 | `driver` | **`in == NULL`** — not checked by the C at all; `strtol(NULL, …)` dereferences the null pointer. Undefined behaviour that in practice faults. | process dies with `SIGSEGV`; must be *identical* in Rust (asserted in a forked child, comparing the terminating signal of the C child vs. the Rust child) |
| 8 | `driver` | **stale `errno`** — the caller's `errno` is non-zero on entry. Line 67 clears it, so this must **not** cause a rejection. Guards against a translation that forgot `errno = 0`. | value parses normally; `run` is called twice |
| 9 | `driver` | **`errno` left set by an earlier failure** — after an `ERANGE` rejection, the next `driver` call with a valid input must still succeed (again exercises line 67). | second call parses normally |
| 10 | `parse_val` / `driver` | **oversized input** — a 4096-byte and a 100 000-byte digit string (`"1"` followed by 99 999 zeros). Length itself is never checked; `strtol` reports `ERANGE`. | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 11 | `parse_val` / `driver` | **zero-length input** — `""` (the pointer is valid, the first byte is `NUL`). Distinguished from row 1 because it is the empty-buffer boundary every C API has. | `parse_val` → `false`; `driver` prints `An error occurred\n` |
| 12 | `run` | **out-of-range / extreme `extra_bedrooms`** — `INT_MIN`, `INT_MAX`, and values chosen so that `bedrooms += extra_bedrooms` overflows `int`. C wraps two's-complement; the Rust must wrap identically and must not panic. | four `The house has …` lines with wrapped `bedrooms` |
| 13 | `run` | **out-of-range "enum" value across FFI** — this API declares **no** enums (grep for `enum` in `c_src/` → 0 hits), so the analogous case is an arbitrary `int` bit pattern with no distinguished meaning: `run` is called with 64 randomised `i32` values incl. `0`, `±1`, `INT_MIN`, `INT_MAX`. C has no validation, so every value must be accepted by both. | both accept; identical output |
| 14 | `driver` | **`*val` untouched on failure** — the C leaves `driver`'s uninitialised `x` unwritten when `parse_val` fails, and never reads it. Asserted indirectly by row 6 (no `run` call means `x` is never observed). | no observable effect |

Rows 1–6 and 8–14 are exercised by `translation/tests/errors.rs`; row 7 is
exercised by the forked-child test in the same file.
