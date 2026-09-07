# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-tFTesG.so   | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libarr_push_lib.so | awk '{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra in Rust
```

The C library is built from a single translation unit (`c_src/src/lib.c`, an
amalgamation of `stb_ds.h` plus two test helpers). `c_src/CMakeLists.txt`
declares no other sources, so this is the complete surface.

## Symbol table

| # | C symbol (`nm -D`) | exported by Rust `.so` | Rust item |
|---|--------------------|------------------------|-----------|
| 1 | `arr_push` | yes | `pub unsafe extern "C" fn arr_push` |
| 2 | `stbds_arrfreef` | yes | `pub unsafe extern "C" fn stbds_arrfreef` |
| 3 | `stbds_arrgrowf` | yes | `pub unsafe extern "C" fn stbds_arrgrowf` |
| 4 | `stbds_hash_bytes` | yes | `pub unsafe extern "C" fn stbds_hash_bytes` |
| 5 | `stbds_hash_string` | yes | `pub unsafe extern "C" fn stbds_hash_string` |
| 6 | `stbds_hmdel_key` | yes | `pub unsafe extern "C" fn stbds_hmdel_key` |
| 7 | `stbds_hmfree_func` | yes | `pub unsafe extern "C" fn stbds_hmfree_func` |
| 8 | `stbds_hmget_key` | yes | `pub unsafe extern "C" fn stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts` | yes | `pub unsafe extern "C" fn stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default` | yes | `pub unsafe extern "C" fn stbds_hmput_default` |
| 11 | `stbds_hmput_key` | yes | `pub unsafe extern "C" fn stbds_hmput_key` |
| 12 | `stbds_rand_seed` | yes | `pub unsafe extern "C" fn stbds_rand_seed` |
| 13 | `stbds_shmode_func` | yes | `pub unsafe extern "C" fn stbds_shmode_func` |
| 14 | `stbds_stralloc` | yes | `pub unsafe extern "C" fn stbds_stralloc` |
| 15 | `stbds_strreset` | yes | `pub unsafe extern "C" fn stbds_strreset` |
| 16 | `strkey` | yes | `pub unsafe extern "C" fn strkey` |

## Static (non-exported) C functions — translated, intentionally not exported

These are `static` in the C TU, so they do not appear in `nm -D` for either
library; they must NOT be exported by Rust either (that would be an "extra"
symbol). All are present as private Rust `fn`s:

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, plus the file-scope statics `stbds_hash_seed` and `buffer`.

`stbds_unit_tests` is declared `extern` in the C source but never defined, so
it is absent from both `.so`s. Correct: no Rust stub.

## Result

- missing from Rust `.so`: **0**
- extra in Rust `.so`: **0**
- undefined non-libc symbols in Rust `.so`: **0**
  (`nm -D -u` shows only `realloc`, `free`, `memcpy`, … from libc / Rust std)

Status: **PASS**
