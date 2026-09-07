# ERRORS.md — error/rejection surface table (Phase A, tested in Phase C)

Derived mechanically from the C sources. The complete set of rejection-ish
constructs in `c_src/` is:

```
$ grep -nE "return|assert|if *\(|switch|default|NULL|-1|#ifndef|#error|exit" c_src/src/*.c c_src/src/*.h
mdmain.c:29:    if (argc < 3) {          <- the ONLY runtime input validation
mdmain.c:31:        return 2;            <- the ONLY error exit code
mdmain.c:47:    return 0;
mdmacros.h:27:#ifndef OP                <- build-time default
mdmacros.h:30:#ifndef REPEAT            <- build-time default
mdmacros.h:83:  switch (n) {             <- DISPATCH_REP range dispatch
mdmacros.h:91:    default: break;        <- silent rejection of out-of-range n
mdmacros.h:99:    return acc;
mdcore.c:28/29/30/44/51/57            <- unconditional value returns
```

There are **no** `assert`s, no `RETURN_ERROR`-style macros, no `return -1`, no
`return NULL`, no error enums, no null-pointer checks and no min/max constants in
this library. Every row below is one distinct way the C code rejects / silently
absorbs input, plus the generic FFI boundaries the API still has to survive.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `main` (`driver`) | `argc < 3` — no arguments at all (`argc == 1`) | `fprintf(stderr,"usage: %s A B\n", argv[0])`, exit status **2**, nothing on stdout | `err_01_main_argc_zero_args` |
| 2 | `main` (`driver`) | `argc < 3` — exactly one argument (`argc == 2`) | same usage line on stderr, exit status **2**, empty stdout | `err_02_main_argc_one_arg` |
| 3 | `main` (`driver`) | `argc >= 3` boundary: extra arguments (`argc == 4`, `argc == 8`) are **not** rejected; argv[3..] ignored | normal run, exit status **0** | `err_03_main_extra_args_not_rejected` |
| 4 | `use_generated` → `DISPATCH_REP` `default:` | `n == 7` (one past the largest `case`, and the largest legal `REPEAT`) | no step executed, accumulator stays `INIT_FOR(OP)` → prints `gen.acc=<INIT>`, returns `INIT` (0 for add/sub, 1 for mul) | `err_04_dispatch_n_seven` |
| 5 | `use_generated` → `DISPATCH_REP` `default:` | `n == 8` (well past the last `case`) | returns/prints `INIT` | `err_05_dispatch_n_eight` |
| 6 | `use_generated` → `DISPATCH_REP` `default:` | `n == -1` (negative, no matching `case`) | returns/prints `INIT` | `err_06_dispatch_n_negative_one` |
| 7 | `use_generated` → `DISPATCH_REP` `default:` | `n == INT_MIN` (`-2147483648`) | returns/prints `INIT` | `err_07_dispatch_n_int_min` |
| 8 | `use_generated` → `DISPATCH_REP` `default:` | `n == INT_MAX` (`2147483647`) | returns/prints `INIT` | `err_08_dispatch_n_int_max` |
| 9 | `use_generated` → `DISPATCH_REP` `default:` | every `n` in `-64..=64` and 4096 random `int`s (fuzz of the whole switch, incl. the accepted `0..=6` cases) | `INIT` outside `0..=6`, `REP<n>(OP,acc)` inside | `err_09_dispatch_full_range_and_fuzz` |
| 10 | `use_generated` | `n` passed as an out-of-range "enum-like" `int` across FFI (`0x7fffffff`, `0x80000000` reinterpreted, `1<<31`, `0xdeadbeef` as `i32`) — C `switch` accepts any `int` | `default: break` → `INIT` | `err_10_dispatch_bit_pattern_ints` |
| 11 | `op_add` | signed-overflow boundary `INT_MAX + 1`, `INT_MIN + (-1)` (C signed overflow; gcc emits wrapping `add`) | wrapping result (`INT_MIN`, `INT_MAX`) | `err_11_op_add_overflow` |
| 12 | `op_sub` | signed-overflow boundary `INT_MIN - 1`, `INT_MAX - (-1)`, `0 - INT_MIN` | wrapping result | `err_12_op_sub_overflow` |
| 13 | `op_mul` | signed-overflow boundary `INT_MIN * -1`, `INT_MAX * 2`, `65536 * 65536`, `INT_MIN * INT_MIN` | wrapping (low 32 bits of the product) | `err_13_op_mul_overflow` |
| 14 | `helper_call` | overflow of the *composed* result `r + acc` at `a=INT_MAX,b=INT_MAX` (add), and of `acc` itself for `mul` with `REPEAT=7` (`7! ` fits, but `r*acc` may wrap) | wrapping result, stdout `helper.call=<r> helper.acc=<acc>` | `err_14_helper_call_overflow` |
| 15 | `helper_ptr` | overflow boundary through the function-pointer call path (`INT_MIN`, `INT_MAX` operands) | wrapping result, stdout `helper.ptr=<r>` | `err_15_helper_ptr_overflow` |
| 16 | `G_OP` (exported mutable global) | a caller **stores** a different function pointer into `G_OP` (legal: `int (*G_OP)(int,int)` is a mutable object in `.data`) | store succeeds (no fault), subsequent read returns the stored pointer; library behaviour unchanged because no C function reads `G_OP` | `err_16_g_op_is_writable` |
| 17 | `G_OP_NAME` (exported mutable global) | a caller **stores** a different `const char *` into `G_OP_NAME` | store succeeds (no fault); pointed-to bytes are the read-only literal `"add"`/`"sub"`/`"mul"` | `err_17_g_op_name_is_writable` |
| 18 | build-time `CHOOSE_REP(REPEAT)` / `REP<n>` | `REPEAT` > 7 (e.g. `-DREPEAT=8`) → `CAT(REP,8)` yields the undefined `REP8`, so `RUN_LOOP` expands to the *function call* `REP8(add, acc)` | **compile-time** rejection, verified: `gcc -DOP=add -DREPEAT=8 ...` exits 1 with `warning: implicit declaration of function 'REP8'` + `error: 'add' undeclared (first use in this function)`; no artifact is produced. Rust mirrors this by having no `"8"` feature (so `--features 8` is rejected by cargo). | documented + `configs_00_build_matrix_matches` (asserts `repeat() ∈ 0..=7`) |
| 19 | build-time `#ifndef OP` / `#ifndef REPEAT` | `OP`/`REPEAT` not defined at all | silently defaults to `add` / `5` — no error. Rust mirrors it via the `cfg(not(any(...)))` fallbacks in `mdconfig.rs`. | `cargo check --no-default-features` (in `check_all.sh`) |
| 20 | `main`'s `atoi` | non-numeric argv (`"abc"`), empty string, `"+"`/`"-"` alone, leading whitespace, `"0x10"`, trailing garbage (`"12abc"`) — `atoi` has no error report | parses the leading numeric prefix, else `0`; no rejection | `err_20_main_atoi_non_numeric` |
| 21 | `main`'s `atoi` | out-of-`int` argv (`"99999999999999999999"`, `"-99999999999999999999"`) — glibc `atoi` is `(int)strtol`, saturating at `LONG_MAX`/`LONG_MIN` then truncating | `-1` for the positive overflow, `0` for the negative one (low 32 bits of `LONG_MAX`/`LONG_MIN`) | `err_21_main_atoi_overflow` |
| 22 | `main` | `argc == 0` (`execve` with an empty `argv`, so `argv[0]` is `NULL` and `%s` is fed a null pointer) | usage line with an empty program name (`"usage:  A B\n"`) on stderr, exit status **2** | `err_22_main_argc_zero_null_argv0` |
| 23 | `main` / `helper_*` / `use_generated` `printf` | stdout write fails (`ENOSPC`: stdout redirected to `/dev/full`) — every `printf` return value is discarded by the C | no error reported, exit status **0**, stderr empty | `err_23_stdout_write_error_ignored` |
| 24 | `main` `fprintf(stderr, ...)` | stderr write fails (`/dev/full`) on the `argc < 3` path | no error reported, exit status still **2** | `err_24_stderr_write_error_ignored` |
| 25 | `main` / `printf` | file descriptor 1 is **closed** before `exec` (`EBADF` on every write) | no error reported, exit status **0**, stderr empty | `err_25_stdout_closed` |

Notes on things that are *not* rows because the C never checks them:

* No pointer arguments exist anywhere in the exported API (`op_*`, `helper_*`,
  `use_generated` take `int`s only), so there is no null-pointer check to
  mirror; rows 16/17 cover the only pointer-typed part of the surface (the two
  exported globals). "Null pointer" is still exercised: row 17 stores `NULL`
  into `G_OP_NAME` in both libraries and checks neither faults.
* No lengths/sizes/buffers exist, so "zero and oversized lengths" degenerate to
  the `int` extremes covered by rows 4–15.
