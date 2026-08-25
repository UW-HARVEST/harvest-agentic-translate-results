# Dynamic Symbol Surface

Derived with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The CMake file has no visibility filter or generated export list, so every
global definition below is part of the shared-library ABI. Macro-only names are
not symbols and do not appear in `nm -D`.

| # | C symbol | Rust implementation/export | parity |
|---|----------|----------------------------|--------|
| 1 | `stbds_arrfreef` | `src/lib.rs` | [x] |
| 2 | `stbds_arrgrowf` | `src/lib.rs` | [x] |
| 3 | `stbds_hash_bytes` | `src/lib.rs` | [x] |
| 4 | `stbds_hash_string` | `src/lib.rs` | [x] |
| 5 | `stbds_hmdel_key` | `src/lib.rs` | [x] |
| 6 | `stbds_hmfree_func` | `src/lib.rs` | [x] |
| 7 | `stbds_hmget_key` | `src/lib.rs` | [x] |
| 8 | `stbds_hmget_key_ts` | `src/lib.rs` | [x] |
| 9 | `stbds_hmput_default` | `src/lib.rs` | [x] |
| 10 | `stbds_hmput_key` | `src/lib.rs` | [x] |
| 11 | `stbds_rand_seed` | `src/lib.rs` | [x] |
| 12 | `stbds_shmode_func` | `src/lib.rs` | [x] |
| 13 | `stbds_stralloc` | `src/lib.rs` | [x] |
| 14 | `stbds_strreset` | `src/lib.rs` | [x] |
| 15 | `str_put` | `src/lib.rs` | [x] |
| 16 | `strkey` | `src/lib.rs` | [x] |

The C object also has normal undefined runtime references to glibc
(`free`, `malloc`, `memcmp`, `memcpy`, `memmove`, `memset`, `printf`,
`realloc`, `sprintf`, `strcmp`, `strlen`, and `__assert_fail`) plus weak ELF
runtime hooks. These are dependencies, not library exports.

Feature combinations from `Cargo.toml` and `c_src/CMakeLists.txt`:

| # | Cargo features | CMake definitions | valid |
|---|----------------|-------------------|-------|
| 1 | no features (`--no-default-features`) | none | yes |
