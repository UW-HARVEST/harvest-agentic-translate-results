# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c`. Every rejection point in the
whole translation unit is enumerated below.

## Mechanical inventory of the C source

```
$ grep -n 'return\|assert\|NULL\|errno\|INT_MIN\|INT_MAX\|else' c_src/src/driver.c
67:    errno = 0;                                     <- explicit errno reset
70:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
72:        return true;                               <- the only success return
73:    } else {
74:        return false;                              <- the ONLY error return
80:    if (parse_val(in, &x)) {
83:    } else {
84:        printf("An error occurred\n");             <- the ONLY error output
```

Facts established by this inventory:

* There are **no** `assert()` calls, **no** error enums, **no** `return -1`,
  **no** `return NULL`, and **no** out-parameter error codes anywhere.
* `parse_val` is the **only** validating function. It has exactly **one**
  `return false` statement guarded by a 4-term conjunction, i.e. **4 distinct
  rejection triggers** (one per falsifiable conjunct).
* `driver` is the only consumer of that rejection; it reacts by printing
  `An error occurred\n` and performing **no** state mutation (`the_house` is
  left untouched).
* `run` performs **zero** validation — it has no error path at all. Its only
  "extreme input" behaviour is signed-integer wraparound (rows 8–9).
* The public API takes **no enum parameters**, so there is no invalid-enum
  value to smuggle across FFI. Row 10 records the equivalent for the only
  non-pointer parameter type in the API (`int`): every one of the 2^32 `int`
  bit patterns is a *valid* `int`, so `run` must accept all of them without
  rejecting. Row 11 covers the same for `driver`'s parsed value.

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `parse_val` / `driver` | conjunct 1 false: `endp == str` — `strtol` converted **no** characters. Empty string `""`. | `parse_val` → `false`; `driver` prints exactly `An error occurred\n`; `the_house` unmodified |
| 2  | `parse_val` / `driver` | conjunct 1 false: `endp == str` — non-numeric text (`"abc"`, `"hello"`, `"++1"`, `"--1"`, `"+"`, `"-"`, `" "`, `"\t\n"`, `"."`, `","`, `"e5"`, `"NaN"`, `"inf"`, `"０"` (U+FF10), `"\x80"`) | `false` → `An error occurred\n` |
| 3  | `parse_val` / `driver` | conjunct 1 false: `endp == str` — digits invalid **for base 10** so nothing is consumed (`"0b1"` consumes `0` → *accepted*; but `"x10"`, `"#10"`, `"o17"` consume nothing) | `false` → `An error occurred\n` |
| 4  | `parse_val` / `driver` | conjunct 2 false: `errno != 0` — `strtol` set `ERANGE` because the value overflows `long` (`"9223372036854775808"`, `"99999999999999999999999"`, 400-digit number) | `strtol` → `LONG_MAX`, `errno == ERANGE` → `false` → `An error occurred\n` |
| 5  | `parse_val` / `driver` | conjunct 2 false: `errno != 0` — `ERANGE` from **underflow** of `long` (`"-9223372036854775809"`, `"-99999999999999999999999"`) | `strtol` → `LONG_MIN`, `errno == ERANGE` → `false` → `An error occurred\n` |
| 6  | `parse_val` / `driver` | conjunct 4 false: `tmp > INT_MAX` — parses fine as `long` but exceeds `int` (`"2147483648"`, `"2147483649"`, `"4294967296"`, `"9223372036854775807"`) | `errno == 0`, `tmp` valid, range check fails → `false` → `An error occurred\n` |
| 7  | `parse_val` / `driver` | conjunct 3 false: `tmp < INT_MIN` — parses fine as `long` but below `int` (`"-2147483649"`, `"-2147483650"`, `"-4294967296"`, `"-9223372036854775808"`) | `errno == 0`, range check fails → `false` → `An error occurred\n` |
| 8  | `run` | boundary `int` argument `extra_bedrooms == INT_MAX` with `the_house.bedrooms > 0` → `bedrooms += INT_MAX` **signed overflow** (UB in C; GCC emits a plain `addl`, i.e. two's-complement wrap) | no rejection; wrapped (negative) `bedrooms` printed by the 4th `printf` |
| 9  | `run` | boundary `int` argument `extra_bedrooms == INT_MIN` → `bedrooms += INT_MIN` signed **underflow** | no rejection; wrapped `bedrooms` printed |
| 10 | `run` | "out-of-range enum"-equivalent: an arbitrary/garbage 32-bit pattern in the only non-pointer parameter (`0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFF`, `0xDEADBEEF`, `0xCAFEBABE` reinterpreted as `int`) | accepted unconditionally — every bit pattern is a valid `int`; four lines printed |
| 11 | `driver` | value exactly **on** the accepted boundary — one step *inside* the range rejected by rows 6/7: `"2147483647"` (`INT_MAX`) and `"-2147483648"` (`INT_MIN`) must be **accepted** | `true`; `run` called **twice** ⇒ 8 lines printed, wrapped `bedrooms` |
| 12 | `driver` | zero-length input: the empty C string `""` (buffer that is just the NUL byte) | identical to row 1 → `An error occurred\n` |
| 13 | `driver` | oversized input: a valid number followed by megabytes of trailing junk, and a 100 000-digit number | trailing junk after a converted prefix is **ignored** (`endp != str`) ⇒ *accepted*; the 100 000-digit number sets `ERANGE` ⇒ rejected |
| 14 | `driver` | leading whitespace / `+` sign / leading zeros, i.e. inputs that *look* malformed but `strtol` accepts (`"   42"`, `"+42"`, `"007"`, `"\t\n\v\f\r 42"`, `"-0"`) | accepted → 8 lines printed |
| 15 | `driver` | trailing-garbage forms `"42abc"`, `"42 43"`, `"1,000"`, `"3.9"`, `"12e3"`, `"0x1A"` (base 10 stops at `x`) | accepted, only the numeric prefix used → 8 lines printed |
| 16 | `driver` | `in == NULL` — NULL pointer across the FFI boundary. C passes it straight to `strtol(NULL, …)`, which is **undefined behaviour**; glibc dereferences it. | process fault (`SIGSEGV`), *no* `An error occurred\n`. The Rust side must behave the same way (it must **not** substitute a graceful error, because that would be an observable divergence). Verified in a forked child process. |
| 17 | `parse_val` (pre-existing `errno`) | caller's `errno` is already non-zero when `driver` is entered (`errno = ERANGE` set beforehand) — line 67 resets it, so this must **not** cause a rejection | accepted; `errno` reset means the input is judged on its own merit |
| 18 | `parse_val` (errno side effect) | observable side effect on the caller's `errno` after `driver` returns for both a rejected-by-`ERANGE` input and an accepted input | `errno == ERANGE` after row 4/5 input; `errno == 0` after an accepted input |

## Row → test mapping (all in `tests/phase_c_errors.rs`)

| row | test | status |
|-----|------|--------|
| 1  | `err01_empty_string` | pass |
| 2  | `err02_non_numeric` (40 hand-picked) + `err02b_random_non_numeric` (400 randomized) | pass |
| 3  | `err03_wrong_base_prefixes` | pass |
| 4  | `err04_erange_overflow` — also asserts `errno == ERANGE` | pass |
| 5  | `err05_erange_underflow` — also asserts `errno == ERANGE` | pass |
| 6  | `err06_above_int_max` + `err06_07_randomized_out_of_int_range` (250 randomized) | pass |
| 7  | `err07_below_int_min` + `err06_07_randomized_out_of_int_range` (250 randomized) | pass |
| 8  | `err08_run_int_max_overflow` | pass |
| 9  | `err09_run_int_min_underflow` | pass |
| 10 | `err10_arbitrary_bit_patterns` (16 fixed + 300 randomized) | pass |
| 11 | `err11_boundary_inside_is_accepted` | pass |
| 12 | `err12_zero_length` (incl. a buffer whose first byte is NUL) | pass |
| 13 | `err13_oversized_input` (2 MiB junk, 100 000 digits, 100 000 zeros, 100 000 spaces) | pass |
| 14 | `err14_looks_malformed_but_accepted` | pass |
| 15 | `err15_trailing_garbage_accepted` | pass |
| 16 | `err16_null_pointer_faults_identically` (fork + `waitpid`, compares the terminating signal) | pass |
| 17 | `err17_preexisting_errno_neutralised` | pass |
| 18 | `err18_errno_side_effect_matches` | pass |
| generic | `err_extra_run_power_of_two_neighbourhoods` — `run` at every 2^k ± 1 | pass |
| generic | `err_extra_exhaustive_short_strings` — **all 2 380 strings** of length 0–3 over `0192+- \t.xeE\n`, each classified by the C and required to match | pass |

## Note: the `errno == 0` conjunct is behaviourally redundant on LP64

Rows 4 and 5 trigger `errno == ERANGE`, but they *also* fail the
`tmp >= INT_MIN && tmp <= INT_MAX` check, because glibc's `strtol` returns
`LONG_MAX`/`LONG_MIN` when it sets `ERANGE`, and on LP64 both lie outside
`int`. So no input exists for which `errno == 0` is the **sole** reason
`parse_val` rejects.

This was verified exhaustively rather than assumed: a probe over every
digit-run of length 1–500 for all 10 leading digits and both signs, all
`long`/`int` boundary literals, and 3 000 000 random byte strings found
**0** inputs with `errno != 0 && endp != str && INT_MIN <= tmp <= INT_MAX`
(9 624 `ERANGE` cases seen, 0 other `errno` values).

Consequence: deleting the `errno == 0` check from the Rust is a *provably
equivalent* mutation and no differential test can catch it — recorded as such
in `mutation_check.py`'s `EQUIVALENT` set. The `errno` *reset* on line 67 is
**not** redundant (row 17 catches its removal), and the `errno` side effect is
pinned by row 18.

## Notes on rows deliberately *not* in the table

* A non-NUL-terminated `char` buffer is unbounded UB (out-of-bounds read) with
  no defined C result, so no differential assertion is possible; it is excluded.
* `the_house.floors` overflow (`floors++` in `add_floor`) needs 2^31 `run`
  calls to reach `INT_MAX` and is not reachable in a test.
* `the_house.bathrooms` is a `double` incremented by `1.0`; IEEE-754 has no
  overflow/rejection path here.
