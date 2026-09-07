# Configuration surface

## Build-time feature combinations

CMake has two independent axes:

- `OP`: `add`, `sub`, or `mul`.
- `REPEAT`: one integer from `0` through `7`; values outside this range cannot
  name a defined `REP<n>` macro where `RUN_LOOP` is expanded.

This gives 24 distinct C configurations.

Cargo has 11 independent, additive features and no `compile_error!` exclusions.
Therefore every subset is accepted: for every
`O ⊆ {add, sub, mul}` and `R ⊆ {0, 1, 2, 3, 4, 5, 6, 7}`, the valid feature
combination is `O ∪ R`. This exactly enumerates `2^3 × 2^8 = 2,048`
combinations, including the empty set. Their effective C configuration is:

- operation: `mul` if enabled, otherwise `sub` if enabled, otherwise `add`;
- repeat: the greatest enabled repeat feature, or `5` if none is enabled.

Thus the 2,048 literal Cargo combinations collapse onto the 24 C
configurations below. All 2,048 must be checked/tested; each runtime row is
applicable whenever a combination resolves to the operation/repeat named by
that row.

## Runtime branch matrix

Rows are derived from the exported declarations and the `switch`, macro
selection, and `argc` branches in the C source. Random arithmetic operands must
stay within the defined signed-C-`int` result domain.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `op_add` | Any build configuration; randomized `int` operands whose sum is representable. | [x] |
| 2 | `op_sub` | Any build configuration; randomized `int` operands whose difference is representable. | [x] |
| 3 | `op_mul` | Any build configuration; randomized `int` operands whose product is representable. | [x] |
| 4 | `helper_ptr` | Effective `OP=add`; randomized representable operands. | [x] |
| 5 | `helper_ptr` | Effective `OP=sub`; randomized representable operands. | [x] |
| 6 | `helper_ptr` | Effective `OP=mul`; randomized representable operands. | [x] |
| 7 | `G_OP` function pointer | Effective `OP=add`; randomized representable operands. | [x] |
| 8 | `G_OP` function pointer | Effective `OP=sub`; randomized representable operands. | [x] |
| 9 | `G_OP` function pointer | Effective `OP=mul`; randomized representable operands. | [x] |
| 10 | `G_OP_NAME` data pointer | Effective `OP=add`; points to NUL-terminated bytes `add`. | [x] |
| 11 | `G_OP_NAME` data pointer | Effective `OP=sub`; points to NUL-terminated bytes `sub`. | [x] |
| 12 | `G_OP_NAME` data pointer | Effective `OP=mul`; points to NUL-terminated bytes `mul`. | [x] |
| 13 | `helper_call` | Effective `OP=add`, `REPEAT=0`; randomized representable operands. | [x] |
| 14 | `helper_call` | Effective `OP=add`, `REPEAT=1`; randomized representable operands. | [x] |
| 15 | `helper_call` | Effective `OP=add`, `REPEAT=2`; randomized representable operands. | [x] |
| 16 | `helper_call` | Effective `OP=add`, `REPEAT=3`; randomized representable operands. | [x] |
| 17 | `helper_call` | Effective `OP=add`, `REPEAT=4`; randomized representable operands. | [x] |
| 18 | `helper_call` | Effective `OP=add`, `REPEAT=5`; randomized representable operands. | [x] |
| 19 | `helper_call` | Effective `OP=add`, `REPEAT=6`; randomized representable operands. | [x] |
| 20 | `helper_call` | Effective `OP=add`, `REPEAT=7`; randomized representable operands. | [x] |
| 21 | `helper_call` | Effective `OP=sub`, `REPEAT=0`; randomized representable operands. | [x] |
| 22 | `helper_call` | Effective `OP=sub`, `REPEAT=1`; randomized representable operands. | [x] |
| 23 | `helper_call` | Effective `OP=sub`, `REPEAT=2`; randomized representable operands. | [x] |
| 24 | `helper_call` | Effective `OP=sub`, `REPEAT=3`; randomized representable operands. | [x] |
| 25 | `helper_call` | Effective `OP=sub`, `REPEAT=4`; randomized representable operands. | [x] |
| 26 | `helper_call` | Effective `OP=sub`, `REPEAT=5`; randomized representable operands. | [x] |
| 27 | `helper_call` | Effective `OP=sub`, `REPEAT=6`; randomized representable operands. | [x] |
| 28 | `helper_call` | Effective `OP=sub`, `REPEAT=7`; randomized representable operands. | [x] |
| 29 | `helper_call` | Effective `OP=mul`, `REPEAT=0`; randomized representable operands. | [x] |
| 30 | `helper_call` | Effective `OP=mul`, `REPEAT=1`; randomized representable operands. | [x] |
| 31 | `helper_call` | Effective `OP=mul`, `REPEAT=2`; randomized representable operands. | [x] |
| 32 | `helper_call` | Effective `OP=mul`, `REPEAT=3`; randomized representable operands. | [x] |
| 33 | `helper_call` | Effective `OP=mul`, `REPEAT=4`; randomized representable operands. | [x] |
| 34 | `helper_call` | Effective `OP=mul`, `REPEAT=5`; randomized representable operands. | [x] |
| 35 | `helper_call` | Effective `OP=mul`, `REPEAT=6`; randomized representable operands. | [x] |
| 36 | `helper_call` | Effective `OP=mul`, `REPEAT=7`; randomized representable operands. | [x] |
| 37 | `use_generated` | Effective `OP=add`; runtime `n=0` switch case. | [x] |
| 38 | `use_generated` | Effective `OP=add`; runtime `n=1` switch case. | [x] |
| 39 | `use_generated` | Effective `OP=add`; runtime `n=2` switch case. | [x] |
| 40 | `use_generated` | Effective `OP=add`; runtime `n=3` switch case. | [x] |
| 41 | `use_generated` | Effective `OP=add`; runtime `n=4` switch case. | [x] |
| 42 | `use_generated` | Effective `OP=add`; runtime `n=5` switch case. | [x] |
| 43 | `use_generated` | Effective `OP=add`; runtime `n=6` switch case. | [x] |
| 44 | `use_generated` | Effective `OP=add`; runtime `n < 0` and `n >= 7` default case. | [x] |
| 45 | `use_generated` | Effective `OP=sub`; runtime `n=0` switch case. | [x] |
| 46 | `use_generated` | Effective `OP=sub`; runtime `n=1` switch case. | [x] |
| 47 | `use_generated` | Effective `OP=sub`; runtime `n=2` switch case. | [x] |
| 48 | `use_generated` | Effective `OP=sub`; runtime `n=3` switch case. | [x] |
| 49 | `use_generated` | Effective `OP=sub`; runtime `n=4` switch case. | [x] |
| 50 | `use_generated` | Effective `OP=sub`; runtime `n=5` switch case. | [x] |
| 51 | `use_generated` | Effective `OP=sub`; runtime `n=6` switch case. | [x] |
| 52 | `use_generated` | Effective `OP=sub`; runtime `n < 0` and `n >= 7` default case. | [x] |
| 53 | `use_generated` | Effective `OP=mul`; runtime `n=0` switch case. | [x] |
| 54 | `use_generated` | Effective `OP=mul`; runtime `n=1` switch case. | [x] |
| 55 | `use_generated` | Effective `OP=mul`; runtime `n=2` switch case. | [x] |
| 56 | `use_generated` | Effective `OP=mul`; runtime `n=3` switch case. | [x] |
| 57 | `use_generated` | Effective `OP=mul`; runtime `n=4` switch case. | [x] |
| 58 | `use_generated` | Effective `OP=mul`; runtime `n=5` switch case. | [x] |
| 59 | `use_generated` | Effective `OP=mul`; runtime `n=6` switch case. | [x] |
| 60 | `use_generated` | Effective `OP=mul`; runtime `n < 0` and `n >= 7` default case. | [x] |
| 61 | `main` | Effective `OP=add`, `REPEAT=0`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 62 | `main` | Effective `OP=add`, `REPEAT=1`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 63 | `main` | Effective `OP=add`, `REPEAT=2`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 64 | `main` | Effective `OP=add`, `REPEAT=3`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 65 | `main` | Effective `OP=add`, `REPEAT=4`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 66 | `main` | Effective `OP=add`, `REPEAT=5`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 67 | `main` | Effective `OP=add`, `REPEAT=6`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 68 | `main` | Effective `OP=add`, `REPEAT=7`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 69 | `main` | Effective `OP=sub`, `REPEAT=0`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 70 | `main` | Effective `OP=sub`, `REPEAT=1`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 71 | `main` | Effective `OP=sub`, `REPEAT=2`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 72 | `main` | Effective `OP=sub`, `REPEAT=3`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 73 | `main` | Effective `OP=sub`, `REPEAT=4`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 74 | `main` | Effective `OP=sub`, `REPEAT=5`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 75 | `main` | Effective `OP=sub`, `REPEAT=6`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 76 | `main` | Effective `OP=sub`, `REPEAT=7`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 77 | `main` | Effective `OP=mul`, `REPEAT=0`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 78 | `main` | Effective `OP=mul`, `REPEAT=1`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 79 | `main` | Effective `OP=mul`, `REPEAT=2`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 80 | `main` | Effective `OP=mul`, `REPEAT=3`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 81 | `main` | Effective `OP=mul`, `REPEAT=4`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 82 | `main` | Effective `OP=mul`, `REPEAT=5`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 83 | `main` | Effective `OP=mul`, `REPEAT=6`; `argc >= 3`, randomized decimal `A B`. | [x] |
| 84 | `main` | Effective `OP=mul`, `REPEAT=7`; `argc >= 3`, randomized decimal `A B`. | [x] |

Final verification: every row passed with fixed-seed randomized inputs for all
24 effective C configurations, and the differential suite passed under all
2,048 literal Cargo feature subsets.
