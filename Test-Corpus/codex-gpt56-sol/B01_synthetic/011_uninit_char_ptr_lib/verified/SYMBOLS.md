# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The C shared object has four globally defined public functions. The Rust
comparison object was built directly from `src/lib.rs` as a `cdylib`.

| symbol | C `nm` type | Rust defined | parity |
|--------|-------------|--------------|--------|
| `bad` | `T` | yes | [x] |
| `driver` | `T` | yes | [x] |
| `good` | `T` | yes | [x] |
| `printLine` | `T` | yes | [x] |

The C object's undefined dynamic symbols are the libc function `puts` plus the
weak ELF runtime hooks `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. They are
imports, not library API exports.

Missing C exports in Rust: **0**
