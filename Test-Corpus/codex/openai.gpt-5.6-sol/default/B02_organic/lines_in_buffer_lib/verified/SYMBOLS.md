# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Inventory command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `UTIL_createLinePointers` | `T` | `UTIL_createLinePointers` | present |

The C library's remaining `nm -D` entries are undefined libc/toolchain imports
(`malloc`, `free`, and weak ELF runtime hooks), not library API exports.

## Completion

- [x] C defined dynamic symbols: 1
- [x] Missing from Rust: 0
- [x] Additional Rust API exports: 0
- [x] Exact-name symbol diff is empty
