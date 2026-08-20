# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically:

```sh
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cargo build
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/debug/libarr_del_lib.so    | awk '{print $3}' | sort > rs_syms.txt
comm -23 c_syms.txt rs_syms.txt   # -> EMPTY
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** → exactly **one** valid feature
  combination: the empty/default one. `cargo check --no-default-features`
  and `cargo check` are therefore the complete matrix (verified, both clean).
* `c_src/CMakeLists.txt` has **no options / no `#ifdef`-driven variants**: it
  compiles the single file `src/lib.c` into one `SHARED` library and links `m`.
  `lib.c` `#define`s all its knobs unconditionally
  (`STBDS_ASSERT=assert`, `STBDS_REALLOC=realloc`, `STBDS_FREE=free`,
  `STBDS_SIPHASH_C_ROUNDS=2`, `STBDS_SIPHASH_D_ROUNDS=4`,
  `STBDS_HAS_TYPEOF`, `STBDS_HAS_LITERAL_ARRAY`, `STBDS_BUCKET_LENGTH=8`),
  so there is a single C configuration too.

## Symbol table

`arr_del`/`strkey` are the C driver helpers; everything else is the stb_ds core.
"Rust site" is the file that carries the `#[unsafe(no_mangle)] extern "C"` wrapper.

| # | symbol | C signature | C site (lib.c) | Rust site | in Rust `.so` |
|---|--------|-------------|----------------|-----------|----------------|
| 1 | `stbds_arrgrowf`      | `void *(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | 276 | `src/array.rs`   | yes |
| 2 | `stbds_arrfreef`      | `void (void *a)`                                                  | 312 | `src/array.rs`   | yes |
| 3 | `stbds_rand_seed`     | `void (size_t seed)`                                              | 355 | `src/hash.rs`    | yes |
| 4 | `stbds_hash_string`   | `size_t (char *str, size_t seed)`                                 | 477 | `src/hash.rs`    | yes |
| 5 | `stbds_hash_bytes`    | `size_t (void *p, size_t len, size_t seed)`                       | 553 | `src/hash.rs`    | yes |
| 6 | `stbds_hmfree_func`   | `void (void *a, size_t elemsize)`                                 | 571 | `src/hashmap.rs` | yes |
| 7 | `stbds_hmget_key_ts`  | `void *(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | 631 | `src/hashmap.rs` | yes |
| 8 | `stbds_hmget_key`     | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | 659 | `src/hashmap.rs` | yes |
| 9 | `stbds_hmput_default` | `void *(void *a, size_t elemsize)`                                | 667 | `src/hashmap.rs` | yes |
| 10 | `stbds_hmput_key`    | `void *(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | 680 | `src/hashmap.rs` | yes |
| 11 | `stbds_shmode_func`  | `void *(size_t elemsize, int mode)`                               | 796 | `src/hashmap.rs` | yes |
| 12 | `stbds_hmdel_key`    | `void *(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | 807 | `src/hashmap.rs` | yes |
| 13 | `stbds_stralloc`     | `char *(stbds_string_arena *a, char *str)`                         | 881 | `src/arena.rs`   | yes |
| 14 | `stbds_strreset`     | `void (stbds_string_arena *a)`                                    | 920 | `src/arena.rs`   | yes |
| 15 | `strkey`             | `char *(int n)`                                                   | 939 | `src/tests.rs`   | yes |
| 16 | `arr_del`            | `void (int num)`                                                  | 945 | `src/tests.rs`   | yes |

**C exports: 16. Rust exports: 16. Missing from Rust: 0.**

## Deliberately NOT exported (`static` in C, so absent from `nm -D` too)

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, `stbds_hash_seed`, `buffer`.
The Rust translation keeps all of these crate-private as well, so the two
`.so`s expose exactly the same surface (no extra Rust symbols either).

Declared `extern` in lib.c but **never defined** by it, hence not in the C
`.so` and correctly not in the Rust `.so`: `stbds_unit_tests`.

## Undefined-symbol check

C `.so` imports only libc (`malloc free realloc memcmp memcpy memmove memset
sprintf strcmp strlen __assert_fail`).
Rust `.so` imports only libc + the platform unwinder/`std` runtime
(`_Unwind_*`, `__cxa_*`, `pthread_key_*`, `mmap64`, `abort`, ...).
**0 missing / undefined non-libc (non-runtime) symbols in the Rust `.so`.**

## Phase D result (re-verified after every change)

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > c.txt
$ nm -D --defined-only target/debug/libarr_del_lib.so    | awk '{print $3}' | sort > r.txt
$ comm -23 c.txt r.txt      # C symbols missing from Rust
$ wc -l < c.txt
16
```

* **debug profile**: 16/16 C symbols exported by the Rust `.so`, 0 missing.
* **release profile** (`panic = "abort"`): 16/16, 0 missing.
* No *extra* public symbol is exported by the Rust `.so` either
  (`rust_so_exports_no_extra_stbds_symbols`).
* Every symbol is additionally proven **callable** through `dlsym`: the test
  harness resolves all 16 with `libloading` and every differential test calls
  them through those function pointers, so the `#[unsafe(no_mangle)]
  extern "C"` wrappers themselves are under test.
* The struct layout the wrappers hand across the ABI is checked too
  (`layout_hash_index_bucket_offset_matches`): `stbds_make_hash_index` derives
  its bucket-array pointer from `sizeof(stbds_hash_index)`, and that derived
  offset is identical in both libraries for every malloc alignment class.

Automated by `./run_all_configs.sh`, which also enumerates the Cargo feature
matrix (empty — the crate has no `[features]`) and runs the whole suite in both
profiles.
