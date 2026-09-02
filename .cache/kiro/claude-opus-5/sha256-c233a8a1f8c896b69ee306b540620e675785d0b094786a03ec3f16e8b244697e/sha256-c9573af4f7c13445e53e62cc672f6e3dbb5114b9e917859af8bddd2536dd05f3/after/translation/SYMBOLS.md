# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-7hFdHr.so   | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libsh_geti_lib.so | awk '{print $3}' | sort -u
```

The C library is built from a single translation unit (`c_src/src/lib.c`, an
inlined copy of `stb_ds.h` plus the `strkey` / `sh_geti` test driver).
`c_src/include/lib.h` declares only `void sh_geti(int num);`, but the `.so`
exports every non-`static` definition in `lib.c`.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C source (lib.c) | Rust source (lib.rs) |
|---|--------|---------|------------|------------------|----------------------|
| 1 | `sh_geti`             | yes | yes | `void sh_geti(int num)` L945 | `pub unsafe extern "C" fn sh_geti` |
| 2 | `stbds_arrfreef`      | yes | yes | L312 | `stbds_arrfreef` |
| 3 | `stbds_arrgrowf`      | yes | yes | L275 | `stbds_arrgrowf` |
| 4 | `stbds_hash_bytes`    | yes | yes | L553 | `stbds_hash_bytes` |
| 5 | `stbds_hash_string`   | yes | yes | L475 | `stbds_hash_string` |
| 6 | `stbds_hmdel_key`     | yes | yes | L807 | `stbds_hmdel_key` |
| 7 | `stbds_hmfree_func`   | yes | yes | L571 | `stbds_hmfree_func` |
| 8 | `stbds_hmget_key`     | yes | yes | L658 | `stbds_hmget_key` |
| 9 | `stbds_hmget_key_ts`  | yes | yes | L627 | `stbds_hmget_key_ts` |
| 10 | `stbds_hmput_default`| yes | yes | L667 | `stbds_hmput_default` |
| 11 | `stbds_hmput_key`    | yes | yes | L679 | `stbds_hmput_key` |
| 12 | `stbds_rand_seed`    | yes | yes | L347 | `stbds_rand_seed` |
| 13 | `stbds_shmode_func`  | yes | yes | L795 | `stbds_shmode_func` |
| 14 | `stbds_stralloc`     | yes | yes | L880 | `stbds_stralloc` |
| 15 | `stbds_strreset`     | yes | yes | L920 | `stbds_strreset` |
| 16 | `strkey`             | yes | yes | L939 | `strkey` |

## Symbols intentionally NOT exported

These are `static` in the C translation unit, so they are not in `nm -D` for
either library. They are exercised indirectly through the exported entry
points.

| C symbol | why not exported |
|----------|------------------|
| `stbds_probe_position` | `static` |
| `stbds_log2` | `static` |
| `stbds_make_hash_index` | `static` |
| `stbds_siphash_bytes` | `static` (reachable via `stbds_hash_bytes`) |
| `stbds_is_key_equal` | `static` |
| `stbds_hm_find_slot` | `static` |
| `stbds_strdup` | `static` |
| `stbds_hash_seed` | `static` file-scope variable |
| `buffer` | `static` file-scope array used by `strkey` |
| `stbds_unit_tests` | only `extern`-declared in lib.c, never defined |

## Diff

```
$ comm -23 /tmp/c_syms.txt /tmp/r_syms.txt      # in C but not Rust
<empty>
$ comm -13 /tmp/c_syms.txt /tmp/r_syms.txt      # in Rust but not C
<empty>
```

**Result: 0 missing symbols.** All 16 exported C symbols are exported by the
Rust `cdylib` with identical names.

Re-checked by `verify_all.sh` for every feature combination and for both the
`release` and `dev` profiles.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the default
build is the only feature configuration. `verify_all.sh` derives this
mechanically from `Cargo.toml` (it enumerates singles, pairs, all-features and
`--no-default-features` if any features are ever added) and additionally runs
both build profiles, because `debug-assertions` / `overflow-checks` change
observable behaviour for a literal C translation. Both are disabled in
`[profile.dev]` for that reason — Rust's debug-only null-pointer check
otherwise turns the C's SIGSEGV on a NULL key into a Rust panic (SIGABRT),
which `errors_crash.rs::e_null_key_crashes_where_c_reads_it` detects.

## Undefined (imported) symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports
(`realloc`, `free`, `memmove`, `memcpy`, `memcmp`→`bcmp`, `strcmp`, `strlen`,
`printf`, `sprintf`, `abort`, plus the Rust std/panic-runtime imports
`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`, `__errno_location`, …).
**0 undefined non-libc symbols.**
