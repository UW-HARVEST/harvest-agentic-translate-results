# CONFIGS.md — valid-input configuration surface of `c_src/src/lib.c`

## Build-time configurations

`Cargo.toml` has **no `[features]`**; `c_src/CMakeLists.txt` has **no
`option()`/`-D`** and `lib.c` contains **no `#if*`**.  Exactly one build
configuration exists.  Verified:

```
cargo check                        -> PASS
cargo check --no-default-features  -> PASS
cargo check --all-features         -> PASS
```

All rows below are therefore exercised under *the* (single) feature
combination, which is simultaneously the default, the empty and the full set.

## Runtime axes the C actually branches on

* **`mode`** argument of `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key`:
  branch `mode >= STBDS_HM_STRING(1)` (string hashing + `strcmp`) vs `< 1`
  (siphash of `keysize` bytes + `memcmp`); `hmdel_key` additionally branches on
  `mode == 1` *exactly*.
* **`string.mode`** of the table (`STBDS_SH_NONE/DEFAULT/STRDUP/ARENA`), set
  either implicitly by `hmput_key` (`DEFAULT` for string mode, `0` otherwise) or
  explicitly by `stbds_shmode_func`.  Selects the `switch` in `hmput_key`
  (strdup / arena-alloc / borrow pointer / raw `memcpy`) and the free path in
  `hmfree_func`/`hmdel_key`.
* **table lifecycle**: no table → first `make_hash_index(8)`; growth when
  `used_count >= used_count_threshold` (`slot_count*2` + full rehash);
  shrink when `used_count < used_count_shrink_threshold && slot_count > 8`;
  rebuild when `tombstone_count > tombstone_count_threshold`.
* **bucket-probe geometry**: match/empty found in the upper half
  (`pos&7 .. 7`) vs the wrapped half (`0 .. pos&7`); tombstone reuse.
* **element shape**: `elemsize` (8/16/24/32/…), `keysize` (0/1/2/4/8/16),
  `keyoffset` (0 for put/get — hard-wired; caller-supplied for `hmdel_key`).
* **array shape**: `arrgrowf(a, elemsize, addlen, min_cap)` — `a` NULL/non-NULL,
  `min_len>min_cap`, `min_cap<=cap` (no-op), doubling vs the `min_cap<4 → 4`
  floor.
* **seed**: global `stbds_hash_seed` (`0x31415926` initially, settable with
  `stbds_rand_seed`) and its LCG advance on every fresh `make_hash_index`.
* **data shape for hashing**: `stbds_hash_bytes` len `0..7` (fall-through tail
  switch, one case per length), whole 8-byte blocks, mixed; bytes `>= 0x80`
  (the sign-extending `d[3] << 24` paths).  `stbds_hash_string`: empty, 1 byte,
  long, bytes `>= 0x80`.
* **arena shape**: `remaining` sufficient vs not; `len <= blocksize` (new block)
  vs `len > blocksize` (dedicated block) with `storage` NULL vs non-NULL;
  `block` 0..255 (the `512u << (block>>1)` shift, incl. shift-count wrap).

`[x]` = differential test over **randomised inputs (fixed seed)** passes.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, 512 random seeds | `hash.rs::bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (each tail `switch` case), 4096 random buffers × random seeds | `hash.rs::bytes_tail_lengths` | [x] |
| 3 | `stbds_hash_bytes` | `len = 8,16,24,…,256` (whole blocks, no tail), random buffers/seeds | `hash.rs::bytes_whole_blocks` | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..255` arbitrary (blocks + every tail length), random | `hash.rs::bytes_random_lengths` | [x] |
| 5 | `stbds_hash_bytes` | buffers biased to bytes `>= 0x80` (sign-extension of `d[3]<<24` in both the block loop and the `case 4` tail) | `hash.rs::bytes_high_bit` | [x] |
| 6 | `stbds_hash_bytes` | boundary seeds `0, 1, MAX, MAX-1, 1<<63, 0x31415926` × boundary buffers (all-`00`, all-`FF`, `80…`, `7F…`) | `hash.rs::bytes_boundary_seeds` | [x] |
| 7 | `stbds_hash_string` | empty string, 512 random seeds | `hash.rs::string_empty` | [x] |
| 8 | `stbds_hash_string` | random ASCII, len 1..64, random seeds | `hash.rs::string_random` | [x] |
| 9 | `stbds_hash_string` | random bytes `0x80..0xFF` (unsigned-char promotion), len 1..64 | `hash.rs::string_high_bytes` | [x] |
| 10 | `stbds_hash_string` | long strings (256, 1024, 4096 bytes) | `hash.rs::string_long` | [x] |
| 11 | `stbds_rand_seed` + `stbds_shmode_func` | seed sequencing: N fresh tables in a row must produce the same `table->seed` chain from the same `rand_seed` | `hash.rs::seed_lcg_chain` | [x] |
| 12 | `stbds_arrgrowf` | `a = NULL`, `elemsize ∈ {1,3,4,8,16,24,32}`, `addlen ∈ {0,1,2,7}`, `min_cap ∈ {0,1,2,3,4,5,17}` (full cross-product) | `arr.rs::growf_fresh_matrix` | [x] |
| 13 | `stbds_arrgrowf` | repeated growth of an existing array (doubling path `min_cap < 2*cap`), 200 random `addlen`/`min_cap` steps | `arr.rs::growf_repeated_growth` | [x] |
| 14 | `stbds_arrgrowf` | `min_cap <= cap` (no-op) and `min_len > min_cap` (clamp) on an existing array | `arr.rs::growf_noop_and_clamp` | [x] |
| 15 | `stbds_arrgrowf` + `stbds_arrfreef` | grow → write payload → `arrfreef`, random element sizes; payload/round-trip identical | `arr.rs::growf_payload_roundtrip` | [x] |
| 16 | `stbds_stralloc` | fresh arena, one small string (`len <= 512`) → first block; check `block`, `remaining`, contents | `arena.rs::stralloc_first_small` | [x] |
| 17 | `stbds_stralloc` | fresh arena, 2000 random strings len 0..64 → many block transitions (`block` 0→…), full `remaining`/`block` trace compared step-by-step | `arena.rs::stralloc_many_small` | [x] |
| 18 | `stbds_stralloc` | string longer than the current `blocksize` on a **fresh** arena (`storage == NULL`) → dedicated block, `remaining = 0` | `arena.rs::stralloc_big_block` | [x] |
| 19 | `stbds_stralloc` | string longer than `blocksize` on a **non-empty** arena (`storage != NULL`) → spliced after head, `remaining` untouched | `arena.rs::stralloc_big_block_after_small` | [x] |
| 20 | `stbds_stralloc` | interleaved small/large (len ∈ {0,1,511,512,513,1024,2048,100000}) × 400 random draws | `arena.rs::stralloc_mixed_sizes` | [x] |
| 21 | `stbds_stralloc` | caller-supplied `a->block ∈ 0..=255` (`512u << (block>>1)` incl. shift-count wrap ≥ 64) | `arena.rs::stralloc_block_field_matrix` | [x] |
| 22 | `stbds_strreset` | arena with 0 / 1 / many blocks (incl. dedicated big blocks) → fully zeroed | `arena.rs::strreset_empty`, `arena.rs::strreset_many` | [x] |
| 23 | `stbds_hmput_key` + `stbds_hmget_key` | **binary** mode 0, `elemsize=8`, `keysize=4` (int key), 1 insert | `hashmap.rs::binary_single` | [x] |
| 24 | `stbds_hmput_key` + `stbds_hmget_key` | binary mode 0, `elemsize=8`, `keysize=4`, 1000 random keys with duplicates → growth 8→16→…→2048, all lookups | `hashmap.rs::binary_many_i32` | [x] |
| 25 | `stbds_hmput_key` + `stbds_hmget_key` | binary mode 0, `elemsize=16`, `keysize=8` (64-bit key) 1000 random keys | `hashmap.rs::binary_many_i64` | [x] |
| 26 | `stbds_hmput_key` + `stbds_hmget_key` | binary mode 0, `elemsize=32`, `keysize=16` (2-int compound key + padding) | `hashmap.rs::binary_compound_key` | [x] |
| 27 | `stbds_hmput_key` | binary mode 0, `keysize ∈ {1,2}` (tiny key domain ⇒ heavy duplicate/probe collision) | `hashmap.rs::binary_tiny_keys` | [x] |
| 28 | `stbds_hmget_key_ts` | binary mode, hit + miss, `temp` out-param, on the **same** map as `hmget_key` (checks `header->temp` is *not* written) | `hashmap.rs::get_ts_vs_get` | [x] |
| 29 | `stbds_hmput_default` + `hmget_key` | `hmdefault` sentinel (`t[-1].value`) then misses return index −1 while element −1 holds the default | `hashmap.rs::default_value` | [x] |
| 30 | `stbds_hmdel_key` | binary mode 0, delete the **last** element (`old_index == final_index`) | `hashmap.rs::del_last` | [x] |
| 31 | `stbds_hmdel_key` | binary mode 0, delete a **middle** element (`old_index != final_index` → `memmove` + index fix-up) | `hashmap.rs::del_middle` | [x] |
| 32 | `stbds_hmdel_key` | binary mode 0, randomised insert/delete/lookup workload (3000 ops, seeded) driving tombstone **rebuild** and table **shrink**; full bucket array compared after every op | `hashmap.rs::binary_churn` | [x] |
| 33 | `stbds_hmput_key` | binary mode, insert → delete → re-insert so the new key lands on a **tombstone** (`tombstone >= 0` path) | `hashmap.rs::tombstone_reuse` | [x] |
| 34 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | **string** mode 1, `string.mode = SH_DEFAULT` (implicit, table created by `hmput_key`), 600 random keys, `elemsize=16` | `hashmap.rs::string_default_mode` | [x] |
| 35 | `stbds_shmode_func(SH_STRDUP)` + put/get/del/free | `string.mode = STBDS_SH_STRDUP(2)`: keys `strdup`ed, `temp_key` set, `hmdel_key` frees, `hmfree_func` frees all | `hashmap.rs::string_strdup_mode` | [x] |
| 36 | `stbds_shmode_func(SH_ARENA)` + put/get/del/free | `string.mode = STBDS_SH_ARENA(3)`: keys arena-allocated, arena `block`/`remaining` trace compared, `strreset` on free | `hashmap.rs::string_arena_mode` | [x] |
| 37 | `stbds_shmode_func(SH_NONE)` | `string.mode = 0` with **string** `mode = 1` ⇒ hash/compare as string but store with raw `memcpy(keysize)` (the `default:` switch arm) | `hashmap.rs::string_none_mode` | [x] |
| 38 | `stbds_shmode_func(SH_DEFAULT)` | `string.mode = 1` explicit, `elemsize = 8` (key-only element) | `hashmap.rs::string_default_explicit` | [x] |
| 39 | string maps | duplicate string keys (same content, *different* pointers) — `strcmp` equality, `temp_key` update path | `hashmap.rs::string_dup_keys` | [x] |
| 40 | string maps | keys sharing hash-prefix / long keys (256 B) / empty key `""` | `hashmap.rs::string_edge_keys` | [x] |
| 41 | string maps | randomised put/del/get churn (2000 ops) under `SH_STRDUP` driving grow + shrink + rebuild | `hashmap.rs::string_churn` | [x] |
| 42 | `stbds_hmdel_key` | `keyoffset = 0` vs `keyoffset = 8` on a binary map (caller-supplied, `hmput_key` always used 0) — miss behaviour must match | `hashmap.rs::del_keyoffset` | [x] |
| 43 | `stbds_hmfree_func` | free a map in each of the four `string.mode`s, incl. one with `length == 1` (no user elements) | `hashmap.rs::free_all_modes` | [x] |
| 44 | `stbds_rand_seed` | the *same* workload under seeds `0`, `1`, `0x31415926`, `usize::MAX`, random — different bucket layouts, still identical between C and Rust | `hashmap.rs::seeded_workloads` | [x] |
| 45 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,12345,-12345,i32::MIN,i32::MAX}` + 200 random | `misc.rs::strkey_matrix` | [x] |
| 46 | `sh_puts` (end-to-end, the only header-declared entry) | `num ∈ {i32::MIN,-1,0,1,2,3,7,8,9,64,512,1000,5000}` — stdout captured and compared byte-for-byte; also exercises `stralloc` `num` times and the whole arena-mode map pipeline | `sh_puts.rs::sh_puts_matrix` | [x] |
| 47 | `sh_puts` | repeated calls in one process (global `stbds_hash_seed` advances, static `buffer` reused) — 50 calls, cumulative stdout compared | `sh_puts.rs::sh_puts_repeated` | [x] |
| 48 | full pipeline | `hmput_default` → `hmput_key`×N → `hmget_key_ts` → `hmdel_key`×M → `hmfree_func`, randomised, 30 independent runs with different seeds, deep struct+bucket comparison at every step | `hashmap.rs::pipeline_random` | [x] |
| 49 | `stbds_hmput_key` (mixed axes) | table created in **binary** mode (`string.mode == 0`) then written with `mode = STBDS_HM_STRING` — string hash/compare, `default:` raw `memcpy` store | `enums.rs::mixed_table_mode_and_call_mode` | [x] |
| 50 | `stbds_hmput_key` (mixed axes) | table created with `SH_DEFAULT` then written with `mode = STBDS_HM_BINARY` — `memcmp` compare, `SH_DEFAULT` pointer store | `enums.rs::mixed_table_mode_and_call_mode` | [x] |
| 51 | `stbds_shmode_func` | full `elemsize x string.mode` cross-product (`8/16/24/32/64/128` x `NONE/DEFAULT/STRDUP/ARENA`) | `enums.rs::shmode_elemsize_matrix` | [x] |
| 52 | full pipeline (branch coverage) | proves the randomised workloads actually drive table **growth**, **shrink**, tombstone **rebuild** and tombstone **reuse**, and that C and Rust take each transition at the same step | `hashmap.rs::table_lifecycle_coverage` | [x] |
| 53 | `sh_puts` + hash-map ops in one process | `sh_puts` from *both* libraries followed by a full map workload — shared `stdout`, separate global seeds and `strkey` buffers | `sh_puts.rs::sh_puts_then_map_ops`, `sh_puts.rs::sh_puts_random` | [x] |

## Verification driver

`./verify.sh` performs the whole matrix mechanically:

1. asserts the configuration enumeration above is still complete
   (fails if a `[features]` section or a CMake `option()` appears),
2. builds the C `.so`,
3. `cargo check` for every feature combination,
4. builds the Rust cdylib in **release** and **debug** for every combination,
5. diffs `nm -D` (C vs Rust, both profiles) and checks for unresolved non-libc
   imports,
6. runs the full differential suite for every feature combination against **both**
   Rust profiles.

Latest run: 95 tests x 3 feature combinations x 2 profiles — all pass, symbol
diff empty.
