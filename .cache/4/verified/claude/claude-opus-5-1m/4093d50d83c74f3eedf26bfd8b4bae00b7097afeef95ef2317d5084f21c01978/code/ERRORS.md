# ERRORS.md — Phase A error-surface table

## Mechanical derivation

The whole library is `c_src/src/main.c` (3 functions, 19 code lines). Grepping it for every
rejection mechanism:

```
$ grep -nE 'assert|return|RETURN|NULL|if *\(|while|for|else|#define|MAX|MIN|sizeof' c_src/src/main.c
29:    for (const char *s = in; s = strchr(s, c); s++) {
32:    return res;
42:    fread(in, 1, sizeof(in), stdin);
44:    return 0;
```

Findings:

* **no** `assert`, **no** `RETURN_ERROR`-style macro, **no** `return -1` / `return NULL`,
  **no** error enum, **no** explicit range check, **no** null check, **no** min/max constant;
* the only branch in the whole file is the implicit `strchr(...) != NULL` loop condition;
* the only size constant is `sizeof(in)` = `1000` (`char in[1000]`);
* both `fread`'s and `printf`'s return values are discarded, so **every** I/O failure is
  silently ignored;
* `main` unconditionally `return 0`.

The error surface therefore consists of the loop-termination path, the unchecked I/O results,
the buffer boundary, and the undefined-behavior paths the C reaches for hostile arguments.
One row per distinct rejection/failure mode:

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | differential test |
|---|----------|---------------------------------------------|-------------------|-------------------|
| 1 | `foo` | `in == NULL` (no null check; `strchr(NULL, c)` dereferences it) | process dies from **SIGSEGV** (signal 11), no output | `c01_foo_null_pointer` |
| 2 | `driver` | `in == NULL` — forwarded to `foo` before anything is printed | process dies from **SIGSEGV** (signal 11), no output (the pending `printf` is never flushed) | `c02_driver_null_pointer` |
| 3 | `foo` | `c == '\0'` — `strchr(s, 0)` *matches the terminating NUL*, so the loop condition is never NULL and `s++` walks forever | walks past the object until it hits an unmapped page → **SIGSEGV** (signal 11); never returns | `c03_foo_nul_needle_runs_off_the_end` |
| 4 | `foo` | `c` above the `char` range pushed in as `int` (`0x141`, `256+120`, `65601`, `INT_MAX`, …) — no validation, the callee only looks at the low 8 bits | silently truncated to `(char)(c & 0xff)`; return value = occurrences of that byte | `c04_foo_needle_out_of_char_range` |
| 5 | `foo` | `c` below the `char` range / negative (`-1`, `-129`, `-191`, `INT_MIN+1`, …) | same 8-bit truncation, e.g. `-1` matches byte `0xff` | `c04_foo_needle_out_of_char_range` |
| 6 | `foo` | needle not present at all — `strchr` returns NULL on the first iteration (the function's only regular rejection path) | returns `0` | `c05_foo_no_occurrence_returns_zero` |
| 7 | `main` | stdin already at EOF / empty (`fread` returns 0, return value discarded) | buffer stays all-zero → `"A: 0\nx: 0\n"`, exit status 0 | `c06_main_unreadable_or_empty_stdin` |
| 8 | `main` | stdin cannot be read at all (fd 0 is a directory → `read(2)` fails with `EISDIR`; `fread`'s error is discarded) | buffer stays all-zero → `"A: 0\nx: 0\n"`, exit status 0 | `c06_main_unreadable_or_empty_stdin` |
| 9 | `main` | more than `sizeof(in) == 1000` bytes available on stdin | everything past byte 1000 is silently dropped (only the first 1000 bytes are ever inspected) | `c07_main_input_longer_than_buffer` (a) |
| 10 | `main` | ≥ 1000 bytes with **no NUL** among the first 1000 → `in` is left **unterminated** | **undefined behavior**: `strchr` keeps scanning the stack behind `in[1000]`. Measured: deterministic per process image but environment-dependent (same binary + same input gave `A: 134` from a shell and `A: 135` when spawned from the test harness). The C can only ever report ≥ the in-buffer counts. | `c07_main_input_longer_than_buffer` (b), `c08_main_exactly_1000_non_nul_bytes`, and the UB branch of `b08_executables_random` |
| 11 | `driver` | `printf` fails (stdout is `/dev/full` → `ENOSPC`); the return value is discarded | failure ignored, no diagnostic, exit status 0 | `c09_output_write_error_is_ignored` |
| 12 | `driver` | `printf` writes to a pipe whose read end is closed | a C program keeps SIGPIPE at `SIG_DFL`, so the process is **killed by signal 13**; the Rust runtime's default `SIG_IGN` had to be reverted in `main_impl` to match | `c10_sigpipe_on_closed_stdout` |
| 13 | `main` | any failure whatsoever (short read, read error, write error) | `main` still `return 0` — success is always reported | `c11_main_always_returns_zero` |
| 14 | `foo` / `driver` | empty string (`in[0] == '\0'`), i.e. the shortest accepted input | `0` for every needle → `"A: 0\nx: 0\n"` | `c12_empty_string_inputs` |
| 15 | `main` | NUL byte inside the data read by `fread` | the C string ends at that NUL; the rest of the buffer is unreachable even though it was read | `b07_main_via_so_embedded_nul` |
| 16 | `foo` | more than `INT_MAX` occurrences → `res++` signed overflow (UB) | **unreachable**: `res` counts occurrences inside one NUL-terminated string; reaching `INT_MAX` would need a ≥ 2 GiB string, and `main`'s only caller passes a 1000-byte buffer. Rust uses `wrapping_add`, so it also cannot panic if a caller ever managed it. | not testable (documented) |

Status: rows 1–15 each have a passing differential test (see `cargo test` output); row 16 is
provably unreachable and is documented instead of tested.

## Generic FFI boundary cases (covered even though the C has no checks for them)

| condition | where |
|-----------|-------|
| null pointer arguments | rows 1–2 |
| zero length / empty input | rows 7, 14 |
| oversized length (> buffer) | rows 9, 10 |
| one step past the documented range (`char`/`int` mismatch, `±256` wrap, `INT_MIN`/`INT_MAX`) | rows 4, 5 — `foo`'s second parameter is the only enum-like value in the API; every out-of-range `int` is accepted and truncated by both implementations |
| value with no valid variant (`c` = `0` is the one value that changes the algorithm's meaning) | row 3 |
| buffer-size boundaries 999 / 1000 / 1001 | rows 9, 10 and `CONFIGS.md` rows 11, 17, 18 |
