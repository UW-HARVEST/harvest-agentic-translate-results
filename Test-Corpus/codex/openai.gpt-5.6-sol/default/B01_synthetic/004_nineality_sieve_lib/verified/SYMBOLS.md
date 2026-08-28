# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libSieve.so
nm -D --defined-only target/release/libSieve.so
```

Only symbols defined by the library are API exports. Undefined libc/runtime
imports shown by unfiltered `nm -D` are not library-defined public symbols.

| C symbol | C type | Rust symbol | Rust type | Status |
|----------|--------|-------------|-----------|--------|
| `sieve` | `T` | `sieve` | `T` | [x] exact match |

Missing C symbols in Rust: **0**

