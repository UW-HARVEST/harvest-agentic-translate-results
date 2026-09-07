# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-6bgy6Y.so
```

| C address | type | symbol | Rust export | status |
|-----------|------|--------|-------------|--------|
| `0000000000001670` | `T` | `tritanopia` | `tritanopia` | [x] |

The complete C dynamic table also contains weak toolchain symbols
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`, plus the undefined system-library dependency `pow`.
These are not library API exports. The only defined public C symbol is
`tritanopia`.

Completion check: [x] zero C API symbols are missing from the Rust shared
object.
