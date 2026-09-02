# ERRORS.md — the error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/{mdcore.c,mdmacros.h,mdmain.c}`.

## What the grep actually found

```
$ grep -niE 'assert|NULL|errno|error|EINVAL|exit|abort' c_src/src/*.c c_src/src/*.h
(no matches)

$ grep -n 'return' c_src/src/*.c c_src/src/*.h
mdcore.c:28: int op_add(...){ return a + b; }
mdcore.c:29: int op_sub(...){ return a - b; }
mdcore.c:30: int op_mul(...){ return a * b; }
mdcore.c:44:     return r + acc;
mdcore.c:51:     return r;
mdcore.c:57:     return r;
mdmain.c:31:         return 2;      <-- the ONLY error return in the codebase
mdmain.c:47:     return 0;
mdmacros.h:99:   return acc;

$ grep -nE 'if *\(|switch|case |default|#ifndef' c_src/src/*.c c_src/src/*.h
mdmain.c:29:     if (argc < 3) {           <-- the only runtime `if`
mdmacros.h:27:   #ifndef OP                <-- build-time fallback
mdmacros.h:30:   #ifndef REPEAT            <-- build-time fallback
mdmacros.h:83:   switch (n) {              <-- DISPATCH_REP
mdmacros.h:84-90:  case 0: .. case 6:
mdmacros.h:91:   default: break;           <-- out-of-range dispatch
```

So the library has **no** `assert`, **no** `RETURN_ERROR`-style macro, **no**
error enum, **no** `errno` use, **no** `-1`/`NULL` sentinel and **no** pointer
parameter to null-check. Every function in the `.so` takes and returns `int`
only and is total over its domain. The rejection surface is therefore
exclusively: the `argc` check, the `switch` `default:` arm, `atoi`'s silent
rejections, signed-overflow wrap-around, and the build-time macro range limits.
`MIN`/`MAX`/`LIMIT`/`<limits.h>` do not appear at all (only in the licence
comment) — the implicit constants are `INT_MIN`/`INT_MAX` and the
`REP0..REP7` / `case 0..6` ranges.

## Table

| # | function (source) | trigger (exact invalid input/condition) | expected C result | test |
|---|-------------------|------------------------------------------|-------------------|------|
| E-01 | `main` — `mdmain.c:29-32` | `argc < 3` (invoked with 0 or 1 operand) | `fprintf(stderr, "usage: %s A B\n", argv[0])`, **nothing** on stdout, `return 2` | `error_paths.rs::e01_usage_error_for_too_few_arguments`, `driver_parity.rs::m_driver_usage_path` |
| E-02 | `use_generated` → `accum_<OP>` `DISPATCH_REP` `default:` — `mdmacros.h:91` | `n == 7` — one past the last `case 6:` (note `REP7` *exists* but `case 7:` does **not**) | no step runs; prints `gen.acc=<INIT_FOR(OP)>`, returns `INIT_FOR(OP)` (0 for add/sub, 1 for mul) | `e02_to_e07_dispatch_default_arm`, `e02_e04_one_past_each_end_of_switch_range`, `u_use_generated_at_repeat` |
| E-03 | same | `n == 8`, `9`, `100` — further above the range | same as E-02 | `e02_to_e07_dispatch_default_arm` |
| E-04 | same | `n == -1` — one below `case 0:` | same as E-02 | `e02_e04_one_past_each_end_of_switch_range` |
| E-05 | same | `n == INT_MIN`, `INT_MIN + 1` | same as E-02 | `e05_e06_extreme_dispatch_values` |
| E-06 | same | `n == INT_MAX`, `INT_MAX - 1` | same as E-02 | `e05_e06_extreme_dispatch_values` |
| E-07 | same | arbitrary `int` outside `0..=6` (2048 randomized values, fixed seed) — the C `switch` accepts *any* `int`, so an out-of-domain value is a real FFI input, exactly the out-of-range-enum class | same as E-02 | `e05_e06_extreme_dispatch_values`, `u_use_generated_randomized` |
| E-08 | `atoi` — `mdmain.c:33-34` | operand with no digits at all (`"abc"`, `""`, `"+"`, `"-"`, `"   "`, `"0x10"`, `"1e3"` past the `1`) | returns `0`, **no diagnostic**, exit status still 0 | `e08_e10_atoi_rejections_are_silent_and_identical` |
| E-09 | `atoi` | numeric prefix followed by garbage (`"12x"`, `"  -12abc"`, `"1.5"`, `"010"`) | parses the prefix, silently discards the rest | same |
| E-10 | `atoi` | magnitude beyond `long` (`"99999999999999999999"`, `"9223372036854775808"`, `"-9223372036854775809"`) | glibc `atoi` is `(int)strtol`, which saturates to `LONG_MAX`/`LONG_MIN` and is then truncated to `int` → `-1` / `0`. Also `"2147483648"` → `-2147483648`, `"4294967296"` → `0` | same |
| E-11 | `G_OP`, `G_OP_NAME` objects — `mdcore.c:36-37` | an external consumer **stores** into the global (both are mutable objects: `const` in `const char *G_OP_NAME` binds to the pointee, not to the pointer) | the store succeeds and is observable — the objects sit in writable `.data`, *outside* `PT_GNU_RELRO` | `e11_g_op_is_writable_data`, `g_op_slot_is_writable_and_helpers_ignore_it` |
| E-12 | `op_add` — `mdcore.c:28` | signed overflow: `INT_MAX + 1`, `INT_MAX + INT_MAX`, `INT_MIN + -1`, `INT_MIN + INT_MIN` | UB in ISO C; `gcc -O2` emits `add` → two's-complement wrap | `e12_op_add_overflow_wraps` |
| E-13 | `op_sub` — `mdcore.c:29` | `INT_MIN - 1`, `INT_MAX - INT_MIN`, `0 - INT_MIN` | wraps (`sub`) | `e13_op_sub_overflow_wraps` |
| E-14 | `op_mul` — `mdcore.c:30` | `INT_MIN * -1`, `INT_MAX * INT_MAX`, `46341 * 46341` (first overflowing square), `65536 * 65536` | wraps (low 32 bits of `imul`) | `e14_op_mul_overflow_wraps` |
| E-15 | `helper_call` — `mdcore.c:44` | `return r + acc;` overflows after `r` already wrapped (`a = b = INT_MAX` with `OP=add`) | wraps; the printed `helper.call=` / `helper.acc=` fields show the wrapped values | `e15_helper_call_return_overflow_wraps` |
| E-16 | `CHOOSE_REP(n)` — `mdmacros.h:73-74`, used by `RUN_LOOP` at `mdcore.c:42` and `mdmain.c:38` | build-time: `-DREPEAT=8` (or any value ∉ `0..7`) pastes `REP8`, which is not defined | **compile error** — `REP8` undeclared. Valid range is exactly `REP0..REP7` | `e16_e17_out_of_range_build_configurations_do_not_exist` (the crate offers no `repeat_8`/`"8"` feature, so `cargo` rejects it the same way) |
| E-17 | `#ifndef OP` — `mdmacros.h:27-29`, `INIT_FOR`/`STEP_OP`/`OP_FN` | build-time: `-DOP=div` — no `INIT_div`, `STEP_div` or `op_div` exists | **compile error**. `OP` undefined instead falls back to `add` (not an error) | `e16_e17_out_of_range_build_configurations_do_not_exist`; the `add` fallback is exercised by the `<no OP>` combinations in `run_all.sh` |
| E-18 | whole `.so` surface | null pointer / oversized length arguments | **not reachable**: no function in `mdcore.c` has a pointer or length parameter, so there is no null-check or bounds path to diverge on. Recorded so the absence is explicit. The pointer-shaped part of the surface — the two exported data objects — must be non-null and dereferenceable in both libraries | `e18_no_pointer_parameters_but_data_objects_are_valid` |

## Status

All 18 rows have a passing differential test, under **every** feature
combination (`./run_all.sh` → 49 combinations × 31 tests).

- [x] E-01  - [x] E-02  - [x] E-03  - [x] E-04  - [x] E-05  - [x] E-06
- [x] E-07  - [x] E-08  - [x] E-09  - [x] E-10  - [x] E-11  - [x] E-12
- [x] E-13  - [x] E-14  - [x] E-15  - [x] E-16  - [x] E-17  - [x] E-18

## Divergence found and fixed

**E-11.** The original translation declared

```rust
pub static G_OP: OpFn = mdmacros::OP_FN;
pub static G_OP_NAME: CStrPtr = CStrPtr(...);
```

An immutable Rust `static` whose initialiser needs a relocation is emitted into
`.data.rel.ro`, which the loader `mprotect`s **read-only** once relocations are
applied:

```
Rust (before): G_OP @ 0x4cd08 in [19] .data.rel.ro, GNU_RELRO = 0x4cce0 +0x2320  -> INSIDE
C:             G_OP @ 0x4020  in [23] .data,        GNU_RELRO = 0x3df0  +0x210   -> outside
```

The C globals are mutable objects, so a consumer store is legal and works;
against the Rust `.so` the identical store `SIGSEGV`s. Changing both to
`static mut` reproduces the C storage class and moves them back into writable
`.data` outside `PT_GNU_RELRO`. Confirmed by mutation testing: reverting to the
plain `static` makes `e11_g_op_is_writable_data` die with
`signal: 11, SIGSEGV: invalid memory reference`.
