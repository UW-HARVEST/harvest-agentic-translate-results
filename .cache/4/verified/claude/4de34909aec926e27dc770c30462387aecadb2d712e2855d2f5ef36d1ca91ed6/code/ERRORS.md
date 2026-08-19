# ERRORS.md — error-surface table (Phase C)

## How this table was derived

`c_src/src/main.c` is the whole C library (51 lines, 23 of which are the licence
header). Mechanical grep for every rejection construct:

```
$ grep -nE 'return|assert|NULL|errno|exit|if|switch|<|>|==|!=|#if|EOF|-1' c_src/src/main.c
24:#include <ctype.h>
25:#include <locale.h>
26:#include <stdio.h>
27:#include <stdlib.h>
```

i.e. **the C code contains no `return` statement, no `assert`, no `if`, no
`switch`, no comparison, no error enum, no null check, no range check and no
min/max constant.** `driver()` returns `void`; `main()` falls off the end
(status 0). Neither the `setlocale()` nor any of the 14 `printf()` return values
is inspected.

The error surface is therefore entirely *implicit*: it consists of the failure
modes of the libc calls the C code makes without checking them, plus the values
that reach `driver()`'s `char` parameter across the FFI boundary. Every such
distinct condition is one row below. "Reference output for `char c`" means the
14 lines `driver(c)` prints.

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `main`   | stdin empty → `getchar()` returns `EOF` (-1), stored into `char` → `c == -1`; no EOF check | prints reference output for `char -1` (all classes `0`, `to lower`/`to upper` = byte `0xFF`), returns 0 | `err_01_main_stdin_empty_eof` |
| 2  | `main`   | stdin is `/dev/null` (immediate EOF) | identical to row 1 | `err_02_main_stdin_devnull` |
| 3  | `main`   | stdin file descriptor 0 **closed** → `read()` fails `EBADF` → `getc` returns `EOF` | identical to row 1, returns 0 | `err_03_main_stdin_closed` |
| 4  | `main`   | stdin is a **write-only** fd (`O_WRONLY` file) → `read()` fails `EBADF` | identical to row 1, returns 0 | `err_04_main_stdin_write_only` |
| 5  | `main`   | stdin is a **directory** fd → `read()` fails `EISDIR` | identical to row 1, returns 0 | `err_05_main_stdin_directory`, `binaries_e2e.rs::e2e_stdin_is_a_directory` |
| 6  | `main`   | stdin's first byte is `0xFF` → same `char` value as `EOF`; the missing EOF check makes the two indistinguishable | identical to row 1 (byte `0xFF` and EOF produce the same 14 lines), returns 0 | `err_06_main_byte_ff_aliases_eof` |
| 7  | `main`   | stdin's first byte is `0x00` (embedded NUL) | reference output for `char 0`: `control: 2`, everything else `0`, `to lower`/`to upper` print a NUL byte | `err_07_main_byte_nul` |
| 8  | `main`   | stdout (fd 1) **closed** → every `printf`/flush fails; return values unchecked | no output, no diagnostic, returns 0 (and nothing on stderr) | `err_08_main_stdout_closed`, `binaries_e2e.rs::e2e_stdout_closed`, `binaries_e2e.rs::e2e_no_stderr_output` |
| 9  | `main`   | stdout is a **broken pipe** (read end closed) with `SIGPIPE` ignored by the caller → writes fail `EPIPE`, unchecked | no output, returns 0 | `err_09_main_stdout_broken_pipe_sigpipe_ignored` |
| 10 | `main`, `driver` | stdout is a **broken pipe** with the default `SIGPIPE` disposition (a C program's startup state) | process is killed by signal 13 (shell status 141), no output | `err_10_broken_pipe_default_sigpipe_kills_both`, `binaries_e2e.rs::e2e_broken_pipe_kills_with_sigpipe` |
| 11 | `driver` | `c == -1` (`0xFF`): negative index into glibc's ctype tables (the classification macros index `__ctype_b_loc()[c]` with a negative subscript) | all 12 classifications `0`; `to lower`/`to upper` print byte `0xFF` | `err_11_driver_minus_one` |
| 12 | `driver` | `c == -128` (`0x80`): lowest possible `char` value, lowest valid negative table index | all 12 classifications `0`; `to lower`/`to upper` print byte `0x80` | `err_12_driver_minus_128` |
| 13 | `driver` | `c == 127` (`0x7F`, DEL): highest positive `char`, one past the printable range | `control: 2`, all other classifications `0`; `to lower`/`to upper` print byte `0x7F` | `err_13_driver_del_127` |
| 14 | `driver` | `c == 0`: `printf("%c", tolower(0))` emits a NUL byte inside the output stream | `control: 2`; `to lower`/`to upper` lines contain a NUL byte | `err_14_driver_nul` |
| 15 | `driver` | every negative `char` (`-128..=-1`, i.e. bytes `0x80..=0xFF`) — the whole out-of-`unsigned char`-range half that `isalnum()` et al. are not documented to accept | all 12 classifications `0`, identity `to lower`/`to upper` (glibc's "C" table repeats `0x80..0xFF` for the negative index range) | `err_15_driver_all_negative_chars` |
| 16 | `driver` | **out-of-range value across the FFI boundary**: caller declares the symbol as `void driver(int)` and passes garbage above the low byte (`0x1234_5641`, `0xFFFF_FF80`, `i32::MIN`, `i32::MAX`, 512 random `i32`s) — the C-enum analogue for this API, since `char` has no invalid bit pattern but the register does | only the low byte is significant: identical to `driver((char)(v & 0xFF))` | `err_16_driver_int_with_garbage_high_bits` |
| 17 | `driver` | `setlocale(LC_ALL, "C")` return value unchecked, and the *caller's* locale was previously switched (`setlocale(LC_ALL, "en_US.iso88591")`, a locale where bytes `0x80..0xFF` *are* alphabetic) | `driver` resets to the `"C"` locale, so the output is the C-locale reference output for every byte — the caller's locale must not leak in | `err_17_driver_ignores_host_locale` |
| 18 | `driver` | `printf` return value unchecked: fd 1 closed | no output, no diagnostic, `driver` returns normally | `err_18_driver_stdout_closed` |
| 19 | `driver` | called repeatedly in one process (repeated `setlocale`, no re-entrancy guard) | outputs simply concatenate; no state drift | `err_19_driver_repeated_calls_no_state_drift` |
| 20 | `main`   | called repeatedly in one process: the second call must observe the *second* byte of stdin, not a re-read of the first (stdio buffering) | successive calls consume successive bytes, then EOF (= row 1 output) forever | `err_20_main_repeated_calls_consume_successive_bytes` |

## Generic FFI boundaries that do not apply to this API

| generic class | status |
|---------------|--------|
| null pointer arguments | N/A — neither `driver(char)` nor `main(void)` takes a pointer |
| zero / oversized length arguments | N/A — no length, size or count parameter exists |
| out-of-range enum values | no enum parameter exists; the equivalent "any `int` bit pattern reaches a narrow parameter" case is row 16 |
| return-value error codes | `driver` returns `void`; `main` returns `int` and always returns 0 (falls off the end) — asserted in rows 1–10 |
| variadic / string arguments | N/A |
| extra arguments passed across the boundary | `int main()` is *unprototyped* in C, so a caller may pass `argc`/`argv`/`envp`, and `driver` may be handed extra register arguments. Both must ignore them: `err_generic_extra_arguments_across_the_ffi_boundary` (64 randomised `argc`/pointer triples) |
| exported symbol surface | `nm -D` on the C `.so` must be exactly `driver`, `main`: `err_generic_no_pointer_or_length_parameters` + `tests/symbols.rs` |
