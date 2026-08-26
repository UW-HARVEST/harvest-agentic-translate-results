# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or feature macros. The complete set of valid feature combinations is:

1. No features: `--no-default-features`

## Runtime Configurations

Rows are derived from the `if`, loop, and `switch` branches in
`c_src/src/lib.c`. "Randomized" means many deterministic, fixed-seed values
within the stated shape.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `safe_double_to_int` | finite interior values, including signed zero and positive/negative fractions | [x] |
| 2 | `safe_double_to_int` | NaN payloads | [x] |
| 3 | `safe_double_to_int` | positive infinity | [x] |
| 4 | `safe_double_to_int` | negative infinity | [x] |
| 5 | `safe_double_to_int` | finite values at or above `INT_MAX` | [x] |
| 6 | `safe_double_to_int` | finite values at or below `INT_MIN` | [x] |
| 7 | `process_array_reverse` | empty shape (`count <= 0`, no dereference) | [x] |
| 8 | `process_array_reverse` | one element, `end` points at that element | [x] |
| 9 | `process_array_reverse` | many elements, `end` points at the final requested element | [x] |
| 10 | `switch_fallthrough_calculator` | operation `0`, cases `0 -> 1 -> 2` | [x] |
| 11 | `switch_fallthrough_calculator` | operation `1`, cases `1 -> 2` | [x] |
| 12 | `switch_fallthrough_calculator` | operation `2` | [x] |
| 13 | `switch_fallthrough_calculator` | operation `3`, cases `3 -> 4` | [x] |
| 14 | `switch_fallthrough_calculator` | operation `4` | [x] |
| 15 | `switch_fallthrough_calculator` | any operation outside `0..4`, default case | [x] |
| 16 | `allocate_and_compute` | `size == 0`; zero loop iterations | [x] |
| 17 | `allocate_and_compute` | `size == 1`; one loop iteration and zero-valued point | [x] |
| 18 | `allocate_and_compute` | `size > 1`, finite multiplier, nonsaturating sum | [x] |
| 19 | `allocate_and_compute` | `size > 1`, NaN multiplier and sum | [x] |
| 20 | `allocate_and_compute` | `size > 1`, positive-infinite multiplier and NaN sum (`0 * infinity`) | [x] |
| 21 | `allocate_and_compute` | `size > 1`, negative-infinite multiplier and NaN sum (`0 * -infinity`) | [x] |
| 22 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum `>= INT_MAX` | [x] |
| 23 | `allocate_and_compute` | `size > 1`, finite multiplier producing sum `<= INT_MIN` | [x] |
| 24 | `foreach_sum` | empty shape (`count <= 0`, no dereference) | [x] |
| 25 | `foreach_sum` | one element | [x] |
| 26 | `foreach_sum` | many elements | [x] |
| 27 | `fallcalc` | `param3 <= 0200`, remainder/operation `0`; randomized other parameters cover derived allocation sizes `<=0`, `1`, and many | [x] |
| 28 | `fallcalc` | `param3 <= 0200`, remainder/operation `1`; randomized other parameters cover derived allocation sizes `<=0`, `1`, and many | [x] |
| 29 | `fallcalc` | `param3 <= 0200`, remainder/operation `2`; randomized other parameters cover derived allocation sizes `<=0`, `1`, and many | [x] |
| 30 | `fallcalc` | `param3 <= 0200`, remainder/operation `3`; randomized other parameters cover derived allocation sizes `<=0`, `1`, and many | [x] |
| 31 | `fallcalc` | `param3 <= 0200`, remainder/operation `4`; randomized other parameters cover derived allocation sizes `<=0`, `1`, and many | [x] |
| 32 | `fallcalc` | negative `param3` with remainder outside `0..4`, selecting the default operation; randomized derived allocation sizes | [x] |
| 33 | `fallcalc` | `param3 > 0200`, remainder/operation `0`, setting `OCTAL_FLAG`; randomized derived allocation sizes | [x] |
| 34 | `fallcalc` | `param3 > 0200`, remainder/operation `1`, setting `OCTAL_FLAG`; randomized derived allocation sizes | [x] |
| 35 | `fallcalc` | `param3 > 0200`, remainder/operation `2`, setting `OCTAL_FLAG`; randomized derived allocation sizes | [x] |
| 36 | `fallcalc` | `param3 > 0200`, remainder/operation `3`, setting `OCTAL_FLAG`; randomized derived allocation sizes | [x] |
| 37 | `fallcalc` | `param3 > 0200`, remainder/operation `4`, setting `OCTAL_FLAG`; randomized derived allocation sizes | [x] |

The API has no mutable runtime option objects, enums, element-type choices,
byte-order modes, or alternate formats.
