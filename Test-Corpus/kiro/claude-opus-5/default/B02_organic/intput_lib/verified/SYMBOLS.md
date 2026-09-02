# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-PqlowB.so    | awk '{print $NF}' | sort -u
nm -D --defined-only translation/target/release/libintput_lib.so | awk '{print $NF}' | sort -u
```

The C `.so` is built from the single translation unit `c_src/src/lib.c`
(an inlined `stb_ds.h` implementation plus the `strkey` / `intput` helpers).
`c_src/include/lib.h` declares only `void intput(int num);`, but every
non-`static` definition in `lib.c` has external linkage and therefore appears in
the dynamic symbol table.

## Defined symbols

| # | C symbol (`nm -D`) | in Rust `.so` | Rust item |
|---|--------------------|---------------|-----------|
| 1 | `intput`              | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn intput` |
| 2 | `stbds_arrfreef`      | yes | `stbds_arrfreef` |
| 3 | `stbds_arrgrowf`      | yes | `stbds_arrgrowf` |
| 4 | `stbds_hash_bytes`    | yes | `stbds_hash_bytes` |
| 5 | `stbds_hash_string`   | yes | `stbds_hash_string` |
| 6 | `stbds_hmdel_key`     | yes | `stbds_hmdel_key` |
| 7 | `stbds_hmfree_func`   | yes | `stbds_hmfree_func` |
| 8 | `stbds_hmget_key`     | yes | `stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts`  | yes | `stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default`| yes | `stbds_hmput_default` |
| 11 | `stbds_hmput_key`    | yes | `stbds_hmput_key` |
| 12 | `stbds_rand_seed`    | yes | `stbds_rand_seed` |
| 13 | `stbds_shmode_func`  | yes | `stbds_shmode_func` |
| 14 | `stbds_stralloc`     | yes | `stbds_stralloc` |
| 15 | `stbds_strreset`     | yes | `stbds_strreset` |
| 16 | `strkey`             | yes | `strkey` |

`comm -23 c_syms rust_syms` → **empty** (0 missing).
`comm -13 c_syms rust_syms` → **empty** (0 extra).

## Static / internal C symbols (correctly NOT exported by either `.so`)

`buffer`, `stbds_hash_seed`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`.

The Rust translation keeps the equivalents private (`static mut
STBDS_HASH_SEED`, `static mut STRKEY_BUFFER`, private `fn`s), so they are absent
from `nm -D` on both sides — matching.

## Undefined (imported) symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
loader entries (`realloc`, `free`, `strlen`, `strcmp`, `bcmp`, `memcpy`,
`memmove`, `memset`, `abort`, `_Unwind_*`, `__cxa_*`, `dl_iterate_phdr`, …).
**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one (`--no-default-features` and the default
build produce the identical crate). Phase D's "every feature combination"
requirement is therefore satisfied by the single default configuration; the
`scripts/check_features.sh` helper enumerates the (empty) feature set and
re-runs `cargo check` / `cargo test` for each combination found.
