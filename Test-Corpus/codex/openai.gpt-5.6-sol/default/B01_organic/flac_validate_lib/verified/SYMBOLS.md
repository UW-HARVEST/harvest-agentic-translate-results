# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-d9nDAf.so
```

Only global text symbols (`T`) are public API symbols. Toolchain-generated
local/runtime symbols are not reported by `nm -D --defined-only`.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `flac_validate` | `T` | `flac_validate` | present |
| `tflac_size_memory` | `T` | `tflac_size_memory` | present |

Missing C symbols in Rust: **0**.
