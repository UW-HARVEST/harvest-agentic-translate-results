# Dynamic Symbol Surface

Generated from:

```sh
nm -D c_src/build/libdriver.so
nm -D --defined-only c_src/build/libdriver.so
```

## Defined public exports

| C symbol | C type | Rust export present |
|----------|--------|---------------------|
| `driver` | `T` | [x] |

The C shared object has no other defined public dynamic symbols.

## Undefined runtime imports

These entries appear in the full `nm -D` output but are dependencies, not
library exports.

| Symbol | Type | Classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | compiler runtime |
| `_ITM_registerTMCloneTable` | `w` | compiler runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc runtime |
| `__gmon_start__` | `w` | compiler runtime |
| `printf@GLIBC_2.2.5` | `U` | libc |
| `putchar@GLIBC_2.2.5` | `U` | libc |

## Parity command

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

