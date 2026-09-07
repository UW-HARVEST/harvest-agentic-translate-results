# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

Undefined imports such as `puts@GLIBC_2.2.5` and weak ELF runtime hooks are not
library exports and are therefore not part of the public-symbol parity set.

| # | C symbol | C type | Rust type | status |
|---|----------|--------|-----------|--------|
| 1 | `bad` | `T` | `T` | [x] |
| 2 | `driver` | `T` | `T` | [x] |
| 3 | `good` | `T` | `T` | [x] |
| 4 | `printLine` | `T` | `T` | [x] |

- [x] Missing C-defined dynamic symbols in Rust: 0.
- [x] No missing symbol represents an untranslated C source module.
