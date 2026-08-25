# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command used:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust symbol | Status |
|----------|------|-------------|--------|
| `max_size_frame` | `T` | `max_size_frame` | [x] |

The C library has one defined public dynamic symbol. The undefined weak
toolchain symbols shown by plain `nm -D` are not library exports.

Final defined-symbol difference: empty.
