# ERRORS.md — error-surface table (Phase C)

Derived mechanically from `c_src/src/main.c`. The whole translation unit is:

```c
void printHexCharLine (char charHex)
{
    printf("%02x\n", charHex);
}

int main()
{
    char data;
    data = ' ';
    fscanf (stdin, "%c", &data);
    {
        char result = data + 1;
        printHexCharLine(result);
    }

    return 0;
}
```

Grep results for every rejection/validation construct:

```text
$ grep -nE 'RETURN_ERROR|return |assert|NULL|errno|exit|if |else|switch|while|for ' c_src/src/main.c
41:    return 0;
```

So the C code contains **no** `assert`, **no** null check, **no** range check,
**no** error enum, **no** min/max constant and exactly one `return` (a constant
`0`). Its entire "error surface" is therefore implicit: the values returned by
the ignored `fscanf`/`printf` calls, and the platform-defined conversions. Every
one of those implicit rejection/edge paths is enumerated below — one row per
distinct condition — and each row has a differential test that asserts C and
Rust produce the *same* bytes and the *same* exit status.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `main` | stdin is empty (immediate EOF) — `fscanf` returns `EOF`, performs **0** conversions, so `data` keeps its initialiser `' '` | prints `21\n`, exit status 0 | `errors.rs::err01_stdin_empty_eof` |
| 2 | `main` | stdin is `/dev/null` (EOF, but a real open fd) — same `EOF` return | prints `21\n`, exit 0 | `errors.rs::err02_stdin_devnull` |
| 3 | `main` | stdin is an **already-closed** fd 0 → `read(2)` fails `EBADF` → `fscanf` returns `EOF`, no conversion, error flag set, return value ignored by the C code | prints `21\n`, exit 0 | `errors.rs::err03_stdin_closed_fd` |
| 4 | `main` | stdin is a **directory** fd → `read(2)` fails `EISDIR` → `fscanf` returns `EOF`, no conversion | prints `21\n`, exit 0 | `errors.rs::err04_stdin_is_directory` |
| 5 | `main` | stdin is the **write end of a pipe** (fd not readable → `EBADF`) → no conversion | prints `21\n`, exit 0 | `errors.rs::err05_stdin_not_readable` |
| 6 | `main` | stdout is a **closed fd 1** → `printf` fails (`EBADF`); the return value of `printf` is **ignored**, so the failure is silently swallowed | no output, exit 0 (no crash, no diagnostic) | `errors.rs::err06_stdout_closed_fd` |
| 7 | `main` | stdout is a full/unwritable target (`/dev/full`, `ENOSPC` on flush) — again ignored | no output, exit 0 | `errors.rs::err07_stdout_dev_full` |
| 8 | `main` | signed overflow boundary: input byte `0x7f` = `CHAR_MAX`; `data + 1` is computed in `int` (= 128) and converted back to `char`, which is *implementation-defined* (gcc: wraps to `-128`) | prints `ffffff80\n` (sign-extended `-128` under `%02x`), exit 0 | `errors.rs::err08_char_max_overflow_boundary` |
| 9 | `main` | one step past the boundary in the other direction: input byte `0xff` = `-1`; `-1 + 1 == 0` | prints `00\n`, exit 0 | `errors.rs::err09_minus_one_wraps_to_zero` |
| 10 | `main` | input byte `0x80` = `CHAR_MIN` (most negative `char`) | prints `ffffff81\n`, exit 0 | `errors.rs::err10_char_min_input` |
| 11 | `main` | input byte `0x00` (embedded NUL — no special meaning to `%c`) | prints `01\n`, exit 0 | `errors.rs::err11_nul_byte_input` |
| 12 | `main` | input first byte is whitespace (`'\n'`, `' '`, `'\t'`): `%c` does **not** skip leading whitespace, unlike every other scanf conversion | `'\n'`(0x0a) → `0b\n`; `' '` → `21\n`; `'\t'` → `0a\n`; exit 0 | `errors.rs::err12_whitespace_not_skipped` |
| 13 | `main` | oversized input (8 KiB … 1 MiB): only the **first** byte is converted, the rest is ignored (never read back) — no overflow, no error | prints hex of `first_byte + 1`, exit 0 | `errors.rs::err13_oversized_input` |
| 14 | `printHexCharLine` | argument whose value does not fit in `char` (out-of-range value crossing the FFI boundary, e.g. `0x1ff`, `256`, `-1000`, `i32::MIN`, `i32::MAX`): the callee uses the **low byte only** (`movsbl %dil,%esi`) | same output as the low byte reinterpreted as a signed `char` (e.g. `0x1ff` → `ffffffff`) | `inprocess.rs::B04_err14_print_hex_char_line_out_of_range_ints` |
| 15 | `printHexCharLine` | every negative `char` (`0x80..=0xff`): `%02x` reinterprets the sign-extended `int` as `unsigned`, so the `02` width is exceeded and 8 digits are printed | `-1` → `ffffffff`, `-128` → `ffffff80` | `inprocess.rs::err15_negative_char_sign_extension` |
| 16 | `printHexCharLine` | called with fd 1 closed → `printf` fails, return value ignored, function returns normally | no output, no crash | `errors.rs::err16_print_with_closed_stdout` |
| 17 | `printHexCharLine` | called repeatedly (1000×) — no state, no error accumulation, output is one line per call | 1000 identical/consecutive lines | `inprocess.rs::err17_repeated_calls_no_state` |
| 18 | `main` | exit status is **always** `0`, even after every failure above (`return 0` is unconditional) | exit 0 | `errors.rs::err18_exit_status_always_zero` (+ asserted in every row above) |

Notes on rows that are *not* differential-testable and are therefore excluded:

* Calling the exported `main` twice **inside one process** is not comparable:
  both implementations buffer stdin (glibc `FILE` buffer / Rust `StdinLock`
  `BufReader`), so the second call sees leftover buffered bytes. The C code
  itself is a process entry point; each row above is therefore executed in a
  fresh process (`examples/so_runner.rs` `dlopen`s the library and calls
  `main`), which is exactly how the C code is invoked in reality.
* There is no `argc`/`argv` in `int main()` (empty parameter list), so command
  line arguments cannot influence any path.

## Generic FFI-boundary boundaries (covered even though the C code has no checks)

| condition | why it is (or is not) applicable here | test |
|-----------|---------------------------------------|------|
| **null pointers** | *Not applicable*: neither exported function takes a pointer — `printHexCharLine(char)` and `main()` (no `argc`/`argv`). There is no pointer parameter that could be null. Verified from the C declarations, not assumed. | — |
| **out-of-range enum / integer values across FFI** | C has no enum here, but a `char` parameter accepts *any* `int` bit pattern from a caller; all extremes and 24 random `i32`s are pushed through both `.so`s. | `errors.rs::generic_wide_and_extreme_arguments_never_diverge`, `inprocess.rs::B04_err14_…` |
| **zero length input** | stdin of length 0 (file, pipe, `/dev/null`) | `errors.rs::err01`, `err02`, `generic_stdin_length_boundaries` |
| **oversized length input** | stdin of 4 KiB, 8 KiB and up to 1 MiB — straddling glibc's `BUFSIZ`/`st_blksize` and Rust's 8 KiB `BufReader` | `errors.rs::err13_oversized_input`, `generic_stdin_length_boundaries` |
| **one step past a documented range** | `0x7f`/`0x80` (`CHAR_MAX`/`CHAR_MIN`), `127`/`128`/`129`, `255`/`256`/`257`, `-129`, `i32::MIN`/`i32::MAX` | `errors.rs::err08`, `err09`, `err10`, `generic_wide_and_extreme_arguments_never_diverge` |
| **extra/unused arguments** | `int main()` takes none, so extra argv must be ignored | `errors.rs::generic_extra_argv_is_ignored` |
| **Rust must never panic/abort where C returns** | every error row additionally asserts the Rust side exits 0 with empty stderr (a Rust panic would show `panicked at` on stderr and a non-zero status) | all rows |

All 18 rows plus the 7 generic boundaries pass against both libraries
(`cargo test --test errors` → 18 passed, `--test inprocess` → 9 passed) under
every feature combination and in both the dev and release profiles.
