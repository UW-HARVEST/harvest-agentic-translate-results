# SYMBOLS.md — Phase A surface map

Mechanically derived from:

```sh
nm -D --defined-only c_src/build/libharvest-work-nHissd.so
nm -D --defined-only translation/target/release/libarr_push_lib.so
```

The C library is `c_src/src/lib.c` (a vendored `stb_ds.h` implementation unit
plus the two test helpers `strkey()` / `arr_push()` at the bottom of the file).
Everything the C `.so` exports is a plain `extern` function — there are no
macro-generated exported symbols, because every `stbds_*` *macro*
(`arrput`, `hmput`, `shget`, …) is header-only and therefore never lands in the
dynamic symbol table.  The macros still matter for testing (they define the
calling protocol into the exported functions) and are covered in `CONFIGS.md`.

## Exported (dynamic, defined) symbols

| # | C symbol | C source | Rust `#[no_mangle]` impl | in Rust `.so`? |
|---|----------|----------|--------------------------|----------------|
| 1 | `stbds_arrgrowf`      | lib.c:276 | `src/lib.rs` `stbds_arrgrowf`      | ✅ |
| 2 | `stbds_arrfreef`      | lib.c:312 | `src/lib.rs` `stbds_arrfreef`      | ✅ |
| 3 | `stbds_rand_seed`     | lib.c:355 | `src/lib.rs` `stbds_rand_seed`     | ✅ |
| 4 | `stbds_hash_string`   | lib.c:477 | `src/lib.rs` `stbds_hash_string`   | ✅ |
| 5 | `stbds_hash_bytes`    | lib.c:553 | `src/lib.rs` `stbds_hash_bytes`    | ✅ |
| 6 | `stbds_hmfree_func`   | lib.c:571 | `src/lib.rs` `stbds_hmfree_func`   | ✅ |
| 7 | `stbds_hmget_key_ts`  | lib.c:631 | `src/lib.rs` `stbds_hmget_key_ts`  | ✅ |
| 8 | `stbds_hmget_key`     | lib.c:659 | `src/lib.rs` `stbds_hmget_key`     | ✅ |
| 9 | `stbds_hmput_default` | lib.c:667 | `src/lib.rs` `stbds_hmput_default` | ✅ |
|10 | `stbds_hmput_key`     | lib.c:680 | `src/lib.rs` `stbds_hmput_key`     | ✅ |
|11 | `stbds_shmode_func`   | lib.c:796 | `src/lib.rs` `stbds_shmode_func`   | ✅ |
|12 | `stbds_hmdel_key`     | lib.c:807 | `src/lib.rs` `stbds_hmdel_key`     | ✅ |
|13 | `stbds_stralloc`      | lib.c:881 | `src/lib.rs` `stbds_stralloc`      | ✅ |
|14 | `stbds_strreset`      | lib.c:920 | `src/lib.rs` `stbds_strreset`      | ✅ |
|15 | `strkey`              | lib.c:939 | `src/lib.rs` `strkey`              | ✅ |
|16 | `arr_push`            | lib.c:945 | `src/lib.rs` `arr_push`            | ✅ |

**Symbol diff (`comm -23 c_syms rust_syms`) is EMPTY.**

## `static` (internal, not exported) C functions

These have no dynamic symbol and therefore need no export wrapper, but they must
still be translated because the exported functions call them.  All are present
in `src/lib.rs`:

| C symbol (static) | C source | Rust private fn |
|-------------------|----------|-----------------|
| `stbds_probe_position`  | lib.c:367 | `stbds_probe_position`  ✅ |
| `stbds_log2`            | lib.c:375 | `stbds_log2`            ✅ |
| `stbds_make_hash_index` | lib.c:385 | `stbds_make_hash_index` ✅ |
| `stbds_siphash_bytes`   | lib.c:498 | `stbds_siphash_bytes`   ✅ |
| `stbds_is_key_equal`    | lib.c:558 | `stbds_is_key_equal`    ✅ |
| `stbds_hm_find_slot`    | lib.c:586 | `stbds_hm_find_slot`    ✅ |
| `stbds_strdup`          | lib.c:870 | `stbds_strdup`          ✅ |
| `stbds_hash_seed` (var) | lib.c:353 | `STBDS_HASH_SEED`       ✅ |
| `buffer[256]` (var)     | lib.c:938 | `BUFFER`                ✅ |

## Declared-but-never-defined C externs

`lib.c:83` declares `extern void stbds_unit_tests(void);` but never defines it
and never calls it, so it appears in **neither** `.so` (not even as `U`).
Nothing to translate.

## Undefined (imported) symbols

The C `.so` imports only libc: `__assert_fail`, `free`, `malloc`, `memcmp`,
`memcpy`, `memmove`, `memset`, `realloc`, `sprintf`, `strcmp`, `strlen`.

The Rust `.so` imports libc + the libgcc unwinder (`_Unwind_*`) and the Rust
std backtrace/allocator plumbing.  **0 undefined non-libc/non-runtime symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set.  `--no-default-features` and
the default build are byte-identical.  See `run_all.sh`.

## ABI / layout parity

`src/lib.rs` carries `const _: () = assert!(...)` layout checks that are
verified again at runtime against the C library in
`tests/helpers.rs::layout_parity` (it reads back every field the C's own
`stbds_arrgrowf` / `stbds_shmode_func` wrote, through the mirror structs in
`tests/common/mod.rs`):

| type | size | notable offsets |
|------|------|-----------------|
| `stbds_array_header` | 32 | length 0, capacity 8, hash_table 16, temp 24 |
| `stbds_string_block` | 16 | next 0, storage 8 |
| `stbds_string_arena` | 24 | storage 0, remaining 8, block 16, mode 17 |
| `stbds_hash_bucket`  | 128 | hash 0, index 64 |
| `stbds_hash_index`   | 104 | temp_key 0, …, string 72, storage 96 |

---

## Phase D verification result

`./run_all.sh` (see the script for the exact commands) reports:

```
C exports:    16
Rust exports: 16
[ok]   symbol diff is EMPTY (0 missing)
[ok]   0 undefined non-libc/non-runtime symbols
```

Both the `default` and `--no-default-features` configurations, against BOTH the
release and the debug Rust `.so`, run the full 138-test differential suite green:

```
[ok]   DEFAULT   / rust-release : 138 tests passed
[ok]   DEFAULT   / rust-debug   : 138 tests passed
[ok]   NODEFAULT / rust-release : 138 tests passed
[ok]   NODEFAULT / rust-debug   : 138 tests passed
```

The debug profile matters because it leaves `overflow-checks` on: the suite
passing there proves the translation never relies on implicit wrapping that the
C performs deliberately (every such site uses `wrapping_*` explicitly).

## C-source coverage achieved by the differential suite

Built with `gcc -O0 --coverage` and driven through `C_SO=…/libcov.so`:

```
Lines executed:       100.00% of 374
Branches executed:     99.10% of 221
Taken at least once:   91.86% of 221
```

Every line of `c_src/src/lib.c` is executed by the differential tests, and every
execution was compared against the Rust `.so`.  The only branch edges never
taken are the **`assert()`-failed** edges of the eight `STBDS_ASSERT`s
(`lib.c:401, 778, 828, 832, 846, 849, 913, 950`) plus the non-NULL arm of
`arrlen(arr)` inside the `arr_push` assert — i.e. exactly the paths that
`ERRORS.md` rows E24/E29–E32/E38/E50/E56 assert must never be reached.  The C
`.so` is built **with asserts live** (it imports `__assert_fail@GLIBC_2.2.5`), so
if any of those invariants had been broken the test process would have aborted.

## Suite validation (negative control)

`./mutation_check.sh` injects 20 small edits into `src/lib.rs`, rebuilds each as
a separate `.so`, and confirms the suite detects them:

```
killed=18  survived=0  provably-equivalent=2
```

The two survivors are mathematically identical to the original
(`(x<<20)<<20 == (x<<21)<<19` for `x <= 255`; and `min_cap < 5 → min_cap = 4`
is a no-op because the only newly-covered value is `4`), so a green result for
them is the correct outcome.  Mutants covering the mode dispatch, the shrink
floor, `temp_key` propagation, `strkey`'s sign handling, the arena block
counter, the `hash < 2` bump, the growth clamp, the siphash tail, the load/
tombstone thresholds, `final_index`, the 64-byte `storage` alignment, the
`hash_string` rotation, `remaining` bookkeeping, the `hmput_default` condition,
the `hmdel` `temp` sentinel and the seed LCG constants were all caught.

## Test-file map

Every test loads BOTH `.so` files with `libloading` and calls only exported
symbols — no Rust function of the crate is ever called directly, so the
`#[no_mangle] extern "C"` wrappers are themselves under test.

| file | tests | covers |
|------|-------|--------|
| `tests/common/mod.rs`      | –  | harness: dual `dlopen`, C struct mirrors, state snapshots, and Rust re-implementations of every `arr*`/`hm*`/`sh*` header macro so both libraries are driven through the identical call protocol |
| `tests/hash_fns.rs`        | 12 | `CONFIGS.md` C1–C17 (`hash_bytes`, `hash_string`, `rand_seed` + seed LCG) |
| `tests/arrays.rs`          |  9 | C18–C27 (`arrgrowf`/`arrfreef` + all `arr*` macros) |
| `tests/hashmap_binary.rs`  | 16 | C28–C48 (BINARY map) |
| `tests/hashmap_string.rs`  | 18 | C49–C70 (STRING map × `SH_NONE`/`SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA`) |
| `tests/arena.rs`           |  9 | C71–C78 (`stralloc`/`strreset`) |
| `tests/helpers.rs`         |  5 | C79–C81 (`strkey`, `arr_push`) + symbol/layout parity |
| `tests/composed.rs`        |  6 | C82–C86 (end-to-end pipelines, fuzz across 8 model configs) |
| `tests/errors.rs`          | 51 | `ERRORS.md` E1–E63 |
| `tests/torture.rs`         | 12 | exhaustive length/position/alignment sweeps, `size_t`-overflow corners, 240 k random hashes, long randomized op sequences cross-checked after every op |
| **total**                  |**138**| |
