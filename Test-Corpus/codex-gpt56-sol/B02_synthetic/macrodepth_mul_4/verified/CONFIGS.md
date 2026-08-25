# Configuration Surface

## Build-Time Configurations

The C source admits `OP={add,sub,mul}` and `REPEAT={0,1,2,3,4,5,6,7}`.
Omitting `OP` means `add`; omitting `REPEAT` means `5`. Cargo therefore has
36 valid mutually exclusive feature sets (some map to the same C
configuration):

| operation selector | valid feature sets |
|--------------------|--------------------|
| omitted (`add`) | `""`, `"0"`, `"1"`, `"2"`, `"3"`, `"4"`, `"5"`, `"6"`, `"7"` |
| `add` | `"add"`, `"add,0"`, `"add,1"`, `"add,2"`, `"add,3"`, `"add,4"`, `"add,5"`, `"add,6"`, `"add,7"` |
| `sub` | `"sub"`, `"sub,0"`, `"sub,1"`, `"sub,2"`, `"sub,3"`, `"sub,4"`, `"sub,5"`, `"sub,6"`, `"sub,7"` |
| `mul` | `"mul"`, `"mul,0"`, `"mul,1"`, `"mul,2"`, `"mul,3"`, `"mul,4"`, `"mul,5"`, `"mul,6"`, `"mul,7"` |

Any set containing two operation selectors or two repeat selectors is
invalid. The 24 unique C configurations are the cross-product of the three
operations and eight repeat values.

## Valid-Path Matrix

Every randomized integer row includes `0`, `INT_MIN`, `INT_MAX`, and
overflow-producing operand pairs. `main` rows use `argc >= 3`, decimal inputs
accepted by `atoi`, and ignored trailing arguments.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | arbitrary `int a, int b` | [x] |
| 2 | `op_sub` | arbitrary `int a, int b` | [x] |
| 3 | `op_mul` | arbitrary `int a, int b` | [x] |
| 4 | `G_OP`, `G_OP_NAME` | `OP=add`; callable target and string bytes | [x] |
| 5 | `G_OP`, `G_OP_NAME` | `OP=sub`; callable target and string bytes | [x] |
| 6 | `G_OP`, `G_OP_NAME` | `OP=mul`; callable target and string bytes | [x] |
| 7 | `helper_call` | `OP=add`, `REPEAT=0`, arbitrary integer pair | [x] |
| 8 | `helper_call` | `OP=add`, `REPEAT=1`, arbitrary integer pair | [x] |
| 9 | `helper_call` | `OP=add`, `REPEAT=2`, arbitrary integer pair | [x] |
| 10 | `helper_call` | `OP=add`, `REPEAT=3`, arbitrary integer pair | [x] |
| 11 | `helper_call` | `OP=add`, `REPEAT=4`, arbitrary integer pair | [x] |
| 12 | `helper_call` | `OP=add`, `REPEAT=5`, arbitrary integer pair | [x] |
| 13 | `helper_call` | `OP=add`, `REPEAT=6`, arbitrary integer pair | [x] |
| 14 | `helper_call` | `OP=add`, `REPEAT=7`, arbitrary integer pair | [x] |
| 15 | `helper_call` | `OP=sub`, `REPEAT=0`, arbitrary integer pair | [x] |
| 16 | `helper_call` | `OP=sub`, `REPEAT=1`, arbitrary integer pair | [x] |
| 17 | `helper_call` | `OP=sub`, `REPEAT=2`, arbitrary integer pair | [x] |
| 18 | `helper_call` | `OP=sub`, `REPEAT=3`, arbitrary integer pair | [x] |
| 19 | `helper_call` | `OP=sub`, `REPEAT=4`, arbitrary integer pair | [x] |
| 20 | `helper_call` | `OP=sub`, `REPEAT=5`, arbitrary integer pair | [x] |
| 21 | `helper_call` | `OP=sub`, `REPEAT=6`, arbitrary integer pair | [x] |
| 22 | `helper_call` | `OP=sub`, `REPEAT=7`, arbitrary integer pair | [x] |
| 23 | `helper_call` | `OP=mul`, `REPEAT=0`, arbitrary integer pair | [x] |
| 24 | `helper_call` | `OP=mul`, `REPEAT=1`, arbitrary integer pair | [x] |
| 25 | `helper_call` | `OP=mul`, `REPEAT=2`, arbitrary integer pair | [x] |
| 26 | `helper_call` | `OP=mul`, `REPEAT=3`, arbitrary integer pair | [x] |
| 27 | `helper_call` | `OP=mul`, `REPEAT=4`, arbitrary integer pair | [x] |
| 28 | `helper_call` | `OP=mul`, `REPEAT=5`, arbitrary integer pair | [x] |
| 29 | `helper_call` | `OP=mul`, `REPEAT=6`, arbitrary integer pair | [x] |
| 30 | `helper_call` | `OP=mul`, `REPEAT=7`, arbitrary integer pair | [x] |
| 31 | `helper_ptr` | `OP=add`, arbitrary integer pair (`REPEAT` is not read) | [x] |
| 32 | `helper_ptr` | `OP=sub`, arbitrary integer pair (`REPEAT` is not read) | [x] |
| 33 | `helper_ptr` | `OP=mul`, arbitrary integer pair (`REPEAT` is not read) | [x] |
| 34 | `use_generated` | `OP=add`, `n=0` switch arm | [x] |
| 35 | `use_generated` | `OP=add`, `n=1` switch arm | [x] |
| 36 | `use_generated` | `OP=add`, `n=2` switch arm | [x] |
| 37 | `use_generated` | `OP=add`, `n=3` switch arm | [x] |
| 38 | `use_generated` | `OP=add`, `n=4` switch arm | [x] |
| 39 | `use_generated` | `OP=add`, `n=5` switch arm | [x] |
| 40 | `use_generated` | `OP=add`, `n=6` switch arm | [x] |
| 41 | `use_generated` | `OP=add`, `n < 0` or `n >= 7` default arm | [x] |
| 42 | `use_generated` | `OP=sub`, `n=0` switch arm | [x] |
| 43 | `use_generated` | `OP=sub`, `n=1` switch arm | [x] |
| 44 | `use_generated` | `OP=sub`, `n=2` switch arm | [x] |
| 45 | `use_generated` | `OP=sub`, `n=3` switch arm | [x] |
| 46 | `use_generated` | `OP=sub`, `n=4` switch arm | [x] |
| 47 | `use_generated` | `OP=sub`, `n=5` switch arm | [x] |
| 48 | `use_generated` | `OP=sub`, `n=6` switch arm | [x] |
| 49 | `use_generated` | `OP=sub`, `n < 0` or `n >= 7` default arm | [x] |
| 50 | `use_generated` | `OP=mul`, `n=0` switch arm | [x] |
| 51 | `use_generated` | `OP=mul`, `n=1` switch arm | [x] |
| 52 | `use_generated` | `OP=mul`, `n=2` switch arm | [x] |
| 53 | `use_generated` | `OP=mul`, `n=3` switch arm | [x] |
| 54 | `use_generated` | `OP=mul`, `n=4` switch arm | [x] |
| 55 | `use_generated` | `OP=mul`, `n=5` switch arm | [x] |
| 56 | `use_generated` | `OP=mul`, `n=6` switch arm | [x] |
| 57 | `use_generated` | `OP=mul`, `n < 0` or `n >= 7` default arm | [x] |
| 58 | `main` | `OP=add`, `REPEAT=0`, valid `argv` | [x] |
| 59 | `main` | `OP=add`, `REPEAT=1`, valid `argv` | [x] |
| 60 | `main` | `OP=add`, `REPEAT=2`, valid `argv` | [x] |
| 61 | `main` | `OP=add`, `REPEAT=3`, valid `argv` | [x] |
| 62 | `main` | `OP=add`, `REPEAT=4`, valid `argv` | [x] |
| 63 | `main` | `OP=add`, `REPEAT=5`, valid `argv` | [x] |
| 64 | `main` | `OP=add`, `REPEAT=6`, valid `argv` | [x] |
| 65 | `main` | `OP=add`, `REPEAT=7`, valid `argv` | [x] |
| 66 | `main` | `OP=sub`, `REPEAT=0`, valid `argv` | [x] |
| 67 | `main` | `OP=sub`, `REPEAT=1`, valid `argv` | [x] |
| 68 | `main` | `OP=sub`, `REPEAT=2`, valid `argv` | [x] |
| 69 | `main` | `OP=sub`, `REPEAT=3`, valid `argv` | [x] |
| 70 | `main` | `OP=sub`, `REPEAT=4`, valid `argv` | [x] |
| 71 | `main` | `OP=sub`, `REPEAT=5`, valid `argv` | [x] |
| 72 | `main` | `OP=sub`, `REPEAT=6`, valid `argv` | [x] |
| 73 | `main` | `OP=sub`, `REPEAT=7`, valid `argv` | [x] |
| 74 | `main` | `OP=mul`, `REPEAT=0`, valid `argv` | [x] |
| 75 | `main` | `OP=mul`, `REPEAT=1`, valid `argv` | [x] |
| 76 | `main` | `OP=mul`, `REPEAT=2`, valid `argv` | [x] |
| 77 | `main` | `OP=mul`, `REPEAT=3`, valid `argv` | [x] |
| 78 | `main` | `OP=mul`, `REPEAT=4`, valid `argv` | [x] |
| 79 | `main` | `OP=mul`, `REPEAT=5`, valid `argv` | [x] |
| 80 | `main` | `OP=mul`, `REPEAT=6`, valid `argv` | [x] |
| 81 | `main` | `OP=mul`, `REPEAT=7`, valid `argv` | [x] |
