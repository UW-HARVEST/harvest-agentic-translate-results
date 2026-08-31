# ERRORS.md — error-surface table (Phase C)

## Derivation

Mechanical grep over the entire C source tree for every rejection / error /
bounds construct:

```sh
grep -nE 'RETURN_ERROR|return *-1|return +NULL|assert|errno|exit\(|abort|
          if *\(.*(<|>|==|!=).*\)|switch|default:|#ifndef|#error' c_src/src/*.c c_src/src/*.h
```

Findings — the complete set of rejection/fallback points in the C code:

| location | construct | classification |
|----------|-----------|----------------|
| `mdmain.c:29` | `if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }` | the **only** explicit error return in the project |
| `mdmacros.h:88` (`DISPATCH_REP`) | `default: break;` — `switch (n)` only has `case 0 … case 6` | silent fallback: accumulator is left at `INIT_FOR(OP)` |
| `mdmacros.h:27-32` | `#ifndef OP / #define OP add`, `#ifndef REPEAT / #define REPEAT 5` | build-time default, not a runtime rejection |
| `mdmain.c:33-34` | `atoi(argv[1])`, `atoi(argv[2])` | no validation at all; `atoi` returns `0` for un-parsable text and has UB on overflow (glibc: `(int)strtol(…,10)`, i.e. clamp-to-`long`-then-truncate) |

There are **no** `assert`s, **no** null-pointer checks, **no** `errno` use,
**no** `return -1` / `return NULL`, **no** error enums and **no** range checks
on `a`/`b`/`n` anywhere in `mdcore.c` or `mdmacros.h`. Consequently the library
half of the code has exactly one "rejection" behaviour (the `DISPATCH_REP`
`default:` arm); everything else accepts all `int` inputs unconditionally. The
generic-boundary rows below (nulls, out-of-range "enum-like" values, extreme
ints) are therefore included explicitly even though the C contains no check for
them, because *absence* of a check is itself the behaviour the Rust must match.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `use_generated` | `n == 7` (first value past the `switch`'s `case 6`) | `switch` takes `default:` → `acc` stays `INIT_FOR(OP)` (`0` for add/sub, `1` for mul); prints `gen.acc=<INIT>`; returns `INIT` | `err_01_use_generated_n_eq_7` |
| 2 | `use_generated` | `n > 7` (`8`, `9`, `100`, `INT_MAX`) | same `default:` arm → returns `INIT_FOR(OP)` | `err_02_use_generated_n_gt_7` |
| 3 | `use_generated` | `n == -1` (first value below `case 0`) | same `default:` arm → returns `INIT_FOR(OP)` | `err_03_use_generated_n_neg_1` |
| 4 | `use_generated` | `n < -1` (`-2`, `-100`, `INT_MIN`) | same `default:` arm → returns `INIT_FOR(OP)` | `err_04_use_generated_n_very_negative` |
| 5 | `use_generated` | every in-range value `n ∈ {0,1,2,3,4,5,6}` (the exhaustive `case` list; boundary check that 0 and 6 are *not* rejected) | `REP<n>` runs steps `i = 0 … n-1`; returns the accumulated value, **not** `INIT` (except `n == 0`, where `REP0` is empty) | `err_05_use_generated_in_range_not_rejected` |
| 6 | `use_generated` | `n` is an out-of-range "enum-like" `int` passed across the FFI boundary (the C `switch` accepts any `int`): `INT_MIN`, `INT_MIN+1`, `-2^31 … 2^31-1` randomized | no variant matches → `default:` → `INIT_FOR(OP)`; must never panic, never wrap into a valid case | `err_06_use_generated_ffi_fuzz_all_int` |
| 7 | `op_add` | signed-overflow inputs (`INT_MAX + 1`, `INT_MIN + (-1)`, `INT_MAX+INT_MAX`) — C has no check, gcc `-O2` two's-complement wraps | wrapped `int` result, no trap | `err_07_op_add_overflow` |
| 8 | `op_sub` | signed-overflow inputs (`INT_MIN - 1`, `INT_MIN - INT_MAX`) | wrapped `int` result, no trap | `err_08_op_sub_overflow` |
| 9 | `op_mul` | signed-overflow inputs (`INT_MAX * INT_MAX`, `INT_MIN * -1`, `INT_MIN * INT_MIN`) | wrapped `int` result, no trap | `err_09_op_mul_overflow` |
| 10 | `helper_call` | `a`/`b` at the `int` extremes, so the internal `OP_FN(OP)(a,b)` overflows **and** the `r + acc` return overflows | wrapped results in both the `printf` and the return value | `err_10_helper_call_overflow` |
| 11 | `helper_ptr` | `a`/`b` at the `int` extremes (overflow inside the indirect call) | wrapped result, no trap | `err_11_helper_ptr_overflow` |
| 12 | `helper_call` / `helper_ptr` / `use_generated` (`OP=mul`, `REPEAT>=7`) | accumulator overflow inside the unrolled `STEP_mul` chain (`1*1*2*…*7`, and `acc *= (i+1)` on already-huge values) | wrapped `int` accumulator | `err_12_mul_accumulator_overflow` |
| 13 | `helper_ptr` | the *writable* global `G_OP` is overwritten (e.g. with `op_mul`) before the call — `helper_ptr` uses `OP_FN(OP)` **directly**, not `G_OP` | result is unaffected by the `G_OP` write; still uses the build-selected op | `err_13_g_op_write_does_not_affect_helper_ptr` |
| 14 | `G_OP` | the function pointer is read through `dlsym` and invoked with extreme/overflowing args | identical wrapped result as calling `op_<OP>` directly | `err_14_g_op_pointer_overflow` |
| 15 | `G_OP_NAME` | the exported `const char *` is dereferenced as a NUL-terminated C string | exactly `"add"` / `"sub"` / `"mul"` (3 bytes + NUL) for the selected `OP`; pointer is non-NULL | `err_15_g_op_name_string` |
| 16 | `main` (`driver`) | `argc < 3`: no arguments at all | writes `usage: <argv0> A B\n` to **stderr**, nothing to stdout, exit status **2** | `err_16_main_no_args` |
| 17 | `main` (`driver`) | `argc < 3`: exactly one argument | same: usage on stderr, exit status **2** | `err_17_main_one_arg` |
| 18 | `main` (`driver`) | `argc > 3`: extra arguments (`A B C D`) — there is no upper-bound check | extra argv entries are ignored; behaves exactly as `A B`; exit status **0** | `err_18_main_extra_args_ignored` |
| 19 | `main` (`driver`) | un-parsable numeric arguments (`""`, `"abc"`, `"+"`, `"-"`, `"12abc"`, `" 7 "`, `"0x10"`) — `atoi` has no error report | `atoi` yields `0` / the leading-digit prefix; program still exits **0** | `err_19_main_atoi_unparsable` |
| 20 | `main` (`driver`) | numeric arguments that overflow `int`/`long` (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) | glibc `atoi` = `(int)strtol(...)`: clamp to `LONG_MIN`/`LONG_MAX` then truncate to `int`; exit **0** | `err_20_main_atoi_overflow` |

### Null pointers

The library exposes no pointer parameters at all — every function has the
signature `int f(int, int)` or `int f(int)`, and the two exported globals are
data, not callbacks invoked with caller data. There is therefore no
null-pointer row to construct: the only pointers in the API surface are the
*values* of `G_OP` / `G_OP_NAME`, covered by rows 13–15. `main`'s `argv` is
always supplied by the loader/`Command`.

### Zero / oversized lengths

There are no length or buffer parameters anywhere in the API, so the
length-boundary class collapses onto the integer-boundary rows (5, 6, 7–12).

## Status

All 20 rows have a passing differential test (`tests/errors.rs`,
`err_01` … `err_20`), verified under **all 36 build configurations**
(`../run_all.sh` → 36/36 PASS).

### Divergence found by this phase

Row 13 uncovered two real bugs in the Rust translation:

1. **`helper_ptr` read the wrong thing.** The C is
   `int (*fp)(int,int) = OP_FN(OP);` — a *direct* token-pasted reference to
   `op_<OP>`. The Rust read the mutable global `G_OP` instead, so once a caller
   overwrote `G_OP` (legal: it is a non-`const` C global) `helper_ptr` changed
   behaviour in Rust but not in C. Fixed to use `OP_FN_SELECTED`.
2. **`G_OP` / `G_OP_NAME` were not writable.** Writing the exported `G_OP`
   worked against the C `.so` but `SIGSEGV`ed against the Rust one, because a
   plain Rust `static` with a relocated initializer lands in read-only
   `.data.rel.ro` while gcc puts the C globals in `.data`. Fixed by using
   `static mut`. See `SYMBOLS.md` for the `readelf` evidence.

### Harness sensitivity (negative control)

To prove the suite is not vacuous, the Rust `mul,7` build was run against the C
`sub/3` `.so`: **34 assertions failed**, i.e. the tests do detect divergence.
