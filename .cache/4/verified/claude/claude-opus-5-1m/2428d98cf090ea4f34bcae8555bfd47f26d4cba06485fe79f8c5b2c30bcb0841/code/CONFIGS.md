# CONFIGS.md — configuration-surface table (Phase B)

Mechanically derived from `c_src/src/lib.c`: every runtime option the public API
can set, every `if` / `switch` / loop-shape the C branches on, and every input
shape the code special-cases.  One row per meaningful **combination**.

Differential tests live in **`tests/configs.rs`** (rows 1-24 and 73-77, the pure
functions / dynamic array / string arena / driver entry points) and
**`tests/maps.rs`** (rows 25-72, the hash map).  Every row is driven with **many
randomised inputs** from a fixed-seed xorshift PRNG (`tests/harness/mod.rs`), and
after every operation both libraries' state is snapshotted and compared
byte-for-byte (array header + full hash index + all bucket `hash[]`/`index[]`
slots + all element bytes, with library-owned key pointers normalised to their
string contents — see `harness::Snapshot`).

Run everything with `./run_tests.sh` (it builds both `.so`s, checks symbol
parity, loops over the feature combinations and then runs the suite
single-threaded, which the `printf`-capturing rows require).

## §0 Build-time configuration axes

| axis | values | source |
|------|--------|--------|
| Cargo features | **none** — `Cargo.toml` has no `[features]` table, so the only combination is the empty set (`--no-default-features` ≡ default) | `Cargo.toml` |
| CMake options | **none** — one unconditional `add_library(... SHARED src/lib.c)`, no `option()`/`-D`/`#ifdef` variants | `c_src/CMakeLists.txt` |
| `#ifdef` in C | `STBDS_HAS_TYPEOF`, `STBDS_HAS_LITERAL_ARRAY`, `STBDS_STATS(x)`→empty, `_CRT_SECURE_NO_WARNINGS` are all defined unconditionally at the top of the single TU; `NDEBUG` is never defined | `c_src/src/lib.c` L1-98, L271 |
| word size | `sizeof(size_t)==8` is enforced by `typedef int STBDS_SIPHASH_2_4_can_only_be_used_in_64_bit_builds[...]` (L495) — LP64 only | `c_src/src/lib.c` L495 |

⇒ Phase D's feature-combination loop has exactly **one** entry; `check_features.sh`
still enumerates it programmatically from `Cargo.toml`.

## §1 Runtime option axes the C branches on

| axis | values the C distinguishes | branch site |
|------|---------------------------|-------------|
| `mode` (hash/compare kind) | `< 1` ⇒ binary (`memcmp` + `hash_bytes`); `>= 1` ⇒ string (`strcmp` + `hash_string`) | L560, L590, L713 |
| `mode` (delete-time, **exact** compare) | `== 1` only | L836, L842 |
| `table->string.mode` (key ownership) | `SH_NONE 0` / `SH_DEFAULT 1` / `SH_STRDUP 2` / `SH_ARENA 3` / anything else ⇒ `memcpy` | `switch` L785-790 |
| how `string.mode` gets set | `shmode_func(elemsize, mode)` → `(unsigned char)mode`; or `hmput_key` bootstrap → `mode>=1 ? 1 : 0` | L803, L707 |
| global `stbds_hash_seed` | `0x31415926` initially, `rand_seed()` overrides, `make_hash_index(_,NULL)` advances it by an LCG | L353-358, L409-412 |
| per-table `seed` | snapshot of the global at index-creation; **inherited** on rehash (`ot != NULL`) | L403-413 |
| `keyoffset` | `0` for every put/get; caller-supplied for `hmdel_key` | L633, L682, L807 |

## §2 Input-shape axes the C special-cases

| axis | values | branch site |
|------|--------|-------------|
| `len` for `hash_bytes` | `0`; `1..7` (tail only); `8` (one block, empty tail); `9..15`; `16`; `17..`; `len >= 256` (only `len & 0xff` survives `len<<56`) | L522-541 |
| byte values | `d[3] >= 0x80` / `d[7] >= 0x80` ⇒ the `int` shift sign-extends into the high 32 bits | L523-524, L536 |
| string content | `""`; bytes `>= 0x80` (`(unsigned char)` cast); length ≫ rotate width | L477-491 |
| element count vs `used_count_threshold` | `slot_count - slot_count/4`: 6/8, 12/16, 24/32, 48/64 … ⇒ grow points at 6, 12, 24, 48, 96, 192, 384, 768 | L698, L395 |
| element count vs `used_count_shrink_threshold` | `slot_count/4`, forced to `0` when `slot_count <= 8` ⇒ shrink points | L397-400, L854 |
| `tombstone_count` vs `tombstone_count_threshold` | `slot_count/8 + slot_count/16` ⇒ same-size rebuild | L396, L858 |
| bucket scan shape | forward scan from `pos & 7`, then wrap-around scan `0..pos&7`, then quadratic-ish `pos += step; step += 8` | L604-627, L728-763 |
| `old_index` vs `final_index` on delete | equal ⇒ no swap-with-last; different ⇒ move + re-find + index fixup | L839-851 |
| arena `len` vs `remaining` | fits / needs a new block | L885 |
| arena `len` vs `blocksize` (`512 << (block>>1)`) | fits in a fresh block / needs a dedicated oversized block | L893 |
| arena `storage` on the oversized path | `NULL` ⇒ becomes head, `remaining = 0`; non-`NULL` ⇒ spliced as `storage->next`, `remaining` kept | L896-903 |
| arena `block` counter | `0,1,2,…,22` then saturates (`512<<11 == 1<<20` is not `< 1<<20`) | L888-891 |
| `elemsize` / `keysize` | `keysize < elemsize` (key + value), `keysize == elemsize`, `keysize == 0`, `elemsize == 0`, unaligned `elemsize` (7, 12) | L563, L789, L840 |
| `str_dups(num)` | `<= 0` (arena loop skipped); `> 0` — `num` 17-byte keys drive arena `block` 0→N | L952 |

## §3 Configuration rows (cross-product, pruned to what the C distinguishes)

`R#` = randomised inputs per row (fixed-seed PRNG).

### Pure hash functions

| # | entry point(s) | configuration (options set + input shape) | R# | test | [ ] |
|---|----------------|-------------------------------------------|----|------|-----|
| 1 | `stbds_hash_bytes` | `len == 0`, `p == NULL`; seeds `{0,1,0x31415926,usize::MAX}` + 64 random | 68 | `cfg_hash_bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` | `len ∈ 1..=7` (tail-only `switch` cases 1-7), random bytes | 7×64 | `cfg_hash_bytes_tail_only` | [x] |
| 3 | `stbds_hash_bytes` | `len == 8` (exactly one block, `switch` case 0) | 64 | `cfg_hash_bytes_one_block` | [x] |
| 4 | `stbds_hash_bytes` | `len ∈ 9..=15` (one block + tail 1-7) | 7×64 | `cfg_hash_bytes_block_plus_tail` | [x] |
| 5 | `stbds_hash_bytes` | `len ∈ 16..=64` (multi-block), random | 512 | `cfg_hash_bytes_multiblock` | [x] |
| 6 | `stbds_hash_bytes` | `len ∈ {255,256,257,511,512,1024}` — `len<<56` keeps only `len&0xff` | 6×16 | `cfg_hash_bytes_len_gt_255` | [x] |
| 7 | `stbds_hash_bytes` | bytes forced all-`0x00` / all-`0xFF` / `d[3],d[7] >= 0x80` (sign-extension paths) | 3×32 | `cfg_hash_bytes_sign_extension` | [x] |
| 8 | `stbds_hash_bytes` | seed sweep across all of the above: `0`, `1`, `0x31415926`, `usize::MAX`, random | folded in | `cfg_hash_bytes_seed_sweep` | [x] |
| 9 | `stbds_hash_string` | `""` (loop never runs), seeds `{0,1,default,MAX}` + random | 68 | `cfg_hash_string_empty` | [x] |
| 10 | `stbds_hash_string` | ASCII length `1..=32`, random | 32×32 | `cfg_hash_string_short` | [x] |
| 11 | `stbds_hash_string` | length `100`, `1000`, `4096` (many `ROTATE_LEFT(hash,9)` rounds) | 3×16 | `cfg_hash_string_long` | [x] |
| 12 | `stbds_hash_string` | bytes in `0x80..=0xFF` — exercises the `(unsigned char)*str` cast | 64 | `cfg_hash_string_high_bytes` | [x] |

### Dynamic array

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 13 | `stbds_arrgrowf` | `a == NULL` × `elemsize ∈ {1,4,7,8,12,16,24,32}` × `addlen ∈ {0,1,2,5}` × `min_cap ∈ {0,1,3,4,5,7,8,100}` | 256 | `cfg_arrgrowf_fresh_matrix` | [x] |
| 14 | `stbds_arrgrowf` | existing array, repeated growth (doubling path) 1→4→8→16…, header (`length`,`hash_table`,`temp`) preserved | 8×20 | `cfg_arrgrowf_repeated_doubling` | [x] |
| 15 | `stbds_arrgrowf` | `min_cap <= arrcap` ⇒ identity return, plus `min_cap == arrcap` and `arrcap+1` boundaries | 8×3 | `cfg_arrgrowf_boundaries` | [x] |
| 16 | `stbds_arrgrowf` + `stbds_arrfreef` | grow N times then free; full alloc/free sequence parity | 32 | `cfg_arrgrow_then_free` | [x] |

### String arena

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 17 | `stbds_stralloc` | fresh arena (`block=0`, `remaining=0`), one string of length `0..=32` | 33 | `cfg_stralloc_fresh_short` | [x] |
| 18 | `stbds_stralloc` | fresh arena, one string of length `510,511,512,513,1023,1024` (crosses `blocksize=512`) | 6 | `cfg_stralloc_fresh_blocksize_boundary` | [x] |
| 19 | `stbds_stralloc` | sequence of N random short strings — drives `remaining` down and `block` up | 8×200 | `cfg_stralloc_sequence_random` | [x] |
| 20 | `stbds_stralloc` | oversized string (`len > blocksize`) on an **empty** arena (`storage == NULL`) ⇒ `remaining` forced to 0 | 8 | `cfg_stralloc_oversize_on_empty` | [x] |
| 21 | `stbds_stralloc` | oversized string on a **non-empty** arena ⇒ spliced as `storage->next`, `remaining` preserved | 8 | `cfg_stralloc_oversize_on_nonempty` | [x] |
| 22 | `stbds_stralloc` | pre-set `a->block ∈ {0,1,2,3,10,20,21,22,23,40}` × short/long string (blocksize `512<<(block>>1)`, saturation at `1<<20`) | 10×3 | `cfg_stralloc_block_presets` | [x] |
| 23 | `stbds_stralloc` | 3000 strings of length 1..40 ⇒ `block` saturates at 22, ≥ 20 blocks chained | 1×3000 | `cfg_stralloc_saturate_block` | [x] |
| 24 | `stbds_strreset` | arena with `0,1,2,5,50` blocks (mix of normal + oversized) ⇒ whole chain freed, struct zeroed, arena reusable afterwards | 5 | `cfg_strreset_chain` | [x] |

### Hash map — binary keys (`mode = STBDS_HM_BINARY`)

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 25 | `stbds_hmput_key` | bootstrap from `NULL`, `elemsize=16`, `keysize=8`, 1 insert (`string.mode` ends up `0`) | 32 | `cfg_bin_single_insert` | [x] |
| 26 | `stbds_hmput_key` | 5 inserts (below the 6/8 grow threshold) | 32 | `cfg_bin_below_grow_threshold` | [x] |
| 27 | `stbds_hmput_key` | exactly 6 inserts ⇒ grow 8→16 on the 6th; then 7 | 32 | `cfg_bin_at_grow_threshold` | [x] |
| 28 | `stbds_hmput_key` | 12, 24, 48 inserts ⇒ grows 16→32→64→128 (rehash of a populated index) | 3×16 | `cfg_bin_multiple_grows` | [x] |
| 29 | `stbds_hmput_key` | 1000 random `u64` keys (deep quadratic probing, many rehashes) | 4×1000 | `cfg_bin_large_random` | [x] |
| 30 | `stbds_hmput_key` | `keysize ∈ {1,2,3,4,5,6,7,8}` with `elemsize=8` (key fills / partially fills the element) | 8×64 | `cfg_bin_keysize_sweep_small` | [x] |
| 31 | `stbds_hmput_key` | `keysize ∈ {9,12,16,17,24,32}` with `elemsize = keysize+8` | 6×64 | `cfg_bin_keysize_sweep_large` | [x] |
| 32 | `stbds_hmput_key` | unaligned `elemsize ∈ {7,12,20}` with `keysize=4` | 3×64 | `cfg_bin_unaligned_elemsize` | [x] |
| 33 | `stbds_hmput_key` | duplicate keys (each key inserted 3×) ⇒ `temp` = existing index, `length` unchanged | 64 | `cfg_bin_duplicates` | [x] |
| 34 | `stbds_hmput_key` | keys drawn from a tiny domain (`0..8`) ⇒ forced collisions + wrap-around bucket scan | 512 | `cfg_bin_forced_collisions` | [x] |
| 35 | `stbds_hmget_key` | hits and misses interleaved on a 200-element map (`header->temp` compared) | 400 | `cfg_bin_get_hits_and_misses` | [x] |
| 36 | `stbds_hmget_key_ts` | same as #35 through the `_ts` entry point (`*temp` out-param, header **not** written) | 400 | `cfg_bin_get_ts` | [x] |
| 37 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`, no swap) | 32 | `cfg_bin_del_last` | [x] |
| 38 | `stbds_hmdel_key` | delete the **first**/middle element (swap-with-last + re-find + index fixup) | 64 | `cfg_bin_del_swap` | [x] |
| 39 | `stbds_hmdel_key` | delete every element in insertion order, then in reverse order | 2×64 | `cfg_bin_del_all` | [x] |
| 40 | `stbds_hmdel_key` | 100 inserts then 90 deletes ⇒ crosses `used_count_shrink_threshold` (shrink path) | 8 | `cfg_bin_del_shrink` | [x] |
| 41 | `stbds_hmdel_key` | put/del alternating on a 64-slot table ⇒ crosses `tombstone_count_threshold` (same-size rebuild) | 8 | `cfg_bin_del_tombstone_rebuild` | [x] |
| 42 | `stbds_hmput_key`+`get`+`del` | 3000 random interleaved ops (put/get/get_ts/del/del-missing) — property test | 4×3000 | `cfg_bin_random_op_stream` | [x] |
| 43 | `stbds_hmdel_key` | `keyoffset ∈ {0,4,8,16}` with the key stored at that offset inside the element | 4×64 | `cfg_bin_del_keyoffset` | [x] |
| 44 | `stbds_hmput_key` | `keysize == 0` ⇒ all keys compare equal, table stays at 1 element | 32 | `cfg_bin_keysize_zero` | [x] |
| 45 | `stbds_hmput_key` | `elemsize == 0 && keysize == 0` (degenerate identity pointers) | 32 | `cfg_bin_elemsize_zero` | [x] |

### Hash map — string keys

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 46 | `stbds_hmput_key` `mode=1` | bootstrap from `NULL` ⇒ `string.mode = SH_DEFAULT (1)`, key pointer stored verbatim | 64 | `cfg_str_default_bootstrap` | [x] |
| 47 | `stbds_shmode_func(SH_STRDUP)` + `hmput_key` | keys `strdup`ed; `temp_key` = the copy; distinct storage from the caller's buffer | 64 | `cfg_str_strdup` | [x] |
| 48 | `stbds_shmode_func(SH_ARENA)` + `hmput_key` | keys copied into the table's arena; arena `block`/`remaining` evolve inside the index | 64 | `cfg_str_arena` | [x] |
| 49 | `stbds_shmode_func(SH_NONE=0)` + `hmput_key` `mode=1` | `switch` **default** ⇒ `memcpy(elem, key, keysize)` copies the first 8 bytes of the key **text** into the element (not a `char *`).  Inserts only: a *lookup* on such a table dereferences those bytes as a pointer and segfaults — see `ERRORS.md` rows 37b/37c | 8×64 | `cfg_str_mode_none_memcpy` | [x] |
| 50 | `stbds_shmode_func(255)` + `hmput_key` `mode=1` | out-of-range `string.mode` ⇒ same `memcpy` default branch | 8×64 | `cfg_str_mode_255_memcpy` | [x] |
| 51 | `stbds_shmode_func(SH_STRDUP)` | key lengths `0..=32` (incl. `""`), 200 keys ⇒ many `strdup`s | 200 | `cfg_str_key_length_sweep` | [x] |
| 52 | `stbds_shmode_func(SH_ARENA)` | key lengths crossing the arena blocksize (`500..=600`, `>1024`) | 64 | `cfg_str_arena_block_boundary` | [x] |
| 53 | string map | duplicate keys ⇒ first loop refreshes `temp_key`, wrap-around loop does not | 128 | `cfg_str_duplicates_temp_key` | [x] |
| 54 | string map | 1000 random keys ⇒ grows 8→…→2048 with string rehashing | 3×1000 | `cfg_str_large_random` | [x] |
| 55 | string map, `SH_STRDUP` | deletes with `mode == 1` ⇒ the `strdup`ed key **is** freed and the re-find uses the string branch | 128 | `cfg_str_del_strdup` | [x] |
| 56 | string map, `SH_ARENA` | deletes with `mode == 1` (arena memory is *not* freed per key) | 128 | `cfg_str_del_arena` | [x] |
| 57 | string map, `SH_DEFAULT` | deletes with `mode == 1` (caller keeps ownership) | 128 | `cfg_str_del_default` | [x] |
| 58 | string map | delete-all then re-insert ⇒ tombstone reuse on the string path | 64 | `cfg_str_del_then_reinsert` | [x] |
| 59 | string map | 2000 random interleaved put/get/get_ts/del ops, `SH_STRDUP` | 2×2000 | `cfg_str_random_op_stream` | [x] |
| 60 | string map | keys that share long prefixes (`"k000000…N"`) ⇒ `hash_string` collisions + `strcmp` fallbacks | 256 | `cfg_str_common_prefix` | [x] |
| 61 | string map | keys with bytes `>= 0x80` | 128 | `cfg_str_high_byte_keys` | [x] |

### Mode / seed cross-product & remaining entry points

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 62 | `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` | `mode ∈ {-2,-1,0,1,2,3,7,255,999,i32::MIN,i32::MAX}` × {binary-shaped key, string-shaped key} — full matrix of the `>=1` vs `==1` divergence.  For `mode >= 2` only the *last* element is deleted (`old_index == final_index`), because the re-find at L842 then asserts — that abort is `ERRORS.md` rows 50/51 | 11×32 | `cfg_mode_matrix` | [x] |
| 62b | `hmput_key` then `hmget_key` with the *other* mode | a binary table looked up with `mode=1`, and a string table looked up with `mode=0`: the two hash functions disagree, so `find_slot` reaches an EMPTY slot and returns -1 without dereferencing anything | 16×16 | `cfg_mode_mismatch_lookup` | [x] |
| 63 | `stbds_shmode_func` | `mode ∈ {0,1,2,3,4,255,256,257,-1,i32::MIN,i32::MAX}` ⇒ `(unsigned char)` truncation, then one insert each | 11×8 | `cfg_shmode_matrix` | [x] |
| 64 | `stbds_rand_seed` | `seed ∈ {0,1,2,0x31415926,usize::MAX,random×8}` then an identical 200-op workload ⇒ per-table `seed` + LCG advance + all bucket hashes compared | 12×200 | `cfg_seed_sweep_workload` | [x] |
| 65 | `stbds_rand_seed` | several tables created in a row **without** re-seeding ⇒ the LCG advance sequence must match across `make_hash_index` calls | 32 | `cfg_seed_lcg_sequence` | [x] |
| 66 | `stbds_hmput_default` | `NULL` / fresh / already-populated map × `elemsize ∈ {8,16,24}`; then `hmget_key` on the result (`hash_table == NULL` path) | 3×3 | `cfg_hmput_default_matrix` | [x] |
| 67 | `stbds_hmput_default` → `hmput_key` | default element written first, then real inserts ⇒ sentinel element 0 must stay intact | 32 | `cfg_hmput_default_then_inserts` | [x] |
| 68 | `stbds_hmfree_func` | `string.mode ∈ {0,1,2,3,255}` × `{0,1,5,100}` elements × `{with,without}` a hash index — full teardown | 5×4×2 | `cfg_hmfree_matrix` | [x] |
| 69 | `stbds_shmode_func` | `elemsize ∈ {8,16,24,32,12}` × `mode ∈ {0..3}` ⇒ initial header/index state | 20 | `cfg_shmode_elemsize_matrix` | [x] |
| 70 | full pipeline | `shmode_func(STRDUP)` → 500 `hmput_key` → 250 `hmdel_key` → 250 `hmput_key` → `hmget_key` all → `hmfree_func` (the composed pipeline, not per-wrapper) | 4×1500 | `cfg_pipeline_strdup` | [x] |
| 71 | full pipeline | binary map: 500 put → 400 del (shrink+rebuild) → 300 put → get all → `hmfree_func` | 4×1200 | `cfg_pipeline_binary` | [x] |
| 72 | full pipeline | arena map: `shmode_func(ARENA)` → 400 puts with key lengths 1..600 → deletes → `hmfree_func` (arena chain freed via `strreset`) | 2×800 | `cfg_pipeline_arena` | [x] |

### Driver entry points

| # | entry point(s) | configuration | R# | test | [ ] |
|---|----------------|---------------|----|------|-----|
| 73 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,i32::MAX,i32::MIN}` + 64 random ⇒ returned C string bytes | 73 | `cfg_strkey_values` | [x] |
| 74 | `strkey` | two consecutive calls ⇒ shared static buffer, second overwrites the first | 16 | `cfg_strkey_static_buffer` | [x] |
| 75 | `str_dups` | `num ∈ {0,1,2,3,10,29,30,31,32,64,100,512,1000}` ⇒ **stdout** compared byte-for-byte (`printf("%s %d\n", struct, value)`) | 13 | `cfg_str_dups_stdout` | [x] |
| 76 | `str_dups` | `num ∈ {-1,-2,i32::MIN}` (arena loop skipped) | 3 | `cfg_str_dups_non_positive` | [x] |
| 77 | `str_dups` | called repeatedly (10×) ⇒ the global `stbds_hash_seed` advances once per call; output must stay identical across libs | 10 | `cfg_str_dups_repeated` | [x] |
