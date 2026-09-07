# Dynamic symbol parity

Mechanically extracted with `nm -D --defined-only` from:

- C: `../c_src/build/libharvest-work-9FYxtf.so`
- Rust: `target/release/libhelxo_lib.so`

| C symbol | C type | Rust export |
|----------|--------|-------------|
| `helxo` | `T` | present |
| `stbds_arrfreef` | `T` | present |
| `stbds_arrgrowf` | `T` | present |
| `stbds_hash_bytes` | `T` | present |
| `stbds_hash_string` | `T` | present |
| `stbds_hmdel_key` | `T` | present |
| `stbds_hmfree_func` | `T` | present |
| `stbds_hmget_key` | `T` | present |
| `stbds_hmget_key_ts` | `T` | present |
| `stbds_hmput_default` | `T` | present |
| `stbds_hmput_key` | `T` | present |
| `stbds_rand_seed` | `T` | present |
| `stbds_shmode_func` | `T` | present |
| `stbds_stralloc` | `T` | present |
| `stbds_strreset` | `T` | present |
| `strkey` | `T` | present |

- [x] Final `nm -D` diff is empty (16 C exports, 16 matching Rust exports).
- [x] Rust has no undefined non-libc project symbols.
