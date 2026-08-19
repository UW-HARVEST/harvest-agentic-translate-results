# ERRORS.md — error-surface table (Phase C)

## Mechanical grep of the C source for rejection sites

```
$ grep -nE 'RETURN_ERROR|return |assert|NULL|errno|goto|exit\(|-1' c_src/src/main.c
44:    return 0;
```

`c_src/src/main.c` contains:

* **no** error-return macro, **no** error enum, **no** `return -1`, **no**
  `return NULL` (`main`'s `return 0` is its only `return`),
* **no** `assert`, **no** explicit range check, **no** null check,
* **no** min/max constant, **no** `if` at all (the only control flow is the
  `for (int i = 0; i < len; i++)` loop in `print_hex`),
* **no** pointer parameter on any *exported* function (`driver` takes an `int`
  by value; `main` takes nothing), hence **no** null-pointer rejection path
  exists in the export surface,
* **no** `enum` type anywhere, hence there is no "invalid enum variant across
  FFI" case for a caller to construct (extreme/garbage `int` bit patterns are
  covered anyway, rows 15–17).

The library's entire *rejection* behaviour therefore lives in (a) the failure
modes of the ignored `scanf("%d", &x)` call, (b) the ignored return value of
`printf`, (c) the implementation-defined `long -> int` conversion glibc's `%d`
performs, and (d) I/O errors on the two descriptors the code touches.  Every one
of those is enumerated below, one row per distinct rejection/condition.

Legend for "expected C result": `x` is `main`'s local, initialised to `0`; the
printed line is what `driver`/`print_hex` emit on stdout (little-endian x86-64).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `main` (`scanf`) | stdin is empty (immediate EOF) — *input failure*, `scanf` returns `EOF` | `x` unchanged `0`; prints `00000000\n`; exit `0` | `err_01_empty_stdin` | [x] |
| 2  | `main` (`scanf`) | stdin holds only whitespace (` \t\n\v\f\r`) then EOF — *input failure* after ws skip | `x` unchanged `0`; `00000000\n`; exit `0` | `err_02_whitespace_only` | [x] |
| 3  | `main` (`scanf`) | first non-ws byte cannot start a decimal integer (`"abc"`, `"?"`, `"."`, `"x1"`) — *matching failure*, `scanf` returns `0` | `x` unchanged `0`; `00000000\n` | `err_03_matching_failure` | [x] |
| 4  | `main` (`scanf`) | sign with no digit after it (`"-"`, `"+"` then EOF) — matching failure | `x` unchanged `0`; `00000000\n` | `err_04_sign_only_eof` | [x] |
| 5  | `main` (`scanf`) | sign followed by a non-digit (`"- 5"`, `"+x"`, `"--5"`, `"-+5"`) — matching failure | `x` unchanged `0`; `00000000\n` | `err_05_sign_then_nondigit` | [x] |
| 6  | `main` (`scanf`) | embedded NUL byte first (`"\0 5"`) — NUL is neither space nor digit ⇒ matching failure | `x` unchanged `0`; `00000000\n` | `err_06_nul_byte` | [x] |
| 7  | `main` (`scanf`) | value `> LONG_MAX` (`"9223372036854775808"`, 40-digit, 5000-digit) — `strtol` sets `ERANGE`, glibc uses `LONG_MAX` | `(int)0x7fffffffffffffff == -1`; `ffffffff\n` | `err_07_over_long_max` | [x] |
| 8  | `main` (`scanf`) | value `< LONG_MIN` (`"-9223372036854775809"`, 40-digit, 5000-digit) — `ERANGE`, glibc uses `LONG_MIN` | `(int)0x8000000000000000 == 0`; `00000000\n` | `err_08_under_long_min` | [x] |
| 9  | `main` (`scanf`) | value fits `long` but not `int`, positive (`2147483648 … 9223372036854775807`) — silent truncation, **not** rejected | low 32 bits of the `long`, e.g. `2147483648` ⇒ `00000080\n` | `err_09_int_overflow_positive` | [x] |
| 10 | `main` (`scanf`) | value fits `long` but not `int`, negative (`-2147483649 … -9223372036854775808`) — silent truncation | low 32 bits, e.g. `-2147483649` ⇒ `ffffff7f\n` | `err_10_int_overflow_negative` | [x] |
| 11 | `main` (`scanf`) | exactly `LONG_MIN` written as `"-9223372036854775808"` (no `ERANGE`, boundary of the saturation branch) | `0`; `00000000\n` | `err_11_long_min_exact` | [x] |
| 12 | `main` (`scanf`) | exactly `LONG_MAX` (`"9223372036854775807"`, no `ERANGE`) | `(int)-1`; `ffffffff\n` | `err_12_long_max_exact` | [x] |
| 13 | `main` (`scanf`) | fd 0 closed ⇒ `read` fails with `EBADF` ⇒ `scanf` returns `EOF` | `x` unchanged `0`; `00000000\n`; exit `0` | `err_13_stdin_closed` | [x] |
| 14 | `main` (`scanf`) | fd 0 is a directory ⇒ `read` fails with `EISDIR` ⇒ `scanf` returns `EOF` | `x` unchanged `0`; `00000000\n`; exit `0` | `err_14_stdin_is_directory` | [x] |
| 15 | `main` (`printf`) | fd 1 closed ⇒ every `printf` fails; return value ignored | no output; still returns `0` | `err_15_stdout_closed` | [x] |
| 16 | `driver` | no validation exists: `INT_MIN` / `INT_MAX` / `0` / `-1` passed across FFI | 4 native-order bytes as `%02x`, e.g. `INT_MIN` ⇒ `00000080\n` | `err_16_driver_extremes` | [x] |
| 17 | `driver` | "garbage"/out-of-range-looking ints (all-bits-set, `0x80000000`, `0x7fffffff`, high-bit-set patterns, `i64`-truncated values passed in the 64-bit register) | never rejected; low 32 bits printed | `err_17_driver_garbage_bits` | [x] |
| 18 | `main` | `scanf` failure never changes the exit status (its return value is dropped) | exit status `0` for **every** input above | asserted in every `main` test | [x] |
| 19 | `print_hex` (static, unreachable from the export surface) | `len <= 0` would print only `"\n"`; `driver` always passes `sizeof(int) == 4`, so the branch is dead | n/a — not exported, cannot be reached by a caller | documented only | [x] |
| 20 | `main` (`scanf`) | a **second** conversion after EOF was already reached: C99 makes EOF sticky (`_IO_EOF_SEEN`), so `_IO_file_underflow` returns EOF without even attempting a `read` | `x` stays `0`; `00000000\n` for every further call | `cfg_33_so_main_repeated_same_process` | [x] |
| 21 | `main` (`scanf`) | a **second** conversion after a read *error* (fd 0 closed): `_IO_ERR_SEEN` is **not** sticky, so the `read` is retried and fails again | `x` stays `0`; `00000000\n` for every call | `cfg_33_so_main_repeated_same_process` (`Stdin::Closed`, n = 1/2/3/5) | [x] |

## Generic FFI boundary cases (also required by Phase C)

| case | applicability | test |
|------|---------------|------|
| null pointer arguments | **no exported function takes a pointer** — `driver(int)`, `main(void)` | n/a (documented above) |
| zero length / oversized length | no length or buffer parameter is exposed; `print_hex`'s `len` is hard-wired to `sizeof(int)` | n/a (row 19) |
| one step past a documented valid range | `int` has no sub-range restriction; both ends (`INT_MIN`, `INT_MAX`) and both ends ±1 of the `long`→`int` conversion are exercised | rows 7–12, 16 |
| out-of-range enum value across FFI | no enum exists in the C source | n/a (documented above) |
| calling the symbol repeatedly / re-entrancy | both `driver` and `main` are stateless apart from stdio | `cfg_29_repeated_calls` |
