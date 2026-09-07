# Configuration-surface table

Mechanically derived from `OP`, `REPEAT`, every public declaration in
`mdmacros.h`, the generated accumulator's `switch (n)`, and the composed
driver. Random scalar pairs include zero, positive, negative, and near-boundary
values that do not invoke signed-overflow undefined behavior in C.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | Random safe `int` pairs, independent of `OP`/`REPEAT`. | [x] |
| 2 | `op_sub` | Random safe `int` pairs, independent of `OP`/`REPEAT`. | [x] |
| 3 | `op_mul` | Random safe `int` pairs, independent of `OP`/`REPEAT`. | [x] |
| 4 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=add`; random safe `int` pairs; global name is `add`. | [x] |
| 5 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=sub`; random safe `int` pairs; global name is `sub`. | [x] |
| 6 | `G_OP`, `G_OP_NAME`, `helper_ptr` | `OP=mul`; random safe `int` pairs; global name is `mul`. | [x] |
| 7 | `helper_call`, `driver` | `OP=add`, `REPEAT=0`; random safe scalar inputs. | [x] |
| 8 | `helper_call`, `driver` | `OP=add`, `REPEAT=1`; random safe scalar inputs. | [x] |
| 9 | `helper_call`, `driver` | `OP=add`, `REPEAT=2`; random safe scalar inputs. | [x] |
| 10 | `helper_call`, `driver` | `OP=add`, `REPEAT=3`; random safe scalar inputs. | [x] |
| 11 | `helper_call`, `driver` | `OP=add`, `REPEAT=4`; random safe scalar inputs. | [x] |
| 12 | `helper_call`, `driver` | `OP=add`, `REPEAT=5`; random safe scalar inputs. | [x] |
| 13 | `helper_call`, `driver` | `OP=add`, `REPEAT=6`; random safe scalar inputs. | [x] |
| 14 | `helper_call`, `driver` | `OP=add`, `REPEAT=7`; random safe scalar inputs. | [x] |
| 15 | `helper_call`, `driver` | `OP=sub`, `REPEAT=0`; random safe scalar inputs. | [x] |
| 16 | `helper_call`, `driver` | `OP=sub`, `REPEAT=1`; random safe scalar inputs. | [x] |
| 17 | `helper_call`, `driver` | `OP=sub`, `REPEAT=2`; random safe scalar inputs. | [x] |
| 18 | `helper_call`, `driver` | `OP=sub`, `REPEAT=3`; random safe scalar inputs. | [x] |
| 19 | `helper_call`, `driver` | `OP=sub`, `REPEAT=4`; random safe scalar inputs. | [x] |
| 20 | `helper_call`, `driver` | `OP=sub`, `REPEAT=5`; random safe scalar inputs. | [x] |
| 21 | `helper_call`, `driver` | `OP=sub`, `REPEAT=6`; random safe scalar inputs. | [x] |
| 22 | `helper_call`, `driver` | `OP=sub`, `REPEAT=7`; random safe scalar inputs. | [x] |
| 23 | `helper_call`, `driver` | `OP=mul`, `REPEAT=0`; bounded random scalar inputs. | [x] |
| 24 | `helper_call`, `driver` | `OP=mul`, `REPEAT=1`; bounded random scalar inputs. | [x] |
| 25 | `helper_call`, `driver` | `OP=mul`, `REPEAT=2`; bounded random scalar inputs. | [x] |
| 26 | `helper_call`, `driver` | `OP=mul`, `REPEAT=3`; bounded random scalar inputs. | [x] |
| 27 | `helper_call`, `driver` | `OP=mul`, `REPEAT=4`; bounded random scalar inputs. | [x] |
| 28 | `helper_call`, `driver` | `OP=mul`, `REPEAT=5`; bounded random scalar inputs. | [x] |
| 29 | `helper_call`, `driver` | `OP=mul`, `REPEAT=6`; bounded random scalar inputs. | [x] |
| 30 | `helper_call`, `driver` | `OP=mul`, `REPEAT=7`; bounded random scalar inputs. | [x] |
| 31 | `use_generated` | `OP=add`, `n=0` switch arm. | [x] |
| 32 | `use_generated` | `OP=add`, `n=1` switch arm. | [x] |
| 33 | `use_generated` | `OP=add`, `n=2` switch arm. | [x] |
| 34 | `use_generated` | `OP=add`, `n=3` switch arm. | [x] |
| 35 | `use_generated` | `OP=add`, `n=4` switch arm. | [x] |
| 36 | `use_generated` | `OP=add`, `n=5` switch arm. | [x] |
| 37 | `use_generated` | `OP=add`, `n=6` switch arm. | [x] |
| 38 | `use_generated` | `OP=add`, `n < 0` or `n >= 7` default arm, including `INT_MIN`, `7`, and `INT_MAX`. | [x] |
| 39 | `use_generated` | `OP=sub`, `n=0` switch arm. | [x] |
| 40 | `use_generated` | `OP=sub`, `n=1` switch arm. | [x] |
| 41 | `use_generated` | `OP=sub`, `n=2` switch arm. | [x] |
| 42 | `use_generated` | `OP=sub`, `n=3` switch arm. | [x] |
| 43 | `use_generated` | `OP=sub`, `n=4` switch arm. | [x] |
| 44 | `use_generated` | `OP=sub`, `n=5` switch arm. | [x] |
| 45 | `use_generated` | `OP=sub`, `n=6` switch arm. | [x] |
| 46 | `use_generated` | `OP=sub`, `n < 0` or `n >= 7` default arm, including `INT_MIN`, `7`, and `INT_MAX`. | [x] |
| 47 | `use_generated` | `OP=mul`, `n=0` switch arm. | [x] |
| 48 | `use_generated` | `OP=mul`, `n=1` switch arm. | [x] |
| 49 | `use_generated` | `OP=mul`, `n=2` switch arm. | [x] |
| 50 | `use_generated` | `OP=mul`, `n=3` switch arm. | [x] |
| 51 | `use_generated` | `OP=mul`, `n=4` switch arm. | [x] |
| 52 | `use_generated` | `OP=mul`, `n=5` switch arm. | [x] |
| 53 | `use_generated` | `OP=mul`, `n=6` switch arm. | [x] |
| 54 | `use_generated` | `OP=mul`, `n < 0` or `n >= 7` default arm, including `INT_MIN`, `7`, and `INT_MAX`. | [x] |
