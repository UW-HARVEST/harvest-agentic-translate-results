# CONFIGS.md — Phase B configuration surface table

Derived **mechanically** from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `mode` (int arg of `hmget_key`, `hmget_key_ts`, `hmput_key`, `hmdel_key`) | `mode >= STBDS_HM_STRING(1)` → string path vs binary path (lines 560, 590, 713, 732, 791); `mode == STBDS_HM_STRING(1)` exactly → strdup free + string re-lookup in `hmdel_key` (lines 836, 842) | so `0`, `1`, `2`(=`PTR_TO_STRING`), `3`, `255`, `INT_MAX` and negatives `-1`, `INT_MIN` are 4 genuinely different classes |
| `table->string.mode` (unsigned char) | `STBDS_SH_NONE(0)`→`memcpy` key, `STBDS_SH_DEFAULT(1)`→store pointer, `STBDS_SH_STRDUP(2)`→`stbds_strdup`, `STBDS_SH_ARENA(3)`→`stbds_stralloc`, anything else→`memcpy` (switch at line 785); STRDUP also selects the free-sweep in `hmfree_func` (line 575) and the free in `hmdel_key` (line 836) | set by `stbds_shmode_func(elemsize, mode)` (line 803, unvalidated `(unsigned char)` cast) or implicitly by `hmput_key` (line 707) |
| `elemsize` | any; interacts with `HASH_TO_ARR`/`ARR_TO_HASH` pointer arithmetic and the `memmove` in `hmdel_key` | every hm function |
| `keysize` | `memcmp`/`memcpy` width in binary mode; `0` is degenerate (everything compares equal) | lines 563, 789 |
| `keyoffset` | `hmdel_key` only (the macros pass `STBDS_OFFSETOF(t,key)`); `0` in `hmget_key*`/`hmput_key` | lines 561, 563, 837, 843, 845 |
| element count `N` | table is created with `slot_count = 8`; grows when `used_count >= slot_count - slot_count/4` (line 698) → thresholds at 6, 12, 24, 48, 96, …; shrinks when `used_count < slot_count>>2` and `slot_count > 8` (line 854); rebuilds when `tombstone_count > slot_count/8 + slot_count/16` (line 858) | lines 698, 854, 858 |
| hash seed | `stbds_hash_seed` starts at `0x31415926`, is reseedable with `stbds_rand_seed`, and self-advances `seed = seed*a + b` on every fresh `make_hash_index` (lines 353, 357, 410-412) | affects every probe position |
| `stbds_hash_bytes` length class | `len/8` full 8-byte blocks + tail `len%8 ∈ {0..7}` fall-through switch (lines 522-541); tail bytes `>= 0x80` sign-extend through `int` | lines 522-541 |
| `stbds_hash_string` content | empty string; bytes `>= 0x80` (cast to `unsigned char`, line 481) | lines 477-491 |
| arena `remaining` / `block` | `len > remaining` → new block; `blocksize = 512 << (block>>1)`; `len > blocksize` → dedicated oversized block, and inside that, `a->storage == NULL` vs `!= NULL` are two different splices; `blocksize < 1<<20` gates `++a->block` (saturation) | lines 885-911 |
| `str_dups(num)` | `num <= 0` skips the arena loop entirely; larger `num` walks the arena block sizes | lines 952-954 |

## Configuration rows

Every row is exercised through **both** `.so`s via `libloading` with many
randomized inputs (fixed seed `0xC0FFEE_...`, see `tests/common/mod.rs`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap ∈ {1,2,3}` → `min_cap` clamped to 4; `elemsize ∈ {1,4,8,12,16,32,64}` | [x] |
| 2 | `stbds_arrgrowf` | `a = NULL`, `min_cap = 0`, random `addlen ∈ 1..4096` → `min_cap = max(addlen,4)` | [x] |
| 3 | `stbds_arrgrowf` | existing array, `min_cap <= arrcap` → identity return, header untouched | [x] |
| 4 | `stbds_arrgrowf` | existing array, `min_cap` in `(cap, 2*cap)` → doubling branch (line 289) | [x] |
| 5 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` → exact-fit branch | [x] |
| 6 | `stbds_arrgrowf` | repeated `arrmaybegrow`-style append loop (1 elem at a time, 0..300 elems) → full capacity growth sequence 4,8,16,… | [x] |
| 7 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free; then grow again (fresh allocation) | [x] |
| 8 | `stbds_hash_bytes` | `len ∈ 0..=80` (all of them), random bytes, default seed | [x] |
| 9 | `stbds_hash_bytes` | `len ∈ 0..=80`, all-`0x00`, all-`0xFF`, all-`0x80`, alternating `0x7F/0x80` patterns (tail sign-extension) | [x] |
| 10 | `stbds_hash_bytes` | seeds `{0, 1, 0x31415926, SIZE_MAX, SIZE_MAX-1, random×32}` × `len ∈ {0,1,7,8,9,15,16,17,63,64}` | [x] |
| 11 | `stbds_hash_string` | random ASCII strings, `len ∈ 0..=64`, default seed | [x] |
| 12 | `stbds_hash_string` | strings containing bytes `0x80..=0xFF` (unsigned-char cast), `len ∈ 1..=32` | [x] |
| 13 | `stbds_hash_string` | seeds `{0,1,SIZE_MAX,random×32}` × strings of length `{0,1,8,9,64}` | [x] |
| 14 | `stbds_rand_seed` + `stbds_shmode_func` | reseed to `{0,1,0x31415926,SIZE_MAX,random}`, create table, read `table->seed`; create a 2nd table → observe `seed = seed*a + b` advance | [x] |
| 15 | `stbds_stralloc` | fresh arena (`{0}`), one string of length `{0,1,10,510,511,512,513,1000}` | [x] |
| 16 | `stbds_stralloc` | fresh arena, 1..400 random short strings (len 1..60) → walks `block` 0→…, `remaining` exhaustion, new blocks | [x] |
| 17 | `stbds_stralloc` | fresh arena, first string oversized (`len > 512`) → dedicated block with `storage == NULL` splice | [x] |
| 18 | `stbds_stralloc` | short string first (storage non-NULL), then oversized string → `sb->next = head->next; head->next = sb` splice, `remaining` preserved | [x] |
| 19 | `stbds_stralloc` | interleaved short/oversized sequence, 200 ops | [x] |
| 20 | `stbds_stralloc` | arena with `block` pre-set to `{0,1,2,3,16,18,19,20,21,22,23,24}` → `512<<(block>>1)` crosses `1<<20` at `block == 22` → `++block` saturates. `err_43_stralloc_block_saturates` additionally sweeps every `block ∈ 0..=24` and asserts the bump/no-bump decision | [x] |
| 21 | `stbds_stralloc` | arena with `block ∈ {110,112,118,126,127,128,129,130,131,238,250,254,255}` → `512 << (block>>1)` shift-count `>= 64` (C UB; x86-64 masks the count to 6 bits). Only values whose *masked* shift yields `blocksize == 0` or a small block are used - e.g. `block = 200` masks to `<< 36`, i.e. a 32 TiB `malloc` that fails and is then dereferenced, which aborts BOTH libraries and measures nothing | [x] |
| 22 | `stbds_strreset` | empty arena; arena with 1 block; arena with many blocks; arena with an oversized-spliced chain; then `stralloc` again after reset | [x] |
| 23 | `stbds_hmput_key` | `a = NULL` bootstrap, `mode = 0` (binary), `keysize = 8`, `elemsize = 16`, N = 1 | [x] |
| 24 | `stbds_hmput_key` | binary `mode = 0`, `string.mode = NONE`, `keysize ∈ {1,2,3,4,5,6,7,8,9,16,32}`, `elemsize = keysize+8` rounded, N ∈ 1..40 random distinct keys | [x] |
| 25 | `stbds_hmput_key` | binary, N ∈ {1,5,6,7,8,11,12,13,23,24,25,47,48,49,100,200} → every table-growth threshold | [x] |
| 26 | `stbds_hmput_key` | binary, key stream with ~50 % duplicates → duplicate-hit path (`temp` = existing index, length unchanged) | [x] |
| 27 | `stbds_hmput_key` | binary, `keysize = 0` (degenerate: `memcmp(…,0)==0` always) , N = 10 | [x] |
| 28 | `stbds_hmput_key` | binary, `elemsize ∈ {8,12,16,24,32,64}` × N ∈ {1,10,60} | [x] |
| 29 | `stbds_hmget_key` / `stbds_hmget_key_ts` | binary map with N ∈ {0,1,10,60}: look up every present key (hit) and 60 absent keys (miss → `-1`) | [x] |
| 30 | `stbds_hmget_key_ts` | `a = NULL` and `a != NULL` with `hash_table == NULL` (array from `hmput_default`) → `*temp = -1` | [x] |
| 31 | `stbds_hmput_default` | `a = NULL`; `a` with `length == 0`; `a` with `length != 0` (no-op); then `hmput_key` on the result | [x] |
| 32 | `stbds_hmdel_key` | binary, `keyoffset = 0`, delete every key in randomized order from maps of N ∈ {1,2,10,60,200} | [x] |
| 33 | `stbds_hmdel_key` | binary, `keyoffset ∈ {4,8,16}` with `elemsize = keyoffset + keysize + 8` | [x] |
| 34 | `stbds_hmdel_key` | binary, N = 200 then delete 175 → crosses `used_count_shrink_threshold` repeatedly (halving 128→64→32→16→8) | [x] |
| 35 | `stbds_hmdel_key` | binary, alternating insert/delete 400 times on a small map → `tombstone_count > tombstone_count_threshold` rebuild path | [x] |
| 36 | `stbds_hmdel_key` | binary, delete the last element (`old_index == final_index`, no `memmove`) and a middle element (`memmove` + re-`find_slot`) | [x] |
| 37 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = STBDS_SH_DEFAULT(1)`, `mode = 1`, caller-owned key pointers, N ∈ {1,10,60}, `elemsize ∈ {16,24,32}` | [x] |
| 38 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = STBDS_SH_STRDUP(2)`, `mode = 1`, N ∈ {1,10,60,200}; key buffers overwritten after the put to prove the strdup | [x] |
| 39 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = STBDS_SH_ARENA(3)`, `mode = 1`, N ∈ {1,10,60,200} short keys → drives `stbds_stralloc` through the map | [x] |
| 40 | `stbds_shmode_func` + `stbds_hmput_key` | `string.mode = STBDS_SH_ARENA(3)` with long keys (len 500..2000) → oversized arena blocks inside the map | [x] |
| 41 | `stbds_hmput_key` | `a = NULL`, `mode = 1` → implicit `string.mode = STBDS_SH_DEFAULT` (line 707), N ∈ {1,10,60} | [x] |
| 42 | `stbds_hmput_key` | `a = NULL`, `mode = 0` → implicit `string.mode = 0` (line 707), N ∈ {1,10,60} | [x] |
| 43 | `stbds_hmput_key` | string modes × duplicate keys → `temp_key` update on the first-loop duplicate hit vs no update on the wrap-around duplicate hit | [x] |
| 44 | `stbds_hmget_key` / `_ts` | each `string.mode ∈ {1,2,3}` map, hits + misses, N ∈ {1,10,60} | [x] |
| 45 | `stbds_hmdel_key` | `string.mode = STRDUP` with `mode = 1` (frees key) vs `mode = 2` (does **not** free, `==` test at line 836) | [x] |
| 46 | `stbds_hmdel_key` | `string.mode ∈ {DEFAULT, ARENA}` × `mode ∈ {1,2}`, delete all keys in random order, N ∈ {1,10,60} | [x] |
| 47 | `stbds_hmdel_key` | string map N = 200 → delete 190 → shrink + rebuild paths with string re-`find_slot` | [x] |
| 48 | `stbds_hmfree_func` | each `string.mode ∈ {0,1,2,3}` × N ∈ {0,1,10,60}; plus an array with `hash_table == NULL`; plus `a = NULL` | [x] |
| 49 | `stbds_shmode_func` | out-of-range `mode ∈ {4,5,7,64,127,128,254,255,256,257,-1,-2,INT_MIN,INT_MAX}` → `(unsigned char)` truncation selects the switch arm; then `hmput_key(mode=1)` on it | [x] |
| 50 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | out-of-range `mode ∈ {2,3,255,256,INT_MAX,-1,-2,INT_MIN}` on a binary and on a string map (`>=1` vs `<1` branch split) | [x] |
| 51 | `stbds_shmode_func` / `stbds_hmput_key` | `elemsize = 0` degenerate | [x] |
| 52 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,12345,INT_MIN,INT_MAX}` + 64 random ints | [x] |
| 53 | `str_dups` | `num ∈ {0,1,-1,-2,2,3,7,8,9,64,100,1000,5000,INT_MIN,INT_MAX?}` (stdout captured and compared byte-for-byte; `INT_MAX` excluded as it would run for hours) | [x] |
| 54 | full pipeline | randomized op-stream fuzz: 2000 random ops (`put`/`get`/`get_ts`/`del`/`put_default`) over a map, for each of `string.mode ∈ {0,1,2,3}` × `mode ∈ {0,1,2}`, snapshotting the full header + hash-index + bucket array + element bytes after **every** op | [x] |
| 55 | full pipeline | same fuzz with `elemsize`/`keysize`/`keyoffset` randomized per run, 40 runs | [x] |

## Cross-product pruning notes

* `keyoffset != 0` is only reachable through `stbds_hmdel_key` (rows 33, 47) —
  `hmget_key*`/`hmput_key` hard-code `keyoffset = 0` (lines 633, 682).
* `string.mode = STBDS_SH_ARENA` combined with `mode = 0` (binary) is not a
  cross-product hole: the switch at line 785 keys off `string.mode` alone, so
  it *is* reachable and is covered by row 50.
* `stbds_hash_bytes` / `stbds_hash_string` are pure functions of
  `(bytes, len, seed)` — their axes (rows 8-13) are fully independent of the
  map axes, so they are enumerated separately instead of crossed.

## Additional rows added while testing (each is a configuration the C
## distinguishes that the first pass of the table missed)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 18b | `stbds_stralloc` | `len == blocksize` **exactly**, on a NON-empty arena — the boundary of `if (len > blocksize)` (c_src/src/lib.c:893). On an *empty* arena `>` and `>=` are indistinguishable (both end with `storage` = the new block and `remaining == 0`); only with an existing head block do they differ (normal block → new head + `remaining = 0` vs oversized → spliced as `head->next`, `remaining` untouched). Probed at `blocksize-1`, `blocksize`, `blocksize+1` for the first 6 block sizes | [x] |
| 33b | `stbds_hmput_key` + `stbds_hmdel_key` | the *realistic* `keyoffset != 0` shape: `stbds_hmput` memcpy's the key into element offset 0 **and** separately assigns `t[temp].key = k` at the struct's real offset, so `stbds_hmdel`'s `STBDS_OFFSETOF(t,key)` lookup actually matches. This is the only way to reach the `memmove` + re-`find_slot` fix-up (c_src/src/lib.c:839-850) with a non-zero keyoffset. `keysize ∈ {4,8}` × `keyoffset ∈ {8,16}` × `N ∈ {1,2,10,60}`, random deletion order | [x] |
| 38b | `stbds_shmode_func(SH_STRDUP)` + `stbds_hmput_key` | after all puts, **scribble over every caller key buffer** - the map must be unaffected, proving `stbds_strdup` really copied | [x] |
| 43b | `stbds_hmput_key` | the `stbds_shputs` flavour, which writes `hash_table->temp_key` back into the element. Duplicates only for `SH_DEFAULT`/`SH_ARENA`; see the note in the test about the upstream double-free with `SH_STRDUP` | [x] |
| 50a | `hmput_key`/`hmget_key`/`hmdel_key` | out-of-range `mode ∈ {1,2,3,255,256,INT_MAX}` (all `>= STBDS_HM_STRING`, so all take the string path) on each of `string.mode ∈ {1,2,3}`; deletes in reverse insertion order | [x] |
| 50b | `hmput_key`/`hmget_key`/`hmdel_key` | out-of-range `mode ∈ {0,-1,-2,INT_MIN}` (all `< STBDS_HM_STRING`, so all take the binary path) on a binary map AND on a string map (where `memcmp` compares the caller's bytes against the stored *pointer*, so every put appends and every delete misses) | [x] |
| 50c | `stbds_shmode_func(SH_NONE)` + `stbds_hmput_key(mode >= 1)` | the `string.mode == SH_NONE` + string-`mode` combination: the insert arm memcpy's the key *pointer bytes* into the element, so only distinct-key inserts (which never reach `stbds_is_key_equal`) are well defined | [x] |
| 54c | full pipeline | delete-free string op streams, which additionally compare `hash_index::temp_key` after **every** operation - covering both the refresh (fresh insert / first-loop duplicate hit) and the deliberate no-refresh (wrap-around duplicate hit) branches | [x] |
| 55b | full pipeline | randomized string shapes: `elemsize ∈ 16..48` **including sizes that are not a multiple of 8**, so the C performs an *unaligned* `char *` store at `a + elemsize*i` which the translation must reproduce | [x] |

## Row → test mapping

| CONFIGS row(s) | test | file |
|---|---|---|
| 1 | `cfg01_arrgrowf_fresh_small_min_cap` | `tests/phase_b_low.rs` |
| 2 | `cfg02_arrgrowf_fresh_random_addlen` | `tests/phase_b_low.rs` |
| 3 | `cfg03_arrgrowf_identity_when_capacity_suffices` | `tests/phase_b_low.rs` |
| 4, 5 | `cfg04_05_arrgrowf_doubling_and_exact_fit` | `tests/phase_b_low.rs` |
| 6 | `cfg06_arrgrowf_append_loop` | `tests/phase_b_low.rs` |
| 7 | `cfg07_arrgrowf_free_regrow` | `tests/phase_b_low.rs` |
| 8 | `cfg08_hash_bytes_all_lengths_random` | `tests/phase_b_low.rs` |
| 9 | `cfg09_hash_bytes_patterns` | `tests/phase_b_low.rs` |
| 10 | `cfg10_hash_bytes_seed_sweep` | `tests/phase_b_low.rs` |
| 11 | `cfg11_hash_string_random_ascii` | `tests/phase_b_low.rs` |
| 12 | `cfg12_hash_string_high_bit_bytes` | `tests/phase_b_low.rs` |
| 13 | `cfg13_hash_string_seed_sweep` | `tests/phase_b_low.rs` |
| 14 | `cfg14_rand_seed_and_advance`, `t03_seed_advance_identical` | `tests/phase_b_low.rs`, `tests/smoke.rs` |
| 15 | `cfg15_stralloc_single_string` | `tests/phase_b_low.rs` |
| 16 | `cfg16_stralloc_many_short` | `tests/phase_b_low.rs` |
| 17 | `cfg17_stralloc_oversized_first` | `tests/phase_b_low.rs` |
| 18 | `cfg18_stralloc_oversized_splice` | `tests/phase_b_low.rs` |
| 18b | `cfg18b_stralloc_len_exactly_blocksize` | `tests/phase_b_low.rs` |
| 19 | `cfg19_stralloc_interleaved` | `tests/phase_b_low.rs` |
| 20 | `cfg20_stralloc_block_saturation` | `tests/phase_b_low.rs` |
| 21 | `cfg21_stralloc_shift_count_overflow` | `tests/phase_b_low.rs` |
| 22 | `cfg22_strreset_shapes` | `tests/phase_b_low.rs` |
| 23 | `cfg23_binary_bootstrap_single_insert` | `tests/phase_b_map.rs` |
| 24 | `cfg24_binary_keysize_sweep` | `tests/phase_b_map.rs` |
| 25 | `cfg25_binary_growth_thresholds` | `tests/phase_b_map.rs` |
| 26 | `cfg26_binary_duplicate_keys` | `tests/phase_b_map.rs` |
| 27 | `cfg27_binary_zero_keysize` | `tests/phase_b_map.rs` |
| 28 | `cfg28_binary_elemsize_sweep` | `tests/phase_b_map.rs` |
| 29 | `cfg29_binary_get_hit_and_miss` | `tests/phase_b_map.rs` |
| 30 | `cfg30_hmget_key_ts_no_table` | `tests/phase_b_map.rs` |
| 31 | `cfg31_hmput_default` | `tests/phase_b_map.rs` |
| 32 | `cfg32_binary_delete_all_random_order` | `tests/phase_b_map.rs` |
| 33 | `cfg33_binary_nonzero_keyoffset` | `tests/phase_b_map.rs` |
| 33b | `cfg33b_keyoffset_macro_shape` | `tests/phase_b_top.rs` |
| 34 | `cfg34_binary_shrink_cascade` | `tests/phase_b_map.rs` |
| 35 | `cfg35_binary_tombstone_rebuild` | `tests/phase_b_map.rs` |
| 36 | `cfg36_binary_delete_last_vs_middle` | `tests/phase_b_map.rs` |
| 37 | `cfg37_sh_default` | `tests/phase_b_string.rs` |
| 38 | `cfg38_sh_strdup` | `tests/phase_b_string.rs` |
| 38b | `cfg38b_strdup_copies_the_key` | `tests/phase_b_string.rs` |
| 39 | `cfg39_sh_arena` | `tests/phase_b_string.rs` |
| 40 | `cfg40_sh_arena_long_keys` | `tests/phase_b_string.rs` |
| 41 | `cfg41_implicit_sh_default` | `tests/phase_b_string.rs` |
| 42 | `cfg42_implicit_sh_none` | `tests/phase_b_string.rs` |
| 43 | `cfg43_string_duplicates_temp_key` | `tests/phase_b_string.rs` |
| 43b | `cfg43b_shputs_writes_temp_key_back` | `tests/phase_b_string.rs` |
| 44 | `cfg44_string_get_hit_and_miss` | `tests/phase_b_string.rs` |
| 45 | `cfg45_string_delete_mode_1_vs_2` | `tests/phase_b_string.rs` |
| 46 | `cfg46_string_delete_random_order` | `tests/phase_b_string.rs` |
| 47 | `cfg47_string_shrink_and_rebuild` | `tests/phase_b_string.rs` |
| 48 | `cfg48_hmfree_all_shapes` | `tests/phase_b_string.rs` |
| 49 | `cfg49_shmode_out_of_range` | `tests/phase_b_string.rs` |
| 50 | `cfg50a_out_of_range_mode_string_path`, `cfg50b_out_of_range_mode_binary_path`, `cfg50c_string_mode_on_sh_none_map_inserts_only` | `tests/phase_b_string.rs` |
| 51 | `cfg51_zero_elemsize` | `tests/phase_b_string.rs` |
| 52 | `cfg52_strkey` | `tests/phase_b_top.rs` |
| 53 | `cfg53_str_dups_stdout`, `cfg53b_str_dups_repeated` | `tests/phase_b_top.rs` |
| 54 | `cfg54_fuzz_binary_streams`, `cfg54_fuzz_string_streams`, `cfg54c_fuzz_string_streams_no_delete_with_temp_key` | `tests/phase_b_top.rs` |
| 55 | `cfg55_fuzz_randomized_shapes` | `tests/phase_b_top.rs` |
| 55b | `cfg55b_fuzz_randomized_string_shapes` | `tests/phase_b_top.rs` |
