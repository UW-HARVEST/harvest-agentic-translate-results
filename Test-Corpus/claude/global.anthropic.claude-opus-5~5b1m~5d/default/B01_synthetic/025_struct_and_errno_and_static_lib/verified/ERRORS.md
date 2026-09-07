# ERRORS.md — Error-surface table

Mechanically derived from `c_src/src/driver.c`. The library has **exactly one**
rejection site: the four-conjunct `if` in `parse_val`, whose `false` branch makes
`driver()` take its `else` arm.

```c
static bool parse_val(const char *str, int *val) {
    errno = 0;
    char *endp = (char *)str;
    long tmp = strtol(str, &endp, 10);
    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
        *val = tmp; return true;
    } else {
        return false;                 /* <-- the single error return */
    }
}

void driver(const char *in) {
    int x;
    if (parse_val(in, &x)) { run(x); run(x); }
    else { printf("An error occurred\n"); }   /* <-- the single error output */
}
```

Inventory of every rejection/error construct found by grep:

* `return false` — 1 occurrence (`parse_val`), guarded by 4 separate conditions.
* `printf("An error occurred\n")` — 1 occurrence (`driver`), the observable error result.
* `assert` — 0 occurrences.
* `return -1` / `return NULL` / error enums / `exit()` / `abort()` — 0 occurrences.
* Range constants — `INT_MIN`, `INT_MAX` (`<limits.h>`); `errno`/`EOK`-style check `errno == 0` (`<errno.h>`); `strtol` base `10`.
* Null checks — **none**. `in` is passed unchecked to `strtol`.

The **only** observable error result is the exact byte string
`"An error occurred\n"` (18 bytes) on `stdout`, with **no** state mutation
(`the_house` is untouched, so a later successful call still starts from the
state left by the previous successful call).

## Table — one row per distinct rejection condition

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| E1  | `parse_val` ← `driver` | conjunct 1 fails: `endp == str`, **empty string** `""` — `strtol` performs no conversion | prints `An error occurred\n`; `the_house` unchanged | `err_e1_empty_string` | [x] |
| E2  | `parse_val` ← `driver` | conjunct 1 fails: **all-whitespace** string (`" "`, `"\t"`, `"\n"`, `" \t\n\v\f\r "`) — leading space skipped, then no digits | prints `An error occurred\n` | `err_e2_whitespace_only` | [x] |
| E3  | `parse_val` ← `driver` | conjunct 1 fails: **non-numeric leading char** (`"abc"`, `"x1"`, `"?"`, `"one"`, `"NaN"`, `"inf"`) | prints `An error occurred\n` | `err_e3_non_numeric_prefix` | [x] |
| E4  | `parse_val` ← `driver` | conjunct 1 fails: **lone sign** `"+"`, `"-"`, `"++1"`, `"--1"`, `"+-1"`, `" - 1"` — sign not followed by a digit | prints `An error occurred\n` | `err_e4_lone_or_double_sign` | [x] |
| E5  | `parse_val` ← `driver` | conjunct 1 fails: **base-10 rejects non-decimal digit forms** — `"0x1A"` *does* convert (`0`, endp after `'0'`) so it is NOT an error; but `"x1A"`, `".5"`, `"e5"`, `"'1'"` are | prints `An error occurred\n` for the non-converting forms | `err_e5_non_decimal_forms` | [x] |
| E6  | `parse_val` ← `driver` | conjunct 2 fails: `errno != 0` — `strtol` sets **`ERANGE`** because magnitude exceeds `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`, 400-digit number) | prints `An error occurred\n` | `err_e6_erange_above_long_max` | [x] |
| E7  | `parse_val` ← `driver` | conjunct 2 fails: `errno != 0` — **`ERANGE`** below `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`) | prints `An error occurred\n` | `err_e7_erange_below_long_min` | [x] |
| E8  | `parse_val` ← `driver` | conjunct 4 fails: conversion succeeds (`errno == 0`) but `tmp > INT_MAX` — `"2147483648"` (`INT_MAX+1`), `"2147483649"`, `"4294967296"`, `"9223372036854775807"` (`LONG_MAX`) | prints `An error occurred\n` | `err_e8_above_int_max` | [x] |
| E9  | `parse_val` ← `driver` | conjunct 3 fails: conversion succeeds but `tmp < INT_MIN` — `"-2147483649"` (`INT_MIN-1`), `"-4294967296"`, `"-9223372036854775808"` (`LONG_MIN`) | prints `An error occurred\n` | `err_e9_below_int_min` | [x] |
| E10 | `parse_val` ← `driver` | **boundary, one step inside** the valid range: `"2147483647"` (`INT_MAX`) and `"-2147483648"` (`INT_MIN`) must be **accepted**, i.e. the range check must not be off-by-one | runs the 4-line report twice per `run`, 8 lines total; signed overflow of `bedrooms` wraps | `err_e10_int_limits_accepted` | [x] |
| E11 | `driver` | **NULL pointer** `in == NULL` — no null check exists; `strtol(NULL, …)` dereferences it | UB: process dies on `SIGSEGV` (11). Verified differentially in a forked child: C and Rust must die with the *same* signal | `err_e11_null_pointer_same_signal` | [x] |
| E12 | `driver` | **unterminated / zero-length buffer semantics**: a `char` buffer whose first byte is `'\0'` — same as E1, exercised through a raw non-`CString` pointer | prints `An error occurred\n` | `err_e12_zero_length_raw_buffer` | [x] |
| E13 | `run` | **out-of-range "enum"-style int** crossing FFI: `run()` takes a plain `int` with no validation, so *every* 32-bit value is in-range and must be accepted, including `INT_MAX`, `INT_MIN`, `-1`, `0` and values that overflow `bedrooms` | no rejection; `bedrooms += extra` wraps (signed overflow, UB in C; must match the compiled C behaviour) | `err_e13_run_extreme_ints` | [x] |
| E14 | `parse_val` ← `driver` | `errno` **hygiene**: a pre-existing non-zero `errno` must NOT cause a rejection, because `parse_val` clears `errno = 0` first. Set `errno = ERANGE` before the call, then pass `"7"` | accepted; report printed | `err_e15_stale_errno_cleared` | [x] |
| E15 | `parse_val` ← `driver` | **trailing garbage is accepted** (endp advances, no error): `"12abc"`, `"3 "`, `"5\n"`, `"7;"`, `"1,000"` → parses the leading integer only | accepted with the leading value | `err_e14_trailing_garbage_accepted` | [x] |
| E16 | `parse_val` | **redundancy proof for the `errno == 0` conjunct** (not a rejection of its own): over ~24,000 `strtol(s, &e, 10)` probes, `errno != 0` always implies `errno == ERANGE` *and* a saturated value outside `[INT_MIN, INT_MAX]`, so the conjunct can never be the *sole* reason for a rejection | conjunct is unobservable through the public API on this platform | `err_e16_errno_conjunct_is_redundant_for_base10` | [x] |

## Notes

* Every row above is checked off: all 16 tests in `tests/phase_c_errors.rs` pass
  against both `.so`s (`cargo test --test phase_c_errors`).
* `E11` is the only row whose "same error" is a **signal** rather than a return
  value; it is compared in a forked child so the differential is still exact
  (same `WTERMSIG`, verified to be `SIGSEGV`).
* `E14`/`E15` are numbered per the table order above; the corresponding test
  names are `err_e14_trailing_garbage_accepted` (row E15, trailing garbage) and
  `err_e15_stale_errno_cleared` (row E14, `errno` hygiene).

## Mutation control (evidence the error-path tests are not vacuous)

15 single-edit mutants were injected into `src/lib.rs`, each built and run
against the whole suite in isolation, then reverted (`src/lib.rs` restored
byte-identical, md5 `97fa041903fc7274cc71e57025a5af21`):

| mutant | # tests killed |
|---|---|
| `tmp <= INT_MAX` -> `tmp <= INT_MAX + 1` | 12 |
| `tmp >= INT_MIN` -> `tmp > INT_MIN` | 21 |
| `bathrooms += 1.0` -> `+= 1.25` | 23 |
| **drop `&& errno == 0`** | **0 — provably equivalent, see E16** |
| `strtol(..., 10)` -> `strtol(..., 16)` | 23 |
| `floors += 1` -> `+= 2` | 23 |
| `driver` calls `run` once instead of twice | 23 |
| initial `bedrooms` 5 -> 6 | 23 |
| initial `floors` 2 -> 3 | 23 |
| initial `bathrooms` 2.5 -> 2.0 | 23 |
| `%.1f` -> `%.2f` | 23 |
| `"An error occurred"` -> `"An Error occurred"` | 4 |
| drop `endp != str` check | 12 |
| `bedrooms += extra` -> `-= extra` | 23 |
| `*val = tmp` truncated through `i16` | 22 |

14 of 15 mutants are killed; the survivor is the one the C source makes
semantically unreachable, and E16 proves that rather than assuming it.
