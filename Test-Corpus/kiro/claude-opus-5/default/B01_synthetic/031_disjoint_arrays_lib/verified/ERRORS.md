# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/driver.c` (63 lines, the only `.c` file).
Exhaustive grep of every rejection construct:

```
grep -n 'return\|assert\|break\|if (\|if(\|NULL\|ERROR\|errno\|exit' c_src/src/driver.c
```

Findings — the C library has **no** error enums, **no** `RETURN_ERROR`-style
macro, **no** `assert`, **no** `NULL` checks and **no** `errno` use. Every
rejection is one of:

* `call_fma`: `if (len == 0) return 0;` (the single explicit guard)
* `driver`:   `if (sscanf(in, "%d%zn", &data[i], &nb) != 1) break;`
* the two loop guards `i < len` (`fma_array`) and `i < 100` (`driver`),
  which reject work rather than signalling
* everything else is unchecked C (undefined behaviour), enumerated below so it
  is still covered.

Each row below is one distinct rejection / boundary condition, one test each in
`tests/errors.rs`.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `call_fma` | `len == 0` (explicit guard, `data` arbitrary incl. non-null) | returns `0`, never dereferences `data` | `err01_call_fma_len_zero` |
| 2 | `call_fma` | `len == 0` **and** `data == NULL` (guard runs before any deref) | returns `0`, no fault | `err02_call_fma_len_zero_null` |
| 3 | `fma_array` | `len == 0` (loop guard `i < len` false on entry) | writes nothing to `out`, returns void | `err03_fma_len_zero` |
| 4 | `fma_array` | `len < 0` (`-1`, `-7`, `INT_MIN`) | loop guard false on entry → writes nothing, no fault | `err04_fma_negative_len` |
| 5 | `fma_array` | `len == 0` with all four pointers `NULL` | no dereference → no fault | `err05_fma_null_ptrs_len_zero` |
| 6 | `fma_array` | signed-overflow operands: `mul1[i]*mul2[i]+add[i]` overflows `int` (`INT_MAX*2`, `INT_MIN*-1`, `INT_MAX+1`) | C signed overflow is UB; gcc/clang emit two's-complement wrapping `imul`/`add` | `err06_fma_overflow_wrap` |
| 7 | `driver` | `in` is the empty string `""` → `sscanf` returns `EOF (-1) != 1` on the first iteration | `i == 0` → `call_fma(data,0)` → row 1 → prints `"0\n"` | `err07_driver_empty` |
| 8 | `driver` | `in` starts with a non-convertible char (`"abc"`, `","`, `"x1"`) → matching failure, `sscanf` returns `0 != 1` | `i == 0` → prints `"0\n"` | `err08_driver_no_leading_int` |
| 9 | `driver` | `in` is whitespace only (`"   "`, `"\t\n "`) → whitespace consumed then EOF → returns `-1` | prints `"0\n"` | `err09_driver_whitespace_only` |
| 10 | `driver` | sign with no digits: `"-"`, `"+"`, `"- 5"`, `"--3"` → matching failure `0` | prints `"0\n"` | `err10_driver_lone_sign` |
| 11 | `driver` | trailing garbage after `k` valid ints (`"1 2 x"`, `"5,6"`) → loop breaks at the garbage | prints the `k`-th value parsed (`out[i-1]` == last int) | `err11_driver_garbage_tail` |
| 12 | `driver` | **more than 100** integers (101, 250, 1000) → `i < 100` guard stops the loop | only the first 100 are parsed; prints the **100th** integer, later ones ignored | `err12_driver_over_100` |
| 13 | `driver` | integer literal outside `int` range (`"99999999999"`, `"-99999999999"`, `"2147483648"`) → `%d` out-of-range, C says undefined | glibc truncates to the low 32 bits (`99999999999 -> 1215752191`, `2147483648 -> -2147483648`); both builds call the same libc | `err13_driver_out_of_range_int` |
| 14 | `driver` | `"0x10"` — `%d` is decimal-only, so it converts `0` and stops at `'x'`; next iteration fails | prints `0` | `err14_driver_hex_like` |
| 15 | `driver` | `"1e5"`, `"1.5"` — `%d` stops at the non-digit; next iteration fails on `'e'`/`'.'` | prints the leading integer (`1`) | `err15_driver_float_like` |
| 16 | `driver` | exactly 100 integers, then end of string (boundary of the `i < 100` guard, one step inside) | prints the 100th integer | `err16_driver_exactly_100` |
| 17 | `driver` | exactly 101 integers (one step past the documented capacity) | prints the 100th integer, the 101st is never read | `err17_driver_101` |
| 18 | `driver` | very long input (10 000+ chars, huge digit runs, embedded NUL handling — parsing stops at the NUL terminator) | identical output; no overrun of `data[100]` | `err18_driver_long_input` |
| 19 | `call_fma` | `len == 1` (one step past the `len == 0` guard, smallest array) | returns `data[0]` | `err19_call_fma_len_one` |
| 20 | *(FFI enum boundary)* | the public API declares **no enum type at all** (`grep -n 'enum' c_src` → no match), so there is no out-of-range enum variant to pass. The equivalent "any `int` is accepted" boundary is the unconstrained `int len` parameter, covered by rows 1, 3, 4, 19 and 21. | n/a — documented, nothing to test | *(documented)* |
| 21 | `call_fma` | `len < 0` → `int out[len]` is a **negative-size VLA**: undefined behaviour. Empirically the C build returns whatever stack garbage lies at `out[len-1]` (observed `2094670112`, `32767`, `464724882` on successive calls with the same arguments — not reproducible run to run). | no defined result; not byte-comparable. Rust returns a deterministic `0` and does not read out of bounds. Test asserts only that neither library faults. | `err21_call_fma_negative_len_ub` |
| 22 | `driver` | `in == NULL` → `sscanf(NULL, ...)` dereferences a null pointer inside glibc | both libraries fault identically (SIGSEGV). Verified out-of-process so the harness survives. | `err22_driver_null_ub` |

Rows 6, 21 and 22 are undefined behaviour in C, marked as such; every other row
is a defined, byte-comparable behaviour and is asserted for exact equality.
