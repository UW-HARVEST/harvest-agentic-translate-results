# SYMBOLS.md — dynamic-symbol parity between the C `.so` and the Rust `.so`

Artifacts compared:

* C   : `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  no `CMAKE_BUILD_TYPE` ⇒ **`NDEBUG` is NOT defined ⇒ `assert()` is live**)
* Rust: `target/debug/libhm_geti_lib.so` (and `target/release/libhm_geti_lib.so`)

Regenerate with:

```sh
comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/debug/libhm_geti_lib.so      | awk '{print $3}' | sort)
```

## Defined (exported) symbols — every C symbol from `nm -D --defined-only`

| # | symbol | C signature (`c_src/src/lib.c`) | in Rust `.so` | notes |
|---|--------|--------------------------------|---------------|-------|
| 1 | `stbds_arrgrowf`     | `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)` | ✅ | `src/lib.rs:257` |
| 2 | `stbds_arrfreef`     | `void stbds_arrfreef(void *a)` | ✅ | `src/lib.rs:304` |
| 3 | `stbds_rand_seed`    | `void stbds_rand_seed(size_t seed)` | ✅ | `src/lib.rs:326` |
| 4 | `stbds_hash_string`  | `size_t stbds_hash_string(char *str, size_t seed)` | ✅ | `src/lib.rs:470` |
| 5 | `stbds_hash_bytes`   | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | ✅ | `src/lib.rs:597` |
| 6 | `stbds_hmfree_func`  | `void stbds_hmfree_func(void *a, size_t elemsize)` | ✅ | `src/lib.rs:628` |
| 7 | `stbds_hmget_key_ts` | `void *stbds_hmget_key_ts(void *a, size_t elemsize, void *key, size_t keysize, ptrdiff_t *temp, int mode)` | ✅ | `src/lib.rs:725` |
| 8 | `stbds_hmget_key`    | `void *stbds_hmget_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | ✅ | `src/lib.rs:759` |
| 9 | `stbds_hmput_default`| `void *stbds_hmput_default(void *a, size_t elemsize)` | ✅ | `src/lib.rs:773` |
| 10 | `stbds_hmput_key`   | `void *stbds_hmput_key(void *a, size_t elemsize, void *key, size_t keysize, int mode)` | ✅ | `src/lib.rs:794` |
| 11 | `stbds_shmode_func` | `void *stbds_shmode_func(size_t elemsize, int mode)` | ✅ | `src/lib.rs:981` |
| 12 | `stbds_hmdel_key`   | `void *stbds_hmdel_key(void *a, size_t elemsize, void *key, size_t keysize, size_t keyoffset, int mode)` | ✅ | `src/lib.rs:992` |
| 13 | `stbds_stralloc`    | `char *stbds_stralloc(stbds_string_arena *a, char *str)` | ✅ | `src/lib.rs:1106` |
| 14 | `stbds_strreset`    | `void stbds_strreset(stbds_string_arena *a)` | ✅ | `src/lib.rs:1164` |
| 15 | `strkey`            | `char *strkey(int n)` | ✅ | `src/lib.rs:1184` |
| 16 | `hm_geti`           | `void hm_geti(int num)` — the only symbol in `include/lib.h` | ✅ | `src/lib.rs:1319` |

**Symbol diff (C → Rust): EMPTY.** 16/16 exported, exact names, no stubs — every
Rust export is a full literal translation of the corresponding C body.

## `static` C functions (correctly NOT exported by either `.so`)

Mechanically: `grep -n '^static' c_src/src/lib.c`

| C symbol | Rust counterpart | exported? |
|----------|------------------|-----------|
| `static size_t stbds_probe_position(...)` | `stbds_probe_position` (private fn) | no / no ✅ |
| `static size_t stbds_log2(...)` | `stbds_log2` (private fn) | no / no ✅ |
| `static stbds_hash_index *stbds_make_hash_index(...)` | `stbds_make_hash_index` (private unsafe fn) | no / no ✅ |
| `static size_t stbds_siphash_bytes(...)` | `stbds_siphash_bytes` (private unsafe fn) | no / no ✅ |
| `static int stbds_is_key_equal(...)` | `stbds_is_key_equal` (private unsafe fn) | no / no ✅ |
| `static ptrdiff_t stbds_hm_find_slot(...)` | `stbds_hm_find_slot` (private unsafe fn) | no / no ✅ |
| `static char *stbds_strdup(char *)` | `stbds_strdup` (private unsafe fn) | no / no ✅ |
| `static size_t stbds_hash_seed` (data) | `STBDS_HASH_SEED: AtomicUsize` | no / no ✅ |
| `static char buffer[256]` (data) | `static mut BUFFER: [c_char;256]` | no / no ✅ |

## Undefined (imported) symbols

The Rust `.so` must not need any non-libc import that the C `.so` does not have.

C `.so` imports (`nm -D` `U` entries): `__assert_fail`, `free`, `malloc`,
`memcmp`, `memcpy`, `memmove`, `memset`, `realloc`, `sprintf`, `strcmp`,
`strlen` — all glibc.

Rust `.so` imports: the same glibc set plus the usual Rust-runtime glibc/`ld.so`
imports (`pthread_*`, `dl_iterate_phdr`, `__tls_get_addr`, `write`, `abort`,
`getenv`, …). **0 undefined non-libc symbols** — the Rust `.so` resolves against
plain `libc`/`libgcc`/`libpthread` only, exactly like the C one.

Verify with:

```sh
nm -D --undefined-only target/debug/libhm_geti_lib.so | awk '{print $2}' | sort -u
ldd target/debug/libhm_geti_lib.so
```

## Declared-but-not-defined in this translation unit

`c_src/src/lib.c:83` declares `extern void stbds_unit_tests(void);` but never
defines it, so **it is not a symbol of the C `.so`** (it appears in neither
`nm -D --defined-only` nor as an `U` entry, since it is never called). The Rust
side correctly does not export it either — inventing it would be a stub that
lies about behaviour.

## Build-time configurations

`Cargo.toml` has **no `[features]` section**, so the complete set of valid
feature combinations is:

1. `--no-default-features` (empty feature set)
2. default features (also the empty feature set — identical)

Both are checked by `check_all_configs.sh`. The remaining build-time axis is the
cargo profile (`dev` vs `release`); both are built, both have their symbols
diffed against the C `.so`, and both are exercised by the full test suite (plus a
cross-check that runs the `dev`-profile tests against the `release` `.so`).

Two `Cargo.toml` changes were needed and are part of the verification result:

* `[dev-dependencies] libloading = "0.8"` — required by the differential tests.
* `[profile.dev] debug-assertions = false` / `overflow-checks = false` — rustc's
  debug assertions inject null-/misaligned-pointer checks around raw-pointer
  dereferences and overflow checks around arithmetic. Both are *off* in
  `release`, so with the cargo defaults the same source produced two artifacts
  with different behaviour: the `dev` `.so` turned C's `SIGSEGV` into `SIGABRT`
  for every null-pointer argument (`ERRORS.md` rows 5, 8, 9, 19, 52, 54, 59).
  Turning them off makes `dev` match both `release` and the C reference.

## Caveat worth knowing

`cargo test` does **not** rebuild a `cdylib`-only `[lib]` target, because an
integration test cannot link one. The `.so` the tests `dlopen` is therefore
easily stale. `tests/common/mod.rs` refuses to run if
`target/<profile>/libhm_geti_lib.so` is older than `src/lib.rs`; always use
`cargo build && cargo test`, or just `./check_all_configs.sh`.
