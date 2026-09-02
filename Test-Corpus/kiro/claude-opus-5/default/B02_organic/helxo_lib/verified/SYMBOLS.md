# SYMBOLS.md — exported-symbol parity

Generated mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-4g3ieR.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libhelxo_lib.so | awk '{print $3}' | sort
```

The C library is built from the single translation unit `c_src/src/lib.c`
(an `stb_ds.h` amalgamation plus the `strkey` / `helxo` demo drivers).
`c_src/include/lib.h` declares only `void helxo(char num);`, but the `.so`
exports every non-`static` definition in the TU.

## Symbol table

| # | symbol | C type | Rust `.so` | Rust definition site |
|---|--------|--------|-----------|----------------------|
| 1 | `helxo` | `T` | ✅ `T` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn helxo` |
| 2 | `stbds_arrfreef` | `T` | ✅ `T` | `stbds_arrfreef` |
| 3 | `stbds_arrgrowf` | `T` | ✅ `T` | `stbds_arrgrowf` |
| 4 | `stbds_hash_bytes` | `T` | ✅ `T` | `stbds_hash_bytes` |
| 5 | `stbds_hash_string` | `T` | ✅ `T` | `stbds_hash_string` |
| 6 | `stbds_hmdel_key` | `T` | ✅ `T` | `stbds_hmdel_key` |
| 7 | `stbds_hmfree_func` | `T` | ✅ `T` | `stbds_hmfree_func` |
| 8 | `stbds_hmget_key` | `T` | ✅ `T` | `stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts` | `T` | ✅ `T` | `stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default` | `T` | ✅ `T` | `stbds_hmput_default` |
| 11 | `stbds_hmput_key` | `T` | ✅ `T` | `stbds_hmput_key` |
| 12 | `stbds_rand_seed` | `T` | ✅ `T` | `stbds_rand_seed` |
| 13 | `stbds_shmode_func` | `T` | ✅ `T` | `stbds_shmode_func` |
| 14 | `stbds_stralloc` | `T` | ✅ `T` | `stbds_stralloc` |
| 15 | `stbds_strreset` | `T` | ✅ `T` | `stbds_strreset` |
| 16 | `strkey` | `T` | ✅ `T` | `strkey` |

**`comm -23 c_syms rust_syms` → empty. 16 / 16 symbols present. 0 missing.**

## Internal (`static`) C functions — intentionally not exported

These are `static` in `lib.c`, so they are absent from the C `.so` too. They are
translated as private Rust `fn`s; exporting them would be a *parity violation*.

| C `static` function | Rust counterpart |
|---------------------|------------------|
| `stbds_probe_position` | `stbds_probe_position` (private) |
| `stbds_log2` | `stbds_log2` (private) |
| `stbds_make_hash_index` | `stbds_make_hash_index` (private) |
| `stbds_siphash_bytes` | `stbds_siphash_bytes` (private) |
| `stbds_is_key_equal` | `stbds_is_key_equal` (private) |
| `stbds_hm_find_slot` | `stbds_hm_find_slot` (private) |
| `stbds_strdup` | `stbds_strdup` (private) |
| `buffer` (`static char[256]`) | `static mut buffer: [c_char; 256]` (private) |
| `stbds_hash_seed` (`static size_t`) | `static mut stbds_hash_seed` (private) |

`stbds_unit_tests` is only `extern`-declared in the C, never defined, so it
appears in neither `.so` as a defined symbol.

## Undefined (imported) symbols

`nm -D --undefined-only` on both `.so`s yields only libc / compiler-runtime
symbols. The C imports `malloc realloc free memset memcpy memmove memcmp
strcmp strlen printf sprintf __assert_fail`. The Rust `.so` imports the same
set (`memcmp` appears as glibc's `bcmp` alias) plus the Rust runtime's
`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `abort`, etc.
**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default (no features). Verified with:

```
grep -c '^\[features\]' Cargo.toml   # -> 0
```

Consequently `--no-default-features` and the default build are the same code,
and the whole matrix below collapses to a single column. This is checked
mechanically by `tests/feature_matrix.rs` / `check_features.sh`.

## Struct-layout parity (required because the tests share memory across the two `.so`s)

Measured with `gcc` on the C definitions vs. `size_of`/`offset_of` on the Rust
`#[repr(C)]` clones:

| struct | C size | Rust size | notable offsets |
|--------|--------|-----------|-----------------|
| `stbds_array_header` | 32 | 32 | length 0, capacity 8, hash_table 16, temp 24 |
| `stbds_string_block` | 16 | 16 | next 0, storage 8 |
| `stbds_string_arena` | 24 | 24 | storage 0, remaining 8, block 16, mode 17 |
| `stbds_hash_bucket` | 128 | 128 | hash 0, index 64 |
| `stbds_hash_index` | 104 | 104 | string 72, storage 96 |
| `helxo` element `{char*;char;}` | 16 | 16 | key 0, value 8 |

These are asserted at compile time in `tests/common/mod.rs`.

## Verification commands

```bash
# build both libraries
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

# Phase D: symbol diff (exits non-zero on any missing symbol)
./check_symbols.sh

# Phases B + C + D across every feature combination
./check_features.sh
```

Recorded output of `./check_symbols.sh`:

```
C   .so: .../c_src/build/libharvest-work-4g3ieR.so  (16 defined symbols)
Rust.so: .../translation/target/release/libhelxo_lib.so (16 defined symbols)

OK: 0 missing symbols (symbol diff is empty).
OK: no C-private symbol is exported by the Rust .so.
```

The same diff is enforced from inside the test suite by
`tests/symbols.rs::phase_d_symbol_parity`, which additionally asserts that none
of the C's `static` functions leaked into the Rust `.so` (exporting them would
also be a parity violation) and that the Rust `.so` has no unresolved non-libc
imports.
