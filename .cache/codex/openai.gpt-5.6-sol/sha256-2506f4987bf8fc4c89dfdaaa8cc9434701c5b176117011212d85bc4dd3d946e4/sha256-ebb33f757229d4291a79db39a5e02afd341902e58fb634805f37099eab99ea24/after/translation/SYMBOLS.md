# Dynamic symbol surface

Mechanically derived with:

```text
nm -D ../c_src/build/libStaticAlias.so
nm -D target/release/libStaticAlias.so
```

The C shared object has the following defined public dynamic symbols. Runtime
imports (`printf`) and weak ELF/toolchain hooks are recorded below but are not
library API exports.

| C symbol | C type | Rust type | parity |
|----------|--------|-----------|--------|
| `driver` | `T` | `T` | [x] |
| `static_alias` | `T` | `T` | [x] |

Missing defined C symbols in Rust: **0**.

## C runtime imports and weak hooks

| symbol | `nm -D` type | classification |
|--------|--------------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain hook |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc runtime hook |
| `__gmon_start__` | `w` | weak profiling hook |
| `printf@GLIBC_2.2.5` | `U` | libc import used by `driver` |

There are no undefined non-libc application symbols in the C shared object.
