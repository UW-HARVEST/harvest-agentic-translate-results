# SYMBOLS.md — Phase A symbol surface

Derived mechanically from

```
nm -D --defined-only c_src/build/libharvest-work-n7doav.so
nm -D --defined-only translation/target/release/libsh_geti_lib.so
```

The C `.so` name is derived by `c_src/CMakeLists.txt` from the *parent
directory* name (`cmake_path(GET parent FILENAME project_name)`), so the file is
`lib<workdir-name>.so`.  The tests glob for `c_src/build/lib*.so`.

## C source inventory (completeness check)

`c_src` contains exactly one translation unit:

| file | status |
|------|--------|
| `c_src/include/lib.h` | 1 line: `void sh_geti(int num);` — translated |
| `c_src/src/lib.c`     | 986 lines — fully translated in `translation/src/lib.rs` |

There is **no** un-translated C module: every non-`static` function definition in
`lib.c` has a `#[no_mangle] extern "C"` counterpart in `src/lib.rs`, and every
`static` (file-local) function is translated as a private Rust `fn`.

### `static` / internal C functions (deliberately NOT exported — C does not export them either)

| C symbol | Rust counterpart |
|----------|------------------|
| `static size_t stbds_probe_position(...)`   | `fn stbds_probe_position` |
| `static size_t stbds_log2(...)`             | `fn stbds_log2` |
| `static stbds_hash_index *stbds_make_hash_index(...)` | `unsafe fn stbds_make_hash_index` |
| `static size_t stbds_siphash_bytes(...)`    | `unsafe fn stbds_siphash_bytes` |
| `static int stbds_is_key_equal(...)`        | `unsafe fn stbds_is_key_equal` |
| `static ptrdiff_t stbds_hm_find_slot(...)`  | `unsafe fn stbds_hm_find_slot` |
| `static char *stbds_strdup(...)`            | `unsafe fn stbds_strdup` |
| `static char buffer[256]`                   | `static mut buffer: [c_char; 256]` |
| `static size_t stbds_hash_seed`             | `static mut stbds_hash_seed: usize` |
| `STBDS_SIPROUND()` macro                    | `fn stbds_sipround` |
| `stbds_load_32_or_64` macro                 | inlined in `stbds_make_hash_index` |
| all `stbds_arr*` / `stbds_hm*` / `stbds_sh*` *macros* | inlined helpers (`sh_len`, `sh_put`, `sh_get`, `sh_del`, `sh_geti_macro`, `stbds_header`, `stbds_temp`, …) |

`stbds_unit_tests` is only `extern`-declared in `lib.c`, never defined, so it is
not (and must not be) an exported symbol.

## Exported (`T`) symbol parity

| # | symbol | C `.so` | Rust `.so` | note |
|---|--------|---------|-----------|------|
| 1  | `stbds_arrgrowf`      | T | T | dynamic-array (re)allocation |
| 2  | `stbds_arrfreef`      | T | T | frees `stbds_header(a)` |
| 3  | `stbds_rand_seed`     | T | T | sets the global `stbds_hash_seed` |
| 4  | `stbds_hash_string`   | T | T | rotate/mix string hash |
| 5  | `stbds_hash_bytes`    | T | T | SipHash-2-4 (64-bit only) |
| 6  | `stbds_hmfree_func`   | T | T | frees map + table + strdup'd keys + arena |
| 7  | `stbds_hmget_key_ts`  | T | T | lookup, index returned through `*temp` |
| 8  | `stbds_hmget_key`     | T | T | lookup, index stored in `header->temp` |
| 9  | `stbds_hmput_default` | T | T | materialises the `[-1]` default element |
| 10 | `stbds_hmput_key`     | T | T | insert / find-or-insert, grows + rebuilds |
| 11 | `stbds_shmode_func`   | T | T | new string map with an explicit `string.mode` |
| 12 | `stbds_hmdel_key`     | T | T | delete (swap-with-last), shrink / rebuild |
| 13 | `stbds_stralloc`      | T | T | string arena bump allocator |
| 14 | `stbds_strreset`      | T | T | frees the arena block chain, zeroes arena |
| 15 | `strkey`              | T | T | `sprintf(buffer, "test_%d", n)` |
| 16 | `sh_geti`             | T | T | the driver / self-test entry point |

Missing from Rust: **none**.  Extra in Rust: **none**.

## Undefined (`U`) symbols

C `.so` imports (all libc): `__assert_fail`, `free`, `malloc`, `memcmp`,
`memcpy`, `memmove`, `memset`, `printf`, `realloc`, `sprintf`, `strcmp`,
`strlen` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same libc set (`realloc`, `free`, `memset`,
`memcpy`, `memmove`, `memcmp`, `strcmp`, `strlen`, `printf`, `sprintf`,
`abort`) plus the usual Rust runtime/unwind-free `panic = "abort"` imports.
`abort` replaces `__assert_fail`: glibc's `assert` failure path ends in
`abort()`/`SIGABRT`, and so does the Rust `STBDS_ASSERT!` macro, so an
assertion failure is observationally identical (process killed by `SIGABRT`)
apart from the diagnostic text glibc prints on `stderr`.

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the complete
feature matrix is a single configuration (the empty default set):

```
cargo test --offline                                  # default (== all)
cargo test --offline --no-default-features             # identical, no features exist
```

Both are exercised by `check_all_features.sh`.

## Reproducing the verification

```bash
# 1. build the C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build the Rust cdylib and run every phase under every feature combination
cd ../../translation
./check_all_features.sh          # cargo check + build + symbol parity + full test suite

# or piecemeal:
cargo build --offline --release
./check_symbols.sh               # Phase D symbol diff (must print "0 missing symbols")
cargo test  --offline --release -- --test-threads=1
```

`--test-threads=1` is recommended: `stbds_hash_seed` is a *global* inside each
`.so` and every fresh hash index advances it, so the two libraries must be
stepped in lock-step.  The harness also guards this with an internal mutex
(`common::seed_guard`), so parallel runs are correct too — single-threaded just
makes failures easier to read.

## Test inventory

| test target | substantive tests | covers |
|-------------|------------------|--------|
| `tests/hash_fns.rs` | 15 | CONFIGS rows 1–13, 16–17 (`stbds_hash_bytes`, `stbds_hash_string`, `strkey`) |
| `tests/arrays.rs`   | 7  | CONFIGS rows 18–24 (`stbds_arrgrowf`, `stbds_arrfreef`) |
| `tests/arena.rs`    | 9  | CONFIGS rows 25–33 (`stbds_stralloc`, `stbds_strreset`) |
| `tests/hashmap.rs`  | 39 | CONFIGS rows 14–15, 34–75 (every hash-map entry point) |
| `tests/fuzz.rs`     | 5  | CONFIGS rows 76–79 (randomized op sequences, full-state compare per op) |
| `tests/sh_geti.rs`  | 8  | CONFIGS rows 80–86 (`sh_geti`, stdout compared byte-for-byte) |
| `tests/errors.rs`   | 63 | ERRORS rows 1–66 (some rows share one test), incl. 2 subprocess `SIGABRT` scenarios |
| **total**           | **146** | |

Plus two no-op helper `#[test]`s used as subprocess entry points
(`common::zzz_shgeti_worker`, present in every binary, and
`errors.rs::zzz_abort_worker`); they return immediately unless the parent sets
the corresponding environment variable, so `cargo test` reports 154 tests.

Every hash-map test compares the **entire** observable state after **every**
operation: the 32-byte array header (`length`, `capacity`, `hash_table != NULL`,
`temp`), every live element (key content — dereferenced for the pointer storage
modes — plus the value bytes), all eleven `stbds_hash_index` scalar fields, the
embedded `stbds_string_arena`, and every `hash[]`/`index[]` entry of every
bucket.  Because the global `stbds_hash_seed` is reset identically in both
libraries first, the bucket layouts must match bit-for-bit, not merely
"behave equivalently".

## Harness notes

* Both libraries are loaded with `libloading::Library::new` and driven **only**
  through their exported C symbols, so the `#[no_mangle]` wrappers are part of
  what is verified.
* Raw addresses are never compared across the two `.so`s (they have independent
  heap histories).  What is compared is the *content* they point at, plus the
  *decisions* the C makes (e.g. "did `stbds_arrgrowf` return the input pointer
  unchanged?", validated against a transcription of the C growth model).
* Newly inserted elements have an **uninitialised value area** (`stbds_hmput_key`
  only writes the key bytes), so the harness fills it with a deterministic
  key-derived pattern immediately after each insert — exactly what the
  `stbds_hmput`/`shput` macros do with the user's value — before comparing.
* `table->temp_key` is never initialised by `stbds_make_hash_index`; it is only
  compared where the C provably writes it, after `MapPair::zero_temp_key`.
* `sh_geti`'s stdout is captured in a **subprocess** (`common::sh_geti_diff`),
  because `capture_stdout` redirects the process-wide fd 1 and libtest also
  writes its progress lines there.
* The harness refuses to run against a Rust `.so` older than `src/lib.rs`
  (`cargo test` does not rebuild a `cdylib`).  `HARVEST_RUST_SO` and
  `HARVEST_C_SO` override the auto-discovered paths.
