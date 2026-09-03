# Configuration surface

## Build-time configurations

`CMakeLists.txt` selects one `OP`; `mdmacros.h` supports `add`, `sub`, and
`mul`. It selects one `REPEAT`; the manual-unroll macros support `0` through
`7`. The full semantic cross-product is therefore 24 C configurations.
Cargo permits either axis to be omitted to select the C default (`add` and
`5`), producing 36 valid, non-conflicting Cargo feature combinations:

| feature shape | combinations | effective C configuration |
|---------------|--------------|---------------------------|
| empty | `""` | `add,5` |
| operation only | `add`; `sub`; `mul` | selected operation, `REPEAT=5` |
| repeat only | `0`; `1`; `2`; `3`; `4`; `5`; `6`; `7` | `OP=add`, selected repeat |
| explicit pair | `add,0` … `add,7`; `sub,0` … `sub,7`; `mul,0` … `mul,7` | selected operation and repeat |

Multiple operation features or multiple repeat features do not correspond to
a scalar C preprocessor configuration and are not valid translation
configurations.

## Runtime/configuration rows

Rows are derived from the public declarations in `mdmacros.h`, the selected
operation branches, the compile-time `RUN_LOOP` expansion, and every arm of
the `DISPATCH_REP` switch. Randomized integer inputs include ordinary,
negative, boundary, and overflowing arithmetic cases.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | all by-value `int a, b`; no runtime branch | [x] |
| 2 | `op_sub` | all by-value `int a, b`; no runtime branch | [x] |
| 3 | `op_mul` | all by-value `int a, b`; no runtime branch | [x] |
| 4 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=add`; randomized `int a, b` | [x] |
| 5 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=sub`; randomized `int a, b` | [x] |
| 6 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=mul`; randomized `int a, b` | [x] |
| 7 | `helper_call` | `OP=add`, `REPEAT=0`; randomized `int a, b` | [x] |
| 8 | `helper_call` | `OP=add`, `REPEAT=1`; randomized `int a, b` | [x] |
| 9 | `helper_call` | `OP=add`, `REPEAT=2`; randomized `int a, b` | [x] |
| 10 | `helper_call` | `OP=add`, `REPEAT=3`; randomized `int a, b` | [x] |
| 11 | `helper_call` | `OP=add`, `REPEAT=4`; randomized `int a, b` | [x] |
| 12 | `helper_call` | `OP=add`, `REPEAT=5`; randomized `int a, b` | [x] |
| 13 | `helper_call` | `OP=add`, `REPEAT=6`; randomized `int a, b` | [x] |
| 14 | `helper_call` | `OP=add`, `REPEAT=7`; randomized `int a, b` | [x] |
| 15 | `helper_call` | `OP=sub`, `REPEAT=0`; randomized `int a, b` | [x] |
| 16 | `helper_call` | `OP=sub`, `REPEAT=1`; randomized `int a, b` | [x] |
| 17 | `helper_call` | `OP=sub`, `REPEAT=2`; randomized `int a, b` | [x] |
| 18 | `helper_call` | `OP=sub`, `REPEAT=3`; randomized `int a, b` | [x] |
| 19 | `helper_call` | `OP=sub`, `REPEAT=4`; randomized `int a, b` | [x] |
| 20 | `helper_call` | `OP=sub`, `REPEAT=5`; randomized `int a, b` | [x] |
| 21 | `helper_call` | `OP=sub`, `REPEAT=6`; randomized `int a, b` | [x] |
| 22 | `helper_call` | `OP=sub`, `REPEAT=7`; randomized `int a, b` | [x] |
| 23 | `helper_call` | `OP=mul`, `REPEAT=0`; randomized `int a, b` | [x] |
| 24 | `helper_call` | `OP=mul`, `REPEAT=1`; randomized `int a, b` | [x] |
| 25 | `helper_call` | `OP=mul`, `REPEAT=2`; randomized `int a, b` | [x] |
| 26 | `helper_call` | `OP=mul`, `REPEAT=3`; randomized `int a, b` | [x] |
| 27 | `helper_call` | `OP=mul`, `REPEAT=4`; randomized `int a, b` | [x] |
| 28 | `helper_call` | `OP=mul`, `REPEAT=5`; randomized `int a, b` | [x] |
| 29 | `helper_call` | `OP=mul`, `REPEAT=6`; randomized `int a, b` | [x] |
| 30 | `helper_call` | `OP=mul`, `REPEAT=7`; randomized `int a, b` | [x] |
| 31 | `use_generated` | `OP=add`, runtime `n=0` switch arm | [x] |
| 32 | `use_generated` | `OP=add`, runtime `n=1` switch arm | [x] |
| 33 | `use_generated` | `OP=add`, runtime `n=2` switch arm | [x] |
| 34 | `use_generated` | `OP=add`, runtime `n=3` switch arm | [x] |
| 35 | `use_generated` | `OP=add`, runtime `n=4` switch arm | [x] |
| 36 | `use_generated` | `OP=add`, runtime `n=5` switch arm | [x] |
| 37 | `use_generated` | `OP=add`, runtime `n=6` switch arm | [x] |
| 38 | `use_generated` | `OP=add`, runtime `n<0` and `n>=7` default arm | [x] |
| 39 | `use_generated` | `OP=sub`, runtime `n=0` switch arm | [x] |
| 40 | `use_generated` | `OP=sub`, runtime `n=1` switch arm | [x] |
| 41 | `use_generated` | `OP=sub`, runtime `n=2` switch arm | [x] |
| 42 | `use_generated` | `OP=sub`, runtime `n=3` switch arm | [x] |
| 43 | `use_generated` | `OP=sub`, runtime `n=4` switch arm | [x] |
| 44 | `use_generated` | `OP=sub`, runtime `n=5` switch arm | [x] |
| 45 | `use_generated` | `OP=sub`, runtime `n=6` switch arm | [x] |
| 46 | `use_generated` | `OP=sub`, runtime `n<0` and `n>=7` default arm | [x] |
| 47 | `use_generated` | `OP=mul`, runtime `n=0` switch arm | [x] |
| 48 | `use_generated` | `OP=mul`, runtime `n=1` switch arm | [x] |
| 49 | `use_generated` | `OP=mul`, runtime `n=2` switch arm | [x] |
| 50 | `use_generated` | `OP=mul`, runtime `n=3` switch arm | [x] |
| 51 | `use_generated` | `OP=mul`, runtime `n=4` switch arm | [x] |
| 52 | `use_generated` | `OP=mul`, runtime `n=5` switch arm | [x] |
| 53 | `use_generated` | `OP=mul`, runtime `n=6` switch arm | [x] |
| 54 | `use_generated` | `OP=mul`, runtime `n<0` and `n>=7` default arm | [x] |

Phase B status: all rows pass for every applicable build-time configuration.
