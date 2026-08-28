# Dynamic Symbol Surface

Source library: `../c_src/build/libharvest-work-cstVVS.so`

Inventory command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-cstVVS.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `ima_parse` | `T` | `ima_parse` | [x] present |

The C shared object has one defined public dynamic symbol. The Rust shared
object exports the same symbol from `target/release/libima_parse_lib.so`.
There are zero missing or undefined C API symbols. The Rust toolchain's normal
GLIBC and GCC runtime imports all resolve under `ldd -r`.
