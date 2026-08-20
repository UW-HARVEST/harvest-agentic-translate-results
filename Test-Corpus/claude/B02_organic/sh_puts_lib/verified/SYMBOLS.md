# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Build configuration surface

* `c_src/CMakeLists.txt` — a single `add_library(... SHARED src/lib.c)` target.
  There are **no** `option()`s, no `target_compile_definitions`, no
  `CMAKE_BUILD_TYPE` branching, and `grep -E '#if|#ifdef|#ifndef|#else|#elif'
  c_src/src/lib.c` returns **nothing** → the C library has exactly **one**
  build configuration.
* `Cargo.toml` has **no `[features]` section** → the crate has exactly **one**
  feature combination (the empty set).  `--no-default-features`,
  `--all-features` and the bare default all resolve to the same build.
  All three were run through `cargo check` and pass (see `CONFIGS.md` header).

Therefore the full enumeration of valid build-time configurations is:

| # | configuration | `cargo check` |
|---|---------------|---------------|
| 1 | (default == no-default-features == all-features) | PASS |

## Symbol tables

C library:   `c_src/build/libtranslated_rust.so`
Rust library: `target/release/libsh_puts_lib.so`

Command used:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `sh_puts`             | T | T | the only symbol declared in `include/lib.h` |
| 2 | `stbds_arrfreef`      | T | T | |
| 3 | `stbds_arrgrowf`      | T | T | |
| 4 | `stbds_hash_bytes`    | T | T | |
| 5 | `stbds_hash_string`   | T | T | |
| 6 | `stbds_hmdel_key`     | T | T | |
| 7 | `stbds_hmfree_func`   | T | T | |
| 8 | `stbds_hmget_key`     | T | T | |
| 9 | `stbds_hmget_key_ts`  | T | T | |
| 10 | `stbds_hmput_default`| T | T | |
| 11 | `stbds_hmput_key`    | T | T | |
| 12 | `stbds_rand_seed`    | T | T | |
| 13 | `stbds_shmode_func`  | T | T | |
| 14 | `stbds_stralloc`     | T | T | |
| 15 | `stbds_strreset`     | T | T | |
| 16 | `strkey`             | T | T | non-`static` helper in `lib.c` |

**Diff: EMPTY.** 16 C exports, 16 Rust exports, identical names.

### Symbols deliberately absent from both

* `stbds_unit_tests` — only an `extern` *declaration* in `lib.c`; never defined,
  so it is not a definition in the C `.so` either (`nm -D --defined-only`
  confirms).  Not exported by Rust.  Correct.
* `stbds_hash_seed`, `buffer` — `static` in C (file-local); private in Rust.
* `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
  `stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
  `stbds_strdup` — `static` in C; private `fn`s in Rust.

### Undefined (imported) symbols

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
`std` runtime imports (`realloc`, `free`, `mem*`, `str*`, `printf`, `sprintf`,
`__assert_fail`, `malloc`, `calloc`, `posix_memalign`, `abort`,
`__errno_location`, `_Unwind_*`, `pthread_*`, `dl_iterate_phdr`, syscalls used
by `std`).  **0 missing/undefined non-libc symbols.**

The C `.so` imports the same libc set (`__assert_fail`, `free`, `malloc`,
`memcmp`, `memcpy`, `memmove`, `memset`, `printf`, `realloc`, `sprintf`,
`strcmp`, `strlen`).  The Rust build imports `bcmp` instead of `memcmp`
(glibc aliases them) plus the extra `std`/unwinder imports; both are satisfied
by the system libc/libgcc, so nothing is unresolved.

## Mechanised re-check

`./verify.sh` re-derives this table on every run: it diffs
`nm -D --defined-only` between the C `.so` and **both** the release and debug
Rust `.so`s, fails if the enumeration of build configurations above is stale
(a `[features]` section or a CMake `option()` appearing), and greps the Rust
`.so`'s undefined-symbol list for anything that is not libc / libgcc-unwind /
`std` runtime.  Latest run:

```
[ok] target/release/libsh_puts_lib.so exports all 16 C symbols (0 missing)
[ok] target/debug/libsh_puts_lib.so   exports all 16 C symbols (0 missing)
[ok] 0 unresolved non-libc symbols
```
