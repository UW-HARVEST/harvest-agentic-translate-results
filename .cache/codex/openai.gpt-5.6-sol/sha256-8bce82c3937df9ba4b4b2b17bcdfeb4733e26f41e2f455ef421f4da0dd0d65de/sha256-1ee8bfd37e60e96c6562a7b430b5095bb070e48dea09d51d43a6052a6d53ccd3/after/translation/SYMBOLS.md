# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

Undefined imports such as `printf`, `putchar`, and ELF runtime support symbols
are not library exports and are therefore excluded from the parity surface.

| # | C symbol | C type | Rust symbol | status |
|---|----------|--------|-------------|--------|
| 1 | `driver` | `T` (global function) | `driver` | [x] present |

The C library has one defined dynamic symbol. The Rust library exports the same
symbol with the exact name.

Phase D status: [x] complete. The sorted `nm -D --defined-only` C-to-Rust
symbol diff is empty under both the default and empty no-default feature
configurations, and `ldd -r` reports no unresolved symbols.
