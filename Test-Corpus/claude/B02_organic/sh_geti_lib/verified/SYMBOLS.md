# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/release/libsh_geti_lib.so` (`[lib] name = "sh_geti_lib"`,
  `crate-type = ["cdylib"]`)

## Build-time configurations

`Cargo.toml` has **no `[features]` section** — therefore the only valid feature
combination is the empty one:

| # | cargo invocation                                     | notes                       |
|---|------------------------------------------------------|-----------------------------|
| 1 | `cargo check --no-default-features`                  | == default, no features exist |

`c_src/CMakeLists.txt` has no `option()`/`add_definitions()`/`target_compile_definitions()`
either: a single `add_library(SHARED src/lib.c)` with `-fPIC`, linking `m`.
`CMAKE_BUILD_TYPE` is empty ⇒ **no `-DNDEBUG`** ⇒ every `STBDS_ASSERT`
(`assert`) in the C source is **live** (confirmed: `__assert_fail@GLIBC_2.2.5`
appears in `nm -D --undefined-only`).

So there is exactly **one** build configuration to verify. It is nevertheless
verified twice below (Rust `debug` and `release` artifacts) because the `dev`
profile enables integer-overflow checks that the C code does not have.

## Exported symbol parity

16 symbols in the C `.so`; all 16 present in the Rust `.so`.

| #  | symbol                | C `.so` | Rust `.so` | Rust definition site            |
|----|-----------------------|---------|------------|---------------------------------|
| 1  | `stbds_arrgrowf`      | T       | T          | `src/stb_ds.rs`                 |
| 2  | `stbds_arrfreef`      | T       | T          | `src/stb_ds.rs`                 |
| 3  | `stbds_rand_seed`     | T       | T          | `src/stb_ds.rs`                 |
| 4  | `stbds_hash_string`   | T       | T          | `src/stb_ds.rs`                 |
| 5  | `stbds_hash_bytes`    | T       | T          | `src/stb_ds.rs`                 |
| 6  | `stbds_hmfree_func`   | T       | T          | `src/stb_ds.rs`                 |
| 7  | `stbds_hmget_key_ts`  | T       | T          | `src/stb_ds.rs`                 |
| 8  | `stbds_hmget_key`     | T       | T          | `src/stb_ds.rs`                 |
| 9  | `stbds_hmput_default` | T       | T          | `src/stb_ds.rs`                 |
| 10 | `stbds_hmput_key`     | T       | T          | `src/stb_ds.rs`                 |
| 11 | `stbds_shmode_func`   | T       | T          | `src/stb_ds.rs`                 |
| 12 | `stbds_hmdel_key`     | T       | T          | `src/stb_ds.rs`                 |
| 13 | `stbds_stralloc`      | T       | T          | `src/stb_ds.rs`                 |
| 14 | `stbds_strreset`      | T       | T          | `src/stb_ds.rs`                 |
| 15 | `strkey`              | T       | T          | `src/harness.rs`                |
| 16 | `sh_geti`             | T       | T          | `src/harness.rs`                |

### Symbols intentionally NOT exported

These are `static` in `c_src/src/lib.c` and therefore do not appear in the C
`.so` dynamic symbol table; the Rust translation keeps them private too:

`stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`, `stbds_hash_seed`, `buffer`.

`stbds_unit_tests` is only `extern`-declared in the C source, never defined, so
it is absent from both `.so` files (it is not referenced either, hence not even
`U`).

### Undefined (imported) symbols

The Rust `.so` must not import anything outside libc. Verified with
`nm -D --undefined-only`: only `libc`/`ld` entries
(`realloc`, `free`, `printf`, `abort`, `write`, `memcpy`, `memmove`, `memset`,
plus the usual weak `_ITM_*`, `__gmon_start__`, `__cxa_finalize`,
`__cxa_thread_atexit_impl`, `_Unwind_*` / `__rust_*` personality glue).

### Result

```
comm -23 <(nm -D --defined-only C.so    | awk '$2=="T"{print $3}' | sort) \
         <(nm -D --defined-only rust.so | awk '$2=="T"{print $3}' | sort)
```
prints nothing ⇒ **0 missing symbols**. Confirmed for both the `release` and the
`debug` Rust artifact by `./run_verification.sh` (which prints
`C exports: 16 / Rust exports: 16 / symbol diff is EMPTY (0 missing)` for each),
and recomputed at test time by `tests/symbols.rs::symbol_parity`.
`tests/symbols.rs::rust_imports_only_libc` additionally asserts the Rust `.so`
imports nothing outside libc, and `tests/symbols.rs::both_libs_load` proves the
two `.so`s can be `dlopen`ed side by side without interposing on each other
(they are loaded `RTLD_LOCAL`, so each resolves its own internal calls).

### How to reproduce

```sh
./run_verification.sh      # C build + every feature combo + both profiles
```
