# ERRORS.md — Error-surface table (Phase A → Phase C)

Mechanically derived from `c_src/src/driver.c`. The library has **no error
enums, no `RETURN_ERROR` macro, no `assert`, and no functions that return a
status code to the caller** — `driver()` and `run()` both return `void`. Grep
evidence:

```
$ grep -nE 'return|assert|NULL|errno|INT_MIN|INT_MAX' c_src/src/driver.c
26:#include <errno.h>
61:    errno = 0;
64:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
66:        return true;
68:        return false;

$ grep -n 'An error occurred' c_src/src/driver.c
79:        printf("An error occurred\n");

$ grep -rnE '#if|#ifdef|#else|#elif' c_src/         # -> only the DRIVER_H_ guard
c_src/include/driver.h:24:#ifndef DRIVER_H_
```

So the *entire* rejection surface is:

* the four ways the conjunction on line 64 can be false (→ `parse_val` returns
  `false` → `driver` prints `An error occurred\n` and performs **no** `run`
  calls, i.e. produces exactly 18 bytes of output instead of 8 lines), and
* the unchecked pointer dereferences (the C code never null-checks), whose
  observable "result" is a fatal signal — which the Rust must reproduce.

`min/max constants` present in the source: `INT_MIN`, `INT_MAX` (line 64, via
`<limits.h>`). No other bounds constants exist.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `parse_val` / `driver` | `endp == str`: `strtol` consumed nothing — empty string `""` | `false` → `driver` prints `An error occurred\n`, no `run` output |
| 2 | `parse_val` / `driver` | `endp == str`: whitespace-only input (`" "`, `"\t\n\v\f\r "`) — `strtol` skips it but finds no digit, so `endptr` is reset to `str` | `false` → `An error occurred\n` |
| 3 | `parse_val` / `driver` | `endp == str`: leading non-numeric byte (`"abc"`, `"+"`, `"-"`, `"++1"`, `"+-1"`, `".5"`, `","`, `"x10"`, high-bit byte `"\xff5"`) | `false` → `An error occurred\n` |
| 4 | `parse_val` / `driver` | `errno != 0` after `strtol`: value overflows `long` (`"9223372036854775808"`, `"99999999999999999999"`, long digit runs) → glibc sets `ERANGE` (34) and returns `LONG_MAX` | `false` → `An error occurred\n` |
| 5 | `parse_val` / `driver` | `errno != 0` after `strtol`: value underflows `long` (`"-9223372036854775809"`, `"-99999999999999999999"`) → `ERANGE`, returns `LONG_MIN` | `false` → `An error occurred\n` |
| 6 | `parse_val` / `driver` | `tmp > INT_MAX` with `errno == 0`: in-`long`, out-of-`int` positive (`"2147483648"`, `"4294967296"`, `"9223372036854775807"`) | `false` → `An error occurred\n` |
| 7 | `parse_val` / `driver` | `tmp < INT_MIN` with `errno == 0`: in-`long`, out-of-`int` negative (`"-2147483649"`, `"-9223372036854775808"`) | `false` → `An error occurred\n` |
| 8 | `driver` | `in == NULL` — never null-checked; `strtol(NULL, …)` dereferences it | fatal `SIGSEGV` (11); process killed by signal, no output |
| 9 | `run` | `the_house == NULL` — never null-checked; `print_house` reads `house->floors` | fatal `SIGSEGV` (11); process killed by signal, no output |
| 10 | `run` | `the_house` = non-null but wild/unmapped pointer (e.g. `0x1`) | fatal `SIGSEGV` (11) |
| 11 | `add_bedrooms` (via `run`) | signed `int` overflow: `bedrooms = INT_MAX`, `extra_bedrooms > 0` (also `INT_MIN` + negative). C UB; gcc at `-O0` wraps two's-complement | wrapped `int` printed via `%d` — Rust must use `wrapping_add` and print the same |
| 12 | `add_floor` (via `run`) | signed `int` overflow: `floors == INT_MAX`, `floors++` | wraps to `INT_MIN` and is printed |
| 13 | `print_house` (via `run`) | non-finite `bathrooms` (`NaN`, `-NaN`, `+inf`, `-inf`) passed through `%.1f` | glibc prints `nan` / `-nan` / `inf` / `-inf`; `+= 1.0` keeps NaN/inf |
| 14 | `driver` | out-of-`int`-range / "invalid enum-style" integer reaching the FFI boundary: `run`'s `extra_bedrooms` is a bare `int` (no enum exists in this API), so **every** `int` bit pattern incl. `INT_MIN`, `INT_MAX`, `-1` is a legal input that must behave identically | value used verbatim in `bedrooms += extra_bedrooms` |

## Notes on rows that are *not* errors (checked, deliberately excluded)

* `"5abc"`, `"5 "`, `"5.9"`, `"0x1A"`, `"010"` — these **succeed**: `strtol`
  consumes the leading digits, `endp != str`, so `parse_val` returns `true`
  (e.g. `"0x1A"` → `0`, `"010"` → `10` because base is fixed at 10). They are
  *valid* inputs and live in `CONFIGS.md`, not here. This is exactly the kind of
  "looks like an error but the C accepts it" behaviour that must not be "fixed".
* `errno` is explicitly zeroed on line 61, so a stale `errno` from earlier calls
  can never cause a rejection.

## Status

| row | test | result |
|-----|------|--------|
| 1 | `phase_c_errors::row01_empty_string` | [x] pass |
| 2 | `phase_c_errors::row02_whitespace_only` | [x] pass |
| 3 | `phase_c_errors::row03_leading_non_numeric` | [x] pass |
| 4 | `phase_c_errors::row04_erange_overflow` | [x] pass |
| 5 | `phase_c_errors::row05_erange_underflow` | [x] pass |
| 6 | `phase_c_errors::row06_above_int_max` | [x] pass |
| 7 | `phase_c_errors::row07_below_int_min` | [x] pass |
| 8 | `phase_c_errors::row08_driver_null_pointer` | [x] pass |
| 9 | `phase_c_errors::row09_run_null_pointer` | [x] pass |
| 10 | `phase_c_errors::row10_run_wild_pointer` | [x] pass |
| 11 | `phase_c_errors::row11_bedrooms_overflow` | [x] pass |
| 12 | `phase_c_errors::row12_floors_overflow` | [x] pass |
| 13 | `phase_c_errors::row13_non_finite_bathrooms` | [x] pass |
| 14 | `phase_c_errors::row14_full_int_range_extra_bedrooms` | [x] pass |
| generic | `phase_c_errors::generic_boundary_fuzz` (zero/oversized lengths, one-past-range, byte fuzz) | [x] pass |

15 error-path tests, all passing in the `dev` and `release` profiles (and with
`RUSTFLAGS="-C debug-assertions=on"`).

Divergences found and fixed in the **Rust** (the C was never touched):

* rows 9 & 10 originally failed — the C `.so` died with `SIGSEGV` (11) while the
  Rust `.so` died with `SIGABRT` (6), because `(*house).field` emits a
  "null pointer dereference occurred" UB-check under `-C debug-assertions`
  (cargo's default for `dev`/`test`). `addr_of!`, `read_volatile::<i32>`,
  `read_unaligned` and `copy_nonoverlapping` all have equivalent null/alignment
  checks. `src/lib.rs` now reads/writes the fields via integer address
  arithmetic + byte-wise `read_volatile::<u8>`/`write_volatile::<u8>`, the only
  check-free formulation, which faults exactly like the C at every address
  (verified for `0x0`, `0x1`, `0x3`, `0x4`, `0x8`, `0x10`, `0xdeadbeef`) and
  moves exactly the same bytes for valid pointers.

One row in the first draft of this table was **wrong about the C** and was
corrected rather than "fixed" in the Rust: `"0 x"` starts with a digit, so
`strtol` consumes `0`, `endp != str`, and the C **accepts** it. It is now
asserted as an accepted input.
