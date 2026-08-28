# Dynamic Symbol Surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-exQmBS.so
```

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `arr_push` | `arr_push` | [x] |
| `stbds_arrfreef` | `stbds_arrfreef` | [x] |
| `stbds_arrgrowf` | `stbds_arrgrowf` | [x] |
| `stbds_hash_bytes` | `stbds_hash_bytes` | [x] |
| `stbds_hash_string` | `stbds_hash_string` | [x] |
| `stbds_hmdel_key` | `stbds_hmdel_key` | [x] |
| `stbds_hmfree_func` | `stbds_hmfree_func` | [x] |
| `stbds_hmget_key` | `stbds_hmget_key` | [x] |
| `stbds_hmget_key_ts` | `stbds_hmget_key_ts` | [x] |
| `stbds_hmput_default` | `stbds_hmput_default` | [x] |
| `stbds_hmput_key` | `stbds_hmput_key` | [x] |
| `stbds_rand_seed` | `stbds_rand_seed` | [x] |
| `stbds_shmode_func` | `stbds_shmode_func` | [x] |
| `stbds_stralloc` | `stbds_stralloc` | [x] |
| `stbds_strreset` | `stbds_strreset` | [x] |
| `strkey` | `strkey` | [x] |

Missing C symbols in Rust: **0**.

`stbds_unit_tests` is declared `extern` in the C source but has no definition
and is not present in the C dynamic symbol table, so it is not part of the
shared-library ABI.
