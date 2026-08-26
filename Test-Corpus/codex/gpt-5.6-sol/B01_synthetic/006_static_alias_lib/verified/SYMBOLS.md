# Dynamic Symbol Surface

Derived from:

```sh
nm -D c_src/build/libStaticAlias.so
nm -D --defined-only c_src/build/libStaticAlias.so
```

The C shared object has two globally defined dynamic symbols. Both are public
API declarations in `c_src/include/staticalias.h`.

| C symbol | C ELF type | Header declaration | Rust ELF type | Status |
|----------|------------|--------------------|---------------|--------|
| `driver` | `T` | `void driver(int initial_value, int iterations)` | `T` | present |
| `static_alias` | `T` | `int *static_alias(int *outer)` | `T` | present |

Exact defined-symbol diff: empty.

The other C dynamic-table entries are undefined runtime/toolchain imports, not
library exports: `printf@GLIBC_2.2.5`, `__cxa_finalize@GLIBC_2.2.5`,
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, and
`__gmon_start__`.

