# SYMBOLS.md — exported-symbol parity between the C `.so` and the Rust `.so`

Generated mechanically:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so | sort

# Rust
cargo build --release
nm -D --defined-only target/release/libintput_lib.so | grep -v ' [wWuv] ' | sort
```

The C build produces `c_src/build/libtranslated_rust.so` (the CMake project name
is derived from the *parent* directory name, i.e. `translated_rust`).
The Rust build produces `target/{debug,release}/libintput_lib.so`.

## Full symbol table

All 16 symbols the C `.so` exports (`T`, global text) and their Rust
counterparts. `src` = the Rust module that owns the `#[no_mangle]` wrapper.

| # | symbol | C source (`c_src/src/lib.c`) | in Rust `.so` | Rust src |
|---|--------|------------------------------|---------------|----------|
| 1 | `stbds_arrgrowf`      | L276 | yes | `src/arr.rs`    |
| 2 | `stbds_arrfreef`      | L312 | yes | `src/arr.rs`    |
| 3 | `stbds_rand_seed`     | L355 | yes | `src/hash.rs`   |
| 4 | `stbds_hash_string`   | L477 | yes | `src/hash.rs`   |
| 5 | `stbds_hash_bytes`    | L553 | yes | `src/hash.rs`   |
| 6 | `stbds_hmfree_func`   | L571 | yes | `src/hmap.rs`   |
| 7 | `stbds_hmget_key_ts`  | L631 | yes | `src/hmap.rs`   |
| 8 | `stbds_hmget_key`     | L659 | yes | `src/hmap.rs`   |
| 9 | `stbds_hmput_default` | L667 | yes | `src/hmap.rs`   |
| 10 | `stbds_hmput_key`    | L680 | yes | `src/hmap.rs`   |
| 11 | `stbds_shmode_func`  | L796 | yes | `src/hmap.rs`   |
| 12 | `stbds_hmdel_key`    | L807 | yes | `src/hmap.rs`   |
| 13 | `stbds_stralloc`     | L881 | yes | `src/arena.rs`  |
| 14 | `stbds_strreset`     | L920 | yes | `src/arena.rs`  |
| 15 | `strkey`             | L939 | yes | `src/api.rs`    |
| 16 | `intput`             | L945 | yes | `src/api.rs`    |

## `static`/internal C functions (intentionally NOT exported by either side)

These are `static` in the C translation unit, so they appear in neither `.so`'s
dynamic symbol table. They are exercised indirectly through the exported API.

| C symbol | C line | Rust counterpart |
|----------|--------|------------------|
| `stbds_probe_position` | L367 | `hash::stbds_probe_position` |
| `stbds_log2`           | L375 | `hash::stbds_log2`           |
| `stbds_make_hash_index`| L385 | `hash::stbds_make_hash_index`|
| `stbds_siphash_bytes`  | L498 | `hash::stbds_siphash_bytes`  |
| `stbds_is_key_equal`   | L558 | `hmap::stbds_is_key_equal`   |
| `stbds_hm_find_slot`   | L586 | `hmap::stbds_hm_find_slot`   |
| `stbds_strdup`         | L870 | `arena::stbds_strdup`        |
| `stbds_hash_seed` (data, L353) | — | `hash::STBDS_HASH_SEED` (`AtomicUsize`) |
| `buffer` (data, L938)  | — | `api::BUFFER` (`static mut [c_char; 256]`) |

`extern void stbds_unit_tests(void);` (L83) is only *declared* in the C source,
never defined, so it is not a defined symbol in the C `.so` either — correctly
absent from both.

## Diff result

```
$ comm -23 c_names.txt r_names.txt     # in C, missing from Rust
<empty>
$ comm -13 c_names.txt r_names.txt     # in Rust, not in C
<empty>
$ wc -l c_names.txt r_names.txt
16 16
```

Verified for **both** the `debug` and the `release` cdylib.

This check is automated as part of the test suite so it cannot silently rot:

| test (`tests/d_symbols.rs`) | what it asserts |
|-----------------------------|-----------------|
| `d_symbol_parity` | `nm -D --defined-only` sets are **equal** (no missing, no extra) and match the expected 16-symbol list |
| `d_no_unresolved_project_symbols` | `nm -D -u` on the Rust `.so` contains only libc / `libgcc_s` / linker symbols |
| `d_all_symbols_resolvable` | all 16 symbols `dlsym` successfully from **both** `.so`s (this is what `Pair::new()` does for every differential test) |
| `d_struct_layout_matches_c_abi` | each library **independently** matches the hard-coded x86-64 LP64 field offsets of all five shared structs (so a layout mistake shared by both sides and the harness cannot hide) |

- [x] `nm -D` shows **0** symbols present in the C `.so` and missing from the
      Rust `.so`.
- [x] The Rust `.so` exports **no extra** non-libc symbols beyond the C set.
- [x] Undefined (`U`) symbols in the Rust `.so` are libc / `libgcc_s` unwinder
      only (`realloc`, `free`, `mem*`, `str*`, `__assert_fail`, `abort`,
      `_Unwind_*`, `dl_iterate_phdr`, …) — **no** unresolved project symbols.
- [x] No symbol is a stub: every one is exercised by at least one differential
      test in `tests/` that calls it through `dlsym` on both `.so`s and compares
      the results (see the `test` columns of `CONFIGS.md` and `ERRORS.md`).

## Documented non-semantic difference: internal-call interposition surface

`readelf -r` shows which internal calls to *exported* helpers go through the
GOT/PLT (and are therefore `LD_PRELOAD`-interposable):

| library | interposable internal calls |
|---------|------------------------------|
| C `.so` (built with an empty `CMAKE_BUILD_TYPE`, i.e. `-O0`) | 8 — `stbds_arrgrowf`, `stbds_hash_bytes`, `stbds_hash_string`, `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmput_key`, `stbds_stralloc`, `stbds_strreset` |
| Rust **debug** `.so` | the same 8 |
| Rust **release** `.so` | 3 — `stbds_hash_bytes`, `stbds_hmput_key`, `stbds_stralloc` (the other 5 were inlined / turned into direct calls) |

This is an optimisation-level artifact, not a translation difference: it is
observable **only** to code that interposes stb_ds symbols with `LD_PRELOAD`,
and a `-O2` C build collapses the same calls. No normal caller can distinguish
the two, and every exported entry point still behaves identically (all 110
differential tests pass against the release `.so`).

## Struct-layout parity (shared heap blocks cross the ABI boundary)

Both libraries hand each other the *same* heap blocks (the array header sits
immediately before the data pointer the caller holds), so the `#[repr(C)]`
mirrors must be byte-identical on x86-64 LP64:

| C type | size | Rust type |
|--------|------|-----------|
| `stbds_array_header` | 32 | `types::StbdsArrayHeader` |
| `stbds_string_block` | 16 | `types::StbdsStringBlock` |
| `struct stbds_string_arena` | 24 | `types::StbdsStringArena` |
| `stbds_hash_bucket` | 128 | `types::StbdsHashBucket` |
| `stbds_hash_index` | 104 | `types::StbdsHashIndex` |

The differential tests read every field of all five structs out of blocks
produced by the *other* library's code path, so any layout mismatch shows up
immediately as a snapshot divergence.
