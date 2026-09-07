# Dynamic symbol surface

Mechanically collected with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-6tAMOr.so
nm -D --defined-only target/release/libsh_geti_lib.so
```

| C symbol | Rust export |
|---|---|
| `sh_geti` | present |
| `stbds_arrfreef` | present |
| `stbds_arrgrowf` | present |
| `stbds_hash_bytes` | present |
| `stbds_hash_string` | present |
| `stbds_hmdel_key` | present |
| `stbds_hmfree_func` | present |
| `stbds_hmget_key` | present |
| `stbds_hmget_key_ts` | present |
| `stbds_hmput_default` | present |
| `stbds_hmput_key` | present |
| `stbds_rand_seed` | present |
| `stbds_shmode_func` | present |
| `stbds_stralloc` | present |
| `stbds_strreset` | present |
| `strkey` | present |

Missing C exports in Rust: **0**

Undefined non-libc C-library symbols in Rust: **0**
