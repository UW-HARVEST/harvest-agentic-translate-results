# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-Dl8FUR.so
```

Toolchain-provided undefined weak symbols are not public library definitions
and are excluded. The C library has one defined dynamic symbol.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `md5_digest` | `T` | `md5_digest` | [x] present |

Missing C symbols in Rust: **0**

Verified for the sole Cargo feature configuration (no features are declared),
both with default feature handling and `--no-default-features`. Rust's
undefined dynamic entries are standard `libc`/`libgcc_s` runtime dependencies;
there are no unresolved application/library symbols.
