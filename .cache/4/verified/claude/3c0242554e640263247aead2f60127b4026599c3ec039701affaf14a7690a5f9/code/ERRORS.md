# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/main.c`. The grep for rejection constructs
is *empty on purpose*:

```
$ grep -n -E 'RETURN_ERROR|assert|errno|NULL|return -1|exit\(' c_src/src/main.c
(no matches)
$ grep -n -E 'return|if|switch|#if' c_src/src/main.c
66:    return 0;      # main's success return — the only `return` in the file
```

There is **no** `assert`, **no** null check, **no** range check, **no** error
enum and **no** error return in the C. Consequently the entire error surface is
implicit and lives in exactly three places:

1. `scanf("%d", &x)` (line 63) — its return value is **discarded**, so every
   rejection it performs (input failure = `EOF`, matching failure, out-of-range
   conversion) manifests only as "`x` keeps its initialiser `0`" or as a
   truncated value.
2. Signed-integer overflow in `add_bedrooms` (`house->bedrooms += extra`,
   line 40) and `add_floor` (`house->floors++`, line 36) — UB per the standard,
   two's-complement wrap in the compiled C (verified identical at `-O0`, `-O2`,
   `-O3` and `-O0 -fwrapv`).
3. `printf` (line 48) — its return value is discarded, so write failures are
   silently ignored (or kill the process through `SIGPIPE`).

`main` is declared `int main()` (no parameters), so `argc`/`argv` are never
inspected and can never be rejected. `run`'s only parameter is a plain `int`
(there is no enum anywhere in the API, so no out-of-range enum variant can be
passed across the FFI boundary); the entire `int` range is valid input, and rows
21–23 pin down its extremes.

Every row below has a differential test that constructs exactly that condition,
runs BOTH programs and asserts they agree; where the C leaves an observable
sentinel (the value `scanf` parked in `x`), `error_paths.rs::check` additionally
pins that exact value with an independent model of the program, so no row can
pass merely because "both failed somehow".

| #  | function | trigger (the exact invalid input/condition) | expected C result | test (`tests/error_paths.rs` unless noted) | [x] |
|----|----------|----------------------------------------------|-------------------|------|-----|
| 1  | `main`/`scanf` | stdin empty (immediate EOF) | input failure, `scanf` → `EOF`, `x` stays `0`; 8 lines for `x=0`; exit `0` | `row01_empty_stdin_is_input_failure` | [x] |
| 2  | `main`/`scanf` | stdin whitespace only (`" "`, `"\t"`, `"\n"`, `"\v"`, `"\f"`, `"\r"`, mixes), then EOF | whitespace skipped, then input failure; `x` stays `0` | `row02_whitespace_only_is_input_failure` | [x] |
| 3  | `main`/`scanf` | first non-whitespace byte is a letter (`"abc"`, `"nan"`, `"inf"`) | matching failure, `scanf` → `0`, nothing stored, `x` stays `0` | `row03_letter_is_matching_failure` | [x] |
| 4  | `main`/`scanf` | lone `"+"` (also `"++"`, `"+++5"`) | matching failure/EOF, nothing stored, `x` stays `0` | `row04_lone_plus_is_matching_failure` | [x] |
| 5  | `main`/`scanf` | lone `"-"` (also `"--"`, `"---5"`) | matching failure/EOF, nothing stored, `x` stays `0` | `row05_lone_minus_is_matching_failure` | [x] |
| 6  | `main`/`scanf` | sign followed by non-digit (`"-x"`, `"+ 5"`, `"--5"`, `"+-5"`, `"-.5"`) | matching failure, nothing stored, `x` stays `0` | `row06_sign_then_nondigit_is_matching_failure` | [x] |
| 7  | `main`/`scanf` | punctuation first (`"."`, `","`, `"*"`, `"#5"`, `"(5)"`, `"'5'"`) | matching failure, `x` stays `0` | `row07_punctuation_is_matching_failure` | [x] |
| 8  | `main`/`scanf` | NUL byte first (`"\0"`, `"\0 5"`, `" \0 5"`) | `'\0'` is neither space, sign nor digit → matching failure, `x` stays `0` | `row08_nul_byte_is_matching_failure` | [x] |
| 9  | `main`/`scanf` | non-ASCII / invalid-UTF-8 byte first (`0xFF`, `0x80`, `"é"`, `"€5"`) | matching failure, `x` stays `0` (byte-oriented; never a UTF-8 error) | `row09_non_ascii_byte_is_matching_failure` | [x] |
| 10 | `main`/`scanf` | hex-looking input (`"0x10"`, `"0X10"`, `"0b1"`, `"-0x10"`) | `%d` is base 10: consumes `"0"`, rejects the rest → `x = 0` | `row10_hex_prefix_rejected_after_zero` | [x] |
| 11 | `main`/`scanf` | digits then garbage (`"5abc"`, `"-7q"`, `"123!!!"`) | conversion succeeds; the garbage is never rejected, just left unread | `row11_digits_then_garbage` | [x] |
| 12 | `main`/`scanf` | two or more tokens (`"1 2"`, `"1\n2"`, `"3 99999999999999999999"`) | only the first conversion happens | `row12_only_first_token_read` | [x] |
| 13 | `main`/`scanf` | float syntax (`"2.5"`, `"-0.75"`, `"1e9"`) | `%d` stops at the first non-digit → `2`, `0`, `1` | `row13_float_syntax_stops_at_dot` | [x] |
| 14 | `main`/`scanf` | above `INT_MAX` but inside `long` (`"2147483648"`, `"2147483649"`, `"-2147483649"`, `"3000000000"`) | converted as `long`, stored `(int)`-truncated → `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `-1294967296` | `row14_above_int_max_truncates` | [x] |
| 15 | `main`/`scanf` | multiples of 2^32 (`"4294967296"`, `"8589934592"`, `"-4294967296"`, `"4294967297"`) | truncation to `int` → `0`, `0`, `0`, `1` | `row15_two_pow_32_truncates_to_zero` | [x] |
| 16 | `main`/`scanf` | exactly `LONG_MAX` (`"9223372036854775807"`) | no saturation; `(int)LONG_MAX == -1` | `row16_exact_long_max` | [x] |
| 17 | `main`/`scanf` | above `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`, 30-digit values) | `strtol` saturates to `LONG_MAX` (`ERANGE` ignored) → stored `-1` | `row17_above_long_max_saturates` | [x] |
| 18 | `main`/`scanf` | exactly `LONG_MIN` (`"-9223372036854775808"`) | `(int)LONG_MIN == 0` | `row18_exact_long_min` | [x] |
| 19 | `main`/`scanf` | below `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`, …) | saturates to `LONG_MIN` → stored `0` | `row19_below_long_min_saturates` | [x] |
| 20 | `main`/`scanf` | overflow-guard boundary: ≥19 digits that are still small (`"0000000000000000005"`, 40 zeros, 100 000 nines) | leading zeros never overflow → `5`; 100 000 nines saturate | `row20_leading_zeros_do_not_overflow` | [x] |
| 21 | `main` → `add_bedrooms` | `x = INT_MAX` → `5 + INT_MAX` | signed overflow wraps: `-2147483644`, then `3` after the second `run` | `row21_int_max_wraps_bedrooms` | [x] |
| 22 | `main` → `add_bedrooms` | `x = INT_MIN` (`"-2147483648"`) | wraps: `-2147483643`, then `5` | `row22_int_min_wraps_bedrooms` | [x] |
| 23 | `run` (FFI) | `extra_bedrooms = INT_MAX` / `INT_MIN` applied repeatedly to the accumulating global | every add wraps modulo 2^32; identical step-by-step sequence in both `.so`s | `differential_ffi.rs::ffi_differential` | [x] |
| 24 | `run` → `add_floor` | `floors++` overflow at `INT_MAX` | unreachable: needs 2^31 calls to `run`. Rust uses `wrapping_add` — the same two's-complement wrap the compiled C performs (verified for `bedrooms`, rows 21–23, which share the code path). A guard-rail test pins the reachable `floors` sequence | `row24_floor_increment_uses_wrapping_semantics` | [x] |
| 25 | `main`/`scanf` | stdin closed before `exec` (fd 0 absent → `read` fails `EBADF`) | read error = input failure, `x` stays `0`, 8 lines printed, exit `0` | `row25_closed_stdin` | [x] |
| 26 | `main`/`scanf` | stdin is a directory (`read` fails `EISDIR`) | read error = input failure, `x` stays `0`, exit `0` | `row26_stdin_is_a_directory` | [x] |
| 27 | `main`/`printf` | stdout closed before `exec` (fd 1 absent) | `printf`/flush failures ignored, no output, exit status `0` | `row27_closed_stdout` | [x] |
| 28 | `main`/`printf` | stdout is a pipe with no reader at all | `SIGPIPE` at its default disposition → killed by signal 13, no exit code | `row28_sigpipe_on_stdout_without_reader` | [x] |
| 29 | `main`/`scanf` | rejection happening exactly at a stdio buffer refill (4095/4096/4097/8192 bytes of whitespace, then junk / `INT_MIN` / a saturating value) | refill is invisible: same rejection and same stored value | `row29_token_across_buffer_boundary` | [x] |
| 30 | `main` | extra `argv` entries (`driver foo bar`, `driver --nonsense`, `driver 😀`) | `int main()` ignores them; identical output, exit `0` | `row30_argv_is_never_rejected` | [x] |

Plus a negative control, `harness_detects_divergence`, which proves the
comparison helper really fails when the two programs disagree (so none of the
rows above can pass vacuously).
