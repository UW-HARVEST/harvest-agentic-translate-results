# Dynamic symbol surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The reference shared object is built from `mdcore.c`; `mdmain.c` is the
separate executable driver. `accum_<OP>` is generated as a `static` function
by `DEFINE_ACCUM`, so it does not appear in the dynamic symbol table.

| # | C symbol | ELF type | Rust export | Status |
|---|----------|----------|-------------|--------|
| 1 | `op_add` | `T` | `op_add` | [x] |
| 2 | `op_sub` | `T` | `op_sub` | [x] |
| 3 | `op_mul` | `T` | `op_mul` | [x] |
| 4 | `helper_call` | `T` | `helper_call` | [x] |
| 5 | `helper_ptr` | `T` | `helper_ptr` | [x] |
| 6 | `use_generated` | `T` | `use_generated` | [x] |
| 7 | `G_OP` | `D` | `G_OP` | [x] |
| 8 | `G_OP_NAME` | `D` | `G_OP_NAME` | [x] |

Missing C symbols in Rust: **0**.

The C object's undefined dynamic references are `printf` plus weak ELF
runtime hooks; there are no undefined project/library symbols.
