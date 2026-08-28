# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Source of truth:

```
nm -D --defined-only c_src/build/libharvest-work-f3vKdh.so
nm -D --defined-only translation/target/release/libarr_del_lib.so
```

The C translation unit is a single file (`c_src/src/lib.c`) — an inlined copy of
`stb_ds.h` plus the `strkey` / `arr_del` helpers.  There is therefore exactly one
module, and no C source file was skipped by the translation.

## Table

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `stbds_arrgrowf`      | T | T | `#[unsafe(no_mangle)] extern "C"` |
| 2 | `stbds_arrfreef`      | T | T | |
| 3 | `stbds_rand_seed`     | T | T | mutates the library-global `stbds_hash_seed` |
| 4 | `stbds_hash_string`   | T | T | |
| 5 | `stbds_hash_bytes`    | T | T | wraps the `static` SipHash-2-4 |
| 6 | `stbds_hmfree_func`   | T | T | |
| 7 | `stbds_hmget_key_ts`  | T | T | |
| 8 | `stbds_hmget_key`     | T | T | |
| 9 | `stbds_hmput_default` | T | T | |
| 10 | `stbds_hmput_key`    | T | T | |
| 11 | `stbds_shmode_func`  | T | T | |
| 12 | `stbds_hmdel_key`    | T | T | |
| 13 | `stbds_stralloc`     | T | T | |
| 14 | `stbds_strreset`     | T | T | |
| 15 | `strkey`             | T | T | `sprintf`-based helper, static 256-byte buffer |
| 16 | `arr_del`            | T | T | the only symbol declared in `include/lib.h` |

**Missing from Rust: 0.**  **Extra in Rust: 0** (after filtering the Rust
runtime's own lowercase/local symbols, which `nm -D` reports as `t`/`d`/`b`
and which are not part of the C surface).

## `static` C functions (no symbol, but reachable through the exported ones)

These are `static` in C so they do not appear in `nm -D`; they are translated as
private Rust `fn`s and are exercised transitively by the tests listed in
`CONFIGS.md` / `ERRORS.md`.

| C static | Rust counterpart | reached via |
|----------|------------------|-------------|
| `stbds_probe_position` | `stbds_probe_position` | every hash lookup |
| `stbds_log2`           | `stbds_log2`           | `stbds_make_hash_index` |
| `stbds_make_hash_index`| `stbds_make_hash_index`| `stbds_hmput_key`, `stbds_shmode_func`, `stbds_hmdel_key` |
| `stbds_siphash_bytes`  | `stbds_siphash_bytes`  | `stbds_hash_bytes` |
| `stbds_is_key_equal`   | `stbds_is_key_equal`   | `stbds_hm_find_slot`, `stbds_hmput_key` |
| `stbds_hm_find_slot`   | `stbds_hm_find_slot`   | `stbds_hmget_key_ts`, `stbds_hmdel_key` |
| `stbds_strdup`         | `stbds_strdup`         | `stbds_hmput_key` with `STBDS_SH_STRDUP` |

## Declared-but-not-defined in this TU

`c_src/src/lib.c` declares `extern void stbds_unit_tests(void);` but never
defines it, so it is **not** exported by the C `.so` and must not be exported by
the Rust `.so` either.  Verified: absent from both.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/*.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libarr_del_lib.so \
         | awk '$2=="T"{print $3}' | sort)
```

Result: empty diff (see `tests/symbols.rs`, which performs this check
programmatically).

## Build note (important)

`cargo test` does **not** rebuild a `cdylib`-only lib target, so
`target/release/libarr_del_lib.so` can be stale while the test binaries are
fresh — the suite would then verify an old library.  `run_all.sh` always runs
`cargo build --release` before `cargo test --release`, and
`tests/common/mod.rs::rust_so_path()` refuses to run when the `.so` is older
than `src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configurations are the default one and `--no-default-features` (which is
identical, since there is no `default` feature).  Both are checked by
`run_all.sh`.
