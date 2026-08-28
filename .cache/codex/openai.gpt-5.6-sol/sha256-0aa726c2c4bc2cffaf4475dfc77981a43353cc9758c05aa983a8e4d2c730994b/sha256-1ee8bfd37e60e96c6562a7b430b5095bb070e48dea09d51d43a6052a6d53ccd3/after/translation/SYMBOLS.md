# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-TUVnNa.so`

Inventory command:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-TUVnNa.so
```

Only globally defined dynamic symbols are part of the C library's callable
surface. Undefined runtime imports and local ELF implementation symbols are
excluded.

| # | C symbol | C type | Rust symbol | Status |
|---|----------|--------|-------------|--------|
| 1 | `tflac_md5_addsample` | `T` | `tflac_md5_addsample` | [x] |
| 2 | `tflac_pack_u64le` | `T` | `tflac_pack_u64le` | [x] |
| 3 | `update_md5` | `T` | `update_md5` | [x] |

Missing C symbols in Rust: **0**.
