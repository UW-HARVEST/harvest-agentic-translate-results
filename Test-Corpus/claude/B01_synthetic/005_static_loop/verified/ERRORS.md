# ERRORS.md — error-surface table (Phase C)

Mechanically derived from `c_src/src/main.c`. The file contains **no** `assert`,
**no** `NULL` check, **no** error enum and **no** `errno` inspection; every
rejection path is one of the two `printf` + `return 1` blocks, plus the
saturation limits inside the `strtol` call it makes.

Grep evidence (the whole error surface of the program):

```
40:  if (argc != 2) {
41:    printf("Error: should only be a single (integer) argument!\n");
42:    return 1;
47:  if (end == argv[1]) {
49:    printf("Error: first argument must be an integer!\n");
50:    return 1;
```

* `E1` = `Error: should only be a single (integer) argument!\n` on stdout, return `1`
* `E2` = `Error: first argument must be an integer!\n` on stdout, return `1`

Every row is asserted twice: once through the `.so` exports
(`tests/ffi_errors.rs`, `main(argc, argv)` called via `libloading` on the C and
on the Rust library) and once through the two executables
(`tests/cli_errors.rs`). Each test asserts the *exact* message and the *exact*
status — not merely "both failed".

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `main` | `argc != 2`, `argc == 1` (no operand at all) | `E1` | `cli_errors::err_argc_1_cli`, `ffi_errors` "rows 1-4" | [x] |
| 2  | `main` | `argc != 2`, `argc == 0` (empty argv; reachable via FFI and `execve`) | `E1` | `ffi_errors` "row 2" (also with a 0-length argv array) | [x] |
| 3  | `main` | `argc != 2`, `argc == 3` (one operand too many) | `E1` | `cli_errors::err_argc_3_cli`, `ffi_errors` "rows 3/4" | [x] |
| 4  | `main` | `argc != 2`, `argc == 4 … 64` | `E1` | `cli_errors::err_argc_many_cli`, `ffi_errors` "rows 3/4" | [x] |
| 5  | `main` | `argc != 2`, `argc` negative or absurd (`-1`, `-2`, `-100`, `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `1000000`, plus 200 random `i32`) — a C `int` accepts any value across the FFI boundary | `E1` | `ffi_errors` "row 5" + "randomized bogus argc" | [x] |
| 6  | `main` | `end == argv[1]`: `argv[1]` is the empty string `""` | `E2` | `ffi_errors` "row 6", `cli_errors::err_no_conversion_cli` | [x] |
| 7  | `main` | `end == argv[1]`: whitespace only (`" "`, `"\t"`, `"\n"`, `"\v"`, `"\f"`, `"\r"`, mixes, 10 spaces) — `strtol` skips the space, then finds no digit | `E2` | `ffi_errors` "row 7", `cli_errors::err_no_conversion_cli` | [x] |
| 8  | `main` | `end == argv[1]`: sign only (`"+"`, `"-"`, `"--"`, `"++"`, `"+-"`, `"-+"`, `"-+3"`, `"+-3"`, `"---5"`, `" - "`, `"\t+"`) | `E2` | `ffi_errors` "row 8", `cli_errors::err_no_conversion_cli` | [x] |
| 9  | `main` | `end == argv[1]`: first non-space byte is neither sign nor digit — 33 cases incl. `"abc"`, `"x1"`, `".5"`, `"/9"`, `":0"`, `"e5"`, `"#"`, `"_1"`, and the exact digit-range boundaries `'/'` (`'0'-1`) and `':'` (`'9'+1`) | `E2` | `ffi_errors` "row 9", `cli_errors::err_no_conversion_cli`, `cli_errors::err_digit_boundary_chars` | [x] |
| 10 | `main` | `end == argv[1]`: sign immediately followed by a non-digit (`"+a"`, `"-x"`, `"+ 1"`, `"- 1"`, `"+.1"`, `"-.1"`, `"+/"`, `"-:"`, `"+_"`, `"-#"`) | `E2` | `ffi_errors` "row 10", `cli_errors::err_no_conversion_cli`, `cli_errors::err_digit_boundary_chars` | [x] |
| 11 | `main` | `end == argv[1]`: whitespace, then sign, then non-digit (`" +z"`, `"\t-"`, `"\n\r+q"`), and whitespace *inside* the number (`" + 1"`, `" - 1"`, `"  -  7"`, `" \t+\t7"`) | `E2` | `ffi_errors` "row 11", `cli_errors::err_no_conversion_cli` | [x] |
| 12 | `main` | `end == argv[1]`: base-prefix-looking inputs without a leading digit (`"x10"`, `"#10"`, `"b1"`, `"o7"`, `"h9"`) — while `"0x10"`, `"0X10"`, `"0b101"` ARE accepted (they start with `'0'`) | `E2` (resp. accept) | `ffi_errors` "row 12" (+ controls), `cli_errors::err_no_conversion_cli` | [x] |
| 13 | `main` | `end == argv[1]`: non-ASCII / high-bit bytes (`"\xff"`, `"\x80"`, `"\x80\x81"`, `"é"` = `"\xc3\xa9"`, `"\xff1"`, `"\xa0 1"`, `"€5"`, NBSP `"\xc2\xa0"`, `"\xff\xff\xff\xff"`) — none is `isspace()`/digit in the "C" locale | `E2` | `ffi_errors` "row 13", `cli_errors::err_non_utf8_arg` | [x] |
| 14 | `main` | `end == argv[1]`: digit-group separators / decimal points are not accepted (`","`, `"_"`, `"'"`, `",5"`, `"_5"`, `"'5"`, `"."`, `"..1"`) | `E2` | `ffi_errors` "row 14", `cli_errors::err_no_conversion_cli` | [x] |
| 15 | `main` | embedded NUL: `main` gets a `char *`, so `"\0"`, `"\0123"`, `"\0 abc"` ⇒ `E2`, while `"1\0 2"`, `"-2\0garbage"`, `"1\0999"` are accepted as `1`, `-2`, `1` | `E2` / accept | `ffi_errors` "row 15" (CLI cannot carry NUL through `execve`) | [x] |
| 16 | `main` | out of `long` range: `strtol` reports `ERANGE` and saturates to `LONG_MAX`/`LONG_MIN`, which the program ignores and truncates to `int` (`"9223372036854775808"` → `-1`, `"-9223372036854775809"` → `0`, 38-digit runs) — **not** a rejection | accepted, return `0` | `cli_errors::err_range_saturation`, `ffi_errors` "row 16/17", `cli_diff::cfg_long_boundaries` | [x] |
| 17 | `main` | out of `int` range: `"2147483648"` → `-2147483648`, `"-2147483649"` → `2147483647`, `"4294967296"` → `0` (silent truncation of `long` → `int`) | accepted, return `0` | `cli_errors::err_range_saturation`, `cli_diff::cfg_int_boundaries` | [x] |
| 18 | `main` | trailing garbage after a valid prefix (`"5abc"`, `"3 4"`, `"7\n"`, `"1)"`, `"1/"`, `"1:"`) — `end != argv[1]`, so the prefix is used | accepted, return `0` | `cli_errors::err_no_conversion_cli` controls, `cli_diff::cfg_trailing_garbage` | [x] |
| 19 | `static_sum` | has **no** error path (no checks, no sentinel); `int` overflow of `sum += update` is UB in C but wraps on the target ABI — verified to wrap identically instead of trapping | wrapping `int` result | `ffi_static_sum::ffi_static_sum_overflow`, `ffi_errors` "row 19" | [x] |
| 20 | `main` (write path) | output cannot be written: stdout = closed pipe ⇒ killed by `SIGPIPE` (signal 13); stdout = `/dev/full` ⇒ `ENOSPC` ignored, exit 0; stdout = closed fd 1 ⇒ `EBADF` ignored, exit 0 | signal 13 / exit 0 / exit 0 | `cli_errors::err_epipe_kills_process`, `cli_errors::err_dev_full_ignored`, `cli_diff::cfg_stdout_closed` | [x] |
| 21 | `main` | `argc == 2` **and** `argv[1] == NULL` | UB in C (`strtol(NULL, …)` dereferences null → SIGSEGV). Both builds fault the same way; deliberately **not** asserted (a test would only prove that both crash), documented instead. | — (documented) | [x] |
| 22 | `main` | `argv == NULL` with `argc != 2` (`0`, `1`, `3`, `4`, `64`, `-1`, `INT_MIN`, `INT_MAX`) — well defined, because the C code never touches `argv` on that path | `E1` | `ffi_errors` "argv == NULL" | [x] |
| 23 | `main` | oversized input: 1 000 / 10 000 / 100 000-byte arguments (digit runs, whitespace runs, junk runs, digits + junk) — no fixed-size buffer exists, so nothing may be rejected differently | accept / `E2` as the C decides | `ffi_errors` "oversized …", `cli_diff::cfg_oversized_arguments` | [x] |

Non-triggers deliberately double-checked (they look like errors but are not):
`"0"`, `"9"`, `"+0"`, `"-0"`, `"0000"`, `"0x10"` (→ `0`), `"0b101"` (→ `0`),
`" 7"`, `"5abc"`, 200-digit runs, and `"2147483647"`.

Randomized error-gate fuzzing (fixed seed): `ffi_errors` "randomized
accept/reject" (600 inputs over an alphabet of digits, signs, all six
whitespace characters, letters and high-bit bytes) and
`cli_errors::err_randomized_reject_decision` (300 inputs). Both assert that the
C decision (accept / `E2`) and message are reproduced exactly, and both assert
that the corpus actually reached both outcomes.
