# Dynamic Symbol Surface

Source library: `c_src/build/libtranslated_rust.so`

Command:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| # | C symbol | Rust `target/release/libstr_dups_lib.so` |
|---|----------|-------------------------------------------|
| 1 | `stbds_arrfreef` | present |
| 2 | `stbds_arrgrowf` | present |
| 3 | `stbds_hash_bytes` | present |
| 4 | `stbds_hash_string` | present |
| 5 | `stbds_hmdel_key` | present |
| 6 | `stbds_hmfree_func` | present |
| 7 | `stbds_hmget_key` | present |
| 8 | `stbds_hmget_key_ts` | present |
| 9 | `stbds_hmput_default` | present |
| 10 | `stbds_hmput_key` | present |
| 11 | `stbds_rand_seed` | present |
| 12 | `stbds_shmode_func` | present |
| 13 | `stbds_stralloc` | present |
| 14 | `stbds_strreset` | present |
| 15 | `str_dups` | present |
| 16 | `strkey` | present |

Missing C-defined symbols in Rust: **0**

The undefined entries reported by `nm -D --undefined-only` are libc,
libgcc/unwind, pthread, and dynamic-loader imports. There are no unresolved
references to symbols defined by this library.

- [x] Symbol completion gate: 0 missing C-defined symbols in Rust.
