# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/driver.c`. Every function in this library
returns `void` and reports exclusively through `stdout`, so "expected C result"
is the exact byte sequence written to `stdout` (captured by redirecting fd 1).

The C source contains exactly **two** literal conditional rejection branches
(`grep -n 'return\|assert\|NULL\|exit\|abort\|errno\|#if\|else'` yields only
line 32 `if(line != NULL)` and line 66 `else`). There are no error return
codes, no error enums, no `assert`s, no length parameters and no allocation
failures. Rows E1–E2 are those two literal branches; rows E3–E13 are the
remaining rejection-shaped / degenerate boundary conditions the C code actually
reaches — chiefly the `double`→`int` conversion whose result is out of range,
which on x86-64 (`cvttsd2si`) yields the "integer indefinite" value `INT_MIN`
(`-2147483648`) rather than trapping.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| E1  | `printLine` | `line == NULL` (line 32 null check fails) | **no output at all** (0 bytes); silent no-op, does not crash | [x] |
| E2  | `good` → `goodB2G` | `fabs(data) > 0.000001` is false (line 61 `else`, line 66) | `50\n` from `goodG2B`, then `This would result in a divide by zero\n` | [x] |
| E3  | `bad` | `data == 0.0f` → `100.0/0.0` = `+inf` → `(int)+inf` out of range | `-2147483648\n` | [x] |
| E4  | `bad` | `data == -0.0f` → `100.0/-0.0` = `-inf` → `(int)-inf` out of range | `-2147483648\n` | [x] |
| E5  | `bad` | `data` is `NaN` → `100.0/NaN` = `NaN` → `(int)NaN` invalid | `-2147483648\n` | [x] |
| E6  | `bad` | `data` positive but tiny (e.g. `1e-30f`, subnormal `1e-45f`) → quotient `> INT_MAX` | `-2147483648\n` | [x] |
| E7  | `bad` | `data` negative but tiny (e.g. `-1e-30f`) → quotient `< INT_MIN` | `-2147483648\n` | [x] |
| E8  | `bad` | `data` exactly at the overflow edge: quotient `>= 2147483648.0` (one step past valid range) | `-2147483648\n` | [x] |
| E9  | `bad` | `data` one step *inside* the edge: quotient `< 2147483648.0` | truncated quotient, **not** `INT_MIN` (proves E8 is a real boundary, not blanket saturation) | [x] |
| E10 | `bad` | `data` is `+inf` / `-inf` → `100.0/inf` = `±0.0` → `(int)` of zero | `0\n` (**not** an error; must not be conflated with E3–E8) | [x] |
| E11 | `good` → `goodB2G` | `data` is `NaN`: `fabs(NaN) > 0.000001` is false (NaN compares false) | `50\n` then `This would result in a divide by zero\n` | [x] |
| E12 | `good` → `goodB2G` | `data == 1e-6f`: `(double)1e-6f` = `9.99999997475e-07` `< 1e-06`, so `>` is **false** | `50\n` then the divide-by-zero message — the literal `0.000001` is a `double`, so the float threshold lands *below* it | [x] |
| E13 | `printLine` | `line` points at a string containing `%d`/`%s`/`%n` conversion specifiers | the string is printed **verbatim** (it is the `%s` argument, never the format) — no format-string interpretation | [x] |

## Generic FFI-boundary conditions also covered

Even though they are not distinct branches in the C, these are exercised by
`tests/differential.rs` because they are the classic blind spots:

| condition | covered by |
|-----------|------------|
| NULL pointer into `printLine` | E1 |
| zero-length input (`""`, empty NUL-terminated string) | `test_e13_and_empty_and_percent_strings` (`""` → `"\n"`) |
| oversized input (8 KiB string, > any internal buffer) | `test_printline_oversized` |
| full `int` range incl. `INT_MIN`/`INT_MAX` across the FFI boundary | `test_printintline_boundaries` |
| values one step past a valid range | E8 / E9 (int-conversion edge), E12 (threshold edge) |
| out-of-range enum values | **N/A** — this library declares no `enum` and takes no `int`-tagged mode/flag parameter. The only integer parameter is `printIntLine`'s payload, whose entire 32-bit domain is valid and is tested at both extremes plus randomized values. |
| non-UTF-8 bytes in `printLine` (Rust `str` would reject these; the C does not) | `test_printline_non_utf8` |
