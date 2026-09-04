# Configuration Surface

## Build-time configurations

The C build has two independent macro axes: `OP` is one of `add`, `sub`, or
`mul`; `REPEAT` is one of `0` through `7`. Cargo represents those values as
additive features, so a valid translation configuration enables exactly one
operation feature and exactly one repeat feature with
`--no-default-features`.

The 24 valid combinations are:

```text
add,0 add,1 add,2 add,3 add,4 add,5 add,6 add,7
sub,0 sub,1 sub,2 sub,3 sub,4 sub,5 sub,6 sub,7
mul,0 mul,1 mul,2 mul,3 mul,4 mul,5 mul,6 mul,7
```

## Runtime and composed configurations

Rows are pruned where the C code does not branch on a build axis. Randomized
integer inputs include zero, `INT_MIN`, `INT_MAX`, and many fixed-seed values.
For `use_generated`, cases `0..=6` are distinct switch arms; the default row
includes negative values, `7`, and values above `7`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | all 24 builds; randomized `(a, b)` over the full C `int` bit pattern | [x] |
| 2 | `op_sub` | all 24 builds; randomized `(a, b)` over the full C `int` bit pattern | [x] |
| 3 | `op_mul` | all 24 builds; randomized `(a, b)` over the full C `int` bit pattern | [x] |
| 4 | `helper_ptr` | selected `OP=add`; randomized `(a, b)` | [x] |
| 5 | `helper_ptr` | selected `OP=sub`; randomized `(a, b)` | [x] |
| 6 | `helper_ptr` | selected `OP=mul`; randomized `(a, b)` | [x] |
| 7 | `helper_call` | `OP=add`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 8 | `helper_call` | `OP=add`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 9 | `helper_call` | `OP=add`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 10 | `helper_call` | `OP=add`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 11 | `helper_call` | `OP=add`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 12 | `helper_call` | `OP=add`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 13 | `helper_call` | `OP=add`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 14 | `helper_call` | `OP=add`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 15 | `helper_call` | `OP=sub`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 16 | `helper_call` | `OP=sub`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 17 | `helper_call` | `OP=sub`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 18 | `helper_call` | `OP=sub`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 19 | `helper_call` | `OP=sub`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 20 | `helper_call` | `OP=sub`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 21 | `helper_call` | `OP=sub`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 22 | `helper_call` | `OP=sub`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 23 | `helper_call` | `OP=mul`, `REPEAT=0`; randomized `(a, b)` | [x] |
| 24 | `helper_call` | `OP=mul`, `REPEAT=1`; randomized `(a, b)` | [x] |
| 25 | `helper_call` | `OP=mul`, `REPEAT=2`; randomized `(a, b)` | [x] |
| 26 | `helper_call` | `OP=mul`, `REPEAT=3`; randomized `(a, b)` | [x] |
| 27 | `helper_call` | `OP=mul`, `REPEAT=4`; randomized `(a, b)` | [x] |
| 28 | `helper_call` | `OP=mul`, `REPEAT=5`; randomized `(a, b)` | [x] |
| 29 | `helper_call` | `OP=mul`, `REPEAT=6`; randomized `(a, b)` | [x] |
| 30 | `helper_call` | `OP=mul`, `REPEAT=7`; randomized `(a, b)` | [x] |
| 31 | `G_OP`, `G_OP_NAME` | selected `OP=add`; randomized calls through `G_OP`, exact name bytes | [x] |
| 32 | `G_OP`, `G_OP_NAME` | selected `OP=sub`; randomized calls through `G_OP`, exact name bytes | [x] |
| 33 | `G_OP`, `G_OP_NAME` | selected `OP=mul`; randomized calls through `G_OP`, exact name bytes | [x] |
| 34 | `use_generated` | `OP=add`, switch case `n=0` | [x] |
| 35 | `use_generated` | `OP=add`, switch case `n=1` | [x] |
| 36 | `use_generated` | `OP=add`, switch case `n=2` | [x] |
| 37 | `use_generated` | `OP=add`, switch case `n=3` | [x] |
| 38 | `use_generated` | `OP=add`, switch case `n=4` | [x] |
| 39 | `use_generated` | `OP=add`, switch case `n=5` | [x] |
| 40 | `use_generated` | `OP=add`, switch case `n=6` | [x] |
| 41 | `use_generated` | `OP=add`, default switch branch (`n<0`, `n=7`, `n>7`) | [x] |
| 42 | `use_generated` | `OP=sub`, switch case `n=0` | [x] |
| 43 | `use_generated` | `OP=sub`, switch case `n=1` | [x] |
| 44 | `use_generated` | `OP=sub`, switch case `n=2` | [x] |
| 45 | `use_generated` | `OP=sub`, switch case `n=3` | [x] |
| 46 | `use_generated` | `OP=sub`, switch case `n=4` | [x] |
| 47 | `use_generated` | `OP=sub`, switch case `n=5` | [x] |
| 48 | `use_generated` | `OP=sub`, switch case `n=6` | [x] |
| 49 | `use_generated` | `OP=sub`, default switch branch (`n<0`, `n=7`, `n>7`) | [x] |
| 50 | `use_generated` | `OP=mul`, switch case `n=0` | [x] |
| 51 | `use_generated` | `OP=mul`, switch case `n=1` | [x] |
| 52 | `use_generated` | `OP=mul`, switch case `n=2` | [x] |
| 53 | `use_generated` | `OP=mul`, switch case `n=3` | [x] |
| 54 | `use_generated` | `OP=mul`, switch case `n=4` | [x] |
| 55 | `use_generated` | `OP=mul`, switch case `n=5` | [x] |
| 56 | `use_generated` | `OP=mul`, switch case `n=6` | [x] |
| 57 | `use_generated` | `OP=mul`, default switch branch (`n<0`, `n=7`, `n>7`) | [x] |
| 58 | `main` | `OP=add`, `REPEAT=0`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 59 | `main` | `OP=add`, `REPEAT=1`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 60 | `main` | `OP=add`, `REPEAT=2`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 61 | `main` | `OP=add`, `REPEAT=3`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 62 | `main` | `OP=add`, `REPEAT=4`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 63 | `main` | `OP=add`, `REPEAT=5`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 64 | `main` | `OP=add`, `REPEAT=6`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 65 | `main` | `OP=add`, `REPEAT=7`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 66 | `main` | `OP=sub`, `REPEAT=0`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 67 | `main` | `OP=sub`, `REPEAT=1`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 68 | `main` | `OP=sub`, `REPEAT=2`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 69 | `main` | `OP=sub`, `REPEAT=3`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 70 | `main` | `OP=sub`, `REPEAT=4`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 71 | `main` | `OP=sub`, `REPEAT=5`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 72 | `main` | `OP=sub`, `REPEAT=6`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 73 | `main` | `OP=sub`, `REPEAT=7`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 74 | `main` | `OP=mul`, `REPEAT=0`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 75 | `main` | `OP=mul`, `REPEAT=1`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 76 | `main` | `OP=mul`, `REPEAT=2`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 77 | `main` | `OP=mul`, `REPEAT=3`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 78 | `main` | `OP=mul`, `REPEAT=4`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 79 | `main` | `OP=mul`, `REPEAT=5`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 80 | `main` | `OP=mul`, `REPEAT=6`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |
| 81 | `main` | `OP=mul`, `REPEAT=7`; valid `argc>=3`, randomized numeric `argv[1..=2]` | [x] |

All rows pass under the complete 24-combination feature matrix.
