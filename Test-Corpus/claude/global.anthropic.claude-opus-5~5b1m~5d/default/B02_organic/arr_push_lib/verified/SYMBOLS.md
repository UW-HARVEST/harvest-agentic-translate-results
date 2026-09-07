# SYMBOLS.md — exported-symbol parity

Source of truth: `nm -D --defined-only` on

* C   : `c_src/build/libharvest-work-404q18.so`
* Rust: `translation/target/release/libarr_push_lib.so`

The C library is a single translation unit (`c_src/src/lib.c`) which is an
inlined copy of `stb_ds.h` plus two test helpers (`strkey`, `arr_push`).
The public header (`c_src/include/lib.h`) declares only `arr_push`, but the
`.so` exports all non-`static` `stbds_*` functions as well; every one of them
is part of the verification surface.

## Defined (dynamic) symbols

| # | symbol | in C .so | in Rust .so | notes |
|---|--------|----------|-------------|-------|
| 1 | `arr_push`             | yes | yes | `void arr_push(int)` — the only symbol in `lib.h` |
| 2 | `strkey`               | yes | yes | `char *strkey(int)` — `sprintf(buffer,"test_%d",n)` into a `static char[256]` |
| 3 | `stbds_rand_seed`      | yes | yes | `void (size_t)` — sets the file-static `stbds_hash_seed` |
| 4 | `stbds_hash_bytes`     | yes | yes | `size_t (void*,size_t,size_t)` — siphash-2-4 |
| 5 | `stbds_hash_string`    | yes | yes | `size_t (char*,size_t)` |
| 6 | `stbds_arrgrowf`       | yes | yes | `void* (void*,size_t elemsize,size_t addlen,size_t min_cap)` |
| 7 | `stbds_arrfreef`       | yes | yes | `void (void*)` |
| 8 | `stbds_hmfree_func`    | yes | yes | `void (void*,size_t elemsize)` |
| 9 | `stbds_hmget_key`      | yes | yes | `void* (void*,size_t,void*,size_t,int mode)` |
|10 | `stbds_hmget_key_ts`   | yes | yes | `void* (void*,size_t,void*,size_t,ptrdiff_t*,int mode)` |
|11 | `stbds_hmput_default`  | yes | yes | `void* (void*,size_t)` |
|12 | `stbds_hmput_key`      | yes | yes | `void* (void*,size_t,void*,size_t,int mode)` |
|13 | `stbds_hmdel_key`      | yes | yes | `void* (void*,size_t,void*,size_t,size_t keyoffset,int mode)` |
|14 | `stbds_shmode_func`    | yes | yes | `void* (size_t elemsize,int mode)` |
|15 | `stbds_stralloc`       | yes | yes | `char* (stbds_string_arena*,char*)` |
|16 | `stbds_strreset`       | yes | yes | `void (stbds_string_arena*)` |

`comm -23 c_syms rust_syms` ⇒ **empty**. `comm -13` ⇒ **empty**.
No symbol is missing in either direction; nothing is stubbed.

## Declared-but-never-defined in the C TU (correctly absent from both .so)

`stbds_unit_tests` is `extern`-declared at `lib.c:83` but never defined, so the
C `.so` does not export it. The Rust `.so` must not export it either — it does
not.

## Undefined (imported) symbols

Both libraries import only libc / toolchain-runtime symbols.

* C imports: `__assert_fail memcmp memcpy memmove memset malloc free realloc
  sprintf strcmp strlen` + `__cxa_finalize __gmon_start__ _ITM_*`.
* Rust imports: `malloc calloc free realloc posix_memalign memcpy memmove
  memset bcmp strlen abort` + Rust std runtime (`_Unwind_*`, `pthread_key_*`,
  `dl_iterate_phdr`, …). All libc / libgcc; **0 missing non-libc symbols.**

Important consequence of the C import list: **`__assert_fail` is present, i.e.
the C library is compiled WITHOUT `-DNDEBUG` (CMakeLists.txt sets no build
type), so every `STBDS_ASSERT` is live in the C `.so`.** See `ERRORS.md`.

## Layout contract shared across the FFI boundary

Verified by `const _: () = assert!(...)` in `src/lib.rs` and re-verified from
the C side by the tests, since callers of the `stbds_*` API poke at the header
directly:

| type | size | notable offsets |
|------|------|-----------------|
| `stbds_array_header` | 32 | `length` 0, `capacity` 8, `hash_table` 16, `temp` 24 |
| `stbds_string_block` | 16 | `next` 0, `storage` 8 |
| `stbds_string_arena` | 24 | `storage` 0, `remaining` 8, `block` 16, `mode` 17 |
| `stbds_hash_bucket`  | 128 | `hash` 0, `index` 64 |
| `stbds_hash_index`   | 104 | `temp_key` 0, `seed` 56, `slot_count_log2` 64, `string` 72, `storage` 96 |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. `cargo check`/`cargo test`
with `--no-default-features` is therefore identical to the default build; both
are exercised by `run_all.sh` for completeness.

`crate-type = ["cdylib"]` only — the project builds **no binary/driver
executable**, so the "compare C and Rust stdout" gate is vacuous (there is no
`main` in either tree; `c_src/CMakeLists.txt` builds `SHARED` only).
`run_all.sh` asserts that premise (it greps for `[[bin]]` / `src/main.rs` /
`add_executable`) instead of assuming it.

## How the parity check is run

`run_all.sh` recomputes the diff for every feature combination:

```
nm -D --defined-only <c.so>    | awk '{print $3}' | sort -u > c_syms
nm -D --defined-only <rust.so> | awk '{print $3}' | sort -u > r_syms
comm -23 c_syms r_syms   # must be empty: symbols missing from Rust
comm -13 c_syms r_syms   # must be empty: symbols Rust exports and C does not
```

It also refuses to pass if `nm` lists 0 symbols for either object, so a
mistyped path or an unwritable scratch directory cannot fake an empty diff.

## Caveat that affects every symbol here

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact (the lib
target is compiled as a test-harness binary instead). Without a guard the
differential suite silently `dlopen`s a **stale** `libarr_push_lib.so` and
reports green for sources it never exercised. `tests/common/mod.rs`
(`assert_so_is_fresh`) now panics if the `.so` is older than any `src/**/*.rs`
or `Cargo.toml`, and `run_all.sh` always runs `cargo build` before
`cargo test`.
