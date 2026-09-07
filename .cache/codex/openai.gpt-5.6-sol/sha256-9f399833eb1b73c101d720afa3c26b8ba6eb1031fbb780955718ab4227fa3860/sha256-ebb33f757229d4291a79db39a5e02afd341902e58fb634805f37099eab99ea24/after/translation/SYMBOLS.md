# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `searchAndReplace` | `T` (global function) | `searchAndReplace` | [x] present |

The C shared library has no other defined public dynamic symbols. Its remaining
`nm -D` entries are undefined libc functions or weak toolchain/runtime symbols,
not library exports.

Final `comm -3` comparison of the two `nm -D --defined-only` symbol-name lists:
empty (exact parity).
