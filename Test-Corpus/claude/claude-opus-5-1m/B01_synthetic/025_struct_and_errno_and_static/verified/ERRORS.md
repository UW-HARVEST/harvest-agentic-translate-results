# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/main.c`. The file has **no** `assert`, no
`return -1`, no `return NULL`, no error enum and no `RETURN_ERROR`-style macro.
Its complete rejection machinery is:

* one `return false;` in `parse_val` (line 72), reached when **any** of the four
  conjuncts of the `if` on line 68 is false — four distinct triggers;
* one `printf("An error occurred\n")` in `main` (line 84), the only observable
  failure output;
* the ignored return value of `fgets` (line 78) — a failed read is silently
  turned into the empty C string, which then trips `parse_val`;
* the implicit `sizeof(in) == 100` bound on `fgets` (line 78) and the implicit
  NUL-termination of `in` (line 77), both of which silently *drop* input;
* the process-level failure modes of `printf` (write error / `SIGPIPE`).

Named constants that participate: `INT_MIN`, `INT_MAX` (`<limits.h>`), `errno` /
`ERANGE` (`<errno.h>`), the literal base `10`, and `sizeof(in) == 100`.

Note that all four `parse_val` conjuncts funnel into the *same* observable
result, so the differential tests assert the exact stdout bytes
(`"An error occurred\n"` and nothing else) **and** the exit status `0`, for each
trigger separately.

One consequence of that is worth writing down, because it looks like a test gap
and is not one: on an LP64 target the `errno == 0` conjunct is **redundant**.
`strtol` only sets `ERANGE` when it returns `LONG_MAX` or `LONG_MIN`, and both of
those already fail `tmp >= INT_MIN && tmp <= INT_MAX`. So rows E8/E9 are
*observably* the same rejection as E10/E11 — deleting the `errno` check from
either implementation cannot change a single byte of output. The translation
keeps the check anyway (`!r.erange` in `parse_val`) to mirror the C exactly, and
rows E8/E9 assert the observable result, which is all that can be asserted.

Tests live in `tests/error_paths.rs` (process level) and `tests/ffi_capture.rs`
(row E20, through `dlopen`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `main` → `fgets` | stdin at EOF immediately (empty file, `/dev/null`) — `fgets` returns `NULL`, `in` stays `""` | `in == ""`, `parse_val` fails, stdout `An error occurred\n`, exit 0 | `err_e1_immediate_eof` |
| E2 | `main` → `fgets` | stdin is not readable (fd closed, or opened on a directory so `read` fails `EISDIR`) — `fgets` returns `NULL`, `in` stays `""` | stdout `An error occurred\n`, exit 0 | `err_e2_unreadable_stdin` |
| E3 | `parse_val` | `endp == str`: no conversion, empty C string (`""`) | `return false` → `An error occurred\n`, exit 0 | `err_e3_empty_string` |
| E4 | `parse_val` | `endp == str`: no conversion, first non-space byte is not a sign or digit (`"abc"`, `"x1"`, `".5"`, `"/9"`, `":9"`) | `return false` → `An error occurred\n`, exit 0 | `err_e4_no_digits` |
| E5 | `parse_val` | `endp == str`: whitespace only (`" "`, `"\n"`, `"\t\v\f\r "`) | `return false` → `An error occurred\n`, exit 0 | `err_e5_whitespace_only` |
| E6 | `parse_val` | `endp == str`: sign present but no digit follows (`"-"`, `"+"`, `"- 5"`, `"+x"`, `"--5"`, `"+-5"`) | `return false` → `An error occurred\n`, exit 0 | `err_e6_sign_without_digits` |
| E7 | `parse_val` | `endp == str`: leading NUL byte, so the C string is empty even though stdin was not (`"\0" "42\n"`) | `return false` → `An error occurred\n`, exit 0 | `err_e7_leading_nul` |
| E8 | `parse_val` | `errno != 0`: `strtol` sets `ERANGE`, value `> LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`) | `return false` → `An error occurred\n`, exit 0 | `err_e8_erange_positive` |
| E9 | `parse_val` | `errno != 0`: `strtol` sets `ERANGE`, value `< LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`) | `return false` → `An error occurred\n`, exit 0 | `err_e9_erange_negative` |
| E10 | `parse_val` | `tmp > INT_MAX` with no `ERANGE`: `tmp ∈ [INT_MAX+1, LONG_MAX]` (`"2147483648"`, `"9223372036854775807"`) | `return false` → `An error occurred\n`, exit 0 | `err_e10_above_int_max` |
| E11 | `parse_val` | `tmp < INT_MIN` with no `ERANGE`: `tmp ∈ [LONG_MIN, INT_MIN-1]` (`"-2147483649"`, `"-9223372036854775808"`) | `return false` → `An error occurred\n`, exit 0 | `err_e11_below_int_min` |
| E12 | `parse_val` | one step *inside* each bound must still be accepted — `INT_MAX` (`2147483647`) and `INT_MIN` (`-2147483648`) | `return true`, 8 house lines, exit 0 | `err_e12_int_bounds_accepted` |
| E13 | `main` → `fgets` | input longer than `sizeof(in) - 1 == 99` bytes: the tail is silently dropped, so the parsed value is whatever the first 99 bytes say (e.g. 98 spaces + `"42"` parses as `4`) | value from the 99-byte prefix only, exit 0 | `err_e13_truncated_at_99` |
| E14 | `main` → `fgets` | 99-byte truncation turns a valid value into an out-of-range one (100+ `'9'` digits ⇒ 99 nines ⇒ `ERANGE`) | `An error occurred\n`, exit 0 | `err_e14_truncation_causes_erange` |
| E15 | `parse_val` | embedded NUL after the digits truncates the C string mid-line (`"12\0" "34\n"` parses as `12`, not `1234`) | value `12`, exit 0 | `err_e15_embedded_nul_truncates` |
| E16 | `parse_val` | trailing garbage after a valid prefix is *not* an error — `strtol` stops and `endp != str` (`"42abc"`, `"0x10"`, `"1e5"`, `"5 6"`) | value from the prefix, exit 0 | `err_e16_trailing_garbage_ok` |
| E17 | `print_the_house` → `printf` | stdout is a pipe whose read end is already closed ⇒ `SIGPIPE` with default disposition | process killed by signal 13 (shell status 141) | `err_e17_sigpipe` |
| E18 | `print_the_house` → `printf` | stdout file descriptor 1 is closed ⇒ every `printf`/flush fails with `EBADF`, return value ignored | no output, exit 0 (no signal) | `err_e18_closed_stdout` |
| E19 | `add_bedrooms` | `bedrooms += extra_bedrooms` overflows `int` (C UB; the emitted code wraps two's-complement) — `extra_bedrooms = INT_MAX`, `INT_MIN`, and the second `run()` call which adds `extra_bedrooms` again | wrapped 32-bit values printed with `%d`, exit 0 | `err_e19_bedroom_overflow_wraps` |
| E20 | `run` (FFI) | out-of-range values crossing the FFI boundary as `int`: the full `int` domain including `INT_MIN`/`INT_MAX`, and repeated calls that drive `bedrooms` and `floors` through wraparound | identical wrapped values / identical stdout bytes | `ffi_run_differential` (`randseq`, `boundseq`, `deepwrap` sections) |
| E21 | `main`/`run` | `argc`/`argv` are ignored (`int main()` takes no parameters); any command-line arguments must have no effect | identical output regardless of argv, exit 0 | `err_e21_argv_ignored` |

## Checklist

- [x] E1  immediate EOF
- [x] E2  unreadable stdin
- [x] E3  empty string / no conversion
- [x] E4  no digits
- [x] E5  whitespace only
- [x] E6  sign without digits
- [x] E7  leading NUL
- [x] E8  `ERANGE` positive
- [x] E9  `ERANGE` negative
- [x] E10 above `INT_MAX`
- [x] E11 below `INT_MIN`
- [x] E12 `INT_MAX`/`INT_MIN` accepted (one step inside the range)
- [x] E13 truncation at 99 bytes
- [x] E14 truncation causing `ERANGE`
- [x] E15 embedded NUL truncates
- [x] E16 trailing garbage accepted
- [x] E17 `SIGPIPE`
- [x] E18 closed stdout
- [x] E19 `int` overflow wraps
- [x] E20 FFI out-of-range `int` values
- [x] E21 argv ignored

## How these rows were validated (mutation / negative control)

A table of green checkmarks is only worth something if the tests can actually go
red, so the suite was validated by injecting six mutations into the Rust
translation and re-running everything:

| mutation | detected by |
|---|---|
| `add_bedrooms`: `wrapping_add` → `saturating_add` | E19, C13, C16, C25, C26, C27, all `ffi_run_differential` rows |
| `main`: `fgets_line(100)` → `fgets_line(101)` (off-by-one on `sizeof(in)`) | E13, C18, C19 |
| `parse_val`: `tmp <= INT_MAX` → `tmp < INT_MAX` | E12, C4, C12, C13, C21 |
| `main`: drop the `SIGPIPE` disposition fix | E17, C34 |
| `fgets_line`: drop the NUL truncation | E15, C21 |
| `parse_val`: drop the `errno == 0` conjunct | **not detected — provably unobservable** (see the note above) |

Five of six mutations were caught, and the sixth is a semantically neutral edit
on LP64 rather than a blind spot.

The suite also caught two *real* bugs in its own harness while being written,
both of which would otherwise have shown up as phantom "divergences":

* the `SIGPIPE` case was racy until the pipe's read end was closed **before**
  `spawn` (otherwise the child can finish and exit 0 first);
* libtest writes its own `test foo ... ok` progress lines straight to file
  descriptor 1, so they leaked into the `run()` captures until the capture work
  was moved into a test binary containing a single `#[test]`.
