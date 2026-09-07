# Dynamic Symbol Surface

Source library: `../c_src/build/libharvest-work-oIOgbn.so`

Mechanical extraction:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-oIOgbn.so
```

| C symbol | ELF type | Rust export | Status |
|----------|----------|-------------|--------|
| `double_value` | `T` | `double_value` | [x] |
| `gotomach` | `T` | `gotomach` | [x] |
| `process_value` | `T` | `process_value` | [x] |
| `triple_value` | `T` | `triple_value` | [x] |

The C library also has the following undefined runtime imports in `nm -D`;
they are not library API exports: `free@GLIBC_2.2.5`,
`malloc@GLIBC_2.2.5`, and `puts@GLIBC_2.2.5`. Its weak toolchain/runtime
symbols are `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, and `__gmon_start__`.

Defined API symbol diff:

```text
(empty)
```

