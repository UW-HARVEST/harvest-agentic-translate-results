# ERRORS.md — Error-surface table (Phase A → gates Phase C)

## Mechanical derivation

Every error-shaped construct was grepped for across **all** C sources
(`c_src/src`, `c_src/include`):

```
grep -nE 'RETURN_ERROR|return +-|return +NULL|assert|abort|exit\(|errno|E[A-Z]{2,}|
          malloc|free|NULL|\*|\[|enum|#define|#if|MIN|MAX|LIMIT' -r src include
```

Result: **no matches** other than three incidental hits on the `*` character
inside the multiplications on lines 30/36/42.

```
grep -nE 'return' -r src include
  src/lib.c:61:    return (uni);          <-- the only return in the library
```

## Conclusion: the C error surface is EMPTY

`encode_quant` is a **total function on its six `int` arguments**. Concretely,
the C source contains:

* 0 error-return macros / `RETURN_ERROR` / `return -1` / `return NULL`
* 0 `assert` / `abort` / `exit` / `errno` uses
* 0 error enums or status codes (return type is a *value*, `int`, with no
  reserved sentinel — every `int` is a legal result)
* 0 pointer parameters, 0 array indexing, 0 allocation → **no null check and no
  bounds check to mismatch**
* 0 explicit range checks and 0 `MIN`/`MAX`/limit constants
* 0 division by a caller-controlled value (both divisions are the constant `/ 8`,
  so `SIGFPE` is unreachable)

There is therefore **no row of the form "invalid input → error result"**: no
input is rejected. Inventing rows here would be guessing, which the task
forbids. The table below instead enumerates, one row per construct, every
*potential* failure/rejection site found by the grep, together with what the C
actually does — which is "accepts and computes".

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `encode_quant` | null pointer argument | **N/A — unrepresentable.** Signature takes six `int` by value; there is no pointer parameter to nullify. |
| E2 | `encode_quant` | zero / oversized length or count | **N/A — unrepresentable.** No length, size, count or buffer parameter exists. |
| E3 | `encode_quant` | out-of-range value for the `lsbit` mode selector, i.e. any `int` that is not one of the "documented" modes (`lsbit ∈ {0, 4, odd, even}`) | **No rejection.** `if (lsbit)` / `if (lsbit == 4)` / `else if (lsbit & 1)` / `else` is an *exhaustive* chain over all 2^32 ints, so every value lands in exactly one mode. Returns a normal quantizer index. |
| E4 | `encode_quant` | `lsbit` negative (`-1`, `-3`, `-4`, `INT_MIN`) — a "no valid variant" enum value across FFI | **No rejection.** `lsbit != 0` and `lsbit != 4`, so dispatch is by `lsbit & 1`: negative odd → set-LSB branch, negative even (incl. `-4`, `INT_MIN`) → clear-LSB branch. |
| E5 | `encode_quant` | `uni` outside the nominal 4-bit `0..15` quantizer range (negative, or ≥ 16, `INT_MIN`, `INT_MAX`) | **No rejection.** `uni` is never bounds-checked; it is only ever consumed through `& 7`, `& 8`, `^ ~7`, `>> 1`, `>> 2`, so any `int` is accepted and produces a defined value. Note the return value can therefore itself be out of `0..15`. |
| E6 | `encode_quant` | `step` = `0` | **No rejection.** `diff = ((2*(uni&7)+1)*0)/8 == 0`, so `p0 == p1 == p2 == pred`; `d0 == d1` and `d0 == d2`, both comparisons are strict `<`, so the original `uni` is returned. |
| E7 | `encode_quant` | `step` negative (incl. `INT_MIN`) | **No rejection.** `diff` becomes negative and the `if (uni & 8) diff = -diff` sign flip inverts as usual; `/ 8` truncates toward zero on the negative numerator. |
| E8 | `encode_quant` | `step` large enough that `(2*(uni&7)+1)*step` overflows `int` (`step > INT_MAX/15`) | **No rejection / no trap.** Signed overflow is UB in C, but the library is built by CMake with no optimization flags (`-O0`) and gcc emits a plain wrapping `imul`, so the observable result is two's-complement wraparound. Rust must reproduce the same wrapped value (`wrapping_mul`). |
| E9 | `encode_quant` | `uni == INT_MAX` (so `uni + 1` overflows) or `uni == INT_MIN` (so `uni - 1` overflows) | **No rejection / no trap.** Wraps; and because `INT_MAX & 7 == 7` / `INT_MIN & 7 == 0`, the `(uni ^ uniN) & ~7` guard clamps the wrapped candidate back to `uni` anyway. |
| E10 | `encode_quant` | `pred`/`tgt`/`tgt2` extreme so that `pred + diff`, `tgt - p0`, `tgt2 - p0` or `d0 + (d3 >> 5)` overflows `int` | **No rejection / no trap.** Wraps (two's complement at `-O0`). In particular the `d ^ (d >> 31)` "absolute value" idiom is *not* `abs`: it maps `d → d` for `d ≥ 0` and `d → -d - 1` for `d < 0`, so `INT_MIN → INT_MAX`. This off-by-one and the possibility of a *negative* `d0/d1/d2` after wrapping must be reproduced verbatim. |
| E11 | `encode_quant` | division by zero / `SIGFPE`; `INT_MIN / -1` | **N/A — unreachable.** The only divisor in the library is the literal `8`. |

## Phase C obligation

Because no row is an actual rejection, "same error code or sentinel" degenerates
to the strictly stronger requirement: for each of E1–E11 the two `.so`s must
return the **identical `int`**. Rows E1, E2 and E11 are unrepresentable /
unreachable and are discharged by the signature and the source itself (documented
above) rather than by a call; every other row has an executable differential
test in `translation/tests/differential.rs`.

| row | test | status |
|-----|------|--------|
| E1 | discharged by signature (no pointer parameter) — no test possible | [x] |
| E2 | discharged by signature (no length parameter) — no test possible | [x] |
| E3 | `err_e3_lsbit_out_of_range_exhaustive_modes` | [x] |
| E4 | `err_e4_lsbit_negative_and_int_min` | [x] |
| E5 | `err_e5_uni_out_of_nominal_range` | [x] |
| E6 | `err_e6_step_zero` | [x] |
| E7 | `err_e7_step_negative` | [x] |
| E8 | `err_e8_step_multiply_overflow` | [x] |
| E9 | `err_e9_uni_increment_decrement_overflow` | [x] |
| E10 | `err_e10_pred_tgt_overflow_and_absminus1` | [x] |
| E11 | discharged by source (constant divisor `8`) — unreachable | [x] |
