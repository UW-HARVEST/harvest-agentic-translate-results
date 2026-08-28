# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-IWODxW.so`

Rust library:
`target/release/libdataentry_lib.so`

The export list was generated with:

```sh
nm -D --defined-only <library> | awk '$2 ~ /^[TDBRWSV]$/ { print $3 }'
```

Undefined libc/toolchain imports and weak runtime hooks are not library
exports.

| C symbol | C type | Rust type | Status |
|----------|--------|-----------|--------|
| `dataentry` | `T` | `T` | [x] exact export present |

Missing C exports in Rust: **0**

