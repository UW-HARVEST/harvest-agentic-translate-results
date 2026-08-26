# ERRORS.md — Phase C: error-surface table

Derived mechanically from `c_src/src/main.c` (49 lines, the only C file).

Grep results for the usual error idioms:

```
$ grep -nE 'return|assert|NULL|errno|exit|RETURN_ERROR|-1|<=|>=' c_src/src/main.c
27:    for (int i = 0; i < len; i++) {          <- range guard (fma_array)
34:    for (int i = 0; i < len; i++) {          <- range guard (driver)
42:    for (i = 0; i < 100; i++) {              <- capacity bound (100)
43:        if (scanf("%d", &data[i]) != 1) {    <- the ONLY rejection test
44:            break;
49:    return 0;                                <- always 0, never an error code
```

So the C code has **no** `assert`, **no** error enum, **no** `return -1`, **no**
`NULL` check and **no** `errno` use.  Its entire rejection surface is:

1. `scanf("%d", …) != 1` (every distinct way glibc's `%d` can fail to assign),
2. the `i < 100` capacity bound,
3. the two `i < len` loop guards (which make non-positive `len` a silent no-op),
4. the implicit `(int) strtol(...)` range clamping inside `%d`.

There are **no enum-typed parameters anywhere in the public surface**, so the
"out-of-range enum value across FFI" class degenerates to the only scalar
parameter, `int len`; its out-of-range values (`-1`, `INT_MIN`, values past the
caller's buffer) are rows 14, 15, 19, 20 and 22 below.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|----------------------------------------------|-------------------|------|
| 1  | `main` (program + `main` export) | stdin empty — immediate EOF | `scanf` → `EOF` (`-1`) at `i==0` → `break` → `driver(data,0)` → **no output**, exit status `0` | `errors.rs::err01_empty_stdin` |
| 2  | `main` | stdin holds only whitespace (`" \t\n\v\f\r"`), then EOF | EOF reached while skipping whitespace → `input_error()` → `-1` → `break` at `i==0` → no output, exit `0` | `errors.rs::err02_whitespace_only_stdin` |
| 3  | `main` | first non-whitespace byte is neither digit nor sign (`"abc"`, `"zz 1 2"`) | matching failure → `scanf` returns `0` → `break` at `i==0` → no output, exit `0` | `errors.rs::err03_leading_non_numeric` |
| 4  | `main` | sign not followed by a digit (`"-x"`, `"+ 5"`, `"--5"`, `"+."`) | `conv_error()` → `0` → `break` → no output, exit `0` | `errors.rs::err04_sign_then_non_digit` |
| 5  | `main` | sign immediately followed by EOF (`"-"`, `"+"`, `"  -"`) | glibc's `wpsize==1 && wp[0]` is a sign test fires → *matching* failure → `0` → `break` → no output, exit `0` | `errors.rs::err05_sign_then_eof` |
| 6  | `main` | invalid token *after* k valid ones (`"1 2 zz 3"`) | first k conversions succeed, the (k+1)-th returns `0` → `break` with `i==k` → exactly k lines printed | `errors.rs::err06_invalid_token_after_k_valid` |
| 7  | `main` | token that stops the conversion mid-way (`"5x"`, `"1.5"`, `"0x10"`, `"1e3"`) | digit prefix converts (returns `1`); the *next* `scanf` sees the offending byte and returns `0` → `break` | `errors.rs::err07_partial_token` |
| 8  | `main` | more than 100 integers on stdin (101 … 150) | `i < 100` bound ends the loop with `i==100`; the surplus is never read; exactly 100 lines printed | `errors.rs::err08_more_than_capacity` |
| 9  | `main` | magnitude in `(INT_MAX, LONG_MAX]` (`"3000000000"`, `"-2147483649"`) | `scanf` returns `1`; `*ARG(int*) = (int) num.l` keeps the low 32 bits | `errors.rs::err09_int_range_truncation` |
| 10 | `main` | magnitude `> LONG_MAX` (`"9223372036854775808"`, `"9"×40`) | `strtol` saturates to `LONG_MAX` (ERANGE); `(int) LONG_MAX == -1` → prints `(-1)*(-1)+(-1) == 0` | `errors.rs::err10_above_long_max` |
| 11 | `main` | magnitude `< LONG_MIN` (`"-9223372036854775809"`, `"-9"×40`) | `strtol` saturates to `LONG_MIN`; `(int) LONG_MIN == 0` → prints `0` | `errors.rs::err11_below_long_min` |
| 12 | `main` | NUL byte / byte ≥ 0x80 / control byte as first byte of a token | not `isspace`, not digit/sign → matching failure → `0` → `break` | `errors.rs::err12_nul_and_high_bytes` |
| 13 | `fma_array` | `len == 0`, four valid buffers | loop body never executes: no load, no store, buffers unchanged | `errors.rs::err13_fma_len_zero` |
| 14 | `fma_array` | `len < 0` (`-1`, `-100`, `INT_MIN`) | `i < len` false on entry → no work, buffers unchanged, returns normally | `errors.rs::err14_fma_len_negative` |
| 15 | `fma_array` | all four pointers `NULL`, `len <= 0` | no dereference happens → returns normally, no crash | `errors.rs::err15_fma_null_ptrs_len_le_zero` |
| 16 | `fma_array` | pointers `NULL` with `len > 0` | no NULL check in C → `mul1[0]` dereferences `NULL` → **SIGSEGV** | `errors.rs::err16_fma_null_ptrs_len_positive_segv` |
| 17 | `driver` | `len == 0`, valid buffer | both loops skipped → **zero bytes** of stdout, buffer unchanged | `errors.rs::err17_driver_len_zero` |
| 18 | `driver` | `len < 0` (`-1`, `INT_MIN`) | both loops skipped → zero bytes of stdout, buffer unchanged | `errors.rs::err18_driver_len_negative` |
| 19 | `driver` | `out == NULL`, `len <= 0` | no dereference → no output, no crash | `errors.rs::err19_driver_null_len_le_zero` |
| 20 | `driver` | `out == NULL`, `len > 0` | **SIGSEGV** | `errors.rs::err20_driver_null_len_positive_segv` |
| 21 | `fma_array` / `driver` | `len` one step past the caller's logical element count (oversized length, memory still mapped) | no bounds check in C → the extra elements are processed too; both implementations must transform the *same* extra elements identically | `errors.rs::err21_oversized_len_reads_past_logical_end` |
| 22 | `fma_array` | `len == INT_MAX` combined with pointer arithmetic (`i` walks to `INT_MAX`) | C increments `int i` up to `len`; the run faults long before wrapping → **SIGSEGV**, no `i` overflow trap | `errors.rs::err22_fma_huge_len_segv` |
| 23 | `main` | any rejection at all (rows 1-12) | `main` still `return 0` — the exit status never reports the failure | `errors.rs::err23_exit_status_is_always_zero` (and asserted inside every `errors.rs::err0*` test) |
| 24 | `fma_array` | **misaligned** `int *` (base at byte offset 1) — no alignment check exists in the C | plain unaligned x86-64 loads/stores, correct results, no fault | `program.rs::row39_fma_array_misaligned_pointers` |

All 24 rows have a passing differential test (rows 1-23 in `tests/errors.rs`,
row 24 in `tests/program.rs`); rows 16, 20 and 22 compare the *termination
signal* of a child process that calls the C `.so` against one that calls the
Rust `.so` (`examples/so_runner.rs`).

### Note on the fault mode of rows 16 / 20 / 22

The C dereferences the NULL pointer and dies with `SIGSEGV`.  rustc's
`-C debug-assertions` (on by default in the dev profile) inserts null/alignment
checks in front of every raw-pointer dereference, which turned those faults into
`panicked at ...: null pointer dereference occurred` → `abort()` → `SIGABRT`,
i.e. a *different* observable termination.  `[profile.dev]` therefore sets
`debug-assertions = false` / `overflow-checks = false` so the debug artifact
faults exactly like the release artifact and like the C.  Same story for row 24:
with the checks on, a misaligned `int *` aborted instead of just working.
