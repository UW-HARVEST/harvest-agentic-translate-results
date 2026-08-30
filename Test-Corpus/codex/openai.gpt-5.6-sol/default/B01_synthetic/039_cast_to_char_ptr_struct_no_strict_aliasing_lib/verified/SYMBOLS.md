# Dynamic Symbol Surface

Reference library: `../c_src/build/libdriver.so`

Inventory command:

```sh
nm -D ../c_src/build/libdriver.so
```

## Defined public API

| symbol | C `nm -D` type | Rust export | status |
|--------|----------------|-------------|--------|
| `driver` | `T` | `driver` (`T`) | present |

The defined-symbol diff is empty:

```sh
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

## Non-API dynamic entries

These entries appear in the full C dynamic table but are not definitions
exported by this library:

| symbol | type | classification |
|--------|------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | weak toolchain hook |
| `_ITM_registerTMCloneTable` | `w` | weak toolchain hook |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak libc runtime hook |
| `__gmon_start__` | `w` | weak toolchain hook |
| `printf@GLIBC_2.2.5` | `U` | libc import |
| `putchar@GLIBC_2.2.5` | `U` | libc import |

No macro-generated exports or additional public entry points exist in the C
source.

## Completion

- [x] Every symbol defined by the C `.so` is defined by the Rust `.so` with
  the exact same name.
- [x] The C-to-Rust defined-symbol diff contains zero entries.
- [x] The Rust `driver` symbol is defined, not undefined.
