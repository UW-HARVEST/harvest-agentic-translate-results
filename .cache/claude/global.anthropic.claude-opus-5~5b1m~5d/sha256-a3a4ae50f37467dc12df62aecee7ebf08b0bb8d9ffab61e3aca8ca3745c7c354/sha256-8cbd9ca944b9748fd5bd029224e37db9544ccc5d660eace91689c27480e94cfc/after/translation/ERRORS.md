# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (35 lines) and
`c_src/include/lib.h` (7 lines).

## Mechanical grep results

```
$ grep -nE 'return|assert|NULL|nullptr|errno|-1|if *\(|#if|goto|abort|exit' c_src/src/lib.c
(no matches)

$ grep -nE 'RETURN_ERROR|return|assert|NULL|enum' c_src/include/lib.h
1: typedef enum cb_impairment {
```

**The C library has NO error-return path whatsoever.**

* `colourblind` returns `void` — there is no error code, no sentinel, no
  out-parameter status.
* There are no `assert`s, no NULL checks, no range checks, no `errno` use,
  no min/max constants, no `if` statements at all.
* The only control-flow construct is the `switch (Impairment)` in
  `colourblind`, which has **no `default:` label**. Falling off the end of a
  `switch` with no matching `case` is not an error — it is a silent no-op.
* The three helpers dereference `Red`/`Green`/`Blue` unconditionally, so a
  NULL (or otherwise invalid) pointer is *undefined behaviour in C*, not a
  detected/rejected input.

## The rejection table

One row per distinct way the C rejects / declines to act on input. Because
there are no error returns, the only "rejection" the C performs is the
implicit no-op fallthrough of the `default`-less `switch`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| E1 | `colourblind` | `Impairment == 3` (one past the last valid enumerator `cbTritanopia`) | silent no-op: `*R`, `*G`, `*B` left **bit-for-bit unmodified**; no crash; no return value | `err_e1_one_past_end` | [x] |
| E2 | `colourblind` | `Impairment == 4 .. 255` (small out-of-range enum values) | silent no-op, `*R/*G/*B` unchanged | `err_e2_small_out_of_range` | [x] |
| E3 | `colourblind` | `Impairment == (cb_impairment)-1` i.e. `0xFFFFFFFF` (a "negative" int passed through the FFI into an enum whose compatible type gcc picks as `unsigned int`) | silent no-op, unchanged | `err_e3_negative_as_unsigned` | [x] |
| E4 | `colourblind` | `Impairment == (cb_impairment)-2 .. -256`, and `INT_MIN` reinterpreted (`0x80000000`) | silent no-op, unchanged | `err_e4_negative_sweep` | [x] |
| E5 | `colourblind` | `Impairment` = `u32::MAX`, `u32::MAX-1`, `0x7FFFFFFF`, `0x80000000`, `1<<31`, `1<<16`, `256`, `65536` — extreme/boundary out-of-range enum values | silent no-op, unchanged | `err_e5_extreme_enum_values` | [x] |
| E6 | `colourblind` | Exhaustive/random sweep of `Impairment` over the whole `u32` domain (random + all of 0..1024) — C and Rust must agree on *which* values act and which no-op | values 0/1/2 transform; all others no-op; C == Rust for every value | `err_e6_enum_domain_sweep` | [x] |
| E7 | `colourblind` | `Impairment` in `{0,1,2}` (valid) but with **NaN / ±Inf** channel values — the C has no validation, so these are *accepted* and propagate | no rejection: NaN/Inf propagate through the matrix multiply; result bits must match C exactly (incl. NaN payload+sign) | `err_e7_nonfinite_accepted` | [x] |
| E8 | `colourblind` | `Impairment` in `{0,1,2}` with **subnormal / ±0.0** channel values — no validation, accepted | no rejection: exact bits must match, incl. sign of zero and flush-to-zero behaviour (none: SSE without FTZ) | `err_e8_subnormal_and_zero` | [x] |

## Generic FFI boundaries also covered (not C-detected, but required by the task)

| # | boundary | C behaviour | Rust must | test | ✅ |
|---|----------|-------------|-----------|------|----|
| B1 | NULL pointers with an **out-of-range** `Impairment` | pointers never dereferenced (switch falls through), so this is *defined* and safe | must also not dereference → no crash | `err_b1_null_ptrs_invalid_enum` | [x] |
| B2 | NULL pointers with a **valid** `Impairment` | UB in C (unconditional deref) — deliberately NOT tested; a segfault in either library proves nothing | documented, not tested | n/a (documented) | [x] |
| B3 | out-of-range enum value passed across the FFI boundary | see E1–E6 | identical | E1–E6 | [x] |
| B4 | "zero and oversized lengths" | **N/A** — the API takes no length/count/size argument at all (3 scalar `float*`, 1 enum) | n/a | n/a | [x] |

## Notes on why there is nothing more to table

The entire public API is a single `void`-returning function with 4 scalar
arguments. There are no allocations, no buffers, no lengths, no strings, no
file/stream handles, no callbacks, no global state, and no initialisation or
teardown functions. Consequently the error surface is limited to
(a) unmatched `switch` values and (b) non-finite/edge float values that the
code accepts and propagates unchecked.
