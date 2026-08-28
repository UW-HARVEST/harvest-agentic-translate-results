# Configuration Surface

## Build-Time Matrix

`Cargo.toml` defines two independent axes corresponding to the C preprocessor
tokens in `CMakeLists.txt` and `mdmacros.h`:

- operation: exactly one of `add`, `sub`, `mul`;
- repeat depth: exactly one of `0`, `1`, `2`, `3`, `4`, `5`, `6`, `7`.

This gives 24 canonical, non-ambiguous feature combinations:

```text
add,0 add,1 add,2 add,3 add,4 add,5 add,6 add,7
sub,0 sub,1 sub,2 sub,3 sub,4 sub,5 sub,6 sub,7
mul,0 mul,1 mul,2 mul,3 mul,4 mul,5 mul,6 mul,7
```

The Cargo default and C macro defaults are both `add,5`. Cargo features are
additive syntactically, but combinations selecting multiple values on one axis
do not correspond to a valid single `OP` or `REPEAT` token in the C build and
are excluded. Every command below uses `--no-default-features`.

## Runtime Branch Matrix

Rows are pruned where the source does not inspect an axis. Each randomized
integer set includes zero, positive, negative, and representable boundary
values that do not trigger signed-overflow undefined behavior in C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | all builds; many randomized `(a,b)` pairs | [x] |
| 2 | `op_sub` | all builds; many randomized `(a,b)` pairs | [x] |
| 3 | `op_mul` | all builds; many randomized non-overflowing `(a,b)` pairs | [x] |
| 4 | `helper_ptr`, `G_OP`, `G_OP_NAME` | selected operation `add`; many randomized `(a,b)` pairs | [x] |
| 5 | `helper_ptr`, `G_OP`, `G_OP_NAME` | selected operation `sub`; many randomized `(a,b)` pairs | [x] |
| 6 | `helper_ptr`, `G_OP`, `G_OP_NAME` | selected operation `mul`; many randomized non-overflowing `(a,b)` pairs | [x] |
| 7 | `helper_call`, `main` | features `add,0`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 8 | `helper_call`, `main` | features `add,1`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 9 | `helper_call`, `main` | features `add,2`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 10 | `helper_call`, `main` | features `add,3`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 11 | `helper_call`, `main` | features `add,4`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 12 | `helper_call`, `main` | features `add,5`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 13 | `helper_call`, `main` | features `add,6`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 14 | `helper_call`, `main` | features `add,7`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 15 | `helper_call`, `main` | features `sub,0`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 16 | `helper_call`, `main` | features `sub,1`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 17 | `helper_call`, `main` | features `sub,2`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 18 | `helper_call`, `main` | features `sub,3`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 19 | `helper_call`, `main` | features `sub,4`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 20 | `helper_call`, `main` | features `sub,5`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 21 | `helper_call`, `main` | features `sub,6`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 22 | `helper_call`, `main` | features `sub,7`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 23 | `helper_call`, `main` | features `mul,0`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 24 | `helper_call`, `main` | features `mul,1`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 25 | `helper_call`, `main` | features `mul,2`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 26 | `helper_call`, `main` | features `mul,3`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 27 | `helper_call`, `main` | features `mul,4`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 28 | `helper_call`, `main` | features `mul,5`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 29 | `helper_call`, `main` | features `mul,6`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 30 | `helper_call`, `main` | features `mul,7`; randomized integers / decimal argument strings, `argc >= 3` | [x] |
| 31 | `use_generated` | selected operation `add`; `n = 0` switch branch | [x] |
| 32 | `use_generated` | selected operation `add`; `n = 1` switch branch | [x] |
| 33 | `use_generated` | selected operation `add`; `n = 2` switch branch | [x] |
| 34 | `use_generated` | selected operation `add`; `n = 3` switch branch | [x] |
| 35 | `use_generated` | selected operation `add`; `n = 4` switch branch | [x] |
| 36 | `use_generated` | selected operation `add`; `n = 5` switch branch | [x] |
| 37 | `use_generated` | selected operation `add`; `n = 6` switch branch | [x] |
| 38 | `use_generated` | selected operation `add`; default branch (`n < 0` or `n >= 7`) | [x] |
| 39 | `use_generated` | selected operation `sub`; `n = 0` switch branch | [x] |
| 40 | `use_generated` | selected operation `sub`; `n = 1` switch branch | [x] |
| 41 | `use_generated` | selected operation `sub`; `n = 2` switch branch | [x] |
| 42 | `use_generated` | selected operation `sub`; `n = 3` switch branch | [x] |
| 43 | `use_generated` | selected operation `sub`; `n = 4` switch branch | [x] |
| 44 | `use_generated` | selected operation `sub`; `n = 5` switch branch | [x] |
| 45 | `use_generated` | selected operation `sub`; `n = 6` switch branch | [x] |
| 46 | `use_generated` | selected operation `sub`; default branch (`n < 0` or `n >= 7`) | [x] |
| 47 | `use_generated` | selected operation `mul`; `n = 0` switch branch | [x] |
| 48 | `use_generated` | selected operation `mul`; `n = 1` switch branch | [x] |
| 49 | `use_generated` | selected operation `mul`; `n = 2` switch branch | [x] |
| 50 | `use_generated` | selected operation `mul`; `n = 3` switch branch | [x] |
| 51 | `use_generated` | selected operation `mul`; `n = 4` switch branch | [x] |
| 52 | `use_generated` | selected operation `mul`; `n = 5` switch branch | [x] |
| 53 | `use_generated` | selected operation `mul`; `n = 6` switch branch | [x] |
| 54 | `use_generated` | selected operation `mul`; default branch (`n < 0` or `n >= 7`) | [x] |
