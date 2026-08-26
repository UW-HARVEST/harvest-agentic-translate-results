# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libtranslated_rust.so`.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `sh_geti` | present |
| 2 | `stbds_arrfreef` | present |
| 3 | `stbds_arrgrowf` | present |
| 4 | `stbds_hash_bytes` | present |
| 5 | `stbds_hash_string` | present |
| 6 | `stbds_hmdel_key` | present |
| 7 | `stbds_hmfree_func` | present |
| 8 | `stbds_hmget_key` | present |
| 9 | `stbds_hmget_key_ts` | present |
| 10 | `stbds_hmput_default` | present |
| 11 | `stbds_hmput_key` | present |
| 12 | `stbds_rand_seed` | present |
| 13 | `stbds_shmode_func` | present |
| 14 | `stbds_stralloc` | present |
| 15 | `stbds_strreset` | present |
| 16 | `strkey` | present |

Missing from Rust: **0**.

Undefined non-libc symbols in Rust: **0**.

