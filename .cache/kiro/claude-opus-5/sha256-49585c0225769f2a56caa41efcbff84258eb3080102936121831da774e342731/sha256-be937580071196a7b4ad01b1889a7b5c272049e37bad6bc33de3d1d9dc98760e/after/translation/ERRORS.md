# ERRORS.md — error / rejection surface of the C library

Derived mechanically. Every `return`, `if`, `switch`/`default`, `assert`, range
check, null check and min/max constant in `c_src/src/*` was enumerated:

```
awk 'NR>23' c_src/src/{mdcore.c,mdmain.c,mdmacros.h} \
  | grep -nE 'return|assert|if *\(|NULL|default|switch|case|ERROR|-1|exit|stderr|<|>|MAX|MIN'
```

Findings from that grep, in full:

* `mdcore.c` — **no** `assert`, **no** `NULL` check, **no** range check, **no**
  error enum, **no** error-return macro. Every function is
  `int f(int, int)` / `int f(int)` and unconditionally returns a value. There is
  no pointer parameter anywhere in the public API, so there is no null-pointer
  rejection path to mirror.
* `mdmacros.h` — the only rejection construct in the whole library is
  `DISPATCH_REP`'s `default: break;` (line 68). It is a *silent* rejection: the
  accumulator is left at `INIT_FOR(op)` and returned.
* `mdmain.c` — one guard, `if (argc < 3)` → `fprintf(stderr, "usage: %s A B\n", argv[0])`
  and `return 2`. Argument parsing goes through `atoi(3)`, which reports no
  errors at all and has its own silent-failure/saturation behaviour.

So the "error surface" of this library is entirely made of *silent* rejections
plus one `exit(2)` path. Each distinct one is a row below.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `use_generated(n)` | `n == 7` — `REP7` exists in the header, but `DISPATCH_REP` has no `case 7`, so it falls to `default: break;` | returns `INIT_FOR(OP)` = `0` for `add`/`sub`, `1` for `mul`; prints `gen.acc=<INIT>\n`. Note `REPEAT=7` is a *valid build*, so the default driver call `use_generated(REPEAT)` hits this row. |
| E2 | `use_generated(n)` | `n == 8` (first value past the last `case`) | returns `INIT_FOR(OP)`; prints `gen.acc=<INIT>\n` |
| E3 | `use_generated(n)` | `n` large positive, e.g. `9, 10, 100, 1000` | returns `INIT_FOR(OP)` |
| E4 | `use_generated(n)` | `n == INT_MAX` (max representable, far past range) | returns `INIT_FOR(OP)` |
| E5 | `use_generated(n)` | `n == -1` (one step below the first `case 0`) | returns `INIT_FOR(OP)` |
| E6 | `use_generated(n)` | `n` other negatives, e.g. `-2, -7, -100` | returns `INIT_FOR(OP)` |
| E7 | `use_generated(n)` | `n == INT_MIN` (min representable) | returns `INIT_FOR(OP)` |
| E8 | `use_generated(n)` | out-of-range "enum-like" selector across FFI: `n` is a plain `int` used as a `switch` selector, so any of the 2^32 − 7 values with no matching `case` is a real input a C caller can pass. Sampled randomly over the whole `i32` range with a fixed seed, plus every value in `-64..=64`. | returns `INIT_FOR(OP)` for every `n` outside `0..=6` |
| E9 | `main` (driver) | `argc == 1` (no operands) → `argc < 3` | `usage: <argv[0]> A B\n` on **stderr**, empty stdout, exit status `2` |
| E10 | `main` (driver) | `argc == 2` (exactly one operand, incl. the empty string `""`) → `argc < 3` | `usage: <argv[0]> A B\n` on stderr, exit status `2` |
| E11 | `main` (driver) | `argc == 0` (`execve(path, {NULL}, {NULL})`) → `argc < 3`, and `argv[0]` is not a usable program name | measured on this platform: `usage:  A B\n` (empty program name, two spaces) on stderr, exit status `2`. Verified with `scratch/argv0.c`; glibc does **not** print `(null)` here. |
| E12 | `atoi` via `main` | operand is entirely non-numeric (`"abc"`, `"--5"`, `"+"`, `"-"`, `"x1"`) — `strtol` converts nothing | value `0`, **no** error reported; the program continues normally and exits `0` |
| E13 | `atoi` via `main` | operand has a numeric prefix then garbage (`"12x"`, `"34y"`, `"0x10"`, `"010"`) — conversion stops at the first non-digit, base is 10 | the leading decimal digits only (`12`, `34`, `0`, `10`); no error |
| E14 | `atoi` via `main` | operand overflows `long` upward (`"9999999999999999999999"`) — glibc `atoi` is `(int)strtol(...)`, `strtol` saturates to `LONG_MAX` and sets `ERANGE`, which `atoi` discards | `(int)LONG_MAX` = `-1`; no error |
| E15 | `atoi` via `main` | operand overflows `long` downward (`"-9999999999999999999999"`) → saturates to `LONG_MIN` | `(int)LONG_MIN` = `0`; no error |
| E16 | `atoi` via `main` | operand fits in `long` but not in `int` (`"2147483648"`, `"-2147483649"`, `"4294967296"`) | silent truncation of the `long` to `int`: `-2147483648`, `2147483647`, `0` |
| E17 | `atoi` via `main` | leading whitespace / explicit sign forms (`"  12"`, `"\t-3"`, `"+5"`, `"-0"`) — `strtol` skips `isspace` then accepts one optional sign | parsed value, no error |
| E18 | `op_add` / `helper_call` / `G_OP` (`add` build) | signed overflow: `INT_MAX + 1`, `INT_MIN + (-1)`, `INT_MAX + INT_MAX`. UB in C; gcc -O2 emits a plain `add` → two's-complement wrap. No check, no rejection. | wrapped result (`INT_MIN`, `INT_MAX`, `-2`) |
| E19 | `op_sub` / `helper_call` / `G_OP` (`sub` build) | signed overflow: `INT_MIN - 1`, `INT_MAX - INT_MIN` | wrapped result (`INT_MAX`, `-1`) |
| E20 | `op_mul` / `helper_call` / `G_OP` (`mul` build) | signed overflow: `INT_MAX * 2`, `INT_MIN * -1`, `123456789 * 123456789` | wrapped low 32 bits (`-2`, `INT_MIN`, `-1757895751`) |
| E21 | `helper_call` | return value `r + acc` overflows (e.g. `a=INT_MAX, b=0` with `REPEAT=5`, `add`: `INT_MAX + 10`) | wrapped result; no check |
| E22 | `G_OP` / `G_OP_NAME` | consumer **assigns** to the exported global (both are declared non-`const` in `mdmacros.h`, and `nm` reports them in writable `.data`) | the store succeeds; a subsequent read/call observes the new value. This is a rejection-adjacent boundary because the *wrong* translation turns it into a `SIGSEGV` (RELRO). |

## Rows deliberately absent

* No null-pointer row for `op_*`/`helper_*`/`use_generated`: none of them takes a
  pointer. The only pointers in the API are the two exported data objects, and
  those are covered by E21 (plus a read of `*G_OP_NAME`, which is never `NULL`
  in either implementation because it is initialised from a string literal at
  load time).
* No zero-length / oversized-buffer rows: the API has no buffer, length or count
  parameter.
* No real `enum` type exists in the API; E8 covers the analogous
  "int used as a bounded selector, called with an out-of-range value" case,
  which is where `DISPATCH_REP`'s `default:` lives.
* `REPEAT` outside `0..=7` and `OP` outside `{add, sub, mul}` are not runtime
  errors but *compile* errors (`CHOOSE_REP(n)` / `CAT(op_, op)` would name an
  undefined `REPn` / `op_<x>`), so they are configuration constraints, recorded
  in `CONFIGS.md`, not rows here.

## Row → test mapping (all rows PASS)

Each row has a differential test in `tests/phase_c_errors.rs` that constructs the
exact condition, calls **both** implementations through their `.so` exports (or
runs both `driver` executables), and asserts the same sentinel / exit status /
bytes — not merely "both failed somehow". Every row passes in all 50 build
configurations driven by `../run_all_features.sh`.

| # | test | assertion |
|---|------|-----------|
| E1 | `e1_use_generated_n_7` | returns `INIT_FOR(OP)` and prints `gen.acc=<INIT>` | 
| E2 | `e2_use_generated_n_8` | same sentinel |
| E3 | `e3_use_generated_large_positive` | `9,10,11,16,32,100,1000,65536,1000000` |
| E4 | `e4_use_generated_int_max` | `INT_MAX`, `INT_MAX-1` |
| E5 | `e5_use_generated_minus_one` | `-1` |
| E6 | `e6_use_generated_negatives` | `-2,-3,-6,-7,-8,-100,-65536,-1000000` |
| E7 | `e7_use_generated_int_min` | `INT_MIN`, `INT_MIN+1` |
| E8 | `e8_use_generated_out_of_range_selector_exhaustive_and_random` | every `n` in `-64..=64` **plus** 4096 seeded full-`i32` draws; return value and the full `gen.acc=` transcript compared per library (C and Rust captured in separate fd-1 redirects, because glibc block-buffers `stdout` while the Rust `.so` line-buffers it) |
| E9 | `e9_driver_argc_1_no_operands` | exit `2`, `usage: ./driver A B\n` on stderr, empty stdout |
| E10 | `e10_driver_argc_2_one_operand` | exit `2` for `"5"`, `""`, `"abc"`, `"-2147483648"` |
| E11 | `e11_driver_argc_0` | `execve(prog, {NULL}, {NULL})` via a generated launcher, and `arg0("")`; identical stderr and exit `2` |
| E12 | `e12_atoi_non_numeric` | 17 unconvertible operands each equal to `0`; four all-unconvertible pairs equal to `0 0`; exit `0` |
| E13 | `e13_atoi_numeric_prefix` | `12x→12`, `34y→34`, `0x10→0`, `010→10`, `9e9→9`, `-5-6→-5`, `7 8→7` |
| E14/E15 | `e14_e15_atoi_long_overflow_saturates` | `(int)LONG_MAX == -1`, `(int)LONG_MIN == 0` |
| E16 | `e16_atoi_int_truncation` | `2147483648→INT_MIN`, `-2147483649→INT_MAX`, `4294967296→0`, `4294967297→1`, `9223372036854775807→-1` |
| E17 | `e17_atoi_whitespace_and_sign` | leading `\t\n\r\v\f`/spaces, `+`/`-`, leading zeros |
| E18/E19/E20 | `e18_e19_e20_op_overflow_wraps_identically` | all three primitives over the overflow + boundary sets |
| E21 | `e21_helper_call_return_overflow` | `helper_call`/`helper_ptr`/`G_OP` over the same sets |
| E22 | `e22_exported_globals_are_writable_and_non_null` | both slots non-NULL, `G_OP_NAME`'s value non-NULL, and stores into both succeed then restore |
| — | `generic_all_symbols_resolve_in_both` | `dlsym` succeeds for all 8 exports in both `.so`s, and **fails** for `accum_add`/`accum_sub`/`accum_mul`/`main` in both |
| — | `generic_calls_are_stateless` | 9 repeated invocations give identical results (the C `acc` is a local, re-initialised each call) |

### Negative controls (the suite was proven to have teeth)

Three bugs were injected into the Rust and the suite re-run; each was caught,
then reverted:

| injected bug | caught by |
|--------------|-----------|
| `op_mul` returns `a*b + 1` | 8 failures: `g1_op_mul`, `g2_g_op_call`, `g4_helper_ptr`, `g5_helper_call`, `g7_mutating_g_op_does_not_affect_helpers`, `g8_driver_stdout_matches`, `g8_driver_reports_active_config`, `g9_unset_macros_equal_add_repeat_5` |
| `accum`'s `default` arm sets `acc = 0` instead of leaving `INIT_FOR(OP)` (only observable for `mul`) | 8 failures: `e1`..`e8` |
| `G_OP` reverted to an immutable `static` (`.data.rel.ro` / RELRO) | `phase_b_valid` aborts with `signal: 11, SIGSEGV` — exactly what a real consumer sees, and what the C `.so` does not do |
