# SYMBOLS.md — Phase A symbol surface

C shared object: `c_src/build/libharvest-work-qeWsld.so` (name derived from the
parent directory by `CMakeLists.txt`: `project(${project_name})`).
Rust shared object: `translation/target/release/libhm_geti_lib.so`.

Command used:

```
nm -D --defined-only <so> | sort
```

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `stbds_arrgrowf`      | T | T | array growth allocator |
| 2 | `stbds_arrfreef`      | T | T | frees header-based array |
| 3 | `stbds_rand_seed`     | T | T | sets the file-static `stbds_hash_seed` |
| 4 | `stbds_hash_string`   | T | T | rotate/mix string hash |
| 5 | `stbds_hash_bytes`    | T | T | wrapper over static `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`   | T | T | frees map + strdup'd keys + arena |
| 7 | `stbds_hmget_key_ts`  | T | T | thread-safe-style lookup, writes `*temp` |
| 8 | `stbds_hmget_key`     | T | T | lookup, writes header `temp` |
| 9 | `stbds_hmput_default` | T | T | allocates the `[-1]` default slot |
| 10 | `stbds_hmput_key`    | T | T | insert / update, grows hash index |
| 11 | `stbds_shmode_func`  | T | T | creates a string map in a given arena mode |
| 12 | `stbds_hmdel_key`    | T | T | delete + tombstone + shrink/rebuild |
| 13 | `stbds_stralloc`     | T | T | string arena allocation |
| 14 | `stbds_strreset`     | T | T | string arena teardown |
| 15 | `strkey`             | T | T | `sprintf(buffer,"test_%d",n)` helper |
| 16 | `hm_geti`            | T | T | the public driver declared in `include/lib.h` |

## Symbols present in C but MISSING from Rust

**None.** The diff is empty:

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-qeWsld.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libhm_geti_lib.so | awk '{print $3}' | sort)
(no output)
```

## Static / non-exported C entities (correctly NOT exported by either side)

`stbds_hash_seed`, `stbds_probe_position`, `stbds_log2`,
`stbds_make_hash_index`, `stbds_siphash_bytes`, `stbds_is_key_equal`,
`stbds_hm_find_slot`, `stbds_strdup`, `buffer`.
Also `stbds_unit_tests` is only `extern`-declared and never defined, so it is
absent from both objects.

## Undefined (imported) symbols

The Rust `.so` imports only libc/`std` symbols (`realloc`, `free`, `memmove`,
`memcpy`, `memset`, `memcmp`, `strlen`, `strcmp`, plus the Rust std runtime).
0 missing non-libc undefined symbols.

## Verification

`verify.sh` re-checks the diff on every run:

```
--- symbol parity ---
  OK: symbol diff empty (16 symbols)
```

All 16 symbols are exercised through `libloading` by the differential tests —
none is merely exported.
