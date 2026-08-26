# ERRORS.md — Phase A: error-surface table

Mechanically derived by grepping **every** rejection/validation construct in the
C source.  Exhaustive grep of `c_src/src/main.c` (the only C file):

```
$ grep -nE 'return|assert|NULL|if|else|switch|error|Error|ERROR|exit|abort|<|>|<=|>=|==|!=|MAX|MIN|LIMIT|sizeof' c_src/src/main.c
28:    if(line != NULL)      <-- the ONE and ONLY validation in the library
63:    return 0;             <-- success status of main, not an error path
```

There are **no** `assert`s, **no** `return -1` / `return NULL` error returns,
**no** error enums or error codes, **no** range checks, **no** min/max
constants, **no** `errno` use, **no** allocation (so no allocation-failure
path), and **no** `#ifdef` in the whole translation unit.  The complete error
surface is therefore a single row (#1); rows #2–#8 are the generic FFI
boundaries mandated for every C API, which are tested even though the C code
contains no explicit check for them.

| # | function | trigger (the exact invalid input/condition) | expected C result | ✅ |
|---|----------|---------------------------------------------|-------------------|----|
| 1 | `printLine` | `line == NULL` (`if (line != NULL)` at line 28 is false) | **no output at all** (0 bytes written), returns `void`, no crash | [x] |
| 2 | `printLine` | zero length: `line` points at `""` (immediate NUL) — the boundary just inside the accepted branch | writes exactly one byte, `"\n"` | [x] |
| 3 | `printLine` | oversized length: 64 KiB / 1 MiB string, i.e. far past `BUFSIZ` and past Rust's `LineWriter` capacity | writes all N bytes then `"\n"`, no truncation | [x] |
| 4 | `printLine` | bytes that are **not** valid UTF-8 (`0x80..0xFF`, lone continuation bytes, truncated sequences) — invalid for Rust `str`, perfectly valid `char*` for C | writes the raw bytes verbatim then `"\n"` (never replacement chars, never a panic) | [x] |
| 5 | `printLine` | the string itself contains printf directives (`%s`, `%d`, `%n`, `%%`) — a naive translation that used it as a format string would diverge/crash | the directives are emitted **literally** as data | [x] |
| 6 | `printIntLine` | one step past each end of the documented `int` range: `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` (there is no range check, so these are accepted) | `"%d\n"` of the value, i.e. `-2147483648\n` / `2147483647\n` (no overflow, no panic — Rust must not use a checked negation) | [x] |
| 7 | `printIntLine` | out-of-range *argument* across the FFI boundary: caller pushes a 64-bit value (e.g. `0x1_0000_0001`, `0xFFFF_FFFF_FFFF_FFFF`) where the callee declares `int` | callee reads only the low 32 bits (`edi`), so both sides must print the identically truncated `i32` | [x] |
| 8 | `main` | `argc = 0` / `argc < 0` / `argv = NULL` / `argv` with NULL entries — parameters the C `main` never dereferences | ignored; full 8-line program output, returns `0` | [x] |

## Out-of-range enum values

`c_src/src/main.c` declares **no** `enum` and **no** function taking an enum, so
there is no enum-valued parameter that could receive an out-of-variant `int`.
The equivalent "any bit pattern the C ABI permits" test for this API is row #7
(an `int` parameter fed a value with junk in the upper 32 bits) plus row #6
(the extremes of the `int` range), both of which are covered.

## Result

All 8 rows are implemented as differential tests in
`tests/phase_c_errors.rs` (`err01`…`err08`, plus three generic-boundary rows) and
all pass against both `.so` files — the assertion is always on the *specific*
result (exact bytes / exact return value), never merely "both failed somehow".

Run with `cargo test --test phase_c_errors`, or `./run_all.sh` for the whole
matrix:

```
suite `Phase C — error paths (ERRORS.md)`: 11 passed, 0 failed, 0 skipped (2568 captured .so calls)
```

The suite was validated against 4 deliberately injected Rust bugs; each was
caught by the rows that cover it (e.g. dropping the `printLine` NULL guard fails
rows 1 and 2, and `String::from_utf8_lossy` fails rows 3 and 4).

## Conditions deliberately NOT tested (undefined behavior in C, not a
## rejection the C code performs)

* a non-NUL-terminated buffer passed to `printLine` — the C `printf`/`puts`
  would read out of bounds (UB); the C code performs no length check, so there
  is nothing to compare against.
* a misaligned / dangling `const char *` — same reason.
