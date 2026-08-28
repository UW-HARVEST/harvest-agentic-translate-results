# Exported Symbol Surface

Derived with:

```sh
nm -D --defined-only --format=posix \
  ../c_src/build/libharvest-work-pC0BvO.so
```

The C library imports only `printf`, `strcmp`, and `time` from libc (plus weak
ELF runtime symbols). The table lists every symbol defined in the C shared
object and the corresponding Rust shared-object status.

| # | C symbol | C type | Rust export |
|---|----------|--------|-------------|
| 1 | `apply_multiplier` | `T` | [x] |
| 2 | `classify_mode` | `T` | [x] |
| 3 | `convert_negative_overflow` | `T` | [x] |
| 4 | `convert_time_factor` | `T` | [x] |
| 5 | `get_modified_time` | `T` | [x] |
| 6 | `hash_time_value` | `T` | [x] |
| 7 | `modeselect` | `T` | [x] |

Missing C-defined symbols in Rust: **0**.

