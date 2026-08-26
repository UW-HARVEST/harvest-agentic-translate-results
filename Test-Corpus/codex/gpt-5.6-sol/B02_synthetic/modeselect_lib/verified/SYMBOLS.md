# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `apply_multiplier` | `T` | `apply_multiplier` | present |
| `classify_mode` | `T` | `classify_mode` | present |
| `convert_negative_overflow` | `T` | `convert_negative_overflow` | present |
| `convert_time_factor` | `T` | `convert_time_factor` | present |
| `get_modified_time` | `T` | `get_modified_time` | present |
| `hash_time_value` | `T` | `hash_time_value` | present |
| `modeselect` | `T` | `modeselect` | present |

The complete `nm -D` output also contains weak ELF runtime symbols and the
undefined libc imports `printf`, `strcmp`, and `time`. They are dynamic
dependencies, not symbols defined by this library.

Missing C-defined symbols in Rust: **0**

