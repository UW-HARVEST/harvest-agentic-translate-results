# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only --format=posix ../c_src/build/libmdcore.so
```

Default C configuration: `OP=add`, `REPEAT=5`.

| C symbol | kind | Rust export |
|----------|------|-------------|
| `G_OP` | global function pointer | [x] |
| `G_OP_NAME` | global C-string pointer | [x] |
| `helper_call` | function | [x] |
| `helper_ptr` | function | [x] |
| `op_add` | function | [x] |
| `op_mul` | function | [x] |
| `op_sub` | function | [x] |
| `use_generated` | function | [x] |

The C object also has the following undefined dynamic runtime symbols:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`, and `printf`. These are libc/toolchain imports, not library
exports.

Phase A default symbol diff: empty.

Phase D symbol diff: empty for all 24 semantic operation/repeat configurations
under all 36 valid Cargo feature combinations.
