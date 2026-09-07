# ERRORS.md — error / rejection surface of the C code

Derived mechanically from `c_src/src/{mdcore.c,mdmain.c,mdmacros.h}`. The greps
that produced it, and what they found:

```
grep -nE 'return'                          -> mdcore.c:28,29,30,44,51,57  mdmain.c:31,47  mdmacros.h:99
grep -nE 'assert|abort|exit\(|errno|NULL|perror|strerror'   -> (no matches)
grep -nE 'RETURN_ERROR|ERROR|ERR_|_ERR|fail|invalid'        -> (no matches)
grep -nE 'if *\(|switch|default:|case '    -> mdmain.c:29  mdmacros.h:77,83-91
grep -nE 'MAX|MIN|LIMIT'                   -> (license text only)
```

So the library has **no error enum, no error-return macro, no sentinel return, no
`assert`, no null check and no numeric range check**. Every public function
returns `int` and every parameter is a by-value `int`; there is no pointer
parameter anywhere, so there is no null-pointer rejection to reproduce. The
complete rejection surface is the 12 rows below.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `use_generated(n)` → `accum_<OP>(n)` → `DISPATCH_REP` (`mdmacros.h:91`, `default: break;`) | `n == 7` — the one value for which `REP7` exists but the `switch` has no `case` | no step applied: returns `INIT_FOR(OP)` (`0` for add/sub, `1` for mul), prints `gen.acc=<INIT>\n`. Not an error return — silent no-op. | [x] `errors::row01_use_generated_seven` |
| 2 | same | `n` any value `> 7` (`8`, `9`, `100`, `1000`) | same silent `INIT_FOR(OP)` | [x] `errors::row02_use_generated_above_range` |
| 3 | same | `n == INT_MAX` (`2147483647`) — one step past the widest positive selector | same silent `INIT_FOR(OP)` | [x] `errors::row03_use_generated_int_max` |
| 4 | same | `n < 0` (`-1`, `-2`, `-7`, random negatives) — the `switch` selector is a signed `int`, so negatives reach `default:` | same silent `INIT_FOR(OP)` | [x] `errors::row04_use_generated_negative` |
| 5 | same | `n == INT_MIN` (`-2147483648`) — extreme negative selector; `-n` would overflow | same silent `INIT_FOR(OP)` | [x] `errors::row05_use_generated_int_min` |
| 6 | same | `n == 6` vs `n == 7` boundary pair — last accepted `case` immediately followed by the first rejected value | `6` → `REP6` result (`15`/`-15`/`720`); `7` → `INIT_FOR(OP)`. The two must differ for add/sub/mul alike. | [x] `errors::row06_use_generated_boundary_6_7` |
| 7 | `main` (`mdmain.c:29-32`) | `argc < 3`, i.e. fewer than two operands: `driver`, `driver A` | `fprintf(stderr, "usage: %s A B\n", argv[0])`, **exit status 2**, nothing on stdout | [x] `driver_cli::row07_argc_too_small` |
| 8 | `main` | `argc > 3` — extra trailing operands (`driver 1 2 3 4`) | *accepted*, not rejected: `argv[3..]` ignored, identical output to `driver 1 2`, exit 0 | [x] `driver_cli::row08_extra_args_ignored` |
| 9 | `atoi` at `mdmain.c:33-34` | operand with no digits at all (`""`, `"abc"`, `"+"`, `"-"`, `"--3"`, `"0x10"` → stops at `x`) | no diagnostic; `atoi` yields `0` (or the prefix it could parse) and execution continues | [x] `driver_cli::row09_non_numeric_operands` |
| 10 | `atoi` at `mdmain.c:33-34` | operand outside `int` range (`"2147483648"`, `"-2147483649"`, `"99999999999999999999999"`) | no diagnostic; glibc `atoi` is `(int)strtol(...)`, so the value saturates at `LONG_MAX`/`LONG_MIN` and is then truncated to the low 32 bits | [x] `driver_cli::row10_out_of_range_operands` |
| 11 | `op_add`/`op_sub`/`op_mul`, and `STEP_*` inside `REP<n>` | signed-overflow operands (`INT_MAX + 1`, `INT_MIN - 1`, `INT_MAX * INT_MAX`) — UB in ISO C, but the reference build is gcc `-O2` two's complement without `-ftrapv` | wraps modulo 2^32; no trap, no diagnostic. Rust must use `wrapping_*`, not `checked_*`/panicking arithmetic. | [x] `errors::row11_overflow_wraps` |
| 12 | build-time rejection, `CHOOSE_REP` (`mdmacros.h:73-74`) / `INIT_FOR` (`:59`) / `OP_FN` (`:45`) | `-DREPEAT=8` (no `REP8`) or `-DOP=div` (no `INIT_div`/`STEP_div`/`op_div`) | **compile error**, no binary produced. Confirmed: `-DREPEAT=8` → `error: 'add' undeclared`; `-DOP=div` → `error: 'INIT_div' undeclared`. Mirrored in Rust by `Cargo.toml` only offering features `add`/`sub`/`mul` and `0`..`7`. | [x] `build_surface::row12_out_of_range_build_config` |

## Generic FFI boundary cases (not in the table, covered anyway)

| case | applicability here | covered by |
|------|--------------------|-----------|
| null pointer arguments | **not applicable** — no public function takes a pointer | — |
| zero / oversized lengths | **not applicable** — no length or size parameter, no buffer | — |
| out-of-range enum value across FFI | **no enum in the API**. The structural equivalent is the `int n` selector of `DISPATCH_REP`'s `switch`, whose valid set is `{0,…,6}`; every `int` outside it is a live input the C accepts and silently maps to `default:`. Covered exhaustively for `n ∈ -16..=16` plus `INT_MIN`/`INT_MAX` plus randomized `i32`. | rows 1–6, `errors::row_all_i32_selector_sweep` |
| one step past a documented valid range | `n = 7` (past the `switch`), `REPEAT = 8` (past `REP7`), `INT_MAX`/`INT_MIN` operands | rows 1, 3, 5, 6, 11, 12 |
| writable global function pointer `G_OP` | `G_OP` is non-`const` in C, so a caller can overwrite it; setting it to `NULL` and calling segfaults in **both** implementations (matching UB, not a matchable return value). Not asserted; what *is* asserted is that the loaded value dispatches to the same `op_<OP>` in both, and that overwriting it with a caller-supplied `extern "C"` pointer is honoured identically. | `configs::row_g_op_rebind` |

## Result

All 12 rows have a passing differential test, in all 24 `(OP, REPEAT)`
configurations (`./run_all.sh`). Every row asserts the *specific* C value —
`INIT_FOR(OP)` for the `default:` arm, exit status exactly `2` for the usage
path, the exact wrapped `int` for overflow — never merely "both sides failed".

Rows 1–6 and 11 are in `tests/errors.rs`; rows 7–10 in `tests/driver_cli.rs`;
row 12 in `tests/build_surface.rs`.

One divergence was found and fixed while working through this table: row 10
(`atoi` on operands outside `int` range) diverged at exactly `LONG_MIN`. See the
"Divergence found and fixed" section of `CONFIGS.md`.
