# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Command:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

| symbol | type | Rust parity |
|--------|------|-------------|
| `main` | `T` | [x] |

The C object also has undefined dynamic references to the libc functions
`__isoc99_scanf`, `printf`, and `puts`, plus weak ELF runtime bookkeeping
symbols. These are imports, not public library exports, so they are not part of
the parity table.
