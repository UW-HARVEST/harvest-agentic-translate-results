# ERRORS.md — error / rejection surface table

## How this table was derived

Mechanical grep of `c_src/src/main.c` for every classic rejection construct:

| construct | occurrences in code |
|-----------|--------------------|
| `RETURN_ERROR` | 0 |
| `return -1` / `return NULL` | 0 |
| `assert` | 0 |
| `errno` | 0 |
| `if` / `switch` | 0 (the only textual hits are the words "if"/"free" in the licence comment) |
| range / null checks | 0 |
| `MIN` / `MAX` constants | 0 |
| `malloc` / `free` | 0 |

The program body is 14 statements and contains **no branches whatsoever**:

```c
void driver(int x, int y) { int result = x | ~y; printf("%d", result); puts(""); }
int main() { int x = 0, y = 0; scanf("%d", &x); scanf("%d", &y); driver(x, y); return 0; }
```

Consequently the entire error surface is **implicit**, and lives in three places:

1. the failure modes of `scanf("%d", ...)` — whose return value is
   **discarded**, so every failure is silent and simply leaves the destination
   variable at its initialiser `0`;
2. the out-of-range behaviour of the `%d` conversion — glibc converts with
   `strtol` into a 64-bit `long` (clamping at `LONG_MAX`/`LONG_MIN` on `ERANGE`)
   and then *assigns* that `long` to an `int`, truncating to 32 bits;
3. the failure modes of `printf`/`puts` — also **discarded**, so a write error
   produces no diagnostic and no non-zero exit status; but the default `SIGPIPE`
   disposition can still terminate the process.

There is no rejection at all on the `driver` FFI entry point: both parameters are
`int`, every one of the 2^32 bit patterns is a valid input, and the function has
no failure path. Rows 19–22 record that as an explicit assertion rather than an
assumption.

`expected C result` below is what the C build actually does — verified by
running it, not inferred from documentation.

## The table

Legend for the result column: `x`/`y` are the two variables in `main`;
`out` is the byte stream on stdout; the program always prints `x | ~y` followed
by `\n` and exits `0` unless stated otherwise.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|---------------------------------------------|-------------------|------|
| 1  | `scanf` #1 | **input failure**: stream is already at EOF (empty stdin) | conversion fails, returns `EOF`, `x` stays `0` → `out = "-1\n"` | `err01_empty_stdin` |
| 2  | `scanf` #1 | **input failure**: stream holds only whitespace, EOF reached while skipping it | whitespace consumed, conversion fails, `x` stays `0` | `err02_whitespace_only` |
| 3  | `scanf` #1 | **matching failure**: first non-space byte is an ASCII letter (`"abc"`) | offending byte pushed back, `x` stays `0`; `scanf` #2 fails on the same byte, `y` stays `0` → `out = "-1\n"` | `err03_leading_alpha` |
| 4  | `scanf` #1 | **matching failure**: first non-space byte is punctuation (`"."`, `","`, `"*"`, `"/"`, `"#"`) | as row 3 | `err04_leading_punct` |
| 5  | `scanf` #1 | **matching failure**: sign then a non-digit (`"- 5"`, `"-a"`, `"+."`) | the **sign is consumed and stays consumed**, only the non-digit is pushed back → `x = 0`, and the pushed-back text is re-scanned by `scanf` #2 | `err05_sign_then_nondigit` |
| 6  | `scanf` #1 | **matching failure**: sign then immediate EOF (`"-"`, `"+"`) | sign consumed, nothing pushed back, `x` stays `0` | `err06_sign_then_eof` |
| 7  | `scanf` #1 | **matching failure**: two signs (`"--5"`, `"+-5"`, `"-+5"`, `"++5"`) | #1 eats one sign, fails, pushes back the second sign; #2 then converts the *signed* remainder, so `y` gets `-5`/`5` and `x` stays `0` | `err07_double_sign` |
| 8  | `scanf` #1 | **matching failure**: byte `0x00` (embedded NUL) at the conversion position | NUL is not a digit and not a space → matching failure, `x` stays `0` | `err08_nul_byte` |
| 9  | `scanf` #1 | **matching failure**: high byte `0x80`–`0xFF` at the conversion position (not space, not digit in the `"C"` locale) | matching failure, `x` stays `0` | `err09_high_byte` |
| 10 | `scanf` #2 | any of rows 1–9 applied to the **second** directive (`"5"`, `"5 abc"`, `"5 -"`, `"5 --3"`) | `x` converts, `y` stays `0` → `out = printf("%d", x | ~0) = x | -1 = "-1\n"` for every `x` | `err10_second_directive_fails` |
| 11 | `scanf` | **`ERANGE` overflow**: magnitude `> LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999999999"`) | `strtol` clamps to `LONG_MAX = 0x7FFF_FFFF_FFFF_FFFF`, assignment to `int` truncates → `0xFFFF_FFFF = -1` | `err11_erange_overflow` |
| 12 | `scanf` | **`ERANGE` underflow**: magnitude `< LONG_MIN` (`"-9223372036854775809"`, `"-1"*40`) | `strtol` clamps to `LONG_MIN = 0x8000_0000_0000_0000`, truncates to `int` → `0` | `err12_erange_underflow` |
| 13 | `scanf` | **`LONG_MAX`/`LONG_MIN` exactly** (`"9223372036854775807"`, `"-9223372036854775808"`) | no `ERANGE`; truncation still applies → `-1` and `0` respectively | `err13_long_boundaries_exact` |
| 14 | `scanf` | **in `long` range, out of `int` range** (`"2147483648"`, `"-2147483649"`, `"4294967296"`, `"4294967297"`) | `long`→`int` conversion wraps (gcc: modulo 2^32) → `INT_MIN`, `INT_MAX`, `0`, `1` | `err14_long_to_int_truncation` |
| 15 | `scanf` | **absurdly long digit run** (10 000 digits, with and without sign) | clamped as rows 11/12; no crash, no timeout | `err15_very_long_digit_run` |
| 16 | `scanf` #1 | **partial match then stop**: digits followed by a non-digit (`"5abc"`, `"5.7"`, `"1e5"`, `"0x5"`) | the digits that were matched are converted (`5`, `5`, `1`, `0`); the non-digit is pushed back and becomes input to `scanf` #2, which then fails | `err16_digits_then_nondigit` |
| 17 | `printf`/`puts` | **write failure**: stdout is a closed descriptor (`>&-`, `EBADF`) | return values discarded, nothing reported, exit status **0**, no output | `err17_stdout_closed_ebadf` |
| 18 | `printf`/`puts` | **write failure**: stdout is a pipe with no reader (`EPIPE`) | `SIGPIPE` is at its default disposition → process is **killed by signal 13** (shell status 141), no output | `err18_stdout_epipe_sigpipe` |
| 19 | `driver` (FFI) | `x = INT_MIN`, `y = INT_MIN` — extreme in-range values, no rejection path exists | computes `x | ~y`; `INT_MIN | ~INT_MIN = INT_MIN | INT_MAX = -1` | `err19_driver_extremes` |
| 20 | `driver` (FFI) | `y = -1` so `~y == 0`; `y = 0` so `~y == -1` (the identity/absorbing cases) | `x | 0 == x`; `x | -1 == -1` | `err20_driver_identity_absorbing` |
| 21 | `driver` (FFI) | signed **overflow is impossible** here, but `~` on `INT_MIN` and `|` on mixed signs are the UB-adjacent spots | `~INT_MIN == INT_MAX`; no trap, no UB (`~` and `|` are always defined for `int`) | `err21_driver_bitwise_edges` |
| 22 | `main` (FFI) | called through the `.so` with stdin redirected, i.e. the entry point invoked as a library function rather than by the loader | returns `0` and writes `x | ~y` + `\n`, identical to the process run | `err22_ffi_main_symbol` |

## Boundary conditions required regardless of the table

| condition | why it does not apply / how it is covered | test |
|-----------|------------------------------------------|------|
| null pointer arguments | Neither public function takes a pointer. `driver` takes two `int`s; `main` takes nothing. There is no pointer to pass as null. | n/a (documented) |
| zero / oversized lengths | No function takes a length or a buffer. | n/a (documented) |
| out-of-range enum values across FFI | The API declares **no enums** (`grep -c enum c_src/src/main.c` → 0), so there is no invalid discriminant to pass. The analogous "any bit pattern is accepted" check for the `int` parameters is rows 19–21, which sweep `INT_MIN`, `INT_MAX`, `-1`, `0` and randomized values. | rows 19–21 |
| one step past a documented valid range | `int` has no sub-range restriction here; the meaningful "one past" boundaries are the *conversion* ranges, covered by rows 11–14. | rows 11–14 |
| unbounded input stream (never reaches EOF) | Not an error for C — `scanf` reads lazily and the program terminates as soon as two conversions are done. An eager reader would hang or exhaust memory. | `cfg30_unbounded_stdin` (CONFIGS.md row 30) |

## Status

All 22 rows have a passing differential test: rows 1–18 in
`tests/differential_errors.rs`, rows 19–22 in `tests/differential_ffi.rs`.

Every row asserts **two** things, so that "both sides are broken in the same way"
cannot pass as success:

1. C and Rust agree exactly — stdout bytes, stderr bytes, exit code and
   terminating signal; and
2. the agreed-upon result equals the value derived by hand from the C semantics
   (the `expected C result` column). If a hand-derivation were wrong, the test
   fails against the C binary itself and says so.

Two rows encode behaviour the Rust translation originally got wrong, and are
therefore genuine regression tests rather than documentation:

- **row 18** — the C build is killed by `SIGPIPE`; the Rust runtime ignores that
  signal by default and exited `0` until `restore_default_sigpipe()` was added.
- (the eager-stdin defect shows up as CONFIGS.md row 30 rather than here, since
  an endless stream is valid input to C, not an error.)

`scripts/negative_control.sh` mutation-tests the suite and requires all 11
injected translation bugs to be rejected, which is what makes the check marks
above meaningful.
