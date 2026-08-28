# CONFIGS.md — Phase B configuration-surface table

Mechanically enumerated from the branches `c_src/src/lib.c` actually takes on
valid input. Every row is exercised with **many randomized inputs** (fixed seed
`0xC0FFEE_...`, see `tests/common/mod.rs::Rng`) through **both** `.so` files and
compared byte-for-byte.

## Axes the C code branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `mode` (hash-map key mode) | `STBDS_HM_BINARY = 0` vs `mode >= STBDS_HM_STRING (1)`; `mode == 1` **exactly** at L836/L842 | L560, L590, L707, L713, L732, L823, L836, L842 |
| `table->string.mode` (arena mode) | `STBDS_SH_NONE 0` (→ `default:` memcpy), `STBDS_SH_DEFAULT 1`, `STBDS_SH_STRDUP 2`, `STBDS_SH_ARENA 3` | L785-790, L575, L836 |
| `elemsize` | `0`, `< header`, `== keysize`, `> keysize`, non-multiple-of-8 | all `elemsize*i` arithmetic |
| `keysize` | `0`, `1`, `2`, `4`, `8` (ptr-sized), `16`, `> elemsize` | L563, L713, L789 |
| `keyoffset` | `0` (hardcoded in `hmput_key`) vs non-zero (only `hmdel_key` accepts it) | L682, L807, L843 |
| hash `seed` | `0`, default `0x31415926`, `SIZE_MAX`, arbitrary; global auto-advance per fresh table | L353, L410-412 |
| `slot_count` | `8` (initial; no shrink) → `16` → `32` → `64` … doubling at `used_count >= sc-(sc>>2)` | L399, L702, L854, L858 |
| population size | `0`, `1`, `5`, `6` (at threshold), `7` (first grow), `12`, `13` (second grow), `100`, `1000` | L698 |
| probe path | in-bucket forward scan (`i = pos&7 .. 7`) vs **wrap-around** scan (`i = 0 .. pos&7`) vs next-bucket (`pos += step`) | L728-763, L604-623 |
| tombstones | none / below `tombstone_count_threshold` / above (→ rebuild) / reuse of a tombstone slot on insert | L740, L766, L858 |
| `stbds_hash_bytes` `len` | `0`, `1..7` (each `switch` fall-through case), `8` (one full word), `9..15`, `16`, `17`, large; high-bit-set bytes at index 3 and 7 (sign-extension quirk) | L522-541 |
| `stbds_hash_string` input | empty, 1 char, 8 chars, long, high-bit (≥0x80) bytes, embedded digits | L477-491 |
| `stbds_arrgrowf` request | `a NULL` vs non-NULL; `addlen` `0`/`1`/`n`; `min_cap` `0`/`1..3` (clamp to 4)/`< 2*cap` (double)/`> 2*cap` | L283-292 |
| arena `block` | `0` (512) … `21` (524288) … `22`+ (saturated at 1 MiB); `remaining` `0` / `< len` / `>= len`; `storage` NULL / non-NULL | L885-911 |
| arena string length | `1` (empty str) … `< blocksize` … `> blocksize` (oversized path) | L893 |
| `str_put` `num` | `0`, `1`, small, exactly the block-fill boundary (~73), multi-block, large | L951 |

---

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1  | `stbds_hash_bytes` | `len = 0`, random `seed` (incl. `0`, `SIZE_MAX`) | `cfg_c1_hash_bytes_len0` | [x] |
| C2  | `stbds_hash_bytes` | `len = 1..7` (every tail `switch` case), random bytes | `cfg_c2_hash_bytes_tail_1_7` | [x] |
| C3  | `stbds_hash_bytes` | `len = 1..7` with **byte 3 ≥ 0x80** (the `d[3]<<24` sign-extension quirk) | `cfg_c3_hash_bytes_tail_high_bit` | [x] |
| C4  | `stbds_hash_bytes` | `len = 8` exactly (one main-loop word, empty tail) | `cfg_c4_hash_bytes_len8` | [x] |
| C5  | `stbds_hash_bytes` | `len = 8` with **byte 3 and/or byte 7 ≥ 0x80** (both sign-extension sites in the main loop) | `cfg_c5_hash_bytes_word_high_bits` | [x] |
| C6  | `stbds_hash_bytes` | `len = 9..64` random (main loop + every tail remainder) | `cfg_c6_hash_bytes_mixed_len` | [x] |
| C7  | `stbds_hash_bytes` | `len = 1024` (many main-loop iterations), random buffer | `cfg_c7_hash_bytes_large` | [x] |
| C8  | `stbds_hash_bytes` | `seed` sweep: `0`, `1`, `0x31415926`, `SIZE_MAX`, `1<<63`, random | `cfg_c8_hash_bytes_seed_sweep` | [x] |
| C9  | `stbds_hash_string` | empty string, random `seed` | `cfg_c9_hash_string_empty` | [x] |
| C10 | `stbds_hash_string` | 1..64 random ASCII chars, random `seed` | `cfg_c10_hash_string_ascii` | [x] |
| C11 | `stbds_hash_string` | bytes `0x80..0xFF` (the `(unsigned char)` cast path), random `seed` | `cfg_c11_hash_string_high_bytes` | [x] |
| C12 | `stbds_hash_string` | long strings (256..4096 bytes) | `cfg_c12_hash_string_long` | [x] |
| C13 | `stbds_rand_seed` + `stbds_shmode_func` | global seed set to `0` / `SIZE_MAX` / random, then a fresh table ⇒ verify the LCG advance `seed*0x27bb2ee687b0b0fd + 0xb504f32d` | `cfg_c13_rand_seed_advance` | [x] |
| C14 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap ∈ {1,2,3}` ⇒ clamp to 4 | `cfg_c14_arrgrowf_fresh_clamp` | [x] |
| C15 | `stbds_arrgrowf` | `a = NULL`, `addlen ∈ {0..64}`, `min_cap ∈ {0..64}`, `elemsize ∈ {1,4,8,12,16,40}` | `cfg_c15_arrgrowf_fresh_matrix` | [x] |
| C16 | `stbds_arrgrowf` | existing array, `min_cap < 2*cap` ⇒ **doubling** path | `cfg_c16_arrgrowf_double` | [x] |
| C17 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` ⇒ exact-`min_cap` path | `cfg_c17_arrgrowf_exact` | [x] |
| C18 | `stbds_arrgrowf` | existing array, `min_len = arrlen + addlen > min_cap` ⇒ `min_cap = min_len` | `cfg_c18_arrgrowf_minlen_wins` | [x] |
| C19 | `stbds_arrgrowf` + `stbds_arrfreef` | grow-chain (repeated growth) then free; header/`length`/`capacity`/`temp`/`hash_table` preserved across every grow | `cfg_c19_arrgrowf_chain_then_free` | [x] |
| C20 | `stbds_hmput_key` | `mode = BINARY`, `elemsize = 8`, `keysize = 4`, 1 insert | `cfg_c20_binary_single` | [x] |
| C21 | `stbds_hmput_key` | `mode = BINARY`, `keysize ∈ {1,2,4,8,16}` × `elemsize ∈ {keysize, keysize+4, 40}`, 5 inserts | `cfg_c21_binary_keysize_matrix` | [x] |
| C22 | `stbds_hmput_key` | `mode = BINARY`, 6 inserts (`used_count == threshold`, no grow yet) | `cfg_c22_binary_at_threshold` | [x] |
| C23 | `stbds_hmput_key` | `mode = BINARY`, 7 inserts (**first grow** 8→16, seed inherited) | `cfg_c23_binary_first_grow` | [x] |
| C24 | `stbds_hmput_key` | `mode = BINARY`, 13 / 25 / 49 inserts (grow 16→32→64→128) | `cfg_c24_binary_multi_grow` | [x] |
| C25 | `stbds_hmput_key` | `mode = BINARY`, 1000 random `u64` keys (deep probing, wrap-around and next-bucket paths) | `cfg_c25_binary_1000` | [x] |
| C26 | `stbds_hmput_key` | `mode = BINARY`, **duplicate** keys re-put (update path, `temp` = existing index, no length change) | `cfg_c26_binary_duplicates` | [x] |
| C27 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 0` (all keys "equal") | `cfg_c27_binary_keysize0` | [x] |
| C28 | `stbds_hmput_key` | `mode = BINARY`, `elemsize = 0` | `cfg_c28_binary_elemsize0` | [x] |
| C29 | `stbds_hmput_key` | `mode = STRING`, no `shmode_func` ⇒ `string.mode` auto-set to `STBDS_SH_DEFAULT`; keys are caller-owned pointers; check `temp_key` | `cfg_c29_string_default_mode` | [x] |
| C30 | `stbds_shmode_func` + `stbds_hmput_key` | `STBDS_SH_STRDUP`, N random strings; keys are `strdup`ed (pointer differs from input), `temp_key` set | `cfg_c30_strdup_mode` | [x] |
| C31 | `stbds_shmode_func` + `stbds_hmput_key` | `STBDS_SH_ARENA`, N random strings; keys arena-allocated; arena `block`/`remaining` progression | `cfg_c31_arena_mode` | [x] |
| C32 | `stbds_shmode_func` + `stbds_hmput_key` | `STBDS_SH_NONE (0)` with `mode = STRING` ⇒ `switch default:` memcpy of the *pointer bytes* | `cfg_c32_sh_none_with_string_mode` | [x] |
| C33 | `stbds_shmode_func` + `stbds_hmput_key` | `STBDS_SH_DEFAULT` explicit, `mode = BINARY` (arena mode 1 but binary hashing) | `cfg_c33_sh_default_binary_mode` | [x] |
| C34 | `stbds_hmput_key` | `mode = STRING`, 100+ strings ⇒ multiple table grows in STRDUP / ARENA / DEFAULT modes | `cfg_c34_string_many_all_modes` | [x] |
| C35 | `stbds_hmput_key` | `mode = STRING`, duplicate strings (distinct pointers, equal content) ⇒ first-inner-loop hit sets `temp_key` | `cfg_c35_string_duplicates` | [x] |
| C36 | `stbds_hmget_key_ts` | populated BINARY map, look up every present key + absent keys, `temp` compared | `cfg_c36_hmget_ts_binary` | [x] |
| C37 | `stbds_hmget_key_ts` | populated STRING map (each of the 4 `string.mode`s), present + absent keys | `cfg_c37_hmget_ts_string` | [x] |
| C38 | `stbds_hmget_key` | same as C36/C37 but through the non-`_ts` wrapper (header `temp` written) | `cfg_c38_hmget_key_wrapper` | [x] |
| C39 | `stbds_hmput_default` | fresh (NULL), then on an existing populated map (no-op), then interleaved with `hmput_key` | `cfg_c39_hmput_default_paths` | [x] |
| C40 | `stbds_hmdel_key` | `mode = BINARY`, delete the **last** element (`old_index == final_index`, no relocation) | `cfg_c40_del_last` | [x] |
| C41 | `stbds_hmdel_key` | `mode = BINARY`, delete a **middle** element (relocation + re-index of the moved key) | `cfg_c41_del_middle` | [x] |
| C42 | `stbds_hmdel_key` | `mode = BINARY`, delete enough to cross `tombstone_count_threshold` ⇒ rebuild at same `slot_count` | `cfg_c42_del_tombstone_rebuild` | [x] |
| C43 | `stbds_hmdel_key` | `mode = BINARY`, grow to 32+ slots then delete below `used_count_shrink_threshold` ⇒ shrink | `cfg_c43_del_shrink` | [x] |
| C44 | `stbds_hmdel_key` | `mode = STRING` × `string.mode ∈ {DEFAULT, STRDUP, ARENA, NONE}` (STRDUP additionally frees the key) | `cfg_c44_del_string_all_modes` | [x] |
| C45 | `stbds_hmdel_key` | `keyoffset != 0` (key not the first struct member) with `mode = BINARY` | `cfg_c45_del_nonzero_keyoffset` | [x] |
| C46 | `stbds_hmput_key` + `stbds_hmdel_key` | insert / delete / re-insert so that an insert lands on a **tombstone** (`tombstone >= 0` at `found_empty_slot`) | `cfg_c46_insert_into_tombstone` | [x] |
| C47 | `stbds_hmput_key`+`hmget`+`hmdel`+`hmfree_func` | full randomized pipeline: 2000 random ops (put/get/del) on a BINARY map, state compared after **every** op | `cfg_c47_random_pipeline_binary` | [x] |
| C48 | `stbds_hmput_key`+`hmget`+`hmdel`+`hmfree_func` | full randomized pipeline: 2000 random ops on a STRING map, for each of the 4 `string.mode`s | `cfg_c48_random_pipeline_string` | [x] |
| C49 | `stbds_hmfree_func` | teardown of DEFAULT / STRDUP / ARENA / NONE maps (STRDUP frees each key, ARENA resets the arena) | `cfg_c49_hmfree_all_modes` | [x] |
| C50 | `stbds_stralloc` | fresh arena (`storage=NULL, remaining=0, block=0`), 1 short string | `cfg_c50_stralloc_fresh` | [x] |
| C51 | `stbds_stralloc` | fresh arena, N short strings until the 512-byte block is exhausted and a new block is taken | `cfg_c51_stralloc_block_refill` | [x] |
| C52 | `stbds_stralloc` | `block` sweep `0..=22` supplied by the caller ⇒ `512 << (block>>1)` up to the 1 MiB saturation | `cfg_c52_stralloc_block_sweep` | [x] |
| C53 | `stbds_stralloc` | `len > blocksize` oversized path with `storage == NULL` **and** with `storage != NULL` (splice-behind) | `cfg_c53_stralloc_oversized_both` | [x] |
| C54 | `stbds_stralloc` | `len == remaining` exactly (boundary: no new block) and `len == remaining+1` (new block) | `cfg_c54_stralloc_remaining_boundary` | [x] |
| C55 | `stbds_stralloc` | empty string (`len == 1`), repeated 1000× | `cfg_c55_stralloc_empty_strings` | [x] |
| C56 | `stbds_stralloc` + `stbds_strreset` | randomized 500-string mix of short/long, then reset; arena state compared after every call | `cfg_c56_stralloc_random_then_reset` | [x] |
| C57 | `stbds_strreset` | on a fresh (all-zero) arena, on a single-block arena, on a many-block arena, called twice | `cfg_c57_strreset_states` | [x] |
| C58 | `strkey` | `n ∈ {0, 1, 9, 10, 99, 100, 12345, -1, -99, INT_MAX, INT_MIN}` + 200 random ints | `cfg_c58_strkey_values` | [x] |
| C59 | `str_put` | `num ∈ {0, 1, 2, 5, 72, 73, 74, 100, 146, 1000, 5000, -1, -100, INT_MIN, INT_MAX-ish}` — stdout captured and compared byte-for-byte | `cfg_c59_str_put_stdout` | [x] |
| C60 | `str_put` | repeated calls in sequence (global `stbds_hash_seed` advances once per call ⇒ later calls use different seeds) | `cfg_c60_str_put_repeated` | [x] |
| C61 | `stbds_hmput_key` | `mode` out-of-range but **valid-path** (`mode = 2`, `7`, `INT_MAX` ⇒ STRING; `mode = -1`, `INT_MIN` ⇒ BINARY) | `cfg_c61_mode_out_of_range_valid_path` | [x] |
| C62 | end-to-end | the exact `shputs` sequence from `str_put` reproduced through the raw entry points (`hmput_key` + header `temp` + `temp_key`), randomized keys/values | `cfg_c62_shputs_pipeline` | [x] |
| C63 | `stbds_rand_seed` + `hmput_key` | the same 40-key map rebuilt under a sweep of 10 global seeds (0, 1, 2, default, `SIZE_MAX`, `1<<63`, ...) — full internal state compared | `cfg_c13b_rand_seed_affects_map_layout` | [x] |
| C64 | `stbds_arrgrowf` | growth must preserve the caller's `temp` and `hash_table` header fields across `realloc` | `cfg_c19b_arrgrowf_temp_and_hashtable_preserved` | [x] |
| C65 | `str_put` | 350 randomized `num` values, stdout compared byte-for-byte | `cfg_c59b_str_put_random` | [x] |
| C66 | end-to-end | `shputs` + `shgeti` + `shdel` macro pipeline over 150 randomized string keys, deleted in shuffled order, state compared after every op | `cfg_c62b_shputs_then_shdel` | [x] |
| C67 | both `.so`s | load both libraries and agree on `hash_bytes` / `hash_string` (harness smoke test) | `smoke_loads_both_libs_and_hashes_match` | [x] |
