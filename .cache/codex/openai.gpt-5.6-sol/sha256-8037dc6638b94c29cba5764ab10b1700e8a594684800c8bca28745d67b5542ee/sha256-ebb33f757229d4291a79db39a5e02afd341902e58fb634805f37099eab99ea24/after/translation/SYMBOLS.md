# Exported symbol surface

Source library: `../c_src/build/libharvest-work-i1tAuE.so`

Rust library: `target/release/libsh_puts_lib.so`

The list below is the mechanically extracted set from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-i1tAuE.so
```

Only defined public symbols (`T`, `D`, `B`, `R`, `W`, or `V`) are part of this
library surface; undefined libc imports are not library exports.

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `sh_puts` | `sh_puts` | present |
| `stbds_arrfreef` | `stbds_arrfreef` | present |
| `stbds_arrgrowf` | `stbds_arrgrowf` | present |
| `stbds_hash_bytes` | `stbds_hash_bytes` | present |
| `stbds_hash_string` | `stbds_hash_string` | present |
| `stbds_hmdel_key` | `stbds_hmdel_key` | present |
| `stbds_hmfree_func` | `stbds_hmfree_func` | present |
| `stbds_hmget_key` | `stbds_hmget_key` | present |
| `stbds_hmget_key_ts` | `stbds_hmget_key_ts` | present |
| `stbds_hmput_default` | `stbds_hmput_default` | present |
| `stbds_hmput_key` | `stbds_hmput_key` | present |
| `stbds_rand_seed` | `stbds_rand_seed` | present |
| `stbds_shmode_func` | `stbds_shmode_func` | present |
| `stbds_stralloc` | `stbds_stralloc` | present |
| `stbds_strreset` | `stbds_strreset` | present |
| `strkey` | `strkey` | present |

Missing C exports in Rust: **0**.

Cargo feature declarations: **none**. The only feature configuration is the
empty/default feature set; `--no-default-features` is therefore equivalent.

## Completion gate

- [x] `nm -D` has zero missing C exports in Rust.
- [x] All `CONFIGS.md` rows pass randomized differential tests.
- [x] `sh_puts` stdout matches byte-for-byte.
- [x] All `ERRORS.md` rows are covered by differential tests.
- [x] Default and `--no-default-features` configurations pass.
