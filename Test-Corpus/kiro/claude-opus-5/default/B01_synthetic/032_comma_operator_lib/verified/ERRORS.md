# ERRORS.md — Phase A: error-surface table

## Mechanical derivation

Every rejection / error construct in the C source was located by grepping the
complete C source tree (`c_src/src/driver.c`, `c_src/include/driver.h`) for
every rejection idiom:

```sh
grep -nE 'return|RETURN|assert|NULL|errno|exit\(|abort|if *\(|switch|#if|<|>|==|!=|min|max|INT_|SIZE_|malloc|free' \
    src/driver.c include/driver.h
```

Non-comment hits — the complete list:

| file:line | text | classification |
|---|---|---|
| `src/driver.c:26` | `#include <stdio.h>` | not a check |
| `src/driver.c:29` | `for (int i = 0, j = 0; i < x; i++, j += 2)` | **the only conditional in the library** |
| `include/driver.h:24` | `#ifndef DRIVER_H_` | include guard, not a check |

Therefore, mechanically:

* error-return macros (`RETURN_ERROR`, …): **0**
* `return -1` / `return NULL` / error enums: **0** — `driver` returns `void`,
  so the library has **no error channel whatsoever**
* `assert`: **0**
* explicit range checks: **1** (the loop guard `i < x`)
* null checks: **0** — the API takes no pointers
* min/max constants: **0** — no `INT_MAX`/`SIZE_MAX`/limit constants are used
* allocation (a failure source): **0** — no `malloc`/`calloc`/`realloc`

The library's *only* input-rejection behaviour is the loop guard: a value of
`x` that fails `0 < x` produces zero iterations and therefore **zero output
bytes**. "Rejection" for this library means "emits nothing"; there is no
return code or sentinel to compare, so each row's expected result is the exact
observable side effect (stdout bytes) plus normal (non-trapping) return.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | `x == 0` — zero "length"; loop guard `0 < 0` false on first evaluation | returns normally, writes 0 bytes to stdout | `err_e1_zero` | [x] |
| E2 | `driver` | `x == -1` — one step past the bottom of the productive range | returns normally, writes 0 bytes | `err_e2_minus_one` | [x] |
| E3 | `driver` | `x < 0`, arbitrary negative values (property-style sweep) | returns normally, writes 0 bytes | `err_e3_negative_sweep` | [x] |
| E4 | `driver` | `x == INT_MIN` (`-2147483648`) — extreme underflow boundary of the `int` parameter | returns normally, writes 0 bytes; no trap on the negation-free comparison | `err_e4_int_min` | [x] |
| E5 | `driver` | `x == INT_MIN + 1` — one step past the `INT_MIN` boundary | returns normally, writes 0 bytes | `err_e5_int_min_plus_one` | [x] |
| E6 | `driver` | out-of-range "enum-like" integer passed across FFI: the C prototype is `int`, so *any* 32-bit pattern is a legal argument with no valid-variant restriction. Sweep of `int` bit patterns that are not sensible counts (`0x80000000`, `0xFFFFFFFF`, `0xDEADBEEF`, `0x80000001`, `0xC0000000` reinterpreted as `i32`) | each is either negative (0 bytes) or a plain count; C never validates, never traps | `err_e6_out_of_range_int_patterns` | [x] |
| E7 | `driver` | argument passed as a 64-bit value whose high half is garbage (caller pushes `i64`, callee reads `int`): tests that the Rust `extern "C"` ABI truncates to 32 bits exactly like the C callee does | both ignore the upper 32 bits identically | `err_e7_dirty_upper_half` | [x] |
| E8 | `driver` | repeated invocation after a rejection (`x<=0` then `x>0`) — checks no error state / no leaked static state is retained | second call produces the normal full output | `err_e8_no_sticky_state` | [x] |

### Not applicable (documented so the absence is deliberate, not an oversight)

| construct | why N/A |
|---|---|
| null pointer arguments | `driver` has no pointer parameters and no pointer return; there is no pointer to null out. |
| oversized length | `x` is bounded by the `int` type itself; every representable value is accepted. `x == INT_MAX` is *valid* (see `CONFIGS.md` row C10) rather than an error, and is verified over an identical 64 MiB prefix of both output streams. |
| error code / sentinel comparison | the function returns `void`; there is no code or sentinel. Equivalence is asserted on the full stdout byte stream and on normal return. |
| `errno` | never set or read by the C. |
| out-of-range enum variant | there is no `enum` in the public API; the closest analogue (arbitrary `int` bit patterns) is covered by E6. |

## Gate

- [x] All 8 rows have a passing differential test against both `.so` files.
