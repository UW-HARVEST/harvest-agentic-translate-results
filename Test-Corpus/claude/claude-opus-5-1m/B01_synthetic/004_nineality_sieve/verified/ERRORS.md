# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/main.c`. That file contains **exactly two
rejection sites** (there are no `assert`s, no `NULL` checks, no error enums, no
explicit range checks, and no min/max constants in the source):

```c
if (argc != 2) {                                                   /* site 1 */
    printf("Error: should only be a single (integer) argument!\n");
    return 1;
}
char *end;
int val = strtol(argv[1], &end, 10);
if (end == argv[1]) {                                              /* site 2 */
    printf("Error: first argument must be an integer!\n");
    return 1;
}
```

Both messages go to **stdout** (gcc lowers them to `puts`), and both paths
`return 1`. Shorthand used below:

* `E_ARGC` = stdout `"Error: should only be a single (integer) argument!\n"`, exit **1**
* `E_INT`  = stdout `"Error: first argument must be an integer!\n"`, exit **1**

## Rejections (one row per distinct trigger)

Site 1 — `argc != 2`:

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `main` / site 1 | `argc == 0` (raw `execve` with `argv = {NULL}`) | `E_ARGC` | `err_01_argc_zero_raw_execve` | [x] |
| 2 | `main` / site 1 | `argc == 1` — program name only, no operand | `E_ARGC` | `err_02_argc_one_no_operand` | [x] |
| 3 | `main` / site 1 | `argc == 3` — two operands (`"1" "2"`) | `E_ARGC` | `err_03_argc_three` | [x] |
| 4 | `main` / site 1 | `argc == 3` where operand 1 alone would be valid (`"5" ""`) — check order: argc test runs *before* any parse | `E_ARGC` (never `E_INT`) | `err_04_argc_precedes_parse` | [x] |
| 5 | `main` / site 1 | `argc` large (12 operands) | `E_ARGC` | `err_05_argc_many` | [x] |
| 6 | `main` / site 1 | `argc == 3` with both operands invalid (`"abc" "def"`) | `E_ARGC` (argc wins) | `err_06_argc_wins_over_bad_parse` | [x] |

Site 2 — `end == argv[1]` (i.e. `strtol(…, 10)` performed **no** conversion).
Each row is a distinct way glibc's `strtol` refuses to consume any character:

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 7 | `main` / site 2 | `argv[1] == ""` (zero-length operand) | `E_INT` | `err_07_empty_operand` | [x] |
| 8 | `main` / site 2 | whitespace only — each of `" "`, `"\t"`, `"\n"`, `"\v"`, `"\f"`, `"\r"`, and all six concatenated | `E_INT` | `err_08_whitespace_only` | [x] |
| 9 | `main` / site 2 | sign only — `"-"`, `"+"` | `E_INT` | `err_09_sign_only` | [x] |
| 10 | `main` / site 2 | whitespace then sign only — `"  -"`, `"\t+"` | `E_INT` | `err_10_ws_then_sign_only` | [x] |
| 11 | `main` / site 2 | sign then whitespace then digits — `"- 5"`, `"+\t7"` (space breaks the numeral) | `E_INT` | `err_11_sign_space_digits` | [x] |
| 12 | `main` / site 2 | doubled/mixed signs — `"--5"`, `"++5"`, `"+-5"`, `"-+5"` | `E_INT` | `err_12_double_sign` | [x] |
| 13 | `main` / site 2 | leading alphabetic — `"abc"`, `"x9"`, `"e5"`, `"inf"`, `"nan"`, `"NULL"` | `E_INT` | `err_13_leading_alpha` | [x] |
| 14 | `main` / site 2 | leading punctuation, incl. the ASCII neighbours of `'0'..'9'` — `"."`, `","`, `"/"` (0x2F), `":"` (0x3A), `"_5"`, `"'5"`, `"#9"` | `E_INT` | `err_14_leading_punct` | [x] |
| 15 | `main` / site 2 | leading non-ASCII / non-UTF-8 byte — `"\xff9"`, `"\x80"`, `"\xc3\x28 5"` (raw `OsStr` bytes) | `E_INT` | `err_15_leading_high_byte` | [x] |
| 16 | `main` / site 2 | Unicode look-alikes — U+2212 minus `"−5"`, U+FF15 fullwidth `"５"`, U+00A0 NBSP + `"5"` (not `isspace` in C locale) | `E_INT` | `err_16_unicode_lookalikes` | [x] |
| 17 | `main` / site 2 | every single byte 0x01..0xFF that is neither an ASCII digit, nor `+`/`-`, nor C-locale whitespace, used as a 1-byte operand (exhaustive byte sweep) | `E_INT` for all of them | `err_17_exhaustive_single_byte_sweep` | [x] |
| 18 | `main` / site 2 | randomized garbage strings whose first non-space, non-sign byte is a non-digit (fixed-seed property test, 512 cases) | `E_INT` | `err_18_random_nonnumeric` | [x] |
| 19 | `main` / site 2 | oversized rejected operand — 100 000 `'x'` bytes | `E_INT` | `err_19_oversized_nonnumeric` | [x] |

## Inputs that look invalid but are **accepted** (the mirror of the table)

The C code performs **no** other validation, so all of the following are *not*
rejected. A translation that "helpfully" errors on them would be wrong, so each
gets a differential test too.

| # | trigger | expected C result | test | [x] |
|---|---------|-------------------|------|-----|
| 20 | trailing garbage after digits — `"12abc"`, `"0x1f"`, `"0x"`, `"1e3"`, `"1,000"`, `"9 9"` | accepted, parses the leading numeral only (`12`→`12..19`, `0x…`→`0..9`, `1e3`→`1..9`) | `err_20_trailing_garbage_accepted` | [x] |
| 21 | `strtol` overflow (`> LONG_MAX`): `"9223372036854775808"`, `"99999999999999999999"`, 400-digit number, `LONG_MAX` itself | `ERANGE` is **ignored**; clamped `LONG_MAX` truncated to `int` → `-1`, prints `-1,0,…,9`, exit 0 | `err_21_erange_positive_clamp` | [x] |
| 22 | `strtol` underflow (`< LONG_MIN`): `"-9223372036854775809"`, `"-99999999999999999999"`, `LONG_MIN` itself | clamped `LONG_MIN` truncated to `int` → `0`, prints `0,…,9`, exit 0 | `err_22_erange_negative_clamp` | [x] |
| 23 | value outside `int` range but inside `long` — `"2147483648"` (= `INT_MIN` after truncation), `"4294967296"` (= 0), `"4294967301"` (= 5) | accepted; `int val = strtol(...)` truncates modulo 2^32 | `err_23_int_truncation` (+ `cfg_*` prefix tests for the huge-output cases) | [x] |
| 24 | out-of-range "enum"-style integers passed across the boundary: one step past each documented bound — `INT_MAX`, `INT_MAX+1`, `INT_MIN`, `INT_MIN-1`, `LONG_MAX±1`, `LONG_MIN±1`, `2^32±1`, `2^31±1`, `2^63±1` | accepted, wrapped/clamped exactly as C does | `err_24_one_past_every_bound` | [x] |
| 25 | negative operand ending in 9 — `"-19"`, `"-9"`, `"-2147483649"`→trunc | C's truncating `%` gives `-9`, so `val % 10 == 9` is **false**; the loop keeps counting up to `+9` | `err_25_negative_mod_never_nine` | [x] |
| 26 | signed-overflow UB at the top of the loop — `"2147483647"` (`INT_MAX`, `% 10 == 7`) | `val++` wraps to `INT_MIN` on this platform and counting continues | `err_26_int_max_overflow_wrap` (prefix compare) | [x] |
| 27 | write failures: stdout closed (`close(1)` before `exec`), stdout is `/dev/null`, stdout is a file, reader closes the pipe early (SIGPIPE) | `printf` failure is ignored → exit 0 unless the process is killed; early-close ⇒ death by `SIGPIPE` (signal 13) | `err_27_closed_stdout`, `err_28_sigpipe_on_early_close` | [x] |

## Generic FFI/CLI boundary cases (required by the gate, none in the C table)

| # | trigger | expected C result | test | [x] |
|---|---------|-------------------|------|-----|
| 28 | "null pointer" analogue: `argv[1]` cannot be `NULL` when `argc == 2`; the reachable analogues are the zero-length operand (row 7) and `argc == 0` (row 1) | as rows 1 & 7 | `err_01`, `err_07` | [x] |
| 29 | zero length / oversized length: `""` and a 100 000-digit numeral | `E_INT` / clamped `LONG_MAX` → `-1` | `err_19`, `err_29_oversized_numeral` | [x] |
| 30 | argument bytes that are not valid UTF-8 but *do* parse (`"5\xff"`, `"\t-7\x80"`) | accepted, numeral parsed, trailing bytes ignored | `err_30_non_utf8_but_parses` | [x] |
| 31 | environment/locale variation on a rejected input (`LC_ALL=C`, `LC_ALL=en_US.UTF-8`, `LC_ALL=tr_TR.UTF-8`, `LC_NUMERIC=de_DE.UTF-8`, unset) | identical `E_INT` regardless of locale | `err_31_locale_invariance` | [x] |
| 32 | `RLIMIT_FSIZE` exceeded while writing (limits 1/3/4/8/64 bytes, on both the numeric and the error path) | kernel raises `SIGXFSZ` (signal 25); the truncated bytes written are exactly `limit` long and identical | `cfg_35_fsize_limit_sigxfsz` | [x] |
| 33 | byte-exactness of both messages themselves (trailing newline present, nothing on stderr, verified against literals for C *and* Rust) | `E_ARGC` / `E_INT` exactly | `err_32_message_bytes_are_exact` | [x] |
| 34 | stdout is a terminal (glibc line-buffers, the Rust port block-buffers) on both a rejected and an accepted operand | identical bytes at EOF (incl. the pty's `\n`->`\r\n`), identical exit code | `cfg_34_stdout_is_a_tty` | [x] |

## Adequacy of these tests

Two of the mutations in `mutation_check.py` target this table specifically —
M5 (write the `E_INT` message to stderr instead of stdout) and M8 (change one
word of the `E_ARGC` message) — and both are caught by the Phase C tests, so the
error-path assertions really do compare bytes and streams rather than just
"failed somehow". M6 (rejecting the trailing garbage that C accepts) and M10
(rejecting `'+'`) confirm the accepted-but-odd rows have teeth too.
