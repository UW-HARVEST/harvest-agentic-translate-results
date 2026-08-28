# Phase A.3 — CONFIGURATION-SURFACE TABLE

Axes derived mechanically from the branches in `c_src/src/lib.c` (there are no
`#ifdef`s left after preprocessing this single TU, and no runtime "options struct" —
the configuration is carried by the `mode` argument, by `table->string.mode`, by the
element/key *shapes* and by the *creation path*):

* **`mode` argument** (`int`, any value): `mode >= STBDS_HM_STRING(1)` ⇒ string keys
  (`stbds_hash_string` + `strcmp`), otherwise binary keys (`stbds_hash_bytes` +
  `memcmp`) — lib.c:560, 590, 713. `mode == 1` *exactly* additionally controls the
  strdup-free and the re-lookup in `stbds_hmdel_key` (lib.c:836, 842).
* **`table->string.mode`** (`unsigned char`): `STBDS_SH_NONE 0` / `DEFAULT 1` /
  `STRDUP 2` / `ARENA 3`, plus any other byte value (reachable through the
  `(unsigned char)mode` truncation in `stbds_shmode_func`, lib.c:803) — selects the
  `switch` at lib.c:785 and the free loop at lib.c:575.
* **creation path**: implicit (`NULL` handed to `stbds_hmput_key` /
  `stbds_hmget_key(_ts)`), `stbds_hmput_default`, or `stbds_shmode_func`.
* **input shapes**: `elemsize`, `keysize` (0,1,2,4,8,16), `keyoffset`, string length
  (0,1,7,8,9,long), byte values (`< 0x80` vs `>= 0x80` — sign-extension in
  `stbds_siphash_bytes`), element count (0/1/many, and exactly the
  `used_count_threshold`, tombstone and shrink boundaries), duplicate vs fresh keys.
* **hash seed**: default `0x31415926`, `stbds_rand_seed`, and the per-table seed
  chain (`seed = seed*a+b` on every *fresh* index, copied on rehash — lib.c:409‑412).
* **lowest-level entry points are driven directly**: `stbds_arrgrowf`,
  `stbds_arrfreef`, `stbds_hash_bytes`, `stbds_hash_string`, `stbds_stralloc`,
  `stbds_strreset`, `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_shmode_func`
  — not only the `helxo` convenience wrapper.

Every row is exercised with **many randomized inputs** (`Rng` = xorshift64\*, fixed
seeds, `tests/phase_b_valid.rs`) and the *full* observable state of both libraries is
compared after **every** call (array header, hash index, every bucket `hash[]`/
`index[]`, arena, and the payload of every element — see `common::snapshot`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, 1000 random seeds (incl. 0, `usize::MAX`) | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (every `switch (len-i)` fall-through case), random bytes `< 0x80` | [x] |
| 3 | `stbds_hash_bytes` | `len = 1..7`, bytes forced `>= 0x80` (int sign-extension of `d[3]<<24`) | [x] |
| 4 | `stbds_hash_bytes` | `len = 8` exactly (one full word, empty tail) | [x] |
| 5 | `stbds_hash_bytes` | `len = 9..71` (multiple full words + every tail length), random bytes | [x] |
| 6 | `stbds_hash_bytes` | `len` large (256..4096), random bytes, random seeds | [x] |
| 7 | `stbds_hash_string` | `""`, 1-byte, 7/8/9-byte, 4 KiB strings; ASCII **and** bytes `>= 0x80` | [x] |
| 8 | `stbds_hash_string` | seeds `0`, `1`, `0x31415926`, `usize::MAX`, 500 random | [x] |
| 9 | `stbds_rand_seed` + `stbds_shmode_func` | seed chain: `table->seed` of the 1st..8th freshly created index after `rand_seed(s)` for `s ∈ {0,1,0x31415926,MAX,random}` | [x] |
| 10 | `stbds_arrgrowf` | `a = NULL`, `elemsize ∈ {1,2,3,8,16,64}`, `addlen = 0`, `min_cap ∈ {0,1,2,3,4,5,1000}` (the `<4` clamp) | [x] |
| 11 | `stbds_arrgrowf` | `a = NULL`, `addlen ∈ {0..1000}`, `min_cap = 0` (`min_len` clamp) | [x] |
| 12 | `stbds_arrgrowf` | existing array, repeated growth by 1 (doubling path `min_cap < 2*cap`) 0…64 times, payload preserved | [x] |
| 13 | `stbds_arrgrowf` | existing array, `min_cap` far above `2*cap` (explicit `arrsetcap`), and `min_cap <= cap` (no-op) | [x] |
| 14 | `stbds_arrgrowf` + `stbds_arrfreef` | grow → write payload → `arrfreef` (allocator handoff: header freed with libc `free`) | [x] |
| 15 | `stbds_hmput_key` (lazy `NULL`) + `stbds_hmget_key` | `mode = 0` (binary), `elemsize/keysize = 16/8`, 1 element | [x] |
| 16 | `stbds_hmput_key` (lazy) | `mode = 0`, `elemsize/keysize = 16/8`, 5 elements (below `used_count_threshold = 6`) | [x] |
| 17 | `stbds_hmput_key` (lazy) | `mode = 0`, 6 elements — exactly the first grow (`8 → 16` slots, rehash) | [x] |
| 18 | `stbds_hmput_key` (lazy) | `mode = 0`, 13 / 25 / 49 elements — grows `16→32→64→128`, random keys | [x] |
| 19 | `stbds_hmput_key` (lazy) | `mode = 0`, 1000 elements, random 8-byte keys with high-bit bytes | [x] |
| 20 | `stbds_hmput_key` (lazy) | `mode = 0`, duplicate keys (update path, `stbds_temp` = existing index, no growth) interleaved with fresh keys | [x] |
| 21 | `stbds_hmput_key` (lazy) | `mode = 0`, `keysize = 1` (only 256 distinct keys ⇒ many updates), `elemsize = 8` | [x] |
| 22 | `stbds_hmput_key` (lazy) | `mode = 0`, `keysize ∈ {2,4,16}`, `elemsize ∈ {8,16,24,32}` (odd shapes, `keysize < elemsize`) | [x] |
| 23 | `stbds_hmput_key` (lazy) | `mode = 0`, `keysize = 0` (every key equal — single entry, all puts update) | [x] |
| 24 | `stbds_hmput_key` (lazy) | `mode < 0` (`-1`, `INT_MIN`) ⇒ binary path with out-of-range enum | [x] |
| 25 | `stbds_hmput_key` (lazy) + `stbds_hmget_key` | `mode = 1` ⇒ `string.mode = STBDS_SH_DEFAULT`; caller-owned key pointers stored verbatim; 1/5/6/13/200 random strings | [x] |
| 26 | `stbds_hmput_key` (lazy) | `mode = 1`, keys that are equal *strings* at different addresses (update path via `strcmp`) | [x] |
| 27 | `stbds_hmput_key` (lazy) | `mode ∈ {2,3,7,INT_MAX}` ⇒ string hashing but `string.mode` still `DEFAULT` | [x] |
| 28 | `stbds_shmode_func(mode = STBDS_SH_STRDUP)` + `hmput_key(mode=1)` | keys are `strdup`ed into the map (`switch` case 2); 1/6/13/200 strings, incl. `""` | [x] |
| 29 | `stbds_shmode_func(mode = STBDS_SH_ARENA)` + `hmput_key(mode=1)` | keys are arena-allocated (`switch` case 3); short strings (many per block), 600-byte and 4000-byte strings (block overflow + oversized block) | [x] |
| 30 | `stbds_shmode_func(mode = STBDS_SH_DEFAULT)` + `hmput_key(mode=1)` | explicit `DEFAULT` mode (pointer stored, `temp_key` set) | [x] |
| 31 | `stbds_shmode_func(mode = STBDS_SH_NONE/4/255/256/-1)` + `hmput_key` | `string.mode` outside `{1,2,3}` ⇒ `switch default:` = `memcpy(key, keysize)` even for `mode = 1` | [x] |
| 32 | `stbds_hmput_default` + `hmput_key` + `hmget_key` | map created through `hmput_default` (table still `NULL` on first `get`), then filled | [x] |
| 33 | `stbds_hmget_key_ts` | all three sentinel paths + hit path, `mode ∈ {0,1}`; `*temp` compared, header `temp` untouched | [x] |
| 34 | `stbds_hmdel_key` | binary map, delete the **last** element (`old_index == final_index`, no memmove) | [x] |
| 35 | `stbds_hmdel_key` | binary map, delete a **middle/first** element (memmove of the final element + re-lookup + index patch) | [x] |
| 36 | `stbds_hmdel_key` | binary map, delete **every** element in random order (drives `tombstone_count` up, `used_count` down) | [x] |
| 37 | `stbds_hmdel_key` | binary map, 100 elements then delete 76 ⇒ `used_count < used_count_shrink_threshold` **shrink** path (`slot_count>>1`, rehash) | [x] |
| 38 | `stbds_hmdel_key` | `slot_count = 8` map (`used_count_shrink_threshold = 0`) ⇒ shrink suppressed, `tombstone_count > tombstone_count_threshold` **rebuild** path | [x] |
| 39 | `stbds_hmdel_key` + `stbds_hmput_key` | delete then re-insert (tombstone reuse: `tombstone >= 0` ⇒ `--tombstone_count`), random interleavings, 500 ops | [x] |
| 40 | `stbds_hmdel_key` | string map `mode = 1`, `string.mode = DEFAULT` / `STRDUP` / `ARENA`, delete last/middle/all | [x] |
| 41 | `stbds_hmdel_key` | `keyoffset ∈ {0,1,4,8}` (the `pshdel` / `STBDS_HM_PTR_TO_STRING` layout — `hmput_key` hardcodes `keyoffset = 0`, so a non-zero value makes the C compare the *wrong* bytes and both libraries must miss identically), 24-byte elements | [x] |
| 42 | `stbds_hmfree_func` | `string.mode = STRDUP` (frees every key) / `ARENA` (frees the block list) / binary; after 0/1/many inserts and deletes | [x] |
| 43 | `stbds_stralloc` | fresh zeroed arena, `len ∈ {1,2,511,512,513}` — the `len > blocksize` boundary at `block = 0` | [x] |
| 44 | `stbds_stralloc` | repeated allocations until the block is exhausted (`remaining` bump, `++block`, `512<<(block>>1)` growth chain up to `1<<20` saturation) | [x] |
| 45 | `stbds_stralloc` | oversized string with a **non-empty** arena (splice *after* head, `remaining` kept) vs an **empty** arena (`remaining = 0`) | [x] |
| 46 | `stbds_stralloc` | `a->block` pre-set to `0..127` (and `>=128`, shift-count out of range) with `remaining = 0` | [x] |
| 47 | `stbds_strreset` | zeroed arena / 1 block / 40 blocks / after oversized allocations (list splicing) | [x] |
| 48 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,INT_MAX,INT_MIN}` + 200 random `int`s; returned pointer contents and static-buffer reuse | [x] |
| 49 | `helxo` | `letter` = every byte value `0..=255` — stdout captured and compared byte-for-byte | [x] |
| 50 | `helxo` | repeated calls (global `stbds_hash_seed` advances ⇒ different table seed each call; output must stay identical) | [x] |

## Row → test mapping

| rows | test |
|------|------|
| 1 | `phase_b_hash::cfg_01_hash_bytes_len0` |
| 2, 3 | `phase_b_hash::cfg_02_03_hash_bytes_tail_lengths` |
| 4, 5 | `phase_b_hash::cfg_04_05_hash_bytes_word_lengths` |
| 6 | `phase_b_hash::cfg_06_hash_bytes_large` |
| 7, 8 | `phase_b_hash::cfg_07_08_hash_string` |
| 9 | `phase_b_hash::cfg_09_rand_seed_chain` |
| 10 | `phase_b_array::cfg_10_arrgrowf_fresh_min_cap` |
| 11 | `phase_b_array::cfg_11_arrgrowf_fresh_addlen` |
| 12 | `phase_b_array::cfg_12_arrgrowf_repeated_arrput` |
| 13 | `phase_b_array::cfg_13_arrgrowf_setcap_and_noop` |
| 14 | `phase_b_array::cfg_14_arrgrowf_free_roundtrip` |
| 15–19 | `phase_b_map::cfg_15_to_19_binary_counts` |
| 20 | `phase_b_map::cfg_20_binary_updates` |
| 21 | `phase_b_map::cfg_21_keysize_one` |
| 22 | `phase_b_map::cfg_22_shapes` |
| 23 | `phase_b_map::cfg_23_keysize_zero` |
| 24 | `phase_b_map::cfg_24_mode_negative` |
| 25, 26 | `phase_b_map::cfg_25_26_string_default_mode` |
| 27 | `phase_b_map::cfg_27_mode_above_string` |
| 28 | `phase_b_map::cfg_28_strdup_mode` |
| 29 | `phase_b_map::cfg_29_arena_mode` |
| 30 | `phase_b_map::cfg_30_explicit_default_mode` |
| 31 | `phase_b_map::cfg_31_string_mode_out_of_range` |
| 32 | `phase_b_map::cfg_32_put_default_path` |
| 33 | `phase_b_map::cfg_33_hmget_key_ts` |
| 34 | `phase_b_map_del::cfg_34_del_last` |
| 35 | `phase_b_map_del::cfg_35_del_middle_and_first` |
| 36 | `phase_b_map_del::cfg_36_del_random_order` |
| 37 | `phase_b_map_del::cfg_37_shrink_path` |
| 38 | `phase_b_map_del::cfg_38_rebuild_path_small_table` |
| 39 | `phase_b_map_del::cfg_39_tombstone_reuse_stress` |
| 40 | `phase_b_map_del::cfg_40_del_string_maps` |
| 41 | `phase_b_map_del::cfg_41_del_keyoffset_variants` |
| 42 | `phase_b_map_del::cfg_42_hmfree_variants` |
| 43 | `phase_b_arena::cfg_43_stralloc_first_block_boundary` |
| 44 | `phase_b_arena::cfg_44_stralloc_block_growth_chain` |
| 45 | `phase_b_arena::cfg_45_stralloc_oversized` |
| 46 | `phase_b_arena::cfg_46_stralloc_block_field_range`, `cfg_46b_stralloc_block_field_shift_overflow` |
| 47 | `phase_b_arena::cfg_47_strreset_shapes`, `cfg_43_47_stralloc_random_stress` |
| 48 | `phase_b_top::cfg_48_strkey` |
| 49, 50 | `phase_b_helxo::cfg_49_50_helxo` |

## Feature combinations

`Cargo.toml` has no `[features]` table ⇒ exactly one combination (the default, which
is also `--no-default-features`). `./check_features.sh` enumerates the features
mechanically and re-runs `cargo check` + the full suite for each; the cardinality is 1.
