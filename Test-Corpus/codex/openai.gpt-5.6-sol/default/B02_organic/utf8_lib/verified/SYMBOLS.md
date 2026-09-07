# Dynamic symbol surface

Source: `nm -D --defined-only ../c_src/build/libdriver.so`.

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `w_utf8_drop` | `T` | `w_utf8_drop` | [x] |
| `w_utf8_filter` | `T` | `w_utf8_filter` | [x] |

The C library exports no other defined dynamic symbols. Phase D result:
`missing_count=0`, `extra_count=0`; `ldd -r` reports no unresolved symbols.
