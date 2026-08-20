# SYMBOLS.md — dynamic symbol parity (Phase A / Phase D)

The C library is `c_src/src/lib.c` (an `stb_ds.h`-derived dynamic-array /
hash-map implementation with the two test helpers `strkey` and `arr_push`
at the bottom of the file, plus the single public header entry
`void arr_push(int num);`).

Build commands used:

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so
#    CMAKE_BUILD_TYPE is empty  => NO -DNDEBUG => assert() is ACTIVE
#    C_FLAGS = -fPIC (no -O)

# Rust
cargo build          # -> target/debug/libarr_push_lib.so
cargo build --release# -> target/release/libarr_push_lib.so
```

## Defined (exported) symbols

`nm -D --defined-only` on both `.so` files.

| # | C symbol | in C `.so` | in Rust `.so` | notes |
|---|----------|------------|---------------|-------|
| 1 | `stbds_arrgrowf`      | T | T | `src/lib.rs` `#[unsafe(no_mangle)] extern "C"` |
| 2 | `stbds_arrfreef`      | T | T | |
| 3 | `stbds_rand_seed`     | T | T | writes the global seed |
| 4 | `stbds_hash_string`   | T | T | |
| 5 | `stbds_hash_bytes`    | T | T | wrapper around `static stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`   | T | T | |
| 7 | `stbds_hmget_key_ts`  | T | T | |
| 8 | `stbds_hmget_key`     | T | T | |
| 9 | `stbds_hmput_default` | T | T | |
| 10 | `stbds_hmput_key`    | T | T | |
| 11 | `stbds_shmode_func`  | T | T | |
| 12 | `stbds_hmdel_key`    | T | T | |
| 13 | `stbds_stralloc`     | T | T | |
| 14 | `stbds_strreset`     | T | T | |
| 15 | `strkey`             | T | T | test helper, `sprintf` into a 256-byte static |
| 16 | `arr_push`           | T | T | the only symbol declared in `include/lib.h` |

**Missing from Rust `.so`: 0.**

### Symbols intentionally NOT exported

* `stbds_unit_tests` — declared `extern` at `c_src/src/lib.c:83` but never
  defined in the translation unit, so it is an *undefined* symbol reference that
  the linker drops (it is not referenced either). It appears in neither `.so`.
* `static` C functions (no external linkage in either build, correctly private
  in Rust too): `stbds_probe_position`, `stbds_log2`,
  `stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
  `stbds_hm_find_slot`, `stbds_strdup`.
* `static char buffer[256]` (used by `strkey`) — internal linkage in C; the Rust
  counterpart `STRKEY_BUFFER` is a private `static mut`. Not exported by either.

## Undefined symbols

Both libraries import only libc / runtime symbols; there are **0 missing
non-libc symbols** in the Rust `.so`.

* C `.so` imports: `__assert_fail`, `free`, `malloc`, `memcmp`, `memcpy`,
  `memmove`, `memset`, `realloc`, `sprintf`, `strcmp`, `strlen`
  (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
* Rust `.so` imports: the same libc allocator/string set (`realloc`, `free`,
  `memset`, `memcpy`, `memmove`, `strcmp`, `strlen`, `bcmp` for `memcmp`,
  `abort` for `assert`) plus the ordinary Rust `std` runtime imports
  (`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `mmap64`, `write`, …).
  All are provided by glibc / libgcc — nothing unresolved.

## Verification snippet

```sh
diff <(nm -D --defined-only c_src/build/libtranslated_rust.so \
        | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/release/libarr_push_lib.so \
        | awk '{print $3}' | sort)
# (empty)
```

produces empty output, for BOTH `target/debug/libarr_push_lib.so` and
`target/release/libarr_push_lib.so`.

## Automated checks

| check | test |
|-------|------|
| both `.so` files exist | `tests/symbols.rs::c_so_exists` |
| `nm -D` symbol diff is empty, and the C exports exactly the 16 documented names | `tests/symbols.rs::symbol_diff_is_empty` |
| every documented name is `dlsym`-able out of BOTH libraries | `tests/symbols.rs::c_symbols_all_present_in_rust` |
| the Rust `.so` has no unresolved non-libc/non-libgcc imports | `tests/symbols.rs::rust_so_has_no_foreign_undefined_symbols` |
| both libraries load and their entry points are callable | `tests/symbols.rs::both_libraries_load_and_call` |
| the diff is empty for **every** feature combination and for **both** profiles | `scripts/check_all_features.sh` |

Latest run of `scripts/check_all_features.sh`:

```
ok: target/debug/libarr_push_lib.so   exports all 16 C symbols (has 16 defined symbols)
ok: target/release/libarr_push_lib.so exports all 16 C symbols (has 16 defined symbols)
```

Neither Rust `.so` exports anything the C `.so` does not, so the two dynamic
symbol tables are *identical sets*, not merely a superset relation.
