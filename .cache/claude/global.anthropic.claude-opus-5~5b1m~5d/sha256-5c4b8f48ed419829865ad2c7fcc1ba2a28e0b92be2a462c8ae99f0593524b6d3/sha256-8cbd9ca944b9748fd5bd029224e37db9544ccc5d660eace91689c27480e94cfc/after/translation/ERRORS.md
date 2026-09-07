# ERRORS.md — Phase A: error-surface table

Mechanically derived from every rejection point in `c_src/src/driver.c`.

Grep evidence — the C source contains **no** `assert`, no `return -1`, no
`return NULL`, no error enum, and no null-pointer check:

```
$ grep -nE 'assert|return -1|return NULL|RETURN_ERROR|errno|INT_MIN|INT_MAX|== *NULL|!= *NULL' c_src/src/driver.c
26:#include <errno.h>
27:#include <limits.h>
61:    errno = 0;
64:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
```

The **only** rejection site is the compound condition in `parse_val`
(line 64). Each of its four conjuncts is a distinct way to reject input, and
`driver` turns any of them into the single observable side effect
`printf("An error occurred\n")` (line 79). `run` has no rejection paths at all
(it unconditionally dereferences its pointer).

## Rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `parse_val` / `driver` | `endp == str`: `strtol` performed **no** conversion — empty string `""` | `parse_val` → `false`; `driver` prints exactly `An error occurred\n` |
| 2 | `parse_val` / `driver` | `endp == str`: string is whitespace only (`" "`, `"\t\n\v\f\r "`) | `false` → `An error occurred\n` |
| 3 | `parse_val` / `driver` | `endp == str`: first non-space char is not a digit/sign (`"abc"`, `"x12"`, `"!5"`, `"+"`, `"-"`, `"++1"`, `"."`, `" -"`) | `false` → `An error occurred\n` |
| 4 | `parse_val` / `driver` | `endp == str`: base-10 parse of `"0x"`-only-prefix-less garbage such as `"e5"`, `"#"`, `"/"`, `":"` (chars adjacent to `'0'..'9'` in ASCII) | `false` → `An error occurred\n` |
| 5 | `parse_val` / `driver` | `errno != 0`: `strtol` sets `ERANGE` because the value exceeds `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999999"`) | `false` → `An error occurred\n` (note: the `errno` conjunct rejects *before* the range conjuncts) |
| 6 | `parse_val` / `driver` | `errno != 0`: `strtol` sets `ERANGE` because the value is below `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999999"`) | `false` → `An error occurred\n` |
| 7 | `parse_val` / `driver` | `tmp > INT_MAX` but within `long` range (`"2147483648"`, `"2147483649"`, `"4294967296"`, `"9223372036854775807"`) | `false` → `An error occurred\n` |
| 8 | `parse_val` / `driver` | `tmp < INT_MIN` but within `long` range (`"-2147483649"`, `"-2147483650"`, `"-9223372036854775808"`) | `false` → `An error occurred\n` |
| 9 | `parse_val` / `driver` | pre-existing non-zero `errno` in the caller's thread — C explicitly clears it (`errno = 0;` line 61), so a valid string must still be **accepted** | `true` → four `print_house` lines per `run`, no error message |
| 10 | `driver` | `in == NULL` — no null check exists; `strtol(NULL, …)` is undefined behaviour and crashes on glibc | SIGSEGV (signal 11), no output. Verified in a forked child comparing the termination signal for both libraries. |
| 11 | `run` | `the_house == NULL` — no null check exists; `print_house` dereferences it | SIGSEGV (signal 11). Verified in a forked child comparing the termination signal for both libraries, for `extra_bedrooms ∈ {0, 7, INT_MAX, INT_MIN}`. |

### Divergence found and fixed via rows 10/11

Row 11 initially FAILED in the `dev` cargo profile: C died with `SIGSEGV` (11)
while the Rust died with `SIGABRT` (6). Two causes, both fixed in the Rust:

1. `run` built a Rust reference (`&mut *the_house`) before use. Rust's
   debug-profile UB check panics with "null pointer dereference occurred"
   instead of faulting. The helpers `add_floor` / `add_bedrooms` /
   `print_house` and `run` now take **raw pointers**, mirroring the C
   signatures exactly.
2. `[profile.dev]` now sets `debug-assertions = false` /
   `overflow-checks = false`, since the C library has no such runtime checks —
   a null deref must fault, not panic.

Both profiles now report `Signaled(11)` for C and Rust alike.

## Boundary / one-past-valid-range coverage (generic, beyond the table)

| # | case | expected |
|---|------|----------|
| B1 | `"2147483647"` (`INT_MAX`, last valid) | accepted |
| B2 | `"2147483648"` (`INT_MAX + 1`, first invalid) | rejected → row 7 |
| B3 | `"-2147483648"` (`INT_MIN`, last valid) | accepted |
| B4 | `"-2147483649"` (`INT_MIN - 1`, first invalid) | rejected → row 8 |
| B5 | `"0"` / `"-0"` / `"+0"` | accepted, value 0 |
| B6 | zero-length input (`""`) | rejected → row 1 |
| B7 | oversized input: 4096-digit numeral | rejected via `ERANGE` → row 5 |
| B8 | out-of-range *enum* values across the FFI boundary | **N/A** — the C API declares no enum parameters. The analogous "any int is a valid C value" case is `int extra_bedrooms` of `run`, covered exhaustively at `INT_MIN`/`INT_MAX`/`0`/`±1` plus randomized values in `CONFIGS.md` rows C9–C13 (signed-overflow behaviour of `bedrooms += extra_bedrooms`). |
| B9 | non-UTF-8 / high-bit bytes in the input string (`"\xff\xfe"`, `"\x80""42"`) | rejected → row 3 (Rust must not assume UTF-8) |

## Extra observable side effect: `errno`

`errno` is part of the C ABI, so its value after the call is observable by the
caller. The C code writes `errno = 0` (line 61) and then lets `strtol` set it.
`tests/errors.rs::errno_left_behind_matches` seeds a hostile `errno` (12345),
calls both libraries, and asserts the stdout **and** the left-behind `errno`
match for accepted, rejected and `ERANGE` inputs.

## Reachability note (rows 5/6 vs 7/8)

Measured on this platform (LP64 glibc, `sizeof(long) == 8`):

| input | `strtol` ret | `errno` | `endp` moved | in int range |
|---|---|---|---|---|
| `"9223372036854775808"` | `LONG_MAX` | 34 (`ERANGE`) | yes | no |
| `"-9223372036854775809"` | `LONG_MIN` | 34 (`ERANGE`) | yes | no |
| `"2147483648"` | 2147483648 | 0 | yes | no |
| `"9223372036854775807"` | `LONG_MAX` | 0 | yes | no |
| `"abc"` / `""` | 0 | 0 | **no** | yes |

Whenever `errno == ERANGE`, `strtol` has returned `LONG_MAX`/`LONG_MIN`, which is
already outside `[INT_MIN, INT_MAX]` — so the `errno == 0` conjunct and the two
range conjuncts reject exactly the same set of inputs here. It is therefore a
*redundant* (but harmless) check on LP64, verified by mutation testing: deleting
`get_errno() == 0 &&` from the Rust leaves all 43 tests green, i.e. it is an
equivalent mutant, not a test gap. The Rust translation nevertheless keeps the
conjunct, in the same position and short-circuit order as the C, so the two stay
identical on any platform where `sizeof(long) == sizeof(int)` (where `ERANGE`
*would* be the only rejecting conjunct).

## Harness sensitivity (negative control)

To prove these tests can actually detect divergence, each of the following
deliberate mutations was injected into the Rust and the suite re-run; every one
was caught (test counts are failing tests):

| injected Rust bug | failing tests |
|---|---|
| `%.1f` → `%.2f` in the format string | 21 |
| `bedrooms.wrapping_add` → `saturating_add` | 13 |
| `floors.wrapping_add` → `saturating_add` | 2 |
| `tmp <= C_INT_MAX` → `tmp < C_INT_MAX` | 6 |
| `tmp >= C_INT_MIN` → `tmp > C_INT_MIN` | 3 |
| `"An error occurred"` → `"An Error occurred"` | 14 |
| `bathrooms += 1.0` → `+= 1.0000000001` | 11 |
| drop `set_errno(0)` | 1 |
| drop the `endp != str` conjunct | 8 |
| `*val = tmp as c_int` → truncate via `i16` | 8 |
| drop the `get_errno() == 0` conjunct | 0 (equivalent mutant, see above) |

All rows are exercised by `translation/tests/errors.rs`.
