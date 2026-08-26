# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The C shared object exports four defined public symbols. The status column is
based on the same command against `target/debug/libgotomach_lib.so`.

| # | C symbol | Type | Rust export |
|---|----------|------|-------------|
| 1 | `double_value` | `T` | [x] |
| 2 | `gotomach` | `T` | [x] |
| 3 | `process_value` | `T` | [x] |
| 4 | `triple_value` | `T` | [x] |

Missing C symbols in Rust: **0**

The undefined entries in the unfiltered C `nm -D` output are runtime/libc
imports, not library exports.
