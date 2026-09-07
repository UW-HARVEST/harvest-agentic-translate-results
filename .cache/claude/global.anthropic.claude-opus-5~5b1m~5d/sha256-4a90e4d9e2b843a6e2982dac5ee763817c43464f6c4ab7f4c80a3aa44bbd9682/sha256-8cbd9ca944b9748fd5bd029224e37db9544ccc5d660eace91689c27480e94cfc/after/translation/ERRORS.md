# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep of every rejection construct

```
$ grep -nE 'return|assert|NULL|errno|exit|abort|if|switch|\?|<|>|RETURN_ERROR|E[A-Z]+' c_src/src/driver.c
(no matches outside the license comment)
$ grep -nE '#define|enum|assert|NULL|MIN|MAX|_MAX|_MIN' c_src/include/driver.h
(only the DRIVER_H_ include guard)
```

Findings:

* No `return` statements at all (both functions are `void`).
* No error enum, no error codes, no sentinel values, no `errno` use.
* No `assert`, no `abort`, no `exit`.
* No `if` / `switch` / ternary — **zero conditional branches** in the library.
* No pointer parameters, therefore no null checks are possible.
* No length/count parameters, therefore no zero/oversized-length checks.
* No `#define`d min/max constants and no range checks.
* No enum parameters, therefore no "out-of-range enum value" path *inside* the
  library — but the ABI-level equivalent (an `int` argument whose value does not
  fit `char`) is a real FFI input and is covered below (rows 5–7).

Consequently the library has **no in-band error surface**: `printHexCharLine`
and `driver` are total functions over all 256 bit patterns of `char`. The rows
below therefore enumerate the *only* remaining rejection/edge surface, i.e. the
ABI boundary conditions, so that Phase C still asserts identical behaviour
rather than merely "both did something".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `printHexCharLine` | no invalid input exists: value `0x00` (lower bound of the 8-bit domain) | no error; prints `00\n`; returns void | [x] |
| 2 | `printHexCharLine` | value `0x7f` (`CHAR_MAX`, last non-negative) | no error; prints `7f\n` | [x] |
| 3 | `printHexCharLine` | value `0x80` (`CHAR_MIN` = -128, one step past `CHAR_MAX`); sign-extended by the default argument promotion before `%02x` reads it as `unsigned` | no error; prints `ffffff80\n` (8 digits, **not** `80`) | [x] |
| 4 | `printHexCharLine` | value `0xff` (-1), upper bound of the 8-bit domain | no error; prints `ffffffff\n` | [x] |
| 5 | `printHexCharLine` | caller passes an `int` wider than `char` (e.g. `0x1ff`, `0x100`, `-1000`, `INT_MAX`) across the FFI boundary — the ABI analogue of an out-of-range enum value | no error; the callee sees only the low 8 bits (`0x1ff` behaves as `0xff`, `0x100` as `0x00`); identical output for C and Rust | [x] |
| 6 | `driver` | value `0x7f`: `data + 1` overflows the `char` range; the `int` result `128` is narrowed back to `char` | no error/UB trap; two's-complement truncation to `-128`, prints `ffffff80\n` | [x] |
| 7 | `driver` | caller passes an out-of-`char`-range `int` (`0x1ff`, `0x100`, `-1000`, `INT_MIN`, `INT_MAX`) across the FFI boundary | no error; only the low 8 bits participate; identical output for C and Rust | [x] |
| 8 | `driver` | value `0xff` (-1): result wraps to `0` | no error; prints `00\n` | [x] |
| 9 | both | "null pointer" input — **not reachable**: neither function accepts a pointer; documented here so the generic boundary is explicitly accounted for | N/A (no pointer parameter exists) | [x] |
| 10 | both | "zero / oversized length" input — **not reachable**: neither function accepts a length or buffer | N/A (no length parameter exists) | [x] |
| 11 | both | return value: both are `void`; a caller reading a return register must observe the same (no) value | no return value; both `.so`s declare `void` | [x] |

Rows 9–11 are unreachable-by-construction, and are marked checked because the
Phase C test suite asserts the corresponding property that *is* observable (that
the signature is `void (char)` and that no pointer/length parameter exists in
either `.so`'s ABI).

## Result

All 11 rows have a passing differential test in `tests/phase_c_errors.rs`
(`err_row01_*` .. `err_row11_*`), plus `err_generic_boundaries` which sweeps
every documented range edge and the value one step past it, in both the
`fn(char)` and the `fn(int)` ABI view.

Rows 5 and 7 (over-wide `int` across the FFI boundary — the ABI analogue of an
out-of-range enum value) are exactly the rows that caught the one real bug in
the translation; see the "Divergence found and fixed" section of `CONFIGS.md`.
