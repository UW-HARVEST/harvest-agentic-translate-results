# Dynamic symbol surface

Source command:

```text
nm -D --defined-only ../c_src/build/libharvest-work-g09eEL.so
```

Only defined public symbols are part of the implementation surface; undefined
libc references are recorded separately below.

| # | C symbol | C type | Rust export | Status |
|---|----------|--------|-------------|--------|
| 1 | `helxo` | `T` | `helxo` | present |
| 2 | `stbds_arrfreef` | `T` | `stbds_arrfreef` | present |
| 3 | `stbds_arrgrowf` | `T` | `stbds_arrgrowf` | present |
| 4 | `stbds_hash_bytes` | `T` | `stbds_hash_bytes` | present |
| 5 | `stbds_hash_string` | `T` | `stbds_hash_string` | present |
| 6 | `stbds_hmdel_key` | `T` | `stbds_hmdel_key` | present |
| 7 | `stbds_hmfree_func` | `T` | `stbds_hmfree_func` | present |
| 8 | `stbds_hmget_key` | `T` | `stbds_hmget_key` | present |
| 9 | `stbds_hmget_key_ts` | `T` | `stbds_hmget_key_ts` | present |
| 10 | `stbds_hmput_default` | `T` | `stbds_hmput_default` | present |
| 11 | `stbds_hmput_key` | `T` | `stbds_hmput_key` | present |
| 12 | `stbds_rand_seed` | `T` | `stbds_rand_seed` | present |
| 13 | `stbds_shmode_func` | `T` | `stbds_shmode_func` | present |
| 14 | `stbds_stralloc` | `T` | `stbds_stralloc` | present |
| 15 | `stbds_strreset` | `T` | `stbds_strreset` | present |
| 16 | `strkey` | `T` | `strkey` | present |

Missing C symbols in Rust: **0**

The C shared object has these undefined runtime dependencies: `__assert_fail`,
`free`, `malloc`, `memmove`, `printf`, `realloc`, `sprintf`, `strcmp`, and
`strlen`. They are libc calls, not library exports that Rust must define.

- [x] Every C-defined dynamic symbol is exported by the Rust shared object.
- [x] There are no undefined non-libc implementation symbols in Rust.
