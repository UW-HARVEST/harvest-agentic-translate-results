# Dynamic symbol surface

Source library:
`../c_src/build/libharvest-work-uc2QuJ.so`

Rust library:
`target/release/libima_parse_lib.so`

The public API list is the set of globally defined dynamic symbols reported by
`nm -D --defined-only`. Undefined C runtime/toolchain symbols are not library
API symbols.

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `ima_parse` | `T` | `T` | [x] exact export present |

Normalized symbol diff:

```text
(empty)
```

The C library also has the usual weak undefined toolchain hooks
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
and `__gmon_start__`; these are imports, not definitions supplied by this
library.
