# Dynamic Symbol Surface

Source artifact: `c_src/build/libtranslated_rust.so`

Command:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust status |
|----------|------|-------------|
| `add_to_accumulator` | `T` | Present |
| `divide_multiplier` | `T` | Present |
| `find_and_replace_char` | `T` | Present |
| `findrep` | `T` | Present |
| `multiply_with_multiplier` | `T` | Present |
| `process_octal_string` | `T` | Present |
| `subtract_from_accumulator` | `T` | Present |
| `validate_and_normalize` | `T` | Present |

The C library's undefined dynamic symbols are libc/toolchain imports:
`memchr`, `sprintf`, `strcpy`, `strlen`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.
There are no undefined project symbols.

Feature combinations:

| # | Cargo feature selection | CMake configuration |
|---|-------------------------|---------------------|
| 1 | `--no-default-features` (empty feature set) | Default; CMake defines no options |

The manifest has no `[features]` table, so the empty set is the only valid
feature combination.
