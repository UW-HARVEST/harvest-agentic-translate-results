# ERRORS.md — Phase A error-surface table

Every distinct way `c_src/src/main.c` rejects, guards, or degrades on input.
Derived by grepping the whole translation unit for every `if`, `else`, `return`,
`NULL` comparison, and every `limits.h` min/max constant. `main.c` contains **no**
`assert`, no error enum, no `RETURN_ERROR`-style macro, no `return -1`, and no
`return NULL` — the complete set of guards is:

```
main.c:30   if(line != NULL)                 <- printLine NULL check
main.c:45   if(data > 0)                     <- bad     positivity guard
main.c:56   if(data > 0)                     <- goodG2B positivity guard
main.c:68   if(data > 0)                     <- goodB2G positivity guard
main.c:70   if (data < (CHAR_MAX/2))         <- goodB2G range check (CHAR_MAX/2 == 63)
main.c:75   else -> printLine("data value is too large ...")   <- the ONLY error message
main.c:89   int x = 0;                       <- the value scanf leaves behind on failure
main.c:90   scanf("%d", &x);                 <- return value DISCARDED: failure is silent
main.c:92   if (x) ... else ...              <- dispatch
main.c:100  return 0;                        <- exit status is ALWAYS 0, even on bad input
```

Constants that bound behaviour: `CHAR_MAX` (127), `CHAR_MAX/2` (63, integer
division), `LONG_MAX`/`LONG_MIN` and `INT` truncation inside glibc's `%d`.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| E1 | `printLine` | `line == NULL` | `main.c:30` guard fails → **no output at all**, returns normally (no crash) |
| E2 | `printLine` | `line` points at `""` (immediate NUL) | guard passes → prints exactly one byte `"\n"` |
| E3 | `printLine` | `line` contains bytes that are not valid UTF-8 (e.g. `0x80 0xFF 0xFE`) | `%s`/`puts` copies bytes verbatim → those raw bytes + `"\n"`; must NOT be replaced with U+FFFD or panic |
| E4 | `printLine` | `line` contains `printf` conversion specifiers (`"%s %n %d %%"`) | format string is the literal `"%s\n"`, so `line` is data → printed verbatim, no format interpretation, no crash |
| E5 | `printLine` | `line` longer than the stdio buffer (e.g. 8 KiB, 64 KiB) | full string + `"\n"`, no truncation |
| E6 | `printHexCharLine` | negative `char` (e.g. `-1`, `-2`, `CHAR_MIN` = `-128`) | default promotion sign-extends to `int`, `%02x` reinterprets as `unsigned` → `ffffffff` / `fffffffe` / `ffffff80` (**not** `ff`/`fe`/`80`) |
| E7 | `printHexCharLine` | `0` | `%02x` zero-pads to the minimum width → `"00"` |
| E8 | `printHexCharLine` | out-of-`char`-range value pushed through the FFI register (e.g. caller passes `int` `0x1234_5678` / `300` / `-1000` where a `char` is expected) | x86-64 SysV only defines the low 8 bits of the argument register; C reads the low byte and sign-extends it (`0x78`→`"78"`, `300`→`0x2c`→`"2c"`, `-1000`→`0x18`→`"18"`) |
| E9 | `bad` | none — `data = CHAR_MAX` makes `main.c:45 if(data > 0)` **always** true, and `127 * 2 == 254` truncated to `char` is the CWE-190 overflow | always prints exactly `"fffffffe\n"` |
| E10 | `goodB2G` (via `good`) | `data = CHAR_MAX` fails `main.c:70 if (data < CHAR_MAX/2)` i.e. `127 < 63` | takes the `else` at `main.c:75` → prints `"data value is too large to perform arithmetic safely.\n"`; the doubling is **never** performed |
| E11 | `good` | none — `goodG2B` then `goodB2G`, in that order | always prints exactly `"04\n"` then `"data value is too large to perform arithmetic safely.\n"` |
| E12 | `main` | **input failure**: stdin is empty / at EOF immediately | `scanf` returns `EOF`, result discarded, `x` keeps its initialiser `0` → `bad()` → `"fffffffe\n"`, exit status `0` |
| E12b | `main` | **read error** (distinct from EOF): fd 0 closed, `/dev/null`, or a *directory* opened as stdin (`read()` fails `EISDIR`) | `scanf` cannot distinguish these from EOF; it returns `EOF`, the value is discarded, `x` stays `0` → `bad()`, exit `0`. A read error must NOT panic or produce different output than EOF |
| E13 | `main` | **input failure**: stdin is whitespace only (`" \t\n\v\f\r"`) | `%d` skips all whitespace then hits EOF → `x` stays `0` → `bad()`, exit `0` |
| E14 | `main` | **matching failure**: first non-whitespace byte is not a digit or sign (`"abc"`, `"x"`, `"."`, `"\0"`, `"0x10"`→ stops after `0`) | `scanf` returns `0`, `x` unchanged/`0` → `bad()`, exit `0` |
| E15 | `main` | **matching failure**: sign then non-digit (`"-"`, `"+"`, `"- 5"`, `"+abc"`) | no conversion → `x` stays `0` → `bad()`, exit `0` |
| E16 | `main` | byte that Rust's `is_ascii_whitespace` misses but C `isspace` accepts: leading **vertical tab** `0x0B` before the digits (`"\x0b5"`) | `0x0B` **is** skipped as whitespace → `x = 5` → `good()` |
| E17 | `main` | byte that merely *looks* like space: `0x1C`..`0x1F`, `0xA0` (NBSP), `0x85` | not `isspace` in the C locale → matching failure → `x` stays `0` → `bad()` |
| E18 | `main` | value above `LONG_MAX` (`"99999999999999999999"`, 100-digit run) | glibc `%d` saturates to `LONG_MAX` then truncates into `int` → `-1`, which is non-zero → `good()` |
| E19 | `main` | value below `LONG_MIN` (`"-99999999999999999999"`) | saturates to `LONG_MIN` = `0x8000000000000000`, truncated to `int` → `0`, which is **false** → `bad()` |
| E20 | `main` | in-range `long` whose low 32 bits are zero (`"4294967296"`, `"8589934592"`) | truncation to `int` yields `0` → `bad()` (even though the parse succeeded) |
| E21 | `main` | explicit zero in every spelling (`"0"`, `"-0"`, `"+0"`, `"0000"`, `"  0  "`) | `x == 0` → `bad()`, exit `0` |
| E22 | `main` | `INT_MIN` / `INT_MAX` boundaries and one step past (`"2147483647"`, `"2147483648"`, `"-2147483648"`, `"-2147483649"`) | parsed as `long`, truncated to `int`; all four are non-zero → `good()` |
| E23 | `main` | digits immediately followed by junk (`"12abc"`, `"5-"`, `"7."`) | `%d` stops at the first non-digit; the conversion already succeeded → `x = 12/5/7` → `good()`; trailing bytes are never examined |
| E24 | `main` | huge digit run with leading zeros (`"0" * 400 + "1"`, `"0" * 400`) | leading zeros are not an error; value is `1` → `good()` / `0` → `bad()` |
| E25 | `main` | very long line / stdin larger than the stdio buffer before the number | irrelevant to output: only the first conversion matters; exit `0` |
| E26 | all five exports | called through the `.so` with the *wrong* effective enum/int width (there are no `enum` parameters in this API; the only integral parameter is `printHexCharLine`'s `char`) | covered by E8 — no enum-valued parameter exists to receive an out-of-range variant |

Notes on rows that are *not* reachable from the public API but whose guards
exist in the source (recorded for completeness, verified by inspection rather
than by an input, because no exported entry point can produce them):

* `main.c:45` / `main.c:56` / `main.c:68` `if(data > 0)` — `data` is a compile-time
  constant (`CHAR_MAX`, `2`, `CHAR_MAX`) at every one of the three sites, so the
  false branch is dead in both C and Rust. Rust keeps the `if` so the structure
  matches. No exported function takes the `data` value as a parameter.

## Status

| row | test | result |
|-----|------|--------|
| E1  | `tests/error_paths.rs::e1_print_line_null` | ✅ |
| E2  | `tests/error_paths.rs::e2_print_line_empty` | ✅ |
| E3  | `tests/error_paths.rs::e3_print_line_non_utf8` | ✅ |
| E4  | `tests/error_paths.rs::e4_print_line_format_specifiers` | ✅ |
| E5  | `tests/error_paths.rs::e5_print_line_very_long` | ✅ |
| E6  | `tests/error_paths.rs::e6_print_hex_char_line_negative` | ✅ |
| E7  | `tests/error_paths.rs::e7_print_hex_char_line_zero` | ✅ |
| E8  | `tests/error_paths.rs::e8_print_hex_char_line_out_of_range_int` | ✅ |
| E9  | `tests/error_paths.rs::e9_bad_always_overflows` | ✅ |
| E10 | `tests/error_paths.rs::e10_good_b2g_rejects_large_value` | ✅ |
| E11 | `tests/error_paths.rs::e11_good_order` | ✅ |
| E12 | `tests/error_paths.rs::e12_main_empty_stdin` | ✅ |
| E12b | `tests/error_paths.rs::e12b_main_unreadable_stdin` | ✅ |
| E13 | `tests/error_paths.rs::e13_main_whitespace_only` | ✅ |
| E14 | `tests/error_paths.rs::e14_main_matching_failure` | ✅ |
| E15 | `tests/error_paths.rs::e15_main_sign_without_digits` | ✅ |
| E16 | `tests/error_paths.rs::e16_main_vertical_tab_is_whitespace` | ✅ |
| E17 | `tests/error_paths.rs::e17_main_lookalike_space_bytes` | ✅ |
| E18 | `tests/error_paths.rs::e18_main_above_long_max` | ✅ |
| E19 | `tests/error_paths.rs::e19_main_below_long_min` | ✅ |
| E20 | `tests/error_paths.rs::e20_main_low_32_bits_zero` | ✅ |
| E21 | `tests/error_paths.rs::e21_main_explicit_zero_spellings` | ✅ |
| E22 | `tests/error_paths.rs::e22_main_int_boundaries` | ✅ |
| E23 | `tests/error_paths.rs::e23_main_digits_then_junk` | ✅ |
| E24 | `tests/error_paths.rs::e24_main_long_leading_zero_runs` | ✅ |
| E25 | `tests/error_paths.rs::e25_main_oversized_stdin` | ✅ |
| E26 | n/a — no enum-typed parameter exists; subsumed by E8 | ✅ |

## Harness validation (proof the tests are not vacuous)

A differential suite that never fails is worthless, so each divergence class was
injected into `src/imp.rs` and the suite re-run. Every **non-equivalent** mutant
is caught:

| injected divergence | detected |
|---------------------|----------|
| `c_isspace` → `u8::is_ascii_whitespace()` (drops vertical tab 0x0B) | ✅ 1 test failed |
| `printHexCharLine` zero-extends (`as u8 as i32`) instead of sign-extending | ✅ 15 tests failed |
| `printLine` routed through `String::from_utf8_lossy` | ✅ 1 test failed |
| `goodB2G` range check widened to `data <= CHAR_MAX` (takes the doubling branch) | ✅ 9 tests failed |
| `goodB2G` message loses its trailing `.` | ✅ 9 tests failed |
| `bad()` positivity guard inverted (`data < 0`) | ✅ 12 tests failed |
| `printLine` emits CRLF instead of LF | ✅ 13 tests failed |
| `good()` calls `goodB2G` before `goodG2B` | ✅ 9 tests failed |
| `printHexCharLine` uses `{:x}` instead of `{:02x}` | ✅ 21 tests failed |
| `scanf` skips only one leading whitespace byte | ✅ 2 tests failed |
| `scanf` saturates to `INT_MAX` instead of `LONG_MAX` | ✅ 1 test failed |
| `scanf` sign-then-EOF returns `Some(1)` | ✅ 1 test failed |
| removing the `#[no_mangle] good` export | ✅ symbol diff + 29 tests failed |

Two mutants were **not** detected, and both are provably *equivalent* (they
cannot change observable behaviour, so there is nothing to detect):

* `if (data < CHAR_MAX/2)` → `if (data < CHAR_MAX)`: `data` is always `CHAR_MAX`,
  and both `127 < 63` and `127 < 127` are false — the same `else` branch is taken.
* `scanf` sign-then-EOF returning `Some(0)` instead of `None`: `x` is already
  initialised to `0` and `scanf`'s return value is discarded, so both spellings
  leave `x == 0` and reach `bad()`. (The non-equivalent `Some(1)` variant above
  *is* caught.)

## Bug found and fixed

`ERRORS.md` row **E16** is a real divergence that existed in the translation
before this verification pass:

```rust
// before — WRONG: Rust's is_ascii_whitespace() excludes the vertical tab 0x0B
Some(b) if (b as char).is_ascii_whitespace() => continue,
// after  — matches C's isspace() in the C locale
fn c_isspace(b: u8) -> bool { matches!(b, 0x09..=0x0d | b' ') }
```

With stdin `"\x0b5"` the C program skips the vertical tab, parses `5`, and runs
`good()` (`"04\ndata value is too large…"`), while the original Rust treated
`0x0B` as a matching failure, left `x == 0`, and ran `bad()` (`"fffffffe"`).
