# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/release/libcomplexmode_lib.so
```

The C shared object exports seven public functions. The default Rust shared
object currently exports every one with the exact same name.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `check_permissions` | `T` | `check_permissions` | [x] |
| 2 | `compare_operations` | `T` | `compare_operations` | [x] |
| 3 | `complexmode` | `T` | `complexmode` | [x] |
| 4 | `copy_and_sum` | `T` | `copy_and_sum` | [x] |
| 5 | `create_result_string` | `T` | `create_result_string` | [x] |
| 6 | `multiply_with_log` | `T` | `multiply_with_log` | [x] |
| 7 | `safe_add` | `T` | `safe_add` | [x] |

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so the feature power set has one valid
combination:

| # | Cargo features | Check/test arguments |
|---|----------------|----------------------|
| 1 | none | `--no-default-features` |

`c_src/CMakeLists.txt` declares no options or conditional definitions. It has
one configuration: a shared library containing `src/lib.c`.

## Completion Checks

- [x] C-to-Rust defined dynamic symbol diff is empty for every feature combination.
- [x] Rust has no undefined non-system symbols for every feature combination.

Verified with the sole feature combination using `cargo check`, `cargo build
--release`, and `cargo test`, all with `--no-default-features`. The final
defined-symbol `comm` diff was empty, both libraries exported seven functions,
and `ldd -r` reported no unresolved symbols.
