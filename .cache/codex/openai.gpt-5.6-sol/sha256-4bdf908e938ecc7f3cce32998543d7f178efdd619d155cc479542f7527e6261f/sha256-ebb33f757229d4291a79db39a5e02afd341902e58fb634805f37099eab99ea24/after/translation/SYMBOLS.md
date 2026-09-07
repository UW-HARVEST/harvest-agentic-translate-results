# Dynamic symbol surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

The C shared object exports exactly these public API symbols. The Rust status
was checked against the release `cdylib`.

| C symbol | C type | Rust export |
|---|---|---|
| `add_task` | `T` | [x] exact match |
| `create_task_manager` | `T` | [x] exact match |
| `destroy_task_manager` | `T` | [x] exact match |
| `driver` | `T` | [x] exact match |
| `finalize_logger` | `T` | [x] exact match |
| `initialize_logger` | `T` | [x] exact match |
| `log_error` | `T` | [x] exact match |
| `log_info` | `T` | [x] exact match |
| `log_warning` | `T` | [x] exact match |
| `print_tasks` | `T` | [x] exact match |

Missing C exports in Rust: **0**.

Undefined symbols are implementation/runtime imports rather than public
library symbols. `ldd -r` reports no unresolved symbols for either shared
object under both tested Cargo configurations.
