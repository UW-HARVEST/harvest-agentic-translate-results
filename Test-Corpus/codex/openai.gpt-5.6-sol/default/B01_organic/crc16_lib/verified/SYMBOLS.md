# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-9JQpsO.so`

Rust library:
`target/release/libcrc16_lib.so`

Mechanically extracted with:

```text
nm -D --defined-only <library> | awk '$2 ~ /^[TDBRWSV]$/ { print $3 }' | sort -u
```

| C symbol | C type | Rust symbol present | Rust type |
|----------|--------|---------------------|-----------|
| `crc16` | `T` | yes | `T` |

Missing C symbols in Rust: **0**

The remaining undefined dynamic symbols in each shared object are platform
runtime/libc/loader dependencies rather than library API symbols.
