# Phase A.1 — Symbol surface

Mechanically derived from `nm -D` on both shared objects.

* C  : `c_src/build/libharvest-work-c7KYZo.so`  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`,
  empty `CMAKE_BUILD_TYPE` ⇒ **`NDEBUG` is NOT defined ⇒ `assert()` is live**, `-fPIC` only)
* Rust: `translation/target/release/libhelxo_lib.so` (`cargo build --release`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-*.so   | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libhelxo_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms     # must be empty
```

## Exported (defined, dynamic) symbols

| # | symbol | in C `.so` | in Rust `.so` | C source | notes |
|---|--------|-----------|---------------|----------|-------|
| 1 | `stbds_arrgrowf`      | T | T | lib.c:276 | array (re)allocation, returns payload ptr |
| 2 | `stbds_arrfreef`      | T | T | lib.c:312 | `free(header(a))`, no NULL check |
| 3 | `stbds_rand_seed`     | T | T | lib.c:355 | writes file-static `stbds_hash_seed` |
| 4 | `stbds_hash_string`   | T | T | lib.c:477 | pure |
| 5 | `stbds_hash_bytes`    | T | T | lib.c:553 | pure, wraps `stbds_siphash_bytes` |
| 6 | `stbds_hmfree_func`   | T | T | lib.c:571 | frees strdup'ed keys + arena + header |
| 7 | `stbds_hmget_key_ts`  | T | T | lib.c:631 | lookup, writes `*temp` |
| 8 | `stbds_hmget_key`     | T | T | lib.c:659 | lookup, writes `header->temp` |
| 9 | `stbds_hmput_default` | T | T | lib.c:667 | allocates the index-0 "default" slot |
| 10 | `stbds_hmput_key`    | T | T | lib.c:680 | insert/update + grow/rehash |
| 11 | `stbds_shmode_func`  | T | T | lib.c:796 | new string map with explicit `string.mode` |
| 12 | `stbds_hmdel_key`    | T | T | lib.c:807 | delete + tombstone + shrink/rebuild |
| 13 | `stbds_stralloc`     | T | T | lib.c:881 | string arena bump allocator |
| 14 | `stbds_strreset`     | T | T | lib.c:920 | frees arena block list, zeroes arena |
| 15 | `strkey`             | T | T | lib.c:939 | `sprintf(static buffer,"test_%d",n)` |
| 16 | `helxo`              | T | T | lib.c:945 | the `lib.h` entry point, prints to stdout |

`comm -23` diff (C-exported symbols missing from Rust): **EMPTY** — 16/16 present with
identical names. No symbol needed a new `#[no_mangle]` wrapper and no C module was
left untranslated (`c_src` contains exactly one translation unit, `src/lib.c`).

## Symbols the C `.so` *declares* but does not define

These are `extern` declarations in `lib.c` that are never defined nor referenced, so
they are absent from both `.so` files (`nm -D` shows neither) and must stay absent:

| symbol | why absent |
|--------|------------|
| `stbds_unit_tests` | declared `extern void stbds_unit_tests(void);` (lib.c:83), never defined/called |

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `malloc` / `realloc` / `free` | U | U | both use the *same* libc allocator (required: callers `free()` headers directly) |
| `memset` / `memcpy` / `memmove` / `memcmp` | U | U | |
| `strlen` / `strcmp` | U | U | |
| `printf` / `sprintf` | U | U | `helxo` / `strkey` |
| `__assert_fail` | U | U | live `assert()`s — Rust replicates them (see `ERRORS.md` rows 20‑25) |

Rust additionally imports the usual `libgcc_s`/`libc` unwinding+`std` symbols
(`__libc_start_main`, `pthread_*`, `dl_iterate_phdr`, …). Those are libc/runtime
symbols, not part of the library's API surface. Verify no *non-libc* symbol is
undefined with:

```sh
nm -D -u translation/target/release/libhelxo_lib.so | grep -v -E 'GLIBC|_ITM_|__gmon|__cxa|libc|libm|pthread|dl_iterate|__tls|_Unwind|__gcc'
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has exactly
one configuration: the default (`--no-default-features` is equivalent). Phases B–D
therefore run once; the automation script `check_features.sh` enumerates the feature
set mechanically and confirms the cardinality is 1.

## Verification log (final)

```
$ nm -D --defined-only ../c_src/build/libharvest-work-c7KYZo.so | wc -l   -> 16
$ nm -D --defined-only target/release/libhelxo_lib.so         | wc -l   -> 16
$ comm -23 <(c syms) <(rust syms)                                        -> (empty)
$ comm -13 <(c syms) <(rust syms)                                        -> (empty)
```

The Rust `.so` exports **exactly** the 16 C symbols and nothing else; every
undefined symbol it references is libc/libgcc (`@GLIBC_*`, `@GCC_*` or weak).
`./check_features.sh` re-runs this diff for the release *and* the dev cdylib.
