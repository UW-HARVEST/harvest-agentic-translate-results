# ERRORS.md — error / rejection surface of the C source

Derived mechanically from `c_src/src/{mdcore.c,mdmain.c,mdmacros.h}`. The grep for
every rejection construct

```
grep -nE 'return|assert|NULL|errno|error|exit|abort|if *\(|switch|default' c_src/src/*.c c_src/src/*.h
```

finds **exactly one** explicit runtime rejection (`mdmain.c:29`), **one** silent
runtime fallthrough (`mdmacros.h:91`), and **two** build-time token-paste failures
(`mdmacros.h:45`/`59`/`52` and `mdmacros.h:73`). There is no `assert`, no
`RETURN_ERROR`-style macro, no error enum, no null check, no length check, and no
range validation anywhere in the library. That absence is itself the contract: the
library is total on its input domain, and any "invalid" value must produce the same
defined-by-implementation result on both sides rather than an error code.

## Rejection table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `main` (`mdmain.c:29-32`) | `argc < 3`, i.e. the program is invoked with 0 or 1 operand | `fprintf(stderr, "usage: %s A B\n", argv[0])` then `return 2`; **nothing** on stdout | `err_01_argc_1_usage_exit2`, `err_02_argc_2_usage_exit2`, `err_03b_argv0_shapes_in_usage_line` (incl. `argc == 0` via `execv` with an empty `argv`) |
| 2 | `main` (`mdmain.c:29`) | `argc == 1` (bare program name) | same as #1: usage line naming `argv[0]`, exit status 2 | `err_01_argc_1_usage_exit2` |
| 3 | `main` (`mdmain.c:29`) | `argc == 2` (one operand) | same as #1, exit status 2 | `err_02_argc_2_usage_exit2`; accepted boundary `argc == 3` in `err_03_argc_boundary_3_is_accepted` |
| 4 | `main` (`mdmain.c:33-34`) via `atoi` | `argv[1]`/`argv[2]` contain no digits at all (`""`, `"abc"`, `"+"`, `"-"`, `"  "`) | no rejection — `atoi` returns `0`; run proceeds with that operand `= 0`, exit status 0 | `err_04_atoi_no_digits` |
| 5 | `main` via `atoi` | operand has trailing garbage (`"12x"`, `"-12abc"`) | no rejection — `atoi` stops at the first non-digit and returns the prefix value | `err_05_atoi_trailing_garbage` |
| 6 | `main` via `atoi` | operand magnitude exceeds `LONG_MAX` while positive (`"99999999999999999999"`) | `strtol` clamps to `LONG_MAX`; `(int)LONG_MAX == -1` | `err_06_atoi_pos_overflow` |
| 7 | `main` via `atoi` | operand magnitude exceeds `-(unsigned long)LONG_MIN` while negative (`"-99999999999999999999"`) | `strtol` clamps to `LONG_MIN`; `(int)LONG_MIN == 0` — **not** `1`; the negative clamp reaches one further out than the positive one | `err_07_atoi_neg_overflow` |
| 8 | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n < 0` (any negative selector, incl. `INT_MIN`) | `default: break;` — accumulator untouched, returns `INIT_FOR(OP)` (`0` for add/sub, `1` for mul) and prints `gen.acc=<INIT>` | `err_08_use_generated_negative` |
| 9 | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n == 7` — `REP7` exists but the `switch` has no `case 7:` | `default: break;` — returns `INIT_FOR(OP)`, **not** the 7-step result. A genuine asymmetry in the header, reproduced verbatim | `err_09_use_generated_7_falls_to_default` |
| 10 | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n > 7` (`8`, `100`, `INT_MAX`) | `default: break;` — returns `INIT_FOR(OP)` | `err_10_use_generated_above_range` |
| 11 | `use_generated` → `DISPATCH_REP` | `n == 6` (last accepted case) and `n == 0` (first accepted case) — the in-range boundaries either side of the rejections | accepted: `REP6`/`REP0` applied normally | `err_11_use_generated_boundaries` |
| 12 | `op_add` (`mdcore.c:28`) | `a + b` overflows `int` (`INT_MAX + 1`, `INT_MIN + (-1)`) — signed overflow, UB in C, **no check** | gcc/clang two's-complement wrap on this target | `err_12_op_add_overflow` |
| 13 | `op_sub` (`mdcore.c:29`) | `a - b` overflows `int` (`INT_MIN - 1`, `INT_MAX - (-1)`) — UB, no check | two's-complement wrap | `err_13_op_sub_overflow` |
| 14 | `op_mul` (`mdcore.c:30`) | `a * b` overflows `int` (`INT_MIN * -1`, `65536 * 65536`) — UB, no check | two's-complement wrap | `err_14_op_mul_overflow` |
| 15 | `helper_call` (`mdcore.c:44`) | `r + acc` overflows `int` — UB, no check | two's-complement wrap | `err_15_helper_call_sum_overflow` |
| 16 | `STEP_mul` inside `REP<n>` (`mdmacros.h:50`) | `acc *= (i+1)` overflows — only reachable with `OP=mul`; unreachable for add/sub since `|acc| <= 21` | two's-complement wrap (not reachable from `INIT_mul = 1` with `REPEAT <= 7`: `7! = 5040`) | `err_16_step_mul_overflow_reachable_range` |
| 17 | `G_OP` (`mdcore.c:36`) | caller overwrites the non-`const` global with an arbitrary pointer and calls through it | no validation — dispatches to whatever was stored; a null store faults identically on both sides (not exercised: it is a crash, not a rejection) | `cfg_23_g_op_writable` (in `tests/phase_b_valid.rs`) |
| 18 | build time — `OP_FN`/`INIT_FOR`/`STEP_OP` (`mdmacros.h:45,59,52`) | `-DOP=div` (any token outside `add`/`sub`/`mul`) | **compile error**: `'INIT_div' undeclared`, implicit `op_div`, no `STEP_div`. Rust side: no such feature exists, so `cargo` rejects `--features div` | `err_18_build_time_bad_op` |
| 19 | build time — `CHOOSE_REP` (`mdmacros.h:73-74`) | `-DREPEAT=8` (only `REP0`..`REP7` defined) | **compile/link error**: implicit declaration of `REP8`, then `undefined reference to REP8` | `err_19_build_time_repeat_8` |
| 20 | build time — `CHOOSE_REP` (`mdmacros.h:73-74`) | `-DREPEAT=-1` (negative token) | **compile error**: `pasting "REP" and "-" does not give a valid preprocessing token` | `err_20_build_time_repeat_negative` |
| 21 | `FOR_EACH`/`DO_LOOP` (`mdmacros.h:77-78`) | `n <= 0` in the runtime-bounded loop | `i < (n)` false on entry — zero iterations, `acc` unchanged. Never instantiated by `mdcore.c`/`mdmain.c`, so it contributes no symbol; translated as `do_loop` for surface parity | `err_21_do_loop_nonpositive` — builds a fixture `.so` from the unmodified header that *does* instantiate `DO_LOOP`, and compares its runtime loop against the unrolled `REP<n>` both real libraries expose |
| 22 | every exported function | a NULL `const char *`/pointer argument | **not applicable** — no exported function takes a pointer argument. `G_OP_NAME` is the only pointer in the ABI and it is an *out*-facing data slot. Recorded so the absence is explicit rather than an oversight | `err_22_no_pointer_params` |
| 23 | `use_generated` | out-of-range "enum" value across the FFI boundary: `DISPATCH_REP`'s `switch (n)` is the only enum-shaped selector in the API, and C accepts any `int` for it | rows #8–#11 cover the full partition: `n < 0`, `0..=6`, `7`, `> 7` | `err_08_use_generated_negative`, `err_09_use_generated_7_falls_to_default`, `err_10_use_generated_above_range`, `err_11_use_generated_boundaries` |

## Notes on what is *not* an error here

- No function returns a sentinel error code. `op_*` return the arithmetic result;
  `helper_call`/`helper_ptr`/`use_generated` return computed ints. `0` and `-1` are
  ordinary results, so "same error code" for this library means "same returned int
  **and** same bytes on stdout".
- `printf`'s return value is discarded at `mdcore.c:43,50,56`, so a failing write is
  silently ignored. `src/stdio.rs` discards its `io::Result` for the same reason.
- Rows #18–#20 are build-time rejections and are asserted by invoking the compilers,
  not by calling into a `.so`.

## Status

All 23 rows have a passing differential test. Rows 1–17 and 21–23 run in
`tests/phase_c_errors.rs` (row 17 in `tests/phase_b_valid.rs`); rows 18–20 assert
the build-time rejections by invoking `gcc` on the unmodified `c_src/` and by
checking `Cargo.toml` declares no feature the C cannot build.

Every row is re-run for all 26 configurations by `./sweep_so.sh`.

### Divergence found and fixed

Row 7 was a **real bug** in the translation. `src/cstdlib.rs` clamped the parsed
magnitude in `i64` and then negated it, giving `-LONG_MAX` for a negative overflow;
glibc's `strtol` clamps to `LONG_MIN`, one further out. `atoi("-99999999999999999999")`
was therefore `1` in Rust versus `0` in C, which propagated into every line
`mdmain.c` prints. The magnitude is now accumulated in `u64` against a
sign-dependent limit (`LONG_MAX` when positive, `2^63` when negative). Regression
covered by `err_07_atoi_neg_overflow`, and `mutation_check.sh` confirms reverting
the fix is caught.
