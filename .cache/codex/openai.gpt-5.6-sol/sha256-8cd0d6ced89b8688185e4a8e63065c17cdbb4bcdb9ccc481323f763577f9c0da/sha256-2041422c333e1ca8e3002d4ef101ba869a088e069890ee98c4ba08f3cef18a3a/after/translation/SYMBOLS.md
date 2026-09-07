# Dynamic symbol surface

Source artifact:
`c_src/build/libmd_driver_add_5.so`, built from both source files with
`cc -shared -fPIC -DOP=add -DREPEAT=5`.

Command used to derive the table:

```text
nm -D --defined-only c_src/build/libmd_driver_add_5.so
```

| C symbol | kind | Rust `.so` status | reason/action when missing |
|----------|------|-------------------|----------------------------|
| `G_OP` | data | [x] | — |
| `G_OP_NAME` | data | [x] | — |
| `helper_call` | function | [x] | — |
| `helper_ptr` | function | [x] | — |
| `main` | function | [x] | Added a real C-ABI export in `src/lib.rs` backed by the translated implementation. |
| `op_add` | function | [x] | — |
| `op_mul` | function | [x] | — |
| `op_sub` | function | [x] | — |
| `use_generated` | function | [x] | — |

Undefined dynamic symbols in the C object are libc/toolchain imports, not
library exports: `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`, `atoi`, `fprintf`, `printf`, `stderr`.

Final verification: defined-symbol diffs were empty for all 24 effective
`OP × REPEAT` configurations.
