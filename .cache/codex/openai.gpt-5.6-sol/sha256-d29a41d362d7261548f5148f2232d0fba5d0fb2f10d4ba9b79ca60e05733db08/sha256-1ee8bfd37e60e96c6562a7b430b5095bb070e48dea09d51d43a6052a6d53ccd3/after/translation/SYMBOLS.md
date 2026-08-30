# Dynamic Symbol Surface

Reference library: `../c_src/build/libdriver.so`

Command used:

```sh
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `bad` | `T` | `bad` | present |
| `driver` | `T` | `driver` | present |
| `good` | `T` | `good` | present |
| `printLine` | `T` | `printLine` | present |

- [x] Final `nm -D` parity check has zero missing C symbols.
- [x] Rust has zero undefined non-system/application symbols. Its undefined
  imports are limited to GLIBC, libgcc unwinding, and weak toolchain hooks.
