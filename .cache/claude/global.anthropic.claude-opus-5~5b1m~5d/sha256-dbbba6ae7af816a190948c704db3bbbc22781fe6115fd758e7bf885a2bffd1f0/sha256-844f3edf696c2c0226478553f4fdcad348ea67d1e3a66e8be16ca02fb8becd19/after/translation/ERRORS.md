# ERRORS.md — Error / rejection surface of the C source

Derived mechanically by grepping **all** of `c_src/src/*.c` and `c_src/src/*.h`
for every `return`, `assert`, `NULL` check, explicit range check, `switch`
`default`, error enum, min/max constant and `#ifndef` fallback:

```sh
grep -n 'return\|assert\|NULL\|default\|case\|if *(\|#ifndef\|#ifdef\|exit\|stderr' c_src/src/*.c c_src/src/*.h
```

The library is deliberately tiny: **`mdcore.c` contains no `assert`, no `NULL`
check, no error enum and no error return at all** — every one of its functions
is total over `int`. The complete set of rejection / fallback behaviours in the
program is therefore the table below. Rows 1–2 live in `mdmain.c` (the `driver`
executable); rows 3–9 are the `DISPATCH_REP` `default: break` path plus the
generic FFI boundaries required by the task; rows 10–12 are the preprocessor
fallbacks in `mdmacros.h`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|----|----------|---------------------------------------------|-------------------|---|
| 1  | `main` (`mdmain.c:29-32`) | `argc < 3` — invoked with 0 or 1 operand (e.g. `driver`, `driver 7`) | writes `usage: <argv[0]> A B\n` to **stderr**, nothing to stdout, process exit status **2** | [x] |
| 2  | `main` (`mdmain.c:33-34`) | operand strings that are not valid integers — `atoi` has no error channel (`"abc"`, `""`, `"  12x"`, `"+8"`, `"99999999999999999999"`) | `atoi` silently yields `0` / the leading-digit prefix / the truncated `(int)` of an out-of-range `strtol` result; **no error is reported**, exit status **0** | [x] |
| 3  | `use_generated` → `accum_<OP>` → `DISPATCH_REP` (`mdmacros.h:91`) | `n < 0` (e.g. `-1`) — no matching `case`, falls to `default: break` | accumulator is left at `INIT_FOR(OP)`; returns `0` for `add`/`sub`, `1` for `mul`; prints `gen.acc=<INIT>\n` | [x] |
| 4  | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n > 6` (e.g. `7`, `8`, `100`) — **note `REP7` exists but is *not* a `switch` case**, so `REPEAT=7` builds still reject `n == 7` here | returns `INIT_FOR(OP)` (`0`/`0`/`1`); prints `gen.acc=<INIT>\n` | [x] |
| 5  | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n == INT_MIN` (`-2147483648`) — extreme out-of-range | returns `INIT_FOR(OP)`; no trap, no overflow | [x] |
| 6  | `use_generated` → `DISPATCH_REP` (`mdmacros.h:91`) | `n == INT_MAX` (`2147483647`) — extreme out-of-range | returns `INIT_FOR(OP)`; no trap, no overflow | [x] |
| 7  | `use_generated` | `n == 6` and `n == 0` — the two in-range **boundary** cases, one step inside the valid `switch` range | `n=0` → `INIT`; `n=6` → `REP6` applied (`add`:15, `sub`:−15, `mul`:720) | [x] |
| 8  | `op_add` / `op_sub` / `op_mul`, `helper_call`, `helper_ptr`, `G_OP` | signed-integer **overflow** operands: `INT_MAX+1`, `INT_MIN-1`, `INT_MIN * -1`, `INT_MAX*INT_MAX` — C signed overflow is UB and there is **no** range check | the emitted code is a bare `add`/`sub`/`imul`, i.e. two's-complement wrap-around; **no error, no trap** | [x] |
| 9  | every exported entry point (`op_*`, `helper_*`, `use_generated`) | out-of-range "enum-like" ints passed across the FFI boundary. The API has **no `enum` parameter** — the only integer with a restricted valid domain is `use_generated`'s `n` (`switch` domain `0..=6`), covered by rows 3–7. There is likewise **no pointer parameter anywhere**, so no null-pointer row exists; `G_OP_NAME` is the only pointer and it is an *output* that must be non-null and point at `STR(OP)` | see rows 3–7; `G_OP` / `G_OP_NAME` slots are always non-null | [x] |
| 10 | `mdmacros.h:27-29` | `OP` not defined on the command line | `#define OP add` — the build silently falls back to the `add` family | [x] |
| 11 | `mdmacros.h:30-32` | `REPEAT` not defined on the command line | `#define REPEAT 5` — silently falls back to 5 | [x] |
| 12 | `mdmacros.h:73-79` (`CHOOSE_REP`) | `REPEAT` outside `0..=7` (e.g. `-1`, `8`) | `REP<n>` does not exist → **translation-unit compile error** ("`REP8` undeclared"), not a runtime error. The Rust mirror expresses the same restriction by only offering features `"0"`..`"7"`, so the invalid value is unrepresentable | [x] |

## Notes on rows the C does *not* have

* No `RETURN_ERROR`-style macro, no `errno` use, no negative sentinel return, no
  `NULL` return: the grep above finds **zero** such sites in `mdcore.c`.
* `printf` return values are ignored in all three helpers, so a write failure is
  not an error path in the C either — the Rust translation likewise ignores its
  `write_all`/`flush` results (`let _ = ...`), which is required for parity.

## Where each row is tested

`tests/error_paths.rs`, one test per row (`err01_main_argc_less_than_3` …
`err12_repeat_out_of_range_is_rejected_at_build_time`). Each asserts the *same*
concrete result on both sides — the same exit status and the same stderr text
for the `main` rows, and the exact `INIT_FOR(OP)` sentinel (not merely "both
returned something") for the `DISPATCH_REP` `default:` rows.

Result: **12/12 rows pass, for all 24 canonical configurations and all 14
degenerate feature sets** (`bash scripts/test_all_features.sh [degenerate]`).
