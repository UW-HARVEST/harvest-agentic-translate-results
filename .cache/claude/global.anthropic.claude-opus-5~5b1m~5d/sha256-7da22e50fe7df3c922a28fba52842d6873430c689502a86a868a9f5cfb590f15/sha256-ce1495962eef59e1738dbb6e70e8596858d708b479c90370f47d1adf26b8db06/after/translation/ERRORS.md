# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Grepping the whole C source for every rejection mechanism:

```
grep -n "return\|assert\|NULL\|errno\|exit\|if\|else\|switch\|#if" c_src/src/driver.c c_src/include/driver.h
```

yields **only `#include` lines and the header's `#ifndef` guard**. Therefore:

* `driver` returns `void` — there is no error code, no sentinel, no out-param.
* There are **zero** `return` statements, `RETURN_ERROR` macros, error enums,
  `assert`s, range checks, null checks, and min/max constants in the library.
* The only parameter is a by-value `char`; no pointer is ever accepted, so there
  is no null-pointer path inside the library.

The error surface is consequently entirely *implicit*: it consists of the
inputs that are unusual/out-of-range for the C-library primitives the function
feeds them to (`is*()` on a possibly-negative `char`, `%c` on a possibly-zero
or negative `int`, and values whose bits exceed the `char` parameter). Those are
enumerated below and each has a differential test, because "the C never rejects
anything" is a claim that must be *verified*, not assumed: any input where the
Rust errored/panicked/aborted while the C returned normally would be a
divergence.

Observable result for every row is "no rejection: returns normally (void) after
writing exactly 14 lines to `stdout`" — the differential test asserts the Rust
also returns normally AND that the 14 lines are byte-identical.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| E1 | `driver` | `c = 0` (NUL) — `printf("%c")` is asked to emit a NUL byte, which truncates nothing but writes a `\0` into the stream | no error; returns void; `to lower: \0\n` and `to upper: \0\n` written literally (line contains an embedded NUL) |
| E2 | `driver` | `c` negative (`-1 ..= -128`, i.e. bytes `0x80..=0xFF` through a *signed* `char`) — the `is*()` macros index glibc's `__ctype_b` table with a **negative** index | no error; all 12 predicates print `0`; `tolower`/`toupper` are identity, `%c` narrows back to the original byte |
| E3 | `driver` | `c = -128` (`0x80`) — the most negative `char`, the extreme low index of the `__ctype_b` / `__ctype_tolower` / `__ctype_toupper` tables (one step below is out of the table) | no error; predicates `0`; `%c` emits byte `0x80` |
| E4 | `driver` | `c = 127` (`0x7F`, DEL) — top of the signed `char` range and the last table slot with `_IScntrl` set | no error; `control: 2`, all other predicates `0`, `printing: 0`, `graphical: 0`; `%c` emits `0x7F` |
| E5 | `driver` | `c = -1` (`0xFF`), the byte identical to `EOF` after promotion — the classic ctype misuse value | no error; all predicates `0`; `%c` emits `0xFF` (NOT the `EOF` slot semantics) |
| E6 | `driver` | argument whose value does not fit in `char`: caller passes an `int` with garbage in bits 8..31 (e.g. `0x1234_5641`, `0x0000_01FF`, `0xFFFF_FF80`) across the FFI boundary — a C `char` parameter accepts any `int` at the ABI level, exactly like an out-of-range enum value | no error; the callee uses only the low 8 bits (sign-extended), so behaviour is that of `(char)(value & 0xFF)` |
| E7 | `driver` | out-of-range "enum-like" ints one step past each documented boundary: `256`, `-129`, `INT_MIN`, `INT_MAX` passed as the argument | no error; same low-8-bits behaviour as E6; no table over-read, no trap |
| E8 | `driver` | called repeatedly / after `setlocale` has been changed by the caller to a non-`"C"` locale — `driver` re-runs `setlocale(LC_ALL, "C")` on every call, so a hostile locale must not leak into the classification | no error; output identical to a fresh call (the locale reset makes the function idempotent) |
| E9 | `driver` | `setlocale(LC_ALL, "C")` return value is ignored by the C (it is never checked for `NULL`) — so a `setlocale` failure must not change control flow | no error; the 14 `printf`s run unconditionally |

## Status

| # | test | status |
|---|------|--------|
| E1 | `tests/diff.rs::error_e1_nul_byte` | [x] pass |
| E2 | `tests/diff.rs::error_e2_negative_chars` | [x] pass |
| E3 | `tests/diff.rs::error_e3_min_char` | [x] pass |
| E4 | `tests/diff.rs::error_e4_del_127` | [x] pass |
| E5 | `tests/diff.rs::error_e5_eof_like_ff` | [x] pass |
| E6 | `tests/diff.rs::error_e6_wide_int_arg` | [x] pass |
| E7 | `tests/diff.rs::error_e7_one_past_range` | [x] pass |
| E8 | `tests/diff.rs::error_e8_locale_hostile_caller` | [x] pass |
| E9 | `tests/diff.rs::error_e9_setlocale_result_ignored` | [x] pass |

All rows pass in **both** the debug and the release Rust `.so` (see
`run_verification.sh`). Rows **E6 and E7 initially FAILED against the release
build with a SIGSEGV** while the C returned normally; see the "Findings" section
of `CONFIGS.md` for the cause (an optimiser-elided bounds check on a table
indexed with a full-width register) and the fix in `src/ctype.rs`.

## Test-suite sensitivity

The suite was mutation-checked to confirm it is not vacuously green: flipping a
single class bit (`isblank` for `\t`) failed 5 rows, and changing the case
mapping of one character (`toupper('a')`) failed 6 rows.
