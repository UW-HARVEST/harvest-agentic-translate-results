# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/main.c`. Every branch that rejects input,
every constant that bounds input, and every unchecked dereference:

```
$ grep -n -E 'return|assert|errno|if|else|NULL|INT_MIN|INT_MAX|sizeof' c_src/src/main.c
24:#include <errno.h>
59:    errno = 0;
62:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
64:        return true;
65:    } else {
66:        return false;
71:    char in[100] = "";
72:    fgets(in, sizeof(in), stdin);
74:    if (parse_val(in, &x)) {
78:    } else {
81:    return 0;
```

Facts this establishes:

* There is exactly **one** rejection *statement* (`return false` at line 66) and
  exactly **one** user-visible rejection *message* (`printf("An error occurred\n")`,
  line 79), but the guard at line 62 is a conjunction of **four** independent
  conditions, i.e. four distinct triggers.
* There are **no** `assert`s, no `exit()`/`abort()`, no allocation and no error
  enums anywhere in the program; `main` always `return 0`, so the process exit
  status is always `0` on both paths.
* `fgets`' return value is **ignored**, so an EOF/read failure is not reported
  directly — it degrades into trigger #1 because `in` was initialised to `""`.
* The only size constant is `sizeof(in) == 100` (line 71/72): `fgets` stores at
  most 99 bytes + NUL, silently truncating longer lines.
* `run`, `print_house`, `add_floor`, `add_bedrooms` validate nothing and
  dereference `house` unconditionally → NULL is UB (SIGSEGV).
* There are **no enum parameters** anywhere in the public surface (the only
  scalar parameter is `int extra_bedrooms`), so "out-of-range enum value" maps
  onto "any `int` value", which is covered exhaustively at the boundaries
  (`INT_MIN`, `INT_MAX`, `0`, `±1`) and randomly over the whole `i32` range.

## Error-surface table

| # | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|---|----------|-------------------------------------------|-------------------|------|---|
| E1 | `parse_val` (via `main`) | `endp == str`: empty string — stdin at EOF, `fgets` returns NULL, `in` stays `""` | `false` → `An error occurred\n`, exit 0 | `err_e1_empty_input` | [x] |
| E2 | `parse_val` (via `main`) | `endp == str`: no digit anywhere (`"abc\n"`, `"hello world\n"`, `"!\n"`) | `false` → `An error occurred\n`, exit 0 | `err_e2_no_digits` | [x] |
| E3 | `parse_val` (via `main`) | `endp == str`: whitespace only (`"\n"`, `" \n"`, `"\t\v\f\r \n"`, 99 spaces) | `false` → `An error occurred\n`, exit 0 | `err_e3_whitespace_only` | [x] |
| E4 | `parse_val` (via `main`) | `endp == str`: sign with no digits (`"+\n"`, `"-\n"`, `"+ 1\n"`, `"--1\n"`, `"+-1\n"`) | `false` → `An error occurred\n`, exit 0 | `err_e4_sign_without_digits` | [x] |
| E5 | `parse_val` (via `main`) | `endp == str`: non-digit prefix before digits (`"x12\n"`, `".5\n"`, `" .5\n"`, `"#1\n"`, `"e5\n"`) | `false` → `An error occurred\n`, exit 0 | `err_e5_garbage_prefix` | [x] |
| E6 | `parse_val` (via `main`) | `errno != 0` (ERANGE): magnitude above `LONG_MAX` (`"9223372036854775808\n"`, `"1"+19..98 zeros`, 99 nines) | `false` → `An error occurred\n`, exit 0 | `err_e6_erange_positive` | [x] |
| E7 | `parse_val` (via `main`) | `errno != 0` (ERANGE): magnitude below `LONG_MIN` (`"-9223372036854775809\n"`, `"-99999999999999999999\n"`) | `false` → `An error occurred\n`, exit 0 | `err_e7_erange_negative` | [x] |
| E8 | `parse_val` (via `main`) | `tmp > INT_MAX` but ≤ `LONG_MAX` (`"2147483648"` … `"9223372036854775807"`) | `false` → `An error occurred\n`, exit 0 | `err_e8_above_int_max` | [x] |
| E9 | `parse_val` (via `main`) | `tmp < INT_MIN` but ≥ `LONG_MIN` (`"-2147483649"` … `"-9223372036854775808"`) | `false` → `An error occurred\n`, exit 0 | `err_e9_below_int_min` | [x] |
| E10 | `main` / `fgets` | line ≥ 100 bytes: truncation at 99 bytes turns an in-range number into an out-of-range/ERANGE one (e.g. 99 digits followed by more, `"-"`+120 digits) | truncated 99-byte prefix is what gets parsed → rejection or a *different* accepted value | `err_e10_truncation_changes_result` | [x] |
| E11 | `main` / `fgets` | NUL byte inside the line (`"\0"`, `"\0" + "5\n"`, `"12\0 34\n"`) — the C string ends at the NUL | prefix before NUL is parsed; `"\0…"` → rejection | `err_e11_embedded_nul` | [x] |
| E12 | `run` (FFI) | `the_house == NULL` (unchecked dereference in `print_house`) | SIGSEGV (UB), no output | `err_e12_null_house_segv` | [x] |
| E13 | `parse_val` (via `main`) | one step past the accepted range on both ends (`INT_MAX+1`, `INT_MIN-1`) vs one step inside (`INT_MAX`, `INT_MIN`) | `2147483648`/`-2147483649` rejected; `2147483647`/`-2147483648` accepted | `err_e13_off_by_one_range` | [x] |
| E14 | `parse_val` (via `main`) | `LONG_MAX`/`LONG_MIN` exactly (no ERANGE) but far outside `int` | `false` (fails the `INT_MIN`/`INT_MAX` test, *not* the `errno` test) | `err_e14_long_boundaries` | [x] |
| E15 | `main` / stdin | oversized and unterminated input: 100 KiB of digits with and without a trailing newline, 100 KiB of spaces, 1-byte lines with no newline | `fgets` stores only the first 99 bytes and NUL-terminates; the result follows that prefix (`9`×99 → ERANGE → rejection, `" 42"` → accepted) | `err_e15_oversized_and_unterminated` | [x] |
| E16 | `run` (FFI) | `extra_bedrooms` at/over the representable edges (`INT_MIN`, `INT_MAX`) combined with `bedrooms` of the same sign → signed overflow in `+=` (UB in C, wraps at -O0) | wraps modulo 2³²; printed with `%d` | `err_e16_extra_bedrooms_extremes` | [x] |
| E17 | `run` (FFI) | `floors == INT_MAX` → `house->floors++` signed overflow (UB in C, wraps at -O0) | wraps to `INT_MIN` | `err_e17_floors_overflow` | [x] |
| E18 | `run` (FFI) | non-finite / non-numeric `bathrooms` (NaN, −NaN, ±Inf, NaN with payload) fed to `%.1f` | glibc prints `nan`/`-nan`/`inf`/`-inf`; `+= 1.0` keeps them | `err_e18_non_finite_bathrooms` | [x] |
| E19 | `run` (FFI) | misaligned `house_t*` (offsets +1…+7) — UB in C, but x86-64 performs the unaligned access | the call succeeds and updates the struct in place | `err_e19_misaligned_house` | [x] |
| E20 | `main` / executable | stdout is a pipe with **no reader**: every `printf` fails with EPIPE | with the inherited (default) SIGPIPE disposition the process is killed by SIGPIPE (signal 13, no output); with SIGPIPE inherited as SIG_IGN it exits 0 with no output | `err_e20_stdout_without_reader` | [x] |
| E21 | `main` / executable | file descriptor 0 closed: `fgets` fails with EBADF and returns NULL, `in` stays `""` | `An error occurred\n`, exit 0 | `err_e21_closed_stdin` | [x] |

Note on E20: Rust's runtime sets SIGPIPE to SIG_IGN before `main`, which made
the first version of the translation exit 0 where the C died with signal 13.
`src/main.rs` therefore records the *inherited* disposition in an ELF
constructor (which runs before the Rust runtime initialises) and restores it as
`main`'s first action, so the executable dies — or does not die — exactly when
the C one does. The `.so`'s exported `main` deliberately does **not** do this,
because the C `.so` does not touch signal dispositions either.

Notes on E12/E19: the Rust export deliberately uses `ptr::read`/`ptr::write`
instead of forming a `&mut House`, because building a reference makes the debug
profile trip Rust's own null/alignment checks (SIGABRT) where the C simply
faults (SIGSEGV) or silently succeeds. With `ptr::read`/`ptr::write` both
implementations die with SIGSEGV on NULL and both succeed when misaligned, in
**both** the dev and release profiles.

## Generic FFI boundary cases (also required, covered)

| case | covered by |
|---|---|
| NULL pointer argument | E12 (`run(NULL, …)`) — both implementations die with SIGSEGV and print nothing |
| zero / oversized lengths | E1, E3, E10, E15 (0-byte stdin, 99/100/101-byte lines, 100 KiB line) |
| one step past a documented range | E13, E14 (`INT_MAX±1`, `INT_MIN±1`, `LONG_MAX`, `LONG_MIN`, `LONG_MAX+1`) |
| out-of-range "enum" value across FFI | no enums exist; the only scalar parameter (`int extra_bedrooms`) is swept at `INT_MIN`, `-1`, `0`, `1`, `INT_MAX` and randomly (E16, `cfg_r04`) |
| non-UTF-8 / arbitrary bytes on stdin | `err_e2_no_digits`, `cfg_m15_high_bytes`, `cfg_m18_random_bytes` |
