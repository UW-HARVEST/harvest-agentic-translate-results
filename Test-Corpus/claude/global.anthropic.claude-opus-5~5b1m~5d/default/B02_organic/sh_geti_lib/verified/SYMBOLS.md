# SYMBOLS.md — exported-symbol parity

C shared library: `c_src/build/libharvest-work-zcFfCI.so`
Rust shared library: `translation/target/release/libsh_geti_lib.so`

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-zcFfCI.so   | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libsh_geti_lib.so | awk '$2=="T"{print $3}' | sort
```

## Global text symbols exported by the C `.so`

| # | symbol | C definition (lib.c) | Rust `#[no_mangle]` (src/lib.rs) | present in Rust `.so` |
|---|--------|----------------------|----------------------------------|------------------------|
| 1 | `sh_geti`             | `void sh_geti(int num)` (L945)                     | `sh_geti`             | yes |
| 2 | `stbds_arrfreef`      | `void stbds_arrfreef(void*)` (L312)                | `stbds_arrfreef`      | yes |
| 3 | `stbds_arrgrowf`      | `void *stbds_arrgrowf(void*,size_t,size_t,size_t)` (L276) | `stbds_arrgrowf` | yes |
| 4 | `stbds_hash_bytes`    | `size_t stbds_hash_bytes(void*,size_t,size_t)` (L553) | `stbds_hash_bytes` | yes |
| 5 | `stbds_hash_string`   | `size_t stbds_hash_string(char*,size_t)` (L477)    | `stbds_hash_string`   | yes |
| 6 | `stbds_hmdel_key`     | `void *stbds_hmdel_key(...)` (L807)                | `stbds_hmdel_key`     | yes |
| 7 | `stbds_hmfree_func`   | `void stbds_hmfree_func(void*,size_t)` (L571)      | `stbds_hmfree_func`   | yes |
| 8 | `stbds_hmget_key`     | `void *stbds_hmget_key(...)` (L659)                | `stbds_hmget_key`     | yes |
| 9 | `stbds_hmget_key_ts`  | `void *stbds_hmget_key_ts(...)` (L631)             | `stbds_hmget_key_ts`  | yes |
| 10 | `stbds_hmput_default`| `void *stbds_hmput_default(void*,size_t)` (L667)   | `stbds_hmput_default` | yes |
| 11 | `stbds_hmput_key`    | `void *stbds_hmput_key(...)` (L680)                | `stbds_hmput_key`     | yes |
| 12 | `stbds_rand_seed`    | `void stbds_rand_seed(size_t)` (L355)              | `stbds_rand_seed`     | yes |
| 13 | `stbds_shmode_func`  | `void *stbds_shmode_func(size_t,int)` (L796)       | `stbds_shmode_func`   | yes |
| 14 | `stbds_stralloc`     | `char *stbds_stralloc(stbds_string_arena*,char*)` (L881) | `stbds_stralloc` | yes |
| 15 | `stbds_strreset`     | `void stbds_strreset(stbds_string_arena*)` (L920)  | `stbds_strreset`      | yes |
| 16 | `strkey`             | `char *strkey(int n)` (L939)                       | `strkey`              | yes |

## `static` (file-local, NOT exported) C functions — intentionally not exported by Rust

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`.  These are `static` in `lib.c`, do not appear in `nm -D`, and
are private `fn`s in the Rust crate.  Correct — no export required.

`stbds_unit_tests` is only `extern`-declared in `lib.c` (L83) and never
defined, so it appears in neither `.so`.  Correct — nothing to translate.

## Result

```
comm -23 c_symbols rust_symbols   ->   (empty)   # nothing missing in Rust
comm -13 c_symbols rust_symbols   ->   (empty)   # nothing extra in Rust
```

- [x] 0 symbols missing from the Rust `.so`.
- [x] 0 undefined non-libc symbols in the Rust `.so`
      (`nm -D -u` shows only libc: `realloc`, `free`, `memset`, `memcpy`,
      `memmove`, `memcmp`, `strcmp`, `strlen`, `printf`, `sprintf`, `abort`,
      plus the usual `__*` runtime helpers).

## Phase D — symbol parity re-verified for every build configuration

`translation/Cargo.toml` declares **no `[features]`** and no optional
dependencies, so the only build configurations are the two profiles.  Both were
checked:

| Rust `.so` | symbols missing vs. C | extra vs. C |
|------------|-----------------------|-------------|
| `target/release/libsh_geti_lib.so` | 0 | 0 |
| `target/debug/libsh_geti_lib.so`   | 0 | 0 |

```sh
nm -D --defined-only c_src/build/libharvest-work-zcFfCI.so | awk '$2=="T"{print $3}' | sort > c.txt
for so in translation/target/{release,debug}/libsh_geti_lib.so; do
  nm -D --defined-only "$so" | awk '$2=="T"{print $3}' | sort > r.txt
  comm -3 c.txt r.txt          # empty for both
done
```

The full differential suite (76 tests) passes against:

* the Rust **release** `.so` and the default (unoptimised) C build,
* the Rust **debug** `.so` and the default C build,
* the Rust release `.so` and a **`-O2`** C build (extra cross-check that the
  shift-count and signed-overflow paths agree under optimisation too).
