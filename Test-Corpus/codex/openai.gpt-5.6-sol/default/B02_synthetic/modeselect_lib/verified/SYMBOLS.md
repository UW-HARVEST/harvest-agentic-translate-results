# Dynamic Symbol Surface

Derived mechanically with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-NhGLyw.so
nm -D --defined-only target/release/libmodeselect_lib.so
```

Only globally defined function symbols (`T`/`W`) are part of this table.
Undefined libc/toolchain symbols are not library API symbols.

| # | C symbol | Rust symbol present | Differential coverage |
|---|----------|---------------------|-----------------------|
| 1 | `apply_multiplier` | yes | `tests/differential.rs` |
| 2 | `classify_mode` | yes | `tests/differential.rs` |
| 3 | `convert_negative_overflow` | yes | `tests/differential.rs` |
| 4 | `convert_time_factor` | yes | `tests/differential.rs` |
| 5 | `get_modified_time` | yes | `tests/differential.rs` |
| 6 | `hash_time_value` | yes | `tests/differential.rs` |
| 7 | `modeselect` | yes | `tests/differential.rs` |

Missing C symbols in Rust: **0**.

