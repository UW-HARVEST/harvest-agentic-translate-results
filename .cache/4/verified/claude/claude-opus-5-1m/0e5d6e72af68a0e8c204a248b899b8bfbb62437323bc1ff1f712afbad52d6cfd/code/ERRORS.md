# ERRORS.md — error / rejection surface of `c_src/src/luggage.c`

Mechanically derived from the C source.  Every `return`-on-bad-input, every
explicit comparison against a sentinel (`EOF`, `NULL`), every `exit(...)`, every
implicit truncation limit (`%8[..]`, `%6[..]`, `%3[..]`, `%80[..]`) and every
range boundary (`%d` → `long` → `int` → `unsigned int`) gets one row.
There are no `assert`s, no error enums and no other exit codes in the C source.

Column "expected C result" = what the C program observably does.
Test column: `tests/differential_exec.rs` (subprocess, `E*` test names) or
`tests/differential_ffi.rs` (through the `.so` boundary).

| #  | function / line | trigger (exact invalid input / condition) | expected C result | test | ok |
|----|-----------------|------------------------------------------|-------------------|------|----|
| 1  | `main` :85–88 | `argc != 5` → 0 user args (`argc==1`) | stderr `Command line error: 4 arguments expected\n`, stdout empty, `exit(1)` | `e01_argc_wrong` | [x] |
| 2  | `main` :85–88 | `argc != 5` → 1 user arg | same as #1 | `e01_argc_wrong` | [x] |
| 3  | `main` :85–88 | `argc != 5` → 2 user args | same as #1 | `e01_argc_wrong` | [x] |
| 4  | `main` :85–88 | `argc != 5` → 3 user args | same as #1 | `e01_argc_wrong` | [x] |
| 5  | `main` :85–88 | `argc != 5` → 5 user args | same as #1 | `e01_argc_wrong` | [x] |
| 6  | `main` :85–88 | `argc != 5` → 6..8 user args | same as #1 | `e01_argc_wrong` | [x] |
| 7  | `main` :85–88 | `argc != 5` but stdin non-empty | stdin is never read, stderr message, `exit(1)` | `e02_argc_wrong_with_stdin` | [x] |
| 8  | `main` :102 | `scanf("%d ")` returns `EOF`: stdin empty | loop breaks, no record, `exit(0)`, stdout empty | `e03_eof_at_timestamp` | [x] |
| 9  | `main` :102 | `scanf("%d ")` returns `EOF`: only whitespace left (` \t\n\v\f\r`) | loop breaks, no record | `e03_eof_at_timestamp` | [x] |
| 10 | `main` :102 | `%d` *matching* failure (non-digit, e.g. `x`), **not** `EOF` (returns 0) → **not** treated as an error | conversion skipped, `time_stamp` keeps its previous/uninitialised stack value, parsing continues in the same iteration | `e04_matchfail_timestamp` | [x] |
| 11 | `main` :102 | `%d` sign with no digits (`-`, `+`, `-x`) → matching failure (glibc consumes the sign, pushes back the next char) | as #10 | `e05_matchfail_sign_only` | [x] |
| 12 | `main` :105 | `scanf("%8[A-Z0-9] %6[A-Z0-9] ")` returns `EOF` (EOF right at `luggage_id`) | loop breaks, record dropped | `e06_eof_at_luggage` | [x] |
| 13 | `main` :105 | `luggage_id` matching failure (first char not `[A-Z0-9]`, returns 0) | `luggage_id` untouched (stale), `flight_id` never attempted, parsing continues | `e07_matchfail_luggage` | [x] |
| 14 | `main` :105 | EOF *after* `luggage_id` matched → `scanf` returns 1, **not** `EOF` | not an error: `flight_id` stale, execution proceeds to line 109 | `e08_eof_after_luggage` | [x] |
| 15 | `main` :105 | `flight_id` matching failure (returns 1) | `flight_id` stale, trailing ws directive not run | `e09_matchfail_flight` | [x] |
| 16 | `main` :109 | `scanf("%3[A-Z] %3[A-Z]")` returns `EOF` (EOF right at `departure`) | loop breaks, record dropped | `e10_eof_at_departure` | [x] |
| 17 | `main` :109 | `departure` matching failure (digit/lowercase first, returns 0) | `departure` + `arrival` stale, continues to line 112 | `e11_matchfail_departure` | [x] |
| 18 | `main` :109 | EOF after `departure` (returns 1, not `EOF`) | `arrival` stale, continues | `e12_eof_after_departure` | [x] |
| 19 | `main` :109 | `arrival` matching failure (returns 1) | `arrival` stale, continues | `e13_matchfail_arrival` | [x] |
| 20 | `main` :112 | `scanf("%80[^\n]")` returns `EOF` (EOF right at comments) | loop breaks, **record dropped** even though all other fields parsed | `e14_eof_at_comments` | [x] |
| 21 | `main` :112 | comments matching failure: next char is `\n` (returns 0) | not an error: `comments` stays `""` (line 100 pre-clears it), record IS created | `e15_matchfail_comments` | [x] |
| 22 | `main` :105 | `luggage_id` longer than 8 `[A-Z0-9]` chars | silently truncated to 8; the rest of the run stays in the stream and is re-parsed as `flight_id`/… | `e16_width_truncation` | [x] |
| 23 | `main` :105 | `flight_id` longer than 6 chars | truncated to 6, remainder re-parsed | `e16_width_truncation` | [x] |
| 24 | `main` :109 | `departure` longer than 3 `[A-Z]` | truncated to 3, remainder re-parsed as `arrival` | `e16_width_truncation` | [x] |
| 25 | `main` :109 | `arrival` longer than 3 `[A-Z]` | truncated to 3, remainder becomes the comment | `e16_width_truncation` | [x] |
| 26 | `main` :112 | comment longer than 80 chars | truncated to 80; the tail (non-whitespace) is left in the stream and mis-parsed by the next iteration | `e17_comment_overflow` | [x] |
| 27 | `main` :102 | `%d` value `> INT_MAX` (e.g. `2147483648`) | `strtol` result truncated to `int`, stored into `unsigned int` → prints `2147483648` | `e18_numeric_range` | [x] |
| 28 | `main` :102 | `%d` value `> UINT_MAX` (e.g. `4294967296`) | truncated modulo 2^32 → prints `0000000000` | `e18_numeric_range` | [x] |
| 29 | `main` :102 | `%d` value `> LONG_MAX` (e.g. `99999999999999999999`) | `strtol` saturates to `LONG_MAX`, truncated to `int` = -1 → prints `4294967295` | `e18_numeric_range` | [x] |
| 30 | `main` :102 | `%d` value `< LONG_MIN` (e.g. `-99999999999999999999`) | saturates to `LONG_MIN`, truncated → prints `0000000000` | `e18_numeric_range` | [x] |
| 31 | `main` :102 | negative `%d` into `unsigned int` (e.g. `-1`, `-42`) | two's-complement reinterpretation (`4294967295`, `4294967254`) and it sorts as a huge unsigned value | `e18_numeric_range` | [x] |
| 32 | `supersedes` :35–37 | `directive == NULL` (end of list reached / empty tail) | returns `0` | `f01_supersedes_null` (FFI) + `e19_supersedes_tail` | [x] |
| 33 | `supersedes` :43–46 | luggage ids equal but departures differ | returns `0` **immediately** — the search does *not* continue past the first luggage-id match | `e20_supersede_stops_at_first_match` | [x] |
| 34 | `printMatchingDirectives` :66 | `first_directive == NULL` (no records survived parsing) | prints nothing, no error | `f02_print_null_list` (FFI) + `e03_eof_at_timestamp` | [x] |
| 35 | `matches` :57 | `expected` is the empty string (`argv[i] == ""`) | `expected[0]` is `'\0'`, so the wildcard test fails and `strcmp("", actual)` is used → matches only empty fields | `e21_empty_filter` | [x] |
| 36 | `matches` :57 | `expected` starts with `-` but has more chars (`-X`, `--`) | still the wildcard (only `expected[0]` is inspected) → matches everything | `e22_dash_prefix_filter` | [x] |
| 37 | `main` :112/`printf` :73 | comment containing a NUL byte | `strcpy`/`%s` stop at the NUL → the tail of the comment is lost | `e23_nul_in_comment` | [x] |
| 38 | `main` :116 | `calloc` returns `NULL` (out of memory) | **unchecked** in C → NULL dereference / crash.  Not a defined rejection; untestable without OOM injection.  Documented, not asserted. | n/a | [x] |
| 39 | `addRoutingDirectiveToList` :24 | `previous_directive == NULL` | **unchecked** → segfault.  Verified through the `.so`: `addRoutingDirectiveToList(NULL,NULL)`, `superseded(NULL)` :50 and `matches(NULL,…)` :57 all kill the process with `SIGSEGV` (status 139).  This is C undefined behaviour and is unreachable from the program itself (`main` always passes `&directive_list_head`, and `argv[i]` is never NULL).  The Rust exports return safely instead of reproducing the crash — the single deliberate, documented deviation, and only for UB inputs.  The FFI tests therefore assert equality only for the *defined* NULL cases (rows 32 and 34). | documented (see `RESULTS.md`) | [x] |
| 40 | `main` :93–127 | input that is entirely unparsable garbage (random bytes, lowercase) | never an error/exit code: garbage records with stale fields are created and printed; `exit(0)` | `e24_garbage_stream` | [x] |
| 41 | `printf` :73 | the reader of `stdout` closes the pipe before the output is consumed | `printf`'s `write()` gets `EPIPE`, `SIGPIPE` has its default disposition → the process is **killed by SIGPIPE** (shell status 141).  Rust's runtime sets `SIGPIPE` to `SIG_IGN`, so `src/main.rs` restores the default before running. | `p32_stdout_failure_modes` | [x] |
| 42 | `printf` :73 | `stdout` is closed (`>&-`) or the device is full (`/dev/full`) | the `printf` error is **not checked**: no message, `exit(0)` | `p32_stdout_failure_modes` | [x] |
| 43 | `scanf` :102 | `stdin` cannot be read at all (e.g. it is a directory → `EISDIR`) | `scanf` reports EOF, the loop breaks, no records, `exit(0)` | `p32`/manual check in `RESULTS.md` | [x] |
| 44 | — | out-of-range enum value crossing the FFI boundary | **N/A**: the C source declares no enums (`grep -c enum c_src/src/luggage.c` → 0).  The only non-pointer parameter types are `int`/`unsigned int`, whose full range is covered by rows 27–31 and by `f13`/`f14` (timestamps `0 … u32::MAX`). | n/a | [x] |
