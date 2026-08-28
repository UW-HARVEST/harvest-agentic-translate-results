# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-uUFhe9.so`

Inventory command:

```sh
nm -D --defined-only --format=posix \
  ../c_src/build/libharvest-work-uUFhe9.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `tfm` | `T` (global function) | `tfm` | Present |

The unfiltered C dynamic table also contains the imported libm symbol
`sqrtf@GLIBC_2.2.5` and weak ELF runtime symbols. These are dependencies, not
public symbols defined by the C library.

Missing C exports in Rust: **0**
