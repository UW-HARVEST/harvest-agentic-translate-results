# ERRORS.md — error-surface table (Phase A → Phase C)

## Mechanical derivation

Every rejection path was found by grepping the *whole* C source
(`c_src/src/container_of.c`, 33 lines) for each error idiom:

```sh
grep -nE 'RETURN_ERROR|return +-?[0-9]|return +NULL|assert|abort|exit|errno|
          if *\(|else|switch|\?|MAX|MIN|goto|perror|fprintf *\( *stderr' \
     c_src/src/container_of.c
# -> no matches
```

**The C code contains zero explicit error handling:** no `if`, no `assert`, no
`return -1`, no `return NULL`, no error enum, no range check, no null check, no
min/max constant, no `goto`, no `exit`. There is also **no enum type anywhere**
in the C source, so there is no "out-of-range enum variant" for this API (the
nearest analogue — a nonsensical `argc` — is covered by rows 12–14).

Consequently the error surface is entirely *implicit*: it consists of the
failures the C code lets happen (faults from unchecked pointer dereferences),
and of the silent value-mangling that its unchecked library calls and
unchecked arithmetic perform. Each distinct such behaviour is one row below.

Legend for "expected C result": `SIGSEGV` means the process dies from signal 11
having produced **no** output; `= v` means the call returns exactly `v`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `main` | `argc < 2`, so `argv[1] == NULL` → `atoi(NULL)` → `strtol` dereferences NULL | `SIGSEGV`, no stdout output, no partial line |
| 2  | `main` | `argc == 2`, so `argv[2] == NULL` → second `atoi(NULL)`. `argv[1]` was already parsed successfully, i.e. the fault happens *after* the first conversion and *before* `printf` | `SIGSEGV`, no stdout output |
| 3  | `main` | `argv == NULL` (only reachable through FFI; a real C runtime never does it) → load of `argv[1]` from address `0x8` | `SIGSEGV`, no stdout output |
| 4  | `main` | `argv[1]` is a non-NULL but unmapped/wild pointer | `SIGSEGV`, no stdout output |
| 5  | `atoi` (via `main`) | subject sequence empty — no digits at all (`"abc"`, `""`, `"@"`, `"\x80"`, `"0x"`→`"0"`… ) | conversion silently yields `0`; **no** error is reported |
| 6  | `atoi` (via `main`) | only whitespace (`" "`, `"\t\n\v\f\r"`) | silently `0` |
| 7  | `atoi` (via `main`) | sign with no following digit (`"+"`, `"-"`, `"+ 5"`, `"--5"`, `"+-5"`) | silently `0` |
| 8  | `atoi` (via `main`) | trailing garbage after digits (`"12abc"`, `"3.9"`, `"7 8"`) | digits before the garbage are used, the rest is discarded, no error (`12`, `3`, `7`) |
| 9  | `atoi` (via `main`) | value `> LONG_MAX` (`"9223372036854775808"`, 100-digit numbers) | `strtol` saturates to `LONG_MAX` and sets `ERANGE`; `atoi` casts → `= -1`; the `ERANGE` is discarded |
| 10 | `atoi` (via `main`) | value `< LONG_MIN` (`"-9223372036854775809"`, `-1e100`) | saturates to `LONG_MIN`, cast → `= 0`; `ERANGE` discarded |
| 11 | `atoi` (via `main`) | value fits `long` but not `int` (`"2147483648"`, `"-2147483649"`, `"4294967296"`) | silent truncation `(int)`: `-2147483648`, `2147483647`, `0` — no error |
| 12 | `main` | `argc` inconsistent with the array (`argc = 0` with a 3-entry `argv`) | `argc` is never read → identical behaviour to a correct `argc`; prints the sum |
| 13 | `main` | `argc` negative (`-1`, `INT_MIN`) — an out-of-domain value for the parameter | ignored → prints the sum |
| 14 | `main` | `argc` absurdly large (`INT_MAX`) | ignored → prints the sum |
| 15 | `main` | `t.a + t.b` overflows `int` (`INT_MAX + 1`, `INT_MIN + (-1)`) | no check; wraps two's-complement (`-2147483648`, `2147483647`) |
| 16 | `find_container_of_a` | `i == NULL` — no null check exists | `= NULL` (`offsetof(struct test, a) == 0`, so the pointer is returned unchanged) |
| 17 | `find_container_of_b` | `i == NULL` — no null check exists | `= (struct test *)0xFFFFFFFFFFFFFFFC` (`0 - offsetof(struct test, b)`, i.e. wraps below zero) |
| 18 | `find_container_of_b` | `0 < i < 4` (`1`, `2`, `3`) — subtraction wraps past address 0 | `= i - 4` computed modulo 2⁶⁴ (`0xFFFFFFFFFFFFFFFD`, `…FE`, `…FF`) |
| 19 | `find_container_of_b` | `i == usize::MAX` / any value whose `- 4` is still nonsense | `= i - 4`, no validation, no fault (nothing is dereferenced) |
| 20 | `find_container_of_a` / `_b` | misaligned `int *` (odd address) — an invalid `int *` in C | accepted, plain address arithmetic, no fault |
| 21 | `main` | `printf` fails (stdout closed / EBADF) | return value discarded → exit status still `0`; no message |

## Row → test mapping (Phase C)

All rows are covered by `tests/error_paths.rs` (fault rows run the call in a
`fork()`ed child so the exact terminating signal can be compared) and
`tests/differential.rs`.

| row | test |
|-----|------|
| 1 | `row01_main_argc0_argv1_null_segv` |
| 2 | `row02_main_argc2_argv2_null_segv` |
| 3 | `row03_main_argv_null_segv` |
| 4 | `row04_main_argv1_wild_pointer_segv` |
| 5 | `row05_atoi_no_digits` |
| 6 | `row06_atoi_whitespace_only` |
| 7 | `row07_atoi_sign_without_digits` |
| 8 | `row08_atoi_trailing_garbage` |
| 9 | `row09_atoi_above_long_max` |
| 10 | `row10_atoi_below_long_min` |
| 11 | `row11_atoi_long_but_not_int` |
| 12 | `row12_argc_zero_ignored` |
| 13 | `row13_argc_negative_ignored` |
| 14 | `row14_argc_int_max_ignored` |
| 15 | `row15_int_addition_overflow_wraps` |
| 16 | `row16_find_container_of_a_null` |
| 17 | `row17_find_container_of_b_null` |
| 18 | `row18_find_container_of_b_underflow` |
| 19 | `row19_find_container_of_b_max` |
| 20 | `row20_misaligned_pointers` |
| 21 | `row21_printf_failure_ignored` |

- [x] row 1
- [x] row 2
- [x] row 3
- [x] row 4
- [x] row 5
- [x] row 6
- [x] row 7
- [x] row 8
- [x] row 9
- [x] row 10
- [x] row 11
- [x] row 12
- [x] row 13
- [x] row 14
- [x] row 15
- [x] row 16
- [x] row 17
- [x] row 18
- [x] row 19
- [x] row 20
- [x] row 21
