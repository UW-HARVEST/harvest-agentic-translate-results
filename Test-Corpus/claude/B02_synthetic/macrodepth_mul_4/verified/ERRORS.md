# ERRORS.md — Phase A: error-surface table

Derived mechanically from the C source. The grep used:

```
grep -nE 'return -1|return NULL|assert|errno|exit\(|abort|if *\(|switch|default:|<|>|==|!=' \
     c_src/src/*.c c_src/src/*.h
```

which yields exactly three places where the C code can reject / ignore input:

* `c_src/src/mdmain.c:29` — `if (argc < 3) { fprintf(stderr, ...); return 2; }`
* `c_src/src/mdmacros.h:91` — `default: break;` inside `DISPATCH_REP`
* `c_src/src/mdmacros.h:77` — `for (int i = 0; i < (n); ++i)` inside `FOR_EACH`
  (the loop guard silently does nothing for `n <= 0`)

plus two *build-time* rejections that come from token pasting:

* `CHOOSE_REP(n)` → `CAT(REP, n)` → `REP<n>` is undeclared for `n` outside `0..7`
* `OP_FN(op)`/`STEP_OP(op,..)`/`INIT_FOR(op)` → `op_<op>` / `STEP_<op>` /
  `INIT_<op>` are undeclared for any `op` outside `{add, sub, mul}`

There are **no** `assert`s, **no** `return -1` / `return NULL`, **no** error
enums, **no** `errno` use, and **no pointer parameters anywhere** in the public
API (`int(int,int)` and `int(int)` only), so there are no null-pointer checks to
mirror. There is likewise no C `enum` in the API; the closest analogue — an
integer used as a `switch` selector with no matching `case` — is
`use_generated`'s `n`, covered by rows 4–10 below.

`INIT_add`/`INIT_sub`/`INIT_mul` (`0`, `0`, `1`) are the only "constants" in the
header; they are the values the silent-rejection paths return.

## Error / rejection table

`INIT` below means `INIT_FOR(OP)` = `0` for `add` and `sub`, `1` for `mul`.
Every row is tested in **all 24** `(OP, REPEAT)` configurations.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `main` (`mdmain.c:29`) | `argc == 1` (no arguments) | stderr `usage: <argv[0]> A B\n`, nothing on stdout, exit status **2** | `c01_argc_one` | [x] |
| 2 | `main` (`mdmain.c:29`) | `argc == 2` (one argument only), incl. an empty-string argument | stderr `usage: <argv[0]> A B\n`, nothing on stdout, exit status **2** | `c02_argc_two` | [x] |
| 3 | `main` (`mdmain.c:29`) | `argc == 0` (`execve` with an empty `argv`) — `argv[0]` is not a valid string | stderr `usage:  A B\n`, exit status **2** (glibc prints nothing for the `%s`) | `c03_argc_zero_via_execve` | [x] |
| 4 | `use_generated` → `DISPATCH_REP` `default:` (`mdmacros.h:91`) | `n == 7` (one past the last `case`) | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c04_use_generated_n_7` | [x] |
| 5 | `use_generated` → `default:` | `n == 8`, `9`, `100`, `255`, `256`, `1000` (well past the last `case`) | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c05_use_generated_n_above_switch` | [x] |
| 6 | `use_generated` → `default:` | `n == INT_MAX` (`2147483647`) | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c06_use_generated_n_int_max` | [x] |
| 7 | `use_generated` → `default:` | `n == -1` (one step below the first `case`) | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c07_use_generated_n_negative` | [x] |
| 8 | `use_generated` → `default:` | `n == INT_MIN` (`-2147483648`) | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c08_use_generated_n_int_min` | [x] |
| 9 | `use_generated` → `default:` | 4000 randomized `n` values drawn from the whole `int` range outside `0..=6` | stdout `gen.acc=<INIT>\n`, returns `INIT` | `c09_use_generated_random_out_of_range` | [x] |
| 10 | `use_generated` → `REP0` / `FOR_EACH` empty-loop guard (`mdmacros.h:77`) | `n == 0` — the body of the unrolled loop never executes | stdout `gen.acc=<INIT>\n`, returns `INIT` (indistinguishable from the `default:` result — asserted to be equal) | `c10_use_generated_n_zero_equals_default` | [x] |
| 11 | `atoi` in `main` (no error signalling in C) | argv that is not a number at all: `"abc"`, `"-"`, `"+"`, `""`, `" "`, `"x1"`, `"."` | parses as `0`; the program still succeeds with exit status **0** | `c11_atoi_non_numeric` | [x] |
| 12 | `atoi` in `main` | argv whose numeric prefix ends early: `"12abc"`, `"3.9"`, `"1e5"`, `"0x10"`, `"007z"` | only the leading decimal digits are used (`12`, `3`, `1`, `0`, `7`); exit **0** | `c12_atoi_partial_numeric` | [x] |
| 13 | `atoi` in `main` | argv beyond `INT` range but inside `long`: `"2147483648"`, `"-2147483649"`, `"4294967296"` | `(int)` truncation of the `long` value (`-2147483648`, `2147483647`, `0`); exit **0** | `c13_atoi_int_overflow` | [x] |
| 14 | `atoi` in `main` | argv beyond `long` range: `"9223372036854775808"`, `"-9223372036854775809"`, 40-digit numbers | `strtol` saturates to `LONG_MAX`/`LONG_MIN`, then `(int)` truncation → `-1` / `0`; exit **0** | `c14_atoi_long_overflow` | [x] |
| 15 | `main` | `argc > 3` — extra arguments | silently ignored (only `argv[1]`, `argv[2]` are read); exit **0** | `c15_extra_args_ignored` | [x] |
| 16 | build-time `CHOOSE_REP` (`mdmacros.h:73-74`) | `-DREPEAT=8` (and `9`, `42`) — `REP8` was never defined | **C compile error** (`implicit declaration of function 'REP8'` / undeclared). Rust: no Cargo feature `8`, so `cargo` rejects the build with `none of the selected packages contains these features`. Both refuse to build. | `c16_build_time_repeat_out_of_range` | [x] |
| 17 | build-time `OP_FN`/`STEP_OP`/`INIT_FOR` | `-DOP=div` (any `op` ∉ `{add,sub,mul}`) — `op_div`, `STEP_div`, `INIT_div` were never defined | **C compile error**. Rust: no Cargo feature `div`, so `cargo` rejects the build. Both refuse to build. | `c17_build_time_bad_op` | [x] |
| 18 | `G_OP` (writable `.data` object) | a consumer stores `NULL` into `G_OP` and calls through it | **undefined behaviour / SIGSEGV in both** implementations — not executed as a test (it would kill the harness); documented instead. The *writability* of `G_OP` (the checkable part) is verified by `b14_g_op_writable_then_call_through`. | documented | [x] |
| 19 | `op_add` / `op_sub` / `op_mul` | signed-integer overflow (`INT_MAX + 1`, `INT_MIN - 1`, `INT_MAX * INT_MAX`, …). C has no check — it is UB, and gcc at the CMake default optimisation level (`-O0`, `CMAKE_C_FLAGS` is overwritten with only the two `-D`s) wraps two's-complement. | the wrapped 32-bit result; no error is reported | `b04_op_fns_edge_cross_product`, `c19_op_overflow_no_rejection` | [x] |
| 20 | `helper_call` | return value `r + acc` overflows `int` (e.g. `a = INT_MAX`, `REPEAT = 7` → `acc = 21`) | the wrapped 32-bit result; no error is reported | `c20_helper_call_return_overflow` | [x] |
| 21 | `main` (generic OS-boundary case, not a C branch) | `argv[0]` and/or `argv[1..2]` contain bytes that are not valid UTF-8 (`\xff12`, `\x80`, a lone `\xc3`, a surrogate encoding) | C copies `argv[0]` into the usage message verbatim and `atoi` reads raw bytes; no rejection. A lossy `String` conversion would corrupt the bytes | `c21_non_utf8_argv` | [x] |
| 22 | `helper_call` / `helper_ptr` / `use_generated` / `main` (generic OS-boundary case) | `stdout` cannot be written (`/dev/full` → `ENOSPC` on every `write`) | `printf`'s return value is discarded, so the functions return their normal result and `main` still exits **0** | `c22_unwritable_stdout` | [x] |

## Notes on the two "no rejection at all" facts

* Neither `op_add`/`op_sub`/`op_mul` nor `helper_ptr` contains a single branch —
  they cannot reject anything. Rows 19–20 pin down the *absence* of a check
  (wrap-around instead of trapping), which is exactly the behaviour the Rust must
  reproduce; `mdcore.rs`/`mdmacros.rs` use `wrapping_*` everywhere for this
  reason (a plain `+` would panic in a debug Rust build).
* `use_generated` is **independent of `REPEAT`**: `DISPATCH_REP(op, acc, n)`
  switches on the *argument* `n`, not on `REPEAT`. Only `helper_call`'s
  `RUN_LOOP(op, acc, REPEAT)` depends on `REPEAT`. That produces the notable
  asymmetry at `REPEAT = 7`: `helper_call` reports `acc = 21` (`REP7` exists as a
  macro) while `use_generated(7)` reports `0` (`case 7:` does *not* exist in
  `DISPATCH_REP`). Both behaviours are asserted.

## Known, documented deviation (not a row above)

`main`'s behaviour when stdout is a **pipe whose reader has already closed**
differs, and cannot be reconciled without `unsafe`:

* C inherits the default `SIGPIPE` disposition, so the process is killed by
  signal 13.
* The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime start-up
  (before `main` runs) for every Rust program; the write then fails with
  `EPIPE`, which -- exactly like every other write failure, ERRORS row 22 -- is
  discarded, and the process exits 0.

Restoring the default disposition needs a raw `signal(2)` call, and would in
turn expose a *second* divergence, because glibc fully buffers a redirected
`stdout` (one `write` at exit) while Rust's `Stdout` is line buffered (one
`write` per line), so the two would die at different points in the output.
Every configuration in which stdout is readable to completion -- i.e. every way
of actually comparing the two programs' output -- is byte-identical and is
covered by `b18`-`b20` and `c01`-`c22`.
