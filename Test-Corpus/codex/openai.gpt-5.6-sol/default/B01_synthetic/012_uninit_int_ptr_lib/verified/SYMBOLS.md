# Dynamic Symbol Surface

Source: `nm -D --defined-only ../c_src/build/libdriver.so`, restricted to the
library's externally callable API functions.

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `bad` | `bad` | [x] |
| `driver` | `driver` | [x] |
| `good` | `good` | [x] |
| `printIntPtrLine` | `printIntPtrLine` | [x] |

Phase D symbol-diff command:

```sh
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '$2 ~ /^[TW]$/ {print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[TW]$/ {print $3}' | sort -u)
```

Final result: 4 C API symbols, 4 matching Rust exports, 0 missing.
