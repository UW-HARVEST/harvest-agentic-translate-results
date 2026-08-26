# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libdriver.so`, built from the
unmodified C sources with the default CMake configuration.

There are no CMake options and `Cargo.toml` has no `[features]` table. The only
valid build-time configuration is therefore `--no-default-features` with an
empty feature set.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `FreeAlertData` | `T` | `FreeAlertData` | [x] |
| 2 | `GetAlertData` | `T` | `GetAlertData` | [x] |
| 3 | `Init_FileQueue` | `T` | `Init_FileQueue` | [x] |
| 4 | `Read_FileMon` | `T` | `Read_FileMon` | [x] |
| 5 | `driver` | `T` | `driver` | [x] |
| 6 | `merror` | `T` | `merror` | [x] |
| 7 | `os_calloc` | `T` | `os_calloc` | [x] |
| 8 | `os_realloc` | `T` | `os_realloc` | [x] |
| 9 | `os_strdup` | `T` | `os_strdup` | [x] |

Missing C symbols in Rust: **0**.

