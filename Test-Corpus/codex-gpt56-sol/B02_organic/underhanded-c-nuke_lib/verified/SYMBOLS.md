# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command:

```text
nm -D c_src/build/libtranslated_rust.so
```

## Defined public symbols

| C symbol | nm type | Rust export | Status |
|----------|---------|-------------|--------|
| `match` | `T` | `match` | [x] |
| `spectral_contrast` | `T` | `spectral_contrast` | [x] |

The defined-symbol diff is empty for the only build configuration:
`--no-default-features --features ""`.

## Other dynamic-table entries

These are imported or weak runtime symbols, not API definitions from the C
library:

| Symbol | nm type | Provider |
|--------|---------|----------|
| `_ITM_deregisterTMCloneTable` | `w` | toolchain runtime |
| `_ITM_registerTMCloneTable` | `w` | toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc |
| `__gmon_start__` | `w` | toolchain runtime |
| `memcpy@GLIBC_2.14` | `U` | libc |
| `sqrt@GLIBC_2.2.5` | `U` | libm |

