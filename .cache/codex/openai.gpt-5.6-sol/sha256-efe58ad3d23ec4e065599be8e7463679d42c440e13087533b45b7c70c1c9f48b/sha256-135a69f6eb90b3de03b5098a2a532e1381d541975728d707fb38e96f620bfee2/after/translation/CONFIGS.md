# Configuration-surface table

Mechanically derived axes:

- Build-time `OP`: `add`, `sub`, or `mul`.
- Build-time `REPEAT`: integer token `0` through `7`, matching the defined
  `REP0` through `REP7` macros.
- Pair inputs: two by-value C `int` values.
- Generated-accumulator input: switch cases `0` through `6`, plus the
  distinguished default shapes `n < 0` and `n > 6`.
- Public shared-object entry points: `op_add`, `op_sub`, `op_mul`,
  `helper_call`, `helper_ptr`, `use_generated`, `G_OP`, and `G_OP_NAME`.
- The executable driver composes the selected operation, repeat accumulator,
  helpers, generated accumulator, and globals.

Cargo feature combinations are exactly one operation feature crossed with
exactly one repeat feature: **3 × 8 = 24**. Every row below is exercised with
many deterministic randomized values. Rows whose repeat is “any” run under
all eight repeat features for the stated operation.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `op_add` | any build; randomized `(a, b)` over the full C `int` bit domain | [x] |
| 2 | `op_sub` | any build; randomized `(a, b)` over the full C `int` bit domain | [x] |
| 3 | `op_mul` | any build; randomized `(a, b)` over the full C `int` bit domain | [x] |
| 4 | `helper_call`, `helper_ptr`, `G_OP`, `G_OP_NAME`, driver | `OP=add`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 5 | same selected-operation surface | `OP=add`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 6 | same selected-operation surface | `OP=add`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 7 | same selected-operation surface | `OP=add`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 8 | same selected-operation surface | `OP=add`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 9 | same selected-operation surface | `OP=add`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 10 | same selected-operation surface | `OP=add`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 11 | same selected-operation surface | `OP=add`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 12 | same selected-operation surface | `OP=sub`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 13 | same selected-operation surface | `OP=sub`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 14 | same selected-operation surface | `OP=sub`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 15 | same selected-operation surface | `OP=sub`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 16 | same selected-operation surface | `OP=sub`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 17 | same selected-operation surface | `OP=sub`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 18 | same selected-operation surface | `OP=sub`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 19 | same selected-operation surface | `OP=sub`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 20 | same selected-operation surface | `OP=mul`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 21 | same selected-operation surface | `OP=mul`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 22 | same selected-operation surface | `OP=mul`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 23 | same selected-operation surface | `OP=mul`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 24 | same selected-operation surface | `OP=mul`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 25 | same selected-operation surface | `OP=mul`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 26 | same selected-operation surface | `OP=mul`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 27 | same selected-operation surface | `OP=mul`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 28 | `use_generated` | `OP=add`, repeat any, `n=0` | [x] |
| 29 | `use_generated` | `OP=add`, repeat any, `n=1` | [x] |
| 30 | `use_generated` | `OP=add`, repeat any, `n=2` | [x] |
| 31 | `use_generated` | `OP=add`, repeat any, `n=3` | [x] |
| 32 | `use_generated` | `OP=add`, repeat any, `n=4` | [x] |
| 33 | `use_generated` | `OP=add`, repeat any, `n=5` | [x] |
| 34 | `use_generated` | `OP=add`, repeat any, `n=6` | [x] |
| 35 | `use_generated` | `OP=add`, repeat any, `n < 0` (default switch branch) | [x] |
| 36 | `use_generated` | `OP=add`, repeat any, `n > 6` (default switch branch) | [x] |
| 37 | `use_generated` | `OP=sub`, repeat any, `n=0` | [x] |
| 38 | `use_generated` | `OP=sub`, repeat any, `n=1` | [x] |
| 39 | `use_generated` | `OP=sub`, repeat any, `n=2` | [x] |
| 40 | `use_generated` | `OP=sub`, repeat any, `n=3` | [x] |
| 41 | `use_generated` | `OP=sub`, repeat any, `n=4` | [x] |
| 42 | `use_generated` | `OP=sub`, repeat any, `n=5` | [x] |
| 43 | `use_generated` | `OP=sub`, repeat any, `n=6` | [x] |
| 44 | `use_generated` | `OP=sub`, repeat any, `n < 0` (default switch branch) | [x] |
| 45 | `use_generated` | `OP=sub`, repeat any, `n > 6` (default switch branch) | [x] |
| 46 | `use_generated` | `OP=mul`, repeat any, `n=0` | [x] |
| 47 | `use_generated` | `OP=mul`, repeat any, `n=1` | [x] |
| 48 | `use_generated` | `OP=mul`, repeat any, `n=2` | [x] |
| 49 | `use_generated` | `OP=mul`, repeat any, `n=3` | [x] |
| 50 | `use_generated` | `OP=mul`, repeat any, `n=4` | [x] |
| 51 | `use_generated` | `OP=mul`, repeat any, `n=5` | [x] |
| 52 | `use_generated` | `OP=mul`, repeat any, `n=6` | [x] |
| 53 | `use_generated` | `OP=mul`, repeat any, `n < 0` (default switch branch) | [x] |
| 54 | `use_generated` | `OP=mul`, repeat any, `n > 6` (default switch branch) | [x] |
