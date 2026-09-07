# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `parse_number` | `T` (global text) | `parse_number` | [x] |

The C library's undefined symbols are `malloc`, `free`, `memcpy`, and `strtod`
from libc, plus weak compiler/runtime hooks. They are dependencies rather than
public API exports and therefore are not parity targets.
