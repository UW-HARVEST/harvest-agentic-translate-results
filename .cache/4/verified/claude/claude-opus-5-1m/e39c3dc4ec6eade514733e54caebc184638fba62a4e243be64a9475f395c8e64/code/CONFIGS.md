# CONFIGS.md — configuration surface table (Phase B)

Mechanically derived from the branches the C in `c_src/src/lib.c` actually
takes. Rows are the pruned cross-product of the axes below.

## Build-time configuration

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` defines
no options / `#ifdef` switches (`C_DEFINES = -Dtranslated_rust_EXPORTS` only).
Therefore there is exactly **one** feature combination:

| # | combination | `cargo check` | `cargo test` |
|---|-------------|---------------|--------------|
| F1 | `--no-default-features` (≡ default, no features exist) | ok | ok — 112 tests |

Confirmed mechanically by `scripts/check_all_features.sh`, which extracts the
`[features]` table from `Cargo.toml` (empty), builds the power set (one element:
the empty set), and for each element runs `cargo check`, `cargo build` (dev +
release), an `nm -D` symbol diff against the C `.so`, and the whole test suite.
The same script greps `c_src/CMakeLists.txt` for `option()`/conditionals and the
C sources for `#ifdef`/`#ifndef`/`#if` (result: **0** of each), proving the C has
no configuration axis either.

The `#define`s inside `lib.c` are all unconditional (`STBDS_HAS_TYPEOF`,
`STBDS_SIPHASH_2_4`, `STBDS_BUCKET_LENGTH 8`, `STBDS_REALLOC/FREE` = libc, …),
so there is no C-side configuration axis either. Verified as part of Phase D by
`scripts/check_all_features.sh`.

## Runtime axes the C branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` (int, any value accepted across FFI) | `< 1` → binary (`memcmp` + `hash_bytes`); `>= 1` → string (`strcmp` + `hash_string`); `== 1` **exactly** for the strdup-free in `hmdel_key` | `lib.c:560`, `590`, `713`, `836` |
| `table->string.mode` | `SH_NONE(0)`/other → `memcpy(key,keysize)`; `SH_DEFAULT(1)` → store key pointer; `SH_STRDUP(2)` → `stbds_strdup`; `SH_ARENA(3)` → `stbds_stralloc` | `lib.c:785-790` |
| how the table is created | implicit (`hmput_key` on `NULL`, `string.mode` derived from `mode`) vs explicit (`stbds_shmode_func(elemsize, mode)`) | `lib.c:698-710` vs `796-805` |
| `elemsize` | any; `>= sizeof(char*)` needed for string modes; drives all pointer arithmetic | everywhere |
| `keysize` | `0`, `4`, `8`, `16`, `> elemsize` | `hash_bytes`, `memcmp`, `memcpy` |
| `keyoffset` | `0` (what `hmput_key`/`hmget_key` hard-code) vs non-zero (only `hmdel_key` takes it) | `lib.c:633`, `682`, `807` |
| entry count vs `used_count_threshold` | `sc - (sc>>2)`: `6` @8 slots, `12` @16, `24` @32 → grow points | `lib.c:698` |
| entry count vs `used_count_shrink_threshold` | `sc>>2` (`0` when `sc<=8`): `4` @16, `8` @32 → shrink points | `lib.c:854` |
| tombstones vs `tombstone_count_threshold` | `(sc>>3)+(sc>>4)`: `1` @8, `3` @16, `6` @32 → rebuild points | `lib.c:858` |
| probe path | `pos & 7 == 0` (forward half only) vs `pos & 7 != 0` (needs the wrap-around `i < limit` half) vs multi-bucket probe (`step += 8`) | `lib.c:604-627`, `728-763` |
| `hash < 2` | bumped by `+2` | `lib.c:596`, `719` |
| `stbds_hash_bytes` `len` | `0`, `1..7` (each fall-through `case`), `8`, `9..15`, exact multiples of 8, long buffers | `lib.c:522-541` |
| tail-byte values | `d[3] >= 0x80` (sign-extended `int` shift quirk), `d[4..6] >= 0x80` (explicit `size_t` casts) | `lib.c:533-539` |
| `seed` | `0`, `1`, `0x31415926` (initial), `usize::MAX`, random | `hash_bytes`, `hash_string`, `rand_seed` |
| global seed evolution | every `stbds_make_hash_index(sc, NULL)` advances `stbds_hash_seed = seed*a + b`; `ot != NULL` inherits instead | `lib.c:403-413` |
| `stbds_arrgrowf` growth ladder | `min_cap <= cap` (no-op) / `min_cap < 2*cap` / `min_cap < 4` / else | `lib.c:283-292` |
| `stbds_arrgrowf` `a` | `NULL` (fresh: zero `length`/`hash_table`/`temp`) vs non-`NULL` (preserve) | `lib.c:300-306` |
| arena `remaining` vs `len` | `len <= remaining` (fast) / `len > remaining` & `len <= blocksize` (new block) / `len > blocksize` (dedicated block) | `lib.c:885-911` |
| arena `block` | `0..21` → `blocksize = 512 << (block>>1)` and `++block`; `>= 22` → saturate | `lib.c:886-891` |
| arena `storage` | `NULL` vs non-`NULL` on the oversize path (different splice + `remaining`) | `lib.c:896-903` |
| `arr_push` `num` | `<= 0`, `1..50`, `51..100`, `> 100` (multiple outer iterations, repeated alloc/free) | `lib.c:951-955` |
| `strkey` `n` | `0`, positive, negative, `INT_MIN`, `INT_MAX` | `lib.c:939-943` |

## Rows

Legend: `B` = `STBDS_HM_BINARY (0)`, `S` = `STBDS_HM_STRING (1)`,
`P` = `2` (`PTR_TO_STRING`). "RNG×N" = N randomized inputs, fixed seed.

Every row N is covered by the test function named `cfgN...`. A name of the form
`cfgN_M_...` covers rows **N through M inclusive** (e.g.
`cfg20_23_binary_grow_thresholds` covers rows 20, 21, 22 and 23 by sweeping the
entry count over `1..=100`, which hits the 1-entry, 5-entry, 6-entry and
7/12/13/24/25-entry shapes those rows name). A trailing letter
(`cfg12b`, `cfg65a`..`cfg65d`, `cfg50b`, `cfg50c`) is an additional test for the
same row.

| rows | tests |
|------|-------|
| 20-23 | `cfg20_23_binary_grow_thresholds` |
| 25-27 | `cfg25_26_27_binary_other_shapes` |
| 32-34 | `cfg32_33_34_binary_delete_positions` |
| 35-36 | `cfg35_36_binary_rebuild_and_shrink` |
| 40-42 | `cfg40_41_42_string_default_mode` |
| 44-45 | `cfg44_45_string_arena` |
| 54-56 | `cfg54_55_56_string_hmfree` |
| 57-58 | `cfg57_58_stralloc_fresh_arena` |
| 5-6 | `cfg05_06_hash_bytes_words_and_tails` |
| 65 | `cfg65a_array_header_layout`, `cfg65b_hash_index_size_and_storage_alignment`, `cfg65c_hash_bucket_layout`, `cfg65d_string_block_layout` |

All other rows N have a dedicated `cfgN_...` test. By file:

| rows | test file |
|------|-----------|
| 1-12  | `tests/hash.rs` (plus the `cfg_exhaustive_*` sweeps) |
| 13-19 | `tests/arr.rs` |
| 20-39 | `tests/map_binary.rs` |
| 40-56 | `tests/map_string.rs` |
| 57-64 | `tests/arena.rs` |
| 65-67 | `tests/crosscut.rs` |

All randomized rows use the fixed-seed splitmix64 in `tests/common/mod.rs`, so
every run is reproducible. Both implementations are reached only through
`dlopen`/`dlsym` on their `.so` files (`tests/common/mod.rs::load`); the Rust
crate is never linked directly, so the `#[no_mangle]` export wrappers are part of
what is under test.

Rows that involve *state machines* (grow / shrink / rehash / tombstone reuse)
compare a full structural snapshot after **every single operation** — array
header, every element, all `stbds_hash_index` counters/thresholds/seed, the
string-arena state, and every bucket's `hash[]`/`index[]` (see
`tests/common/mod.rs::snap_map`).

### Group 1 — `stbds_hash_bytes` / `stbds_hash_string` / `stbds_rand_seed`

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 1 | `stbds_hash_bytes` | `len = 0`, `seed` ∈ {0, 1, 0x31415926, MAX, RNG×64}, buffer ignored | [x] |
| 2 | `stbds_hash_bytes` | `len` ∈ `1..=7` (every fall-through case), all-zero bytes | [x] |
| 3 | `stbds_hash_bytes` | `len` ∈ `1..=7`, tail bytes forced `>= 0x80` (sign-extension quirk of `d[1..3]<<k`) | [x] |
| 4 | `stbds_hash_bytes` | `len` ∈ `1..=7`, RNG×256 bytes each, RNG seeds | [x] |
| 5 | `stbds_hash_bytes` | `len = 8` exactly (one full word, empty tail) | [x] |
| 6 | `stbds_hash_bytes` | `len` ∈ `8..=64`, RNG×512 bodies, RNG seeds (word loop + every tail length) | [x] |
| 7 | `stbds_hash_bytes` | `len` ∈ {128, 256, 1024, 4096} RNG bodies (many word iterations) | [x] |
| 8 | `stbds_hash_bytes` | `len` = multiple of 8 with every byte `0xFF` / `0x80` / `0x00` (extreme words) | [x] |
| 9 | `stbds_hash_string` | `""`, `"a"`, `"test_0"`, 1..64-byte RNG ASCII strings ×512, RNG seeds | [x] |
| 10 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (unsigned-char read) ×256 | [x] |
| 11 | `stbds_hash_string` | long strings (256, 1024, 4096 bytes) RNG | [x] |
| 12 | `stbds_rand_seed` + `stbds_shmode_func` | seed ∈ {0, 1, MAX, RNG×32}: check the global seed evolution `seed*a+b` is identical by observing `table->seed` of successive fresh indices | [x] |

### Group 2 — `stbds_arrgrowf` / `stbds_arrfreef` / `arr_push` / `strkey`

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 13 | `stbds_arrgrowf` | `a = NULL`, `elemsize` ∈ {1,2,4,8,16,24,32,100}, `addlen` ∈ {0,1,2,3,4,5,7,8,100}, `min_cap` ∈ {0,1,2,3,4,5,8,100} — full cross product, check `length/capacity/hash_table/temp` | [x] |
| 14 | `stbds_arrgrowf` | non-`NULL` `a` from a previous grow, `addlen`/`min_cap` cross product (preserves `length`, `hash_table`, `temp`; doubling ladder) | [x] |
| 15 | `stbds_arrgrowf` | repeated grow chain (`push` one element at a time, 0→300 elements) — capacity sequence must match exactly | [x] |
| 16 | `stbds_arrgrowf` | no-op branch: `min_cap <= cap` returns the *same* pointer, header untouched | [x] |
| 17 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (valid pointer), and grow/free interleaved ×100 RNG sizes | [x] |
| 18 | `arr_push` | `num` ∈ {0, 1, 49, 50, 51, 100, 101, 150, 500, 1000, 5000} — must not crash and must leave no state | [x] |
| 19 | `strkey` | `n` ∈ {0, 1, -1, 9, 10, 99, 100, 12345, -12345, INT_MAX, INT_MIN, RNG×256} — compare the 256-byte result buffer contents | [x] |

### Group 3 — binary-mode hash map (`mode = 0`), the low-level entry points

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 20 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=8`, `keysize=4` (int key/int value), 1 entry | [x] |
| 21 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=8`, `keysize=4`, 5 entries (below the 8-slot grow threshold of 6) | [x] |
| 22 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=8`, `keysize=4`, 6 entries (exactly at the grow threshold) | [x] |
| 23 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=8`, `keysize=4`, 7/12/13/24/25 entries (each grow point) | [x] |
| 24 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=8`, `keysize=4`, 500 RNG entries incl. duplicate keys | [x] |
| 25 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=16`, `keysize=8` (`size_t`/`int[2]` key) 200 RNG entries | [x] |
| 26 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=32`, `keysize=16` 200 RNG entries | [x] |
| 27 | `hmput_key`+`hmget_key` | `mode=B`, `elemsize=4`, `keysize=4` (element == key, no value) | [x] |
| 28 | `hmput_key`+`hmget_key_ts` | `mode=B`, same shapes as 24, using the `_ts` variant and checking `*temp` **and** that `header->temp` is *not* written | [x] |
| 29 | `hmput_key`+`hmget_key` | `mode=B`, `keysize=0` (all keys collapse) | [x] |
| 30 | `hmput_key` | `mode=B`, keys chosen so the hash lands on `pos & 7 != 0` (wrap-around probe half) — RNG×200 with `keysize=4` covers it; assert coverage via bucket dump | [x] |
| 31 | `hmput_default`+`hmput_key`+`hmget_key` | `mode=B`, default value set first, then puts/gets | [x] |
| 32 | `hmdel_key` | `mode=B`, delete the only entry | [x] |
| 33 | `hmdel_key` | `mode=B`, delete the last entry (`old_index == final_index`) | [x] |
| 34 | `hmdel_key` | `mode=B`, delete a middle entry (swap-with-last + re-lookup path) | [x] |
| 35 | `hmdel_key` | `mode=B`, delete enough to cross `tombstone_count_threshold` (rebuild at same size) | [x] |
| 36 | `hmdel_key` | `mode=B`, 16/32-slot table, delete below `used_count_shrink_threshold` (shrink) | [x] |
| 37 | `hmput_key`+`hmdel_key`+`hmget_key` | `mode=B`, RNG×2000 mixed put/get/del ops, `keysize=4`, `elemsize=8` (long-running churn: grow, shrink, rebuild, tombstone reuse) | [x] |
| 38 | `hmdel_key` | `mode=B`, non-zero `keyoffset` with a matching `elemsize`/struct layout | [x] |
| 39 | `hmfree_func` | `mode=B`, after the churn of row 37 — must free without crashing (valgrind-free equivalence: both survive) | [x] |

### Group 4 — string-mode hash map

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 40 | `hmput_key`+`hmget_key` | `mode=S`, implicit table (`string.mode = SH_DEFAULT`), `elemsize=16`, `keysize=8`, 1 entry, caller-owned key pointers | [x] |
| 41 | `hmput_key`+`hmget_key` | `mode=S`, `SH_DEFAULT`, 6/7/12/13 entries (grow points), `strkey`-style keys | [x] |
| 42 | `hmput_key`+`hmget_key` | `mode=S`, `SH_DEFAULT`, 300 RNG string keys incl. duplicates | [x] |
| 43 | `shmode_func(SH_STRDUP)`+`hmput_key`+`hmget_key` | `mode=S`, `string.mode = SH_STRDUP`, 200 RNG keys; stored key must be a *copy* (differs from the input pointer, equal bytes) | [x] |
| 44 | `shmode_func(SH_ARENA)`+`hmput_key`+`hmget_key` | `mode=S`, `string.mode = SH_ARENA`, 200 RNG keys of assorted lengths → exercises `stbds_stralloc` block chaining from inside the map | [x] |
| 45 | `shmode_func(SH_ARENA)`+`hmput_key` | `mode=S`, arena, keys longer than the current blocksize (oversize dedicated blocks) | [x] |
| 46 | `shmode_func(SH_NONE)`+`hmput_key` | `mode=S` with `string.mode = SH_NONE` → `default:` `memcpy(key, keysize)` branch | [x] |
| 47 | `hmput_key`+`hmdel_key` | `mode=S`, `SH_STRDUP`, delete → key freed; delete middle → swap + string re-lookup | [x] |
| 48 | `hmput_key`+`hmdel_key` | `mode=S`, `SH_ARENA`, deletes (no free, arena retained) | [x] |
| 49 | `hmput_key`+`hmdel_key`+`hmget_key` | `mode=S`, `SH_DEFAULT`, RNG×1500 mixed churn | [x] |
| 50 | `hmput_key`+`hmdel_key`+`hmget_key` | `mode=S`, `SH_STRDUP`, RNG×1500 mixed churn | [x] |
| 51 | `hmput_key` | `mode=P (2)`, `string.mode=SH_DEFAULT` — `>= STBDS_HM_STRING` so string hashing/compare, and the `temp_key` write in the forward probe half | [x] |
| 52 | `hmput_key`+`hmdel_key` | `mode=P (2)`, `string.mode=SH_STRDUP` → row 31 of ERRORS.md: `hmdel_key` does **not** free (because `mode != 1`) | [x] |
| 53 | `hmput_key` | `mode=S`, duplicate key hitting the **wrap-around** probe half where the C forgets to set `temp_key` (`lib.c:746-751`) — the quirk must be reproduced | [x] |
| 54 | `hmfree_func` | `mode=S`, `SH_STRDUP` table after churn → per-entry frees + `strreset` | [x] |
| 55 | `hmfree_func` | `mode=S`, `SH_ARENA` table after churn → `strreset` frees the block chain | [x] |
| 56 | `hmfree_func` | `mode=S`, `SH_DEFAULT` table → only `strreset` (no per-entry frees) | [x] |

### Group 5 — string arena (`stbds_stralloc` / `stbds_strreset`) driven directly

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 57 | `stbds_stralloc` | fresh zeroed arena (`storage=NULL, remaining=0, block=0`), one short string | [x] |
| 58 | `stbds_stralloc` | fresh arena, `""` (len 1) | [x] |
| 59 | `stbds_stralloc` | fresh arena, sequence of RNG short strings until the 512-byte block is exhausted then beyond (`block` 0→1→2…, blocksize 512,512,1024,1024,…) | [x] |
| 60 | `stbds_stralloc` | fresh arena, single string longer than 512 (oversize, `storage == NULL` path) | [x] |
| 61 | `stbds_stralloc` | arena with an existing block, then an oversize string (`storage != NULL` splice path, `remaining` untouched) | [x] |
| 62 | `stbds_stralloc` | pre-set `block` ∈ `0..=24` (blocksize `512<<(block>>1)`, saturation at 22/23) with a string that forces a new block | [x] |
| 63 | `stbds_stralloc` | RNG×400 mixed lengths `1..=2000` on one arena (full block-chain + oversize interleaving) | [x] |
| 64 | `stbds_strreset` | arena with 0 / 1 / many chained blocks (incl. oversize splices) → all freed and arena zeroed | [x] |

### Group 6 — cross-cutting

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|------------------------------------------|---|
| 65 | all of `stbds_*` | struct layout / ABI: `sizeof` and field offsets of `stbds_array_header` (32), `stbds_hash_bucket` (128), `stbds_string_block` (16), `stbds_string_arena` (24), `stbds_hash_index` (104) — verified via the observable `hash_table`/`storage` offsets the exported functions produce | [x] |
| 66 | `stbds_rand_seed` + full map lifecycle | identical global seed at test start ⇒ identical `table->seed` ⇒ identical bucket placement for the whole run; sequencing of `make_hash_index` calls must match 1:1 | [x] |
| 67 | interleaved multi-map | two maps alive at once (different `elemsize`/`mode`), interleaved puts: the *shared global* `stbds_hash_seed` must advance in the same order | [x] |
