# Dynamic Symbol Surface

Source binary: `c_src/build/libtranslated_rust.so`

Command:

```sh
nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so
```

| C symbol | Type | Rust export | Status |
|----------|------|-------------|--------|
| `crc16` | `T` (global function) | `crc16` | [x] |

The C library has one defined public dynamic symbol. The missing-symbol diff is
empty:

```sh
comm -23 \
  <(nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so |
    awk '{print $3}' | sort -u) \
  <(nm -D --defined-only --extern-only target/debug/libcrc16_lib.so |
    awk '{print $3}' | sort -u)
```

- [x] Zero C public symbols are missing from the Rust shared object.
- [x] Zero undefined non-libc C-library symbols must be supplied by Rust.
