# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only --extern-only ../c_src/build/libharvest-work-oGS0vJ.so
nm -D --defined-only --extern-only target/release/libjumpnode_lib.so
```

| symbol | C type | Rust type | parity |
|--------|--------|-----------|--------|
| `jumpnode` | `T` | `T` | [x] |

The C library has no other defined dynamic symbols. Its undefined entries
(`sprintf`, `sqrt`, `strlen`, and ELF runtime hooks) are system-library imports,
not public library exports or unresolved project symbols.

Completion status: **0 C API symbols missing from Rust**.
