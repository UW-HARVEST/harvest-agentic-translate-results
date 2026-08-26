# ERRORS.md — error-surface table (Phase A / Phase C)

Mechanically derived from `c_src/src/main.c`. Every `return 1;` in the C source,
every comparison that guards one, and every generic FFI boundary condition
(NULL pointers, zero/oversized lengths, one-past-the-range values,
out-of-range "enum"-style integers) gets one row.

Grep evidence: the file contains **6 `return 1;` statements reached from 7
distinct conditions** (`argc > 4`, `argc == 1`, `end == argv[2]`,
`start > len`, `end == argv[3]`, `stop > len`, `stop <= start`) and **no**
`assert`, `RETURN_ERROR`, `errno` check, `return NULL` or error enum.

```
$ grep -n 'return 1;\|if (\|else' c_src/src/main.c
36:    if ((argc > 4) || (argc == 1)) {
39:        return 1;
47:    if (argc >= 3) {
49:        if (end == argv[2]) {
51:            return 1;
53:        if (start > len) {
55:            return 1;
57:    } else {
61:    if (argc == 4) {
63:        if (end == argv[3]) {
65:            return 1;
68:        if (stop > len) {
70:            return 1;
73:        if (stop <= start) {
75:            return 1;
78:    } else stop = len;
```

All messages go to **stdout** (`printf`), never stderr; note rows E3 and E5
print **without a trailing newline**.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| E1 | `main` | `argc > 4` — 4 or more user arguments (`driver s 1 2 3`) | stdout `"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n"`, returns `1` | `err_e1_too_many_args` |
| E2 | `main` | `argc == 1` — no user argument at all | same two-line usage message, returns `1` | `err_e2_no_args` |
| E3 | `main` | `argc >= 3` and `strtol(argv[2],&end,10)` performs no conversion, i.e. `end == argv[2]`: `""`, `"abc"`, `"-"`, `"+"`, `" "`, `"x9"`, `"."` , `"--1"` | stdout `"Second argument must be an integer!"` (**no newline**), returns `1` | `err_e3_start_not_an_integer` |
| E4 | `main` | `argc >= 3` and `start > len` where `start` is `int` and `len` is `size_t` → *unsigned* comparison, so this fires for `start > strlen(argv[1])` **and for every negative `start`** (`-1`, `"-9"`, and any value whose `long`→`int` truncation is negative, e.g. `"99999999999999999999"` → `LONG_MAX` → `(int)-1`) | stdout `"Error: start is off the end of the string!\n"`, returns `1` | `err_e4_start_off_end`, `err_e4_negative_start`, `err_e4_truncated_start` |
| E5 | `main` | `argc == 4` and `end == argv[3]`, where `end` still points **into `argv[2]`** because the third `strtol` was called with a NULL `endptr`. Reachable only when the caller's `argv` places `argv[3]` exactly at `argv[2] + (bytes consumed by strtol)` — e.g. `argv[2] = "12"`, `argv[3] = argv[2] + 2` (the NUL byte, an empty string). With a kernel-supplied contiguous `argv` this can never happen (`end <= argv[2]+strlen(argv[2]) = argv[3]-1`), which is why the message is unreachable from the command line, but it *is* reachable through the `main` FFI export. | stdout `"Third argument must be an integer!"` (**no newline**), returns `1` | `err_e5_third_arg_alias`, `err_e5_unreachable_from_cli` |
| E6 | `main` | `argc == 4` and `stop > len` — same signed/unsigned trap as E4, so also every negative `stop` | stdout `"Error: stop is off the end of the string!\n"`, returns `1` | `err_e6_stop_off_end`, `err_e6_negative_stop` |
| E7 | `main` | `argc == 4` and `stop <= start` (plain signed `int` comparison), e.g. `s 3 3`, `s 3 2`, `s 0 0` | stdout `"Error: stop must come after start!\n"`, returns `1` | `err_e7_stop_before_start` |

## Generic FFI-boundary conditions (no dedicated C check exists)

| # | condition | C behavior | test |
|---|-----------|------------|------|
| B1 | `argc == 0` (no `argv[0]`): neither `argc > 4` nor `argc == 1` holds, so the C **reads `argv[1]` anyway**, one element past the NULL terminator of a kernel-supplied vector. | Whatever `argv[1]` holds is used as the string; with `argc == 0` supplied through the FFI export this is a perfectly ordinary read of the caller's array. Returns `0` and prints that string. (Unreachable from `execve`: Linux ≥ 5.18 rewrites an empty `argv` to `argc == 1, argv[0] == ""`, which lands in E2 — asserted by `boundary_b1_argc0_via_exec_becomes_argc1`.) | `boundary_b1_argc0_ffi` |
| B2 | `argc` negative or huge (e.g. `-1`, `INT_MIN`, `5`, `INT_MAX`) — a C `int` accepts any value, exactly like an out-of-range enum | `argc > 4` catches every value `> 4` (incl. `INT_MAX`); `argc == 1` catches 1; `argc <= 0` and `argc == 2..4` fall through to the body, where `argv[1]`/`argv[2]`/`argv[3]` are read according to `argc >= 3` / `argc == 4`. Negative `argc` behaves like `argc == 0` (body runs, only `argv[1]` read). | `boundary_b2_out_of_range_argc` |
| B3 | `argv[1] == NULL` with `2 <= argc <= 4` | `strlen(NULL)` dereferences NULL → `SIGSEGV`. Both implementations must fault the same way. | `boundary_b3_b4_b5_null_pointers` (forks a child per library and compares the death signal) |
| B4 | `argv[2] == NULL` with `argc >= 3` | `strtol(NULL, ...)` dereferences NULL → `SIGSEGV` | `boundary_b3_b4_b5_null_pointers` |
| B5 | `argv[3] == NULL` with `argc == 4` | `strtol(NULL, ...)` dereferences NULL → `SIGSEGV` | `boundary_b3_b4_b5_null_pointers` |
| B6 | zero length: `argv[1] == ""` (`len == 0`) with `argc == 2`, `3`, `4` | `argc==2` → prints `"\n"`, returns 0. `argc==3` → only `start == 0` passes E4, prints `"\n"`. `argc==4` → every `stop` fails E6 (`stop>0`) or E7 (`stop<=0`), so `argc==4` with an empty string **always** returns 1. | `boundary_b6_empty_string`, and rows C1/C6/C12 of CONFIGS.md |
| B7 | one step past the valid range: `start == len` (ok, prints `"\n"`), `start == len+1` (E4), `stop == len` (ok), `stop == len+1` (E6), `stop == start` (E7), `stop == start+1` (ok, one byte) | as noted | `boundary_b7_one_past_range` |
| B8 | oversized / overflowing numbers: `"2147483647"`, `"2147483648"`, `"4294967296"` (→ `(int)0`), `"9223372036854775807"`, `"9223372036854775808"` (→ `LONG_MAX` → `(int)-1`), `"-9223372036854775808"`, `"-9223372036854775809"`, `"99999999999999999999999"` | `strtol` saturates to `LONG_MAX`/`LONG_MIN`, the assignment to `int` truncates modulo 2³², then E4/E6/E7 judge the truncated value | `boundary_b8_overflow_values`, `cfg_c16_argc4_truncation_values`, `cfg_c26_long_digit_strings` |
| B9 | `stop - start` overflow / negative precision in `printf("%.*s", ...)` | Unreachable for any input that survives E4/E6/E7 as long as `strlen(argv[1]) <= INT_MAX`: E4 forces `0 <= start <= len` and E7 forces `stop > start`. For `strlen(argv[1]) > INT_MAX` the C narrows `stop = (int)len` to a negative number and `%.*s` then treats the precision as omitted; both implementations perform the identical narrowing and the identical "negative precision ⇒ print the whole string" rule (`imp::put_precision_str_nl`). Not executed as a test: it needs a >2 GiB argument *and* would emit >2 GiB of output. | analysed, code-matched |
| B10 | non-UTF-8 / high-bit bytes and embedded whitespace in `argv[1]` | bytes are copied verbatim (`%.*s`), no encoding involved | randomized rows of CONFIGS.md, `boundary_b10_non_utf8` |
| B11 | `argv` itself is NULL | `argc == 1` / `argc > 4` return `1` before `argv` is touched; every other `argc` (`-1`, `0`, `2`, `3`, `4`) dereferences `argv[1]` → `SIGSEGV` | `boundary_b11_null_argv` |
| B12 | `argv[1]` is a NUL-terminated string whose terminator is the last readable byte before an unmapped page | prints normally; neither implementation may read past the terminator | `boundary_b12_string_at_page_boundary` |
| B13 | `argv[1]` is *not* NUL-terminated before an unmapped page | `strlen` runs into the guard page → `SIGSEGV` | `boundary_b13_unterminated_string_faults_identically` |
| B14 | stdout is a pipe whose reader closed (broken pipe) | the C keeps the default `SIGPIPE` disposition and is killed by signal 13; the Rust runtime installs `SIG_IGN`, so the translation restores `SIG_DFL` in `main` (**divergence found and fixed**) | `cfg_c28_cli_broken_pipe` |
| B15 | stdout closed before `main` runs (write always fails) | `printf`'s return value is ignored, so the exit status is unchanged (`0` / `1`) | `cfg_c29_cli_closed_stdout` |
