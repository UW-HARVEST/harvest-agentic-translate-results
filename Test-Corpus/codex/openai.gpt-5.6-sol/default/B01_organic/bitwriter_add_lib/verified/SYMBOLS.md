# Dynamic symbol surface

Generated from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-UTQhvA.so
nm -D --defined-only target/release/libbitwriter_add_lib.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `bitwriter_add` | `T` | `bitwriter_add` (`T`) | [x] present |

## Symbol parity

- C public dynamic symbols: 1
- Rust matching public dynamic symbols: 1
- Missing from Rust: 0
- Extra application API symbols in Rust: 0
- Undefined non-system API symbols in Rust: 0
