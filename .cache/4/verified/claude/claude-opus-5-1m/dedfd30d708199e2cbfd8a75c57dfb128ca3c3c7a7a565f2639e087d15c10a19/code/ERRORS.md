# ERRORS.md — error-surface table (Phase C)

Mechanically derived from `c_src/src/main.c`. Every rejection site in the file:

```
$ grep -n -E 'return|assert|NULL|printf|if *\(|for|LONG|INT_|malloc|exit' c_src/src/main.c
30:  if(*outer >= inner) {          <- branch, not a rejection
32:    return &inner;
35:    return outer;
45:  if (argc != 3) {               <- rejection #1
47:    return 1;
52:  if (end == argv[1]) {          <- rejection #2
55:    return 1;
59:  if (end == argv[2]) {          <- rejection #3
62:    return 1;
66:  for (int i = 0; i < iterations; i++) {   <- range check #4 (rejects iterations <= 0)
71:  return 0;
```

The file contains **no** `assert`, no `NULL` checks, no allocation, no `exit()`,
no error enums and no explicit min/max constants; `static_alias()` has no
failure mode at all (it always returns a non-NULL pointer). The only implicit
range limits are the ones inside the libc functions it calls (`strtol`
saturation) and the implicit `long`→`int` narrowing of its result. Rows 5–8
below capture those, rows 9–16 the generic C-API boundaries and rows 17–20 the
NULL-pointer boundary.

Exit code convention: `main` returns `1` for every rejection, `0` otherwise.
All messages are printed on **stdout** (`printf`, not `stderr`); stderr is never
written by either implementation.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `main` | `argc != 3` — tested with `argc` = 0, 1, 2, 4, 5, 17 and negative `argc` (-1, `INT_MIN`) | prints `Error: should only be two (integer) arguments!\n` on stdout, returns `1`; nothing else is read from `argv` | `#1 argc != 3` + `#1 negative argc` (in `ffi_errors::error_surface_differential`), `cli_err_argc_sweep` | [x] |
| 2 | `main` | `strtol(argv[1], &end, 10)` performs no conversion, so `end == argv[1]`: `argv[1]` is `""`, whitespace only (`" "`, `"\t\n\v\f\r"`), sign only (`"+"`, `"-"`), `"+ 1"`, `"--1"`, `"++1"`, `"abc"`, `".5"`, `"x10"`, `"e5"`, `","`, `"O"`/`"o"`, `"٣"` (non-ASCII digit), `"\x80\xff"` (non-UTF-8 bytes), `" - 3"` | prints `Error: first argument must be an integer!\n`, returns `1`; `argv[2]` is never parsed | `#2 argv[1] unparsable`, `cli_err_unparsable` | [x] |
| 3 | `main` | `strtol(argv[2], &end, 10)` performs no conversion, so `end == argv[2]` (same input shapes as row 2), while `argv[1]` *is* parsable | prints `Error: second argument must be an integer!\n`, returns `1`; the loop never runs, `inner` is not modified | `#3 argv[2] unparsable`, `cli_err_unparsable` | [x] |
| 4 | `main` | `iterations <= 0` (`argv[2]` = `"0"`, `"-1"`, `"-2147483648"`, `"-99999999999999999999"`) — the `for` guard `i < iterations` rejects the body | no output at all, returns `0`, `inner` unchanged | `#4 iterations <= 0`, `cli_err_iterations_non_positive` | [x] |
| 5 | `main` | `strtol` overflow (ERANGE) on `argv[1]`: magnitude above `LONG_MAX` (`"9223372036854775808"`, `"99999999999999999999"`, 400-digit number, `"+1"`+300 zeros) | `strtol` saturates to `LONG_MAX`; the implicit narrowing to `int` yields `-1`; **accepted**, `initial_value == -1` | `#5/#6 argv[1] strtol saturation` | [x] |
| 6 | `main` | `strtol` underflow (ERANGE) on `argv[1]`: below `LONG_MIN` (`"-9223372036854775809"`, `"-99999999999999999999"`) | saturates to `LONG_MIN`; narrowing yields `0`; **accepted**, `initial_value == 0` | `#5/#6 argv[1] strtol saturation` | [x] |
| 7 | `main` | same saturation on `argv[2]` (`iterations`) — `LONG_MAX`→`-1` (loop does not run), `LONG_MIN`→`0` (loop does not run) | no output, returns `0` | `#7 argv[2] strtol saturation` | [x] |
| 8 | `main` | value out of `int` range but inside `long` range, i.e. one step past the `int` boundaries: `"2147483648"`, `"-2147483649"`, `"4294967296"`, `"4294967295"` | accepted; the implicit `long`→`int` conversion truncates (`2147483648`→`INT_MIN`, `-2147483649`→`INT_MAX`, `4294967296`→`0`, `4294967295`→`-1`) | `#8 long -> int narrowing`, `cli_shape_matrix` | [x] |
| 9 | `main` | trailing garbage after a valid prefix (`"12abc"`, `"5 5"`, `"0x10"`, `"1.9"`, `"-3junk"`, `"08"`, `"1e5"`) — `end != argv[i]`, so **accepted** and the prefix is used | accepted, prefix value used, returns `0` | `#9 trailing garbage accepted`, `cli_shape_matrix` | [x] |
| 10 | `main` | `argv[1]`/`argv[2]` = empty string combined with the *other* argument being valid, and both empty | row 2 / row 3 message, `1` | `#10 empty string combinations`, `cli_err_argc_sweep` | [x] |
| 11 | `main` | oversized argument strings: 4096-byte digit string, 4096 leading blanks + digits, 4096 leading zeros then a digit | accepted, saturation/narrowing as above | `#11 oversized strings`, `cli_oversized_arguments` | [x] |
| 12 | `main` | zero-length `argv` array with `argc == 0` (`argv[0] == NULL`) | row 1 (`argc != 3`), returns `1` before touching `argv` | `#12 argc == 0, argv[0] == NULL`, `cli_empty_argv` | [x] |
| 13 | `main` | `argc == 3` with extra (ignored) `argv` entries beyond index 2 | accepted, extras ignored | `#13 extra argv entries ignored`, `cfg20_extra_argv` | [x] |
| 14 | `static_alias` | there is no rejection path; the closest boundaries are `*outer == INT_MIN`, `*outer == INT_MAX`, and signed overflow of `inner + *outer` / `*outer + inner` | never returns NULL; wraps (two's complement) at `-O0`, which is what both implementations do | `alias_boundary_and_enumlike_values`, `alias_overflow_both_branches`, `alias_state_value_matrix` | [x] |
| 15 | `static_alias` | `outer == &inner` (the pointer the function itself returned) — the aliasing "configuration" that makes `*outer >= inner` trivially true | `inner` doubles, `&inner` returned again | `alias_self_aliasing_chain` | [x] |
| 16 | both | out-of-range *enum* values across the FFI boundary | **not applicable**: the C source declares no enum and no flag parameter; the only parameters are `int`, `int*` and `char**`, and every `int` bit pattern is exercised by rows 5–8/14 (`INT_MIN`, `-1`, `0`, `1`, `INT_MAX`, random 32-bit values) | `alias_boundary_and_enumlike_values`, `#1 negative argc` | [x] |

## NULL-pointer boundary (undefined behaviour in C, still compared)

The C code dereferences its pointer parameters unconditionally, so NULL is
undefined behaviour — there is no defined result, but the *observable* outcome
(termination by a fatal signal) is compared anyway. Each case runs in its own
process (`tests/ffi_null_ub.rs` re-executes the test binary) so the signal can be
observed.

| # | function | trigger | expected C result | test | ✔ |
|---|----------|---------|-------------------|------|---|
| 17 | `static_alias` | `outer == NULL` | SIGSEGV (signal 11), no return value | `null_pointer_boundary_parity` / `static_alias_null` | [x] |
| 18 | `main` | `argv == NULL` with `argc == 3` | SIGSEGV | `null_pointer_boundary_parity` / `main_null_argv` | [x] |
| 19 | `main` | `argv[1] == NULL` with `argc == 3` (`strtol` dereferences it) | SIGSEGV | `null_pointer_boundary_parity` / `main_null_arg1` | [x] |
| 20 | `main` | `argv[2] == NULL` with `argc == 3` and a valid `argv[1]` | SIGSEGV | `null_pointer_boundary_parity` / `main_null_arg2` | [x] |

Rows 17-20 hold exactly (same signal, same absent exit code) in the **release**
profile. In the **dev** profile rustc's UB checks (`debug-assertions = on`) turn
row 17's NULL dereference into a panic that aborts the `extern "C"` frame
(SIGABRT, signal 6) instead of performing the faulting load; rows 18-20 still
give SIGSEGV because the fault happens inside `strlen`/`strtol`. The test
therefore asserts exact signal parity in release and "terminates abnormally, does
not return" in dev. This is a property of Rust's UB instrumentation for an input
the C language leaves undefined, not a behavioural difference for any defined
input.

## Excluded from differential testing

* `i++` overflow when `iterations == INT_MAX` — reaching it requires 2^31
  iterations of `printf`; not testable in bounded time. Both implementations use
  the wrapping increment gcc emits at `-O0` (`i = i.wrapping_add(1)`).
* Concurrent calls from several threads: `inner` is a plain `static int` with no
  synchronisation, i.e. a data race in the C code; single-threaded use is the
  contract both implementations honour.
