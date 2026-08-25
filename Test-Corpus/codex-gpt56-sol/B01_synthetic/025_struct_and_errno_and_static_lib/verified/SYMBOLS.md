# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

| C address | type | symbol | Rust export | status |
|-----------|------|--------|-------------|--------|
| `00000000000012c8` | `T` | `driver` | `driver` | present |
| `00000000000011cf` | `T` | `run` | `run` | present |

The C shared object has no undefined non-libc library dependencies. Its
undefined dynamic symbols are glibc functions (`__errno_location`, `printf`,
`puts`, and `strtol`) plus weak ELF runtime hooks.

Missing C exports in the Rust shared object: **0**.
