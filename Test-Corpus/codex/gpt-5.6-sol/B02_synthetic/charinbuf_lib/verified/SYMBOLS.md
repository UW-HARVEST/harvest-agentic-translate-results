# Exported Symbol Surface

Source of truth:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The CMake configuration has no options. `Cargo.toml` has no `[features]`
table or optional dependencies, so the only valid Rust feature combination is
the empty set (`--no-default-features`).

| C symbol | Rust symbol | Status |
|----------|-------------|--------|
| `apply_operation` | `apply_operation` | present |
| `charinbuf` | `charinbuf` | present |
| `create_buffer` | `create_buffer` | present |
| `decrement_counter` | `decrement_counter` | present |
| `find_char_in_buffer` | `find_char_in_buffer` | present |
| `increment_counter` | `increment_counter` | present |
| `is_string_empty` | `is_string_empty` | present |
| `multiply_counter` | `multiply_counter` | present |
| `reset_counter` | `reset_counter` | present |
| `validate_uint16_range` | `validate_uint16_range` | present |

Missing C symbols in Rust: **0**

Undefined non-libc C symbols in Rust: **0**

## Completion Gate

- [x] C shared library rebuilt from the default CMake configuration.
- [x] Rust checked, release-built, and tested with the empty feature set.
- [x] All 10 C exports are present in Rust with exact names.
- [x] Exact defined-symbol diff is empty.
- [x] `ldd -r` reports no unresolved Rust shared-library symbols.
