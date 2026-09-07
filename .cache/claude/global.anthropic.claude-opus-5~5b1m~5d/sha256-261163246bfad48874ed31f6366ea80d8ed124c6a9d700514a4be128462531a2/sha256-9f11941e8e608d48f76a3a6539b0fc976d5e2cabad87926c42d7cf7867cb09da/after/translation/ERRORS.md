# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/{mdcore.c,mdmain.c,mdmacros.h}`.
Exhaustive grep for every rejection construct:

```
grep -n 'return|assert|NULL|if |switch|case |default|fprintf|stderr' c_src/src/*
```

Findings: **no** `assert`, **no** `RETURN_ERROR`-style macro, **no** error enum,
**no** `return -1`, **no** `return NULL`, **no** pointer parameters anywhere
(every public function takes only `int`s). The whole rejection surface is:

1. `mdmain.c:29-32` — `if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }`
2. `mdmacros.h:82-93` — `DISPATCH_REP`'s `switch (n)` covers **only `case 0..6`**;
   everything else falls into `default: break`, so the accumulator is left at
   `INIT_FOR(op)`. Note the asymmetry the C actually has: `REP7` exists and
   `RUN_LOOP` can use it, but the `switch` has **no `case 7`** — so
   `use_generated(7)` silently returns `INIT`, unlike `RUN_LOOP` with `REPEAT=7`.
   *(Do not "fix" this — replicate it.)*
3. `atoi()` (`mdmain.c:33-34`) — glibc `atoi` = `(int)strtol(s,NULL,10)`: never
   errors, returns `0` for un-parsable input, saturates to `LONG_MAX`/`LONG_MIN`
   on overflow and then truncates to `int`.
4. Signed-overflow paths in `op_add`/`op_sub`/`op_mul` and `STEP_mul` — formally
   UB in C, in practice two's-complement wrap under `gcc -O2`. Rust must produce
   the identical wrapped value (hence `wrapping_*`, and `panic = "abort"` must
   never be reachable via an arithmetic overflow panic).
5. Build-time (not runtime) rejections, listed for completeness: `OP` not in
   `{add,sub,mul}` → `op_<OP>` / `STEP_<OP>` / `INIT_<OP>` undefined → C compile
   error; `REPEAT` outside `0..7` → `CHOOSE_REP` pastes `REP8`… which is not a
   macro → C compile error. Rust models these as the closed feature sets
   `{add,sub,mul}` and `{"0".."7"}`, so they are unrepresentable.

## Table

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| E1 | `main` (driver) | `argc == 1` (no args at all) | `stderr` = `usage: <argv0> A B\n`, empty stdout, exit code `2` | `err_e1_argc_1` | [x] |
| E2 | `main` (driver) | `argc == 2` (exactly one arg) | same as E1 — exit `2` | `err_e2_argc_2` | [x] |
| E3 | `main` (driver) | `argc == 3` but argv[1] is non-numeric (`"abc"`, `""`, `"+"`, `"-"`, `"x9"`) | no rejection: `atoi` → `0`, runs normally, exit `0` | `err_e3_atoi_nonnumeric` | [x] |
| E4 | `main` (driver) | argv numeric string overflowing `long` (`"99999999999999999999"`, `"-99999999999999999999"`) | `strtol` saturates then truncates → `-1` / `0`; exit `0` | `err_e4_atoi_overflow` | [x] |
| E5 | `main` (driver) | argv numeric string overflowing only `int` (`"2147483648"`, `"-2147483649"`, `"4294967296"`) | `strtol` fits in `long`, cast truncates to `int`; exit `0` | `err_e5_atoi_int_trunc` | [x] |
| E6 | `main` (driver) | argv with leading whitespace / `+` sign / trailing garbage (`" 12"`, `"+12"`, `"12abc"`, `"1 2"`) | `strtol` semantics: `12,12,12,1`; exit `0` | `err_e6_atoi_partial` | [x] |
| E7 | `use_generated` | `n == 7` — `REP7` exists but `switch` has **no `case 7`** → `default` | returns `INIT_FOR(OP)` (0 for add/sub, 1 for mul); prints `gen.acc=<INIT>` | `err_e7_use_generated_7` | [x] |
| E8 | `use_generated` | `n > 7` (8, 9, 100, `INT_MAX`) → `default` | returns `INIT_FOR(OP)` | `err_e8_use_generated_above` | [x] |
| E9 | `use_generated` | `n < 0` (-1, -7, `INT_MIN`) → `default` | returns `INIT_FOR(OP)` | `err_e9_use_generated_negative` | [x] |
| E10 | `use_generated` | every "one step past the valid range" boundary: `n ∈ {-1, 0, 6, 7}` | `-1`→`INIT`, `0`→`INIT`, `6`→`REP6` result, `7`→`INIT` | `err_e10_use_generated_boundary` | [x] |
| E11 | `op_add` | signed overflow: `INT_MAX + 1`, `INT_MIN + (-1)`, `INT_MAX+INT_MAX`, `INT_MIN+INT_MIN` | two's-complement wrap | `err_e11_op_add_overflow` | [x] |
| E12 | `op_sub` | signed overflow: `INT_MIN - 1`, `INT_MAX - INT_MIN`, `0 - INT_MIN` | two's-complement wrap | `err_e12_op_sub_overflow` | [x] |
| E13 | `op_mul` | signed overflow: `INT_MAX*INT_MAX`, `INT_MIN*-1`, `INT_MIN*INT_MIN`, `65536*65536` | two's-complement wrap | `err_e13_op_mul_overflow` | [x] |
| E14 | `helper_call` | overflow inside the op **and** inside `RUN_LOOP`'s `STEP_*` (`a=b=INT_MAX`, `INT_MIN`) | wrapped op result + wrapped accumulator, no panic/abort | `err_e14_helper_call_overflow` | [x] |
| E15 | `helper_ptr` | overflow inside the indirect call (`INT_MAX`, `INT_MIN`) | wrapped result | `err_e15_helper_ptr_overflow` | [x] |
| E16 | `G_OP` (function-pointer global) | called with overflowing operands | identical wrap to the direct `op_<OP>` call; pointer must be non-NULL and equal to the exported `op_<OP>` address | `err_e16_g_op_overflow_and_identity` | [x] |
| E17 | `G_OP_NAME` (pointer global) | dereferencing the exported `const char*` | non-NULL, NUL-terminated, equals `STR(OP)`; both sides byte-identical | `err_e17_g_op_name_not_null` | [x] |
| E18 | out-of-range "enum"-like ints across FFI | the API has no `enum` parameters, so the equivalent is: every function takes a *raw* `int` and must accept any bit pattern. Sweep `op_*`, `helper_*`, `use_generated` over `{INT_MIN, INT_MIN+1, -2,-1,0,1,2, INT_MAX-1, INT_MAX}` cross-product | never rejects; value-identical to C for every pair | `err_e18_full_int_boundary_sweep` | [x] |
| E19 | null / oversized pointer arguments | **not applicable**: `nm`-exported functions take no pointers. The only pointers are the two *globals* (E16/E17). Asserted explicitly so the row is covered rather than assumed. | n/a — verified by E16/E17 | `err_e16_*`, `err_e17_*` | [x] |
| E20 | `main` (driver) | `argc > 3` — extra args are silently ignored (no rejection) | identical stdout to `argc == 3`, exit `0` | `err_e20_argc_extra` | [x] |
| E21 | `main` (driver) | `argc == 0` (`argv[0] == NULL`, `printf("%s", NULL)` → glibc `(null)`) | unreachable from `execve` through `std::process::Command`/`posix_spawn`, which always pass at least `argv[0]`. Documented, not testable from either side. | n/a | [n/a] |
