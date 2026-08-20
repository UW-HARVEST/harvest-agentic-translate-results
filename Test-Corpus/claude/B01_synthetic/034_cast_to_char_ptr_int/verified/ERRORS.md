# ERRORS.md — error-surface table (Phase A → Phase C)

## Mechanical derivation

`c_src/src/main.c` is 41 lines and contains **no** classic error plumbing.
Grepped for every rejection idiom:

```
$ grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|exit\(|abort|ERROR|goto' c_src/src/main.c
(no matches)
$ grep -nE 'return|if|while|for|<|>|==|!=' c_src/src/main.c | grep -v '^[0-9]*://'
24:#include <stdio.h>
27:    for (int i = 0; i < len; i++) {     # the only comparison in the file
41:    return 0;                          # the only return  (main, unconditional)
```

So there is exactly **one** conditional (`i < len`), **one** `return` (an
unconditional `return 0`), no `assert`, no null check, no range check and no
min/max constant. All rejection behaviour therefore lives in

1. the library call `scanf("%d", &x)` (whose result is **never checked**),
2. the ignored return values of `printf`/`putchar`,
3. the implicit `long`→`int` narrowing glibc performs when storing `%d`,
4. the *absence* of validation in `print_hex` / `driver` (every bit pattern is
   accepted), and
5. the process-level signal disposition inherited by the C program.

Each distinct rejection/failure mode below is one row. "expected C result" is
the measured behaviour of the C build (`c_src/build/driver` and
`build_c/libdriver_c.so`) on this platform (x86-64 Linux, glibc 2.34+,
`sizeof(int)==4`, `sizeof(long)==8`, little endian).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `main` / `scanf("%d")` | input failure: stdin is empty (immediate EOF) | `scanf` returns `EOF`; `x` keeps its initialiser `0`; stdout `00000000\n`; exit 0 | `e1_empty_stdin_input_failure` | [x] |
| E2 | `main` / `scanf("%d")` | input failure: stdin holds only whitespace (` `, `\t`, `\n`, `\v`, `\f`, `\r`, and mixes) then EOF | `EOF`; `x==0`; `00000000\n`; exit 0 | `e2_whitespace_only_input_failure` | [x] |
| E3 | `main` / `scanf("%d")` | input failure: read error on stdin — fd 0 closed (`EBADF`), and fd 0 open on a directory (`EISDIR`) | `EOF`; `x==0`; `00000000\n`; exit 0 | `e3_unreadable_stdin_input_failure` | [x] |
| E4 | `main` / `scanf("%d")` | matching failure: first non-whitespace byte cannot start a `%d` conversion (`a`, `.`, `,`, `/`, `:`, `\`, `x`, `#`, `\0`, `\x80`, `\xff`, `-`+letter…) | returns `0`; `x` untouched (`0`); `00000000\n`; exit 0; the offending byte is handed back to the stream, which *is* observable — a later conversion re-reads it (CONFIGS.md C28) and it counts towards the exit-time seek-back (C30) | `e4_leading_non_numeric_matching_failure` | [x] |
| E5 | `main` / `scanf("%d")` | matching failure: sign then EOF (`"-"`, `"+"`, `"   -"`) | `0`; `x==0`; `00000000\n`; exit 0 | `e5_sign_then_eof_matching_failure` | [x] |
| E6 | `main` / `scanf("%d")` | matching failure: sign then a non-digit (`"-a"`, `"+ 5"`, `"- 5"`, `"--5"`, `"+-5"`, `"-."`, `"-\n5"`) | `0`; `x==0`; `00000000\n`; exit 0 | `e6_sign_then_non_digit_matching_failure` | [x] |
| E7 | `main` / `scanf("%d")` | out of range, positive: digit string with magnitude `> LONG_MAX` (`9223372036854775808`, `2^64`, 10⁴⁰, 5000 nines) | glibc's internal `strtol` saturates at `LONG_MAX` (`ERANGE`, ignored); stored as `(int)LONG_MAX == -1`; stdout `ffffffff\n`; exit 0 | `e7_positive_overflow_saturates_to_long_max` | [x] |
| E8 | `main` / `scanf("%d")` | out of range, negative: `-` and magnitude `> LONG_MAX` (`-9223372036854775809`, `-2^64`, `-`10⁴⁰) | saturates at `LONG_MIN`; stored as `(int)LONG_MIN == 0`; stdout `00000000\n`; exit 0 | `e8_negative_overflow_saturates_to_long_min` | [x] |
| E9 | `main` / `scanf("%d")` | one step past the documented `int` range but inside `long`: `2147483648`, `-2147483649`, `4294967295`, `4294967296`, `9223372036854775807`, `-9223372036854775808` | no error at all — the `long` result is silently narrowed to the low 32 bits (`00000080`, `ffffff7f`, `ffffffff`, `00000000`, `ffffffff`, `00000000`) | `e9_int_range_overflow_is_silently_narrowed` | [x] |
| E10 | `main` | `scanf`'s return value (`EOF`/`0`/`1`) is never inspected | no failure is ever reported: exactly 9 bytes of output and exit status 0 for **every** input, valid or not | `e10_scanf_result_never_checked` | [x] |
| E11 | `driver`, `main` | stdout write error: fd 1 is closed (`EBADF`) | `printf`/`putchar` failures ignored; function returns normally; `main` returns 0; no output; no crash | `e11_stdout_closed_is_ignored` | [x] |
| E12 | `driver`, `main` | stdout write error: fd 1 is `/dev/full` (`ENOSPC` at flush) | error ignored; returns normally / exit 0 | `e12_stdout_enospc_is_ignored` | [x] |
| E13 | `driver`, `main` | stdout is a pipe whose read end is closed. The outcome depends on the **inherited** `SIGPIPE` disposition, which a C program never changes: `SIG_DFL` (a normal shell) vs `SIG_IGN` (a parent that ignores it, e.g. many daemons) vs a custom handler | with `SIG_DFL` the process is killed by `SIGPIPE` (signal 13) and no output appears — and no exit-time rewind of fd 0 happens; with `SIG_IGN` the write fails with `EPIPE`, the error is ignored and the process exits 0 (with the rewind) | `e13_stdout_epipe_raises_sigpipe` (exports), `e13b_executables_die_from_sigpipe_too` and `e13c_sigpipe_disposition_is_inherited` (the real programs: Rust's runtime overwrites the disposition with `SIG_IGN` before `main`, so `src/main.rs` captures the inherited one in an ELF constructor and restores it) | [x] |
| E14 | `print_hex` | `len <= 0` (`0`, negative) — no length validation exists | loop body never runs, only `"\n"` is printed | unreachable: `driver` hard-codes `sizeof(int)==4` and `print_hex` is `static` (absent from `nm -D`), so no FFI caller can trigger it; documented as not-exported in `SYMBOLS.md` | [x] |
| E15 | `print_hex` | `p == NULL` with `len > 0` — no null check exists | undefined behaviour in C (SIGSEGV in practice) | unreachable for the same reason as E14; `driver` always passes `&x`. Generic-null coverage for the exported surface is `e16_null_and_wide_arguments` (neither exported function takes a pointer) | [x] |
| E16 | `driver` | out-of-range argument across the FFI boundary: caller passes a value that does not fit `int` (the "C enum accepts any int" analogue — `driver` accepts all 2³² bit patterns, so the only invalid input is a *wider* one). Called through a `extern "C" fn(i64)` signature with garbage in the upper 32 bits, plus `INT_MAX+1`, `UINT_MAX`, `i64::MIN`, `-1i64` | SysV AMD64 passes `int` in the low half of the register: both builds print the low 32 bits and ignore the upper half; no crash | `e16_null_and_wide_arguments`, plus `e16b_main_called_with_extra_arguments` for the `main(argc, argv)` shape a real C start-up uses on an `int main(void)` | [x] |
| E17 | `main` | any failing input (E1–E9) | exit status is unconditionally `0` — the failure is invisible to the caller (`return 0` is the only `return` in the file) | `e17_exit_status_always_zero` | [x] |
| E18 | `main` / `scanf("%d")` | the `read` behind the conversion fails with `EINTR`: a signal arrives while `scanf` blocks on stdin and its handler was installed **without** `SA_RESTART` | `_IO_new_file_underflow` treats *any* failed `read` as a stream error — it sets the error indicator, reports end of input and never retries — so the conversion fails and `x` stays `0` (`00000000\n`), and the byte that arrives later is not consumed. With `SA_RESTART` (glibc `signal()`'s default) the read resumes and the value is converted normally | `e18_interrupted_read_is_an_input_failure`, `e18b_restarted_read_still_converts` | [x] |

All 18 rows have a passing differential test in `tests/error_paths.rs`
(E18 lives in `tests/stdin_stream.rs`, which owns the stream-level rows).

Beyond the table, `tests/error_paths.rs` also covers the generic API
boundaries the instructions call for regardless of the source:
`generic_zero_length_and_oversized_input` (empty stdin, 1 MiB of digits, 1 MiB of
whitespace, 1 MiB of junk) and `e16b_main_called_with_extra_arguments`
(out-of-signature arguments across the FFI boundary), while
`tests/executables.rs::known_good_outputs` pins absolute expected bytes so a
regression that hits *both* implementations cannot pass silently.

Two notes on what those tests can observe:

* **E14/E15 are documented-unreachable.** `print_hex` is `static`, so the symbol
  does not exist at the FFI boundary and there is nothing to compare;
  `e14_e15_print_hex_is_not_reachable_through_ffi` pins exactly that (both `.so`s
  lack the symbol, and the only reachable length is `sizeof(int)`), so the rows
  turn into real differential tests the moment someone exports it. The code path
  itself is covered through `driver` by every C1–C9 row.
* **E11/E12/E13 can only compare the exit status**, because their whole point is
  a stdout that cannot receive data (closed fd, `ENOSPC`, no reader). The output
  content for those wirings is empty by construction on both sides; the content
  comparisons live in the rows with a working stdout.

### Not emulated (deliberate, documented in CONFIGS.md)

`vfscanf` copies the digit run into a heap scratch buffer, so under an artificial
`RLIMIT_AS` a multi-megabyte digit run makes the *conversion* fail in C
(`00000000`) while the translation, which parses in constant memory, still
saturates (`ffffffff`). See the "Deliberately not emulated" table in
`CONFIGS.md`; without an artificial memory limit the two agree for every length
tested (up to 1 MiB of digits).
