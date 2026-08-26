# ERRORS.md — Phase C error-surface table

Mechanically derived from every statement in `c_src/src/main.c` that can reject,
fail on, or mis-handle input. The whole C file is 3 functions / 15 statements, so
the grep is exhaustive:

```
$ grep -n -E "return|assert|NULL|-1|scanf|printf|memcpy|for |if |sizeof" c_src/src/main.c
34:    for (int i = 0; i < len; i++) {      <- only bound/guard in the file
35:        printf("%02x", p[i]);
37:    printf("\n");
45:    char raw[sizeof(house)];
46:    memcpy(raw, &house, sizeof(house));
47:    print_hex((unsigned char *)&raw, sizeof(raw));
52:    scanf("%d", &x);                     <- only failable call
54:    return 0;                            <- only return value, always 0
```

Findings:

* There is **no** `RETURN_ERROR`-style macro, **no** error enum, **no**
  `return -1` / `return NULL`, **no** `assert`, **no** null check, **no**
  explicit range check and **no** min/max constant in the C source.
* `driver(int)` takes a plain `int`; every one of the 2^32 values is accepted
  (no validation whatsoever), so it has no rejection surface — only the
  extremes are interesting, and they are covered as boundary rows below.
* `print_hex` has the only guard in the file (`i < len`); it is `static` and its
  single call site always passes `sizeof(raw) == 16`, so `len <= 0` is
  unreachable through the public API.
* The entire real error surface is therefore **`scanf("%d", &x)`**, whose return
  value the C code *ignores*. Consequence: every failure mode degrades to
  "`x` keeps its initializer `0`", and `main` still returns `0`.

`OUT0` below is the output for `x == 0`:
`00000000030000000000000000000040\n`

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| E1 | `main` / `scanf("%d")` | stdin empty → immediate EOF (input failure) | `scanf` returns `EOF`, `x` stays `0` → `OUT0`, exit `0` |
| E2 | `main` / `scanf("%d")` | stdin is only whitespace (` `, `\t`, `\n`, `\v`, `\f`, `\r`, and mixes) then EOF | input failure, `x` stays `0` → `OUT0`, exit `0` |
| E3 | `main` / `scanf("%d")` | fd 0 closed → `read()` fails `EBADF` | treated as input failure, `x` stays `0` → `OUT0`, exit `0` |
| E4 | `main` / `scanf("%d")` | fd 0 is a directory → `read()` fails `EISDIR` | input failure, `x` stays `0` → `OUT0`, exit `0` |
| E5 | `main` / `scanf("%d")` | first non-space byte is not sign/digit (`abc`, `.5`, `x`, `,`, `/`, `:`) | matching failure, `x` stays `0` → `OUT0`, exit `0` |
| E6 | `main` / `scanf("%d")` | `-` alone (sign then EOF) | matching failure, `x` stays `0` → `OUT0` |
| E7 | `main` / `scanf("%d")` | `+` alone (sign then EOF) | matching failure, `x` stays `0` → `OUT0` |
| E8 | `main` / `scanf("%d")` | sign then non-digit (`- 5`, `+a`, `--1`, `-+1`, `-.5`) | matching failure, `x` stays `0` → `OUT0` |
| E9 | `main` / `scanf("%d")` | byte `0x00` (NUL) as first byte | matching failure, `x` stays `0` → `OUT0` |
| E10 | `main` / `scanf("%d")` | non-ASCII bytes / UTF-8 "digits" (`٥` = `0xd9 0xa5`, `0xff`, `０`) | matching failure (`%d` is ASCII-only in the C locale), `x` stays `0` → `OUT0` |
| E11 | `main` / `scanf("%d")` | magnitude above `LONG_MAX` (`9223372036854775808`, `99999999999999999999`, `18446744073709551616`, 1000-digit number) | glibc's `strtol` saturates to `LONG_MAX` (`ERANGE`, which `scanf` ignores), stored into `int` → `-1` → `ffffffff030000000000000000000040` |
| E12 | `main` / `scanf("%d")` | magnitude below `LONG_MIN` (`-9223372036854775809`, `-99999999999999999999`, 1000-digit negative) | saturates to `LONG_MIN`, truncated to `int` → `0` → `OUT0` |
| E13 | `main` / `scanf("%d")` | exactly `LONG_MAX` = `9223372036854775807` (boundary, no `ERANGE`) | low 32 bits → `-1` → `ffffffff…` |
| E14 | `main` / `scanf("%d")` | exactly `LONG_MIN` = `-9223372036854775808` (boundary, no `ERANGE`) | low 32 bits → `0` → `OUT0` |
| E15 | `main` / `scanf("%d")` | one step past `INT_MAX`: `2147483648` (valid `long`, out of `int` range — no check in C) | silent truncation → `0x80000000` → `00000080030000000000000000000040` |
| E16 | `main` / `scanf("%d")` | one step past `INT_MIN`: `-2147483649` | silent truncation → `0x7fffffff` → `ffffff7f030000000000000000000040` |
| E17 | `main` / `scanf("%d")` | other in-`long`/out-of-`int` values (`4294967296`, `-4294967295`, `4294967297`, `1099511627776`) | silent truncation to low 32 bits (`0`, `1`, `1`, `0`) |
| E18 | `main` / `scanf("%d")` | `0x10` / `0b1` / `010` — a base prefix, but `%d` is base 10 only | converts the leading `0`, stops at the prefix letter → `x = 0` (and `010` → `10`) |
| E19 | `main` / `scanf("%d")` | digits followed by garbage (`42xyz`, `42.9`, `42 43`) | conversion succeeds with the leading digits, remainder ignored (program exits) |
| E20 | `main` / `scanf("%d")` | huge digit run with leading zeros (`0000…0005`, 5000 zeros) | leading zeros do not overflow; value `5` |
| E21 | `driver` (exported) | boundary `int` arguments `INT_MIN`, `INT_MAX`, `-1`, `0` — the C validates nothing | no rejection; prints the raw little-endian image, e.g. `INT_MIN` → `00000080030000000000000000000040` |
| E22 | `print_hex` (static, unreachable) | `len <= 0` | loop body never runs → prints only `"\n"`. Not reachable through any exported symbol (`sizeof(raw)` is always 16); documented for completeness and mirrored by the `len < 0 → 0` clamp in `src/imp.rs::print_hex`. |
| E23 | `driver` / `main` (write side) | stdout cannot absorb the output: `/dev/full` → every `write` fails `ENOSPC`, and the C ignores `printf`'s return value | no output, **no** error, `main` still returns `0` |
| E24 | `driver` / `main` (write side) | stdout is a pipe whose read end is closed → `SIGPIPE` | the process is killed by signal 13 (no exit code). The Rust bin matches because it uses `#![cfg_attr(not(test), no_main)]`; a normal Rust `fn main` would have installed `SIG_IGN` for `SIGPIPE` and diverged here |
| E25 | `main` / `scanf("%d")` | pathological sizes: 100 KiB of `a`, 100 KiB of spaces then `77`, 100 KiB of digits | matching failure / `77` / saturation → `-1`; no crash, no truncation of the *program's* behaviour, exit `0` |

## Generic FFI boundary items required by the task

| item | applicability here | how covered |
|---|---|---|
| null pointers | **N/A** — neither exported function takes a pointer (`void driver(int)`, `int main()`) | nothing to pass; documented |
| zero / oversized lengths | **N/A** — no length parameter is exposed (`print_hex`'s `len` is internal, see E22) | E22 row |
| one step past a documented valid range | `driver`'s `int` domain is fully saturated; the ranges that *can* be exceeded are `int`/`long` in `scanf` | E13–E17, E21 |
| out-of-range enum values across FFI | **N/A** — the C source declares no `enum` and no exported function takes one | documented |
| return-code parity | `main` must return `0` for *every* input, valid or not | asserted for every row (exit status compared) |

## Row → test mapping (all passing)

| rows | test in `tests/differential.rs` |
|------|---------------------------------|
| E1 | `error_path_e1_empty_stdin` |
| E2 | `error_path_e2_whitespace_only` |
| E3 | `error_path_e3_read_error_ebadf` (write-only fd **and** closed fd 0) |
| E4 | `error_path_e4_read_error_eisdir` |
| E5 | `error_path_e5_leading_garbage` (32 leading bytes + whitespace-prefixed variants) |
| E6 | `error_path_e6_lone_minus` |
| E7 | `error_path_e7_lone_plus` |
| E8 | `error_path_e8_sign_then_nondigit` (16 cases) |
| E9 | `error_path_e9_nul_byte` |
| E10 | `error_path_e10_non_ascii` (10 byte sequences) |
| E11 | `error_path_e11_overflow_positive` (incl. a 1000-digit number) |
| E12 | `error_path_e12_overflow_negative` (incl. a 1000-digit negative) |
| E13, E14 | `error_path_e13_e14_long_exact_boundaries` |
| E15, E16 | `error_path_e15_e16_one_past_int_range` |
| E17 | `error_path_e17_int_truncation` |
| E18 | `error_path_e18_base_prefixes` (`0x`, `0X`, `0b`, `0o`, `010`, `0x` alone, signed variants) |
| E19 | `error_path_e19_trailing_garbage` |
| E20 | `error_path_e20_huge_leading_zeros` (5000 zeros) |
| E21 | `error_path_e21_driver_extremes` |
| E22 | `error_path_e22_print_hex_len_is_always_16` (asserts 32 lowercase hex digits + `\n` for both libraries) |
| E23 | `error_path_e23_write_failure_ignored`, `config_c28_stdout_dev_full` |
| E24 | `error_path_e24_sigpipe_parity`, `config_c29_stdout_closed_pipe_sigpipe` |
| E25 | `error_path_e25_huge_garbage_input` |

Every test runs the case against **both** `.so` files through `libloading`
(`examples/so_runner.rs`, one fresh process per case) and compares stdout bytes,
exit status **and** terminating signal — not merely "both failed somehow".
Rows E23/E24 additionally compare the two executables.

Additionally, `tests/common/mod.rs` puts a 60 s watchdog on every child process,
so a translation that hangs (rather than rejecting) fails the test instead of
stalling the suite. This is what caught the real
`read-error-not-treated-as-EOF` class of bug during harness validation
(`scripts/mutation_check.py`).
