# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-33y1E1.so   | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libintput_lib.so | awk '{print $3}' | sort -u
```

C `.so`: `c_src/build/libharvest-work-33y1E1.so`
Rust `.so`: `translation/target/release/libintput_lib.so`

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | C source location |
|---|--------|-----------|---------------|-------------------|
| 1 | `intput`              | yes | yes | `lib.c` `void intput(int num)` (public header `include/lib.h`) |
| 2 | `strkey`              | yes | yes | `lib.c` `char *strkey(int n)` |
| 3 | `stbds_arrgrowf`      | yes | yes | `lib.c` |
| 4 | `stbds_arrfreef`      | yes | yes | `lib.c` |
| 5 | `stbds_rand_seed`     | yes | yes | `lib.c` |
| 6 | `stbds_hash_bytes`    | yes | yes | `lib.c` |
| 7 | `stbds_hash_string`   | yes | yes | `lib.c` |
| 8 | `stbds_hmfree_func`   | yes | yes | `lib.c` |
| 9 | `stbds_hmget_key`     | yes | yes | `lib.c` |
| 10 | `stbds_hmget_key_ts` | yes | yes | `lib.c` |
| 11 | `stbds_hmput_default`| yes | yes | `lib.c` |
| 12 | `stbds_hmput_key`    | yes | yes | `lib.c` |
| 13 | `stbds_hmdel_key`    | yes | yes | `lib.c` |
| 14 | `stbds_shmode_func`  | yes | yes | `lib.c` |
| 15 | `stbds_stralloc`     | yes | yes | `lib.c` |
| 16 | `stbds_strreset`     | yes | yes | `lib.c` |

## Symbols intentionally NOT exported (both sides agree)

These are `static` in the C translation unit and therefore absent from `nm -D`
on both libraries. They are still translated in Rust as private functions and
are exercised transitively:

- `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
  `stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
  `stbds_strdup`
- `static size_t stbds_hash_seed` (module-private mutable state)
- `static char buffer[256]` (the `strkey` scratch buffer)
- `stbds_unit_tests` is only *declared* `extern` in the C file and never
  defined or called, so it is neither defined nor undefined in the `.so`.

## Diff result

```
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # C-only symbols
<empty>
```

**0 missing symbols.** The Rust `.so` defines exactly the same 16 names as the
C `.so` (Rust defines 16 total, no extras). Undefined symbols on the Rust side
are libc/runtime only (`realloc`, `free`, `memcmp`, `strcmp`, `strlen`,
`memcpy`, `memmove`, `memset`, plus Rust runtime/ELF plumbing) — no
non-libc undefined symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default (empty) feature set. Verified with:

```
grep -n '\[features\]' translation/Cargo.toml   # no match
cargo metadata --no-deps --format-version 1 | jq '.packages[0].features'  # {}
```

Phases B–C therefore have exactly one feature combination to cover, and it is
also re-run under `--no-default-features` (equivalent, since there is no
`default` feature) to satisfy the Phase D matrix requirement.
