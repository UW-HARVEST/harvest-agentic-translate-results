# CONFIGS.md — Phase A configuration surface table (valid inputs)

Axes derived mechanically from `c_src/src/lib.c` — every runtime flag the public
API can set and every input shape the code branches on.

## Axes

| axis | values the C code actually distinguishes | source |
|------|------------------------------------------|--------|
| **A. entry point** | L0 pure: `stbds_hash_bytes`, `stbds_hash_string`, `stbds_rand_seed`, `strkey` · L1: `stbds_arrgrowf`, `stbds_arrfreef`, `stbds_stralloc`, `stbds_strreset` · L2 map primitives: `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_shmode_func`, `stbds_hmfree_func` · L3 composed: `helxo` | `nm -D` |
| **B. `mode` argument** | `0` = `STBDS_HM_BINARY`; `1` = `STBDS_HM_STRING`; `>= 2` (also "string": `mode >= STBDS_HM_STRING`); `< 0` (binary) | L560, L590, L713, L732, L836, L842 |
| **C. table `string.mode`** | `SH_NONE 0` → `memcpy` key bytes · `SH_DEFAULT 1` → store caller pointer · `SH_STRDUP 2` → `stbds_strdup` · `SH_ARENA 3` → `stbds_stralloc` · out-of-range → truncated `(unsigned char)`, hits `default:` | L785-790, L707, L803 |
| **D. table creation path** | implicit (`hmput_key` with `a == NULL`) · `shmode_func` first · `hmput_default` first · `hmget_key`/`hmget_key_ts` first (creates array with **no** index) | L686, L698, L796, L669, L634 |
| **E. `elemsize`/`keysize` shape** | `(8,4)` int key + int val · `(16,8)` ptr key + val · `(16,4)` int key + padding · `(4,4)` key only · `(8,8)` · `(24,16)` 2-int key + vals · `(32,32)` · `keysize` = 0,1,2,3,4,5,6,7,8,9,16,17 (siphash tail + `memcmp` widths) | L561-563, L713, L789 |
| **F. element count** | 0, 1, 2, 5, **6** (= `used_count_threshold` of an 8-slot index ⇒ growth), 7, 8, 12 (threshold of a 16-slot index), 17, 64, 300, 1000 | L698, L702 |
| **G. key distribution** | all distinct random · sequential · duplicates (re-put existing key) · same key repeated | L729-735, L747-751 |
| **H. delete pattern** | none · delete last element (`old_index == final_index`) · delete first (forces `memmove` + slot re-point) · delete middle · delete-all · delete missing key · 2 deletes on an 8-slot index (⇒ tombstone rebuild) · delete down past `used_count_shrink_threshold` on a ≥16-slot index (⇒ shrink) · delete then re-insert (⇒ tombstone reuse) | L807-866 |
| **I. hash seed** | default `0x31415926` · `rand_seed(0)` · `rand_seed(SIZE_MAX)` · `rand_seed(random)` · seed advancement across successive index creations (`hash_seed = hash_seed*a + b`) | L353, L355, L409-412 |
| **J. arena string shape** | len 0,1,7,8,63,64,511,512,513 (block boundary `512`), > blocksize (dedicated block), 1<<20 saturation of `a->block`, empty arena vs. populated arena, reuse after `strreset` | L881-918 |
| **K. byte-order / value dependence** | siphash reads little-endian byte-by-byte and **sign-extends** `d[3]<<24` / `d[7]<<24`; keys with high bit set (`>= 0x80`) take a different arithmetic path than keys without | L523-524, L533-539 |

## Rows (cross product, pruned to what the C distinguishes)

Status: **45/45 rows checked**, all passing (`./scripts/verify.sh`).
Every row is driven through the `.so` exports of *both* libraries with
`libloading`; after every single call the harness compares an
address-independent fingerprint of the **whole** state: `hdr.{length,capacity,
temp}`, all 12 scalar fields of `stbds_hash_index`, the complete
`slot_count`-entry bucket array (`hash[]` + `index[]`) and every element's key
and value bytes (`tests/common/mod.rs::fingerprint`).

Every row is exercised with **many randomized inputs** — a fixed-seed
`SplitMix64` (`tests/common/mod.rs::Rng`), so every run is reproducible — and
never with a single hand-picked value.  Raw pointer *values* (heap addresses)
and the uninitialised `temp_key` field are excluded from the fingerprint;
`temp_key` is instead compared by *content* right after each string-mode put,
where the C actually defines it.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| C1 | `stbds_hash_bytes` | len = 0..72 (all), 200 random buffers per len, seed = `0x31415926` (axes E,K) | `cfg_hash_bytes_all_lengths` | [x] |
| C2 | `stbds_hash_bytes` | len random 0..256 × 64 buffers per seed, seed ∈ {0,1,2,3,0x31415926,SIZE_MAX,SIZE_MAX-1,SIZE_MAX/2,2^63,2^63+1, random×64} (axis I) | `cfg_hash_bytes_seed_matrix` | [x] |
| C3 | `stbds_hash_bytes` | buffers of all-`0x00`, all-`0xff`, high-bit-only, `0x80` at every position (axis K sign-extension) | `cfg_hash_bytes_high_bit_patterns` | [x] |
| C4 | `stbds_hash_string` | random NUL-terminated strings len 0..64, bytes `0x01..0x7f`, seed matrix (axes E,I) | `cfg_hash_string_ascii` | [x] |
| C5 | `stbds_hash_string` | strings containing bytes `0x80..0xff` (`(unsigned char)*str` path), len 1..64 | `cfg_hash_string_high_bit` | [x] |
| C6 | `stbds_rand_seed` + `stbds_shmode_func` | `rand_seed(s)` for s ∈ {0,1,0x31415926,SIZE_MAX,random×32} then create 4 indices → observe `table->seed` and the seed advancement chain (axis I) | `cfg_rand_seed_and_advance` | [x] |
| C7 | `stbds_arrgrowf` | `a = NULL`, elemsize ∈ {1,2,4,8,16,24,32,64,4096}, addlen ∈ {0,1,2,7,64}, min_cap ∈ {0,1,2,3,4,5,8,100} (full cross product) | `cfg_arrgrowf_fresh_matrix` | [x] |
| C8 | `stbds_arrgrowf` | existing array, repeated growth 0→N (doubling path `min_cap < 2*cap`), length preserved, capacity/temp/hash_table tracked | `cfg_arrgrowf_growth_chain` | [x] |
| C9 | `stbds_arrgrowf`+`stbds_arrfreef` | grow, write payload, grow again, free — payload survives realloc identically | `cfg_arrgrowf_payload_survives` | [x] |
| C10 | `stbds_hmput_key` | mode = 0 (BINARY), implicit creation (D), shapes (8,4)/(16,4)/(16,8)/(4,4)/(24,16)/(32,32), counts 1..64 distinct random keys | `cfg_binary_insert_matrix` | [x] |
| C11 | `stbds_hmput_key` | mode = 1 (STRING) implicit creation ⇒ `string.mode = SH_DEFAULT`, distinct random strings, counts 1..64 | `cfg_string_default_insert` | [x] |
| C12 | `stbds_hmput_key` | mode = 0, **duplicate** keys (re-put) → `temp` = existing index, no growth (axis G) | `cfg_binary_duplicate_keys` | [x] |
| C13 | `stbds_hmput_key` | mode = 1, duplicate keys → `temp` **and** `table->temp_key` updated to the *stored* pointer (L732-733) | `cfg_string_duplicate_keys_temp_key` | [x] |
| C14 | `stbds_hmput_key` | count crossing **every** growth threshold: 6, 12, 24, 48, 96, 192 inserts (axis F) — `slot_count`, thresholds, full rehashed bucket array | `cfg_growth_thresholds` | [x] |
| C15 | `stbds_hmput_key` | keysize ∈ 0..17 with elemsize = 32 (siphash tail widths × `memcmp` widths) | `cfg_keysize_matrix` | [x] |
| C16 | `stbds_hmget_key` | mode 0/1, hits and misses interleaved, on maps of size 0,1,2,6,7,64 | `cfg_get_hit_miss_matrix` | [x] |
| C17 | `stbds_hmget_key_ts` | same as C16 but through the `_ts` entry point (caller-supplied `temp`), asserting `*temp` **and** that `hdr.temp` is *not* touched | `cfg_get_ts_matrix` | [x] |
| C18 | `stbds_hmget_key`/`_ts` | first call on a **fresh** `NULL` map (creation path D) then a real get | `cfg_get_creates_map` | [x] |
| C19 | `stbds_hmput_default` | on NULL map, on a map made by `hmput_key`, on a map made by `shmode_func`, then put/get (writes elem `[-1]`) | `cfg_hmput_default_paths` | [x] |
| C20 | `stbds_shmode_func` | mode ∈ {SH_NONE 0, SH_DEFAULT 1, SH_STRDUP 2, SH_ARENA 3}, elemsize ∈ {16,24,32} → then string puts (axis C) | `cfg_shmode_all_modes` | [x] |
| C21 | `stbds_shmode_func`(SH_STRDUP) + `hmput_key`(mode 1) | 1..64 random strings, keys are `strdup`ed → compare key *contents*; then `hmdel_key` frees them | `cfg_strdup_mode_lifecycle` | [x] |
| C22 | `stbds_shmode_func`(SH_ARENA) + `hmput_key`(mode 1) | 1..64 strings incl. > 512 bytes → arena block chain (`string.block`, `string.remaining`) evolves identically (axes C,J) | `cfg_arena_mode_lifecycle` | [x] |
| C23 | `stbds_shmode_func`(SH_NONE) + `hmput_key`(mode 1) | distinct keys only ⇒ switch `default:` branch `memcpy`s the *string bytes* into the element (axis C quirk) | `cfg_sh_none_copies_bytes` | [x] |
| C24 | `stbds_hmdel_key` | mode 0, delete **last** element (`old_index == final_index`) | `cfg_del_last` | [x] |
| C25 | `stbds_hmdel_key` | mode 0, delete **first**/middle element ⇒ `memmove` of the tail element + slot re-point | `cfg_del_first_and_middle` | [x] |
| C26 | `stbds_hmdel_key` | mode 0, delete **all** elements one by one, in insertion order and in reverse | `cfg_del_all_orders` | [x] |
| C27 | `stbds_hmdel_key` | mode 0, 2 deletes on an 8-slot index ⇒ tombstone rebuild (`tombstone_count_threshold == 1`) (axis H) | `cfg_del_tombstone_rebuild` | [x] |
| C28 | `stbds_hmdel_key` | mode 0, 24 inserts (⇒ 32 slots) then delete down below `used_count_shrink_threshold` ⇒ shrink chain 32→16→8 | `cfg_del_shrink_chain` | [x] |
| C29 | `stbds_hmdel_key` | mode 1 + `SH_DEFAULT`, delete/re-insert cycles (tombstone reuse) | `cfg_string_del_reinsert` | [x] |
| C30 | `stbds_hmdel_key` | mode 1 + `SH_STRDUP` (frees the key) vs `SH_ARENA` (keeps arena) vs `SH_DEFAULT` | `cfg_del_across_string_modes` | [x] |
| C31 | `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` | **randomized op sequence** (property test, 8 seeds × 3 key-pool sizes × 600 ops): put 55% / get 15% / get_ts 7% / del 23%, key pools of 4 (⇒ many dups + tombstones), 17 and 400 (⇒ repeated growth), mode 0, shape (16,8); the full state is compared after **every** op | `cfg_random_ops_binary`, `cfg_random_ops_binary_small` | [x] |
| C32 | `hmput_key`/`hmget_key`/`hmdel_key` | same randomized driver, mode 1 + `SH_STRDUP`, key strings from a narrow pool | `cfg_random_ops_string_strdup` | [x] |
| C33 | `hmput_key`/`hmget_key`/`hmdel_key` | same randomized driver, mode 1 + `SH_ARENA` | `cfg_random_ops_string_arena` | [x] |
| C34 | full map lifecycle + `stbds_hmfree_func` | 1000 keys, mode 0 and mode 1×{DEFAULT,STRDUP,ARENA}: insert-all, get-all, delete-all, then free (no leak/double-free crash) | `cfg_large_map_lifecycle` | [x] |
| C35 | `stbds_stralloc` | fresh arena, strings of len 0,1,7,8,63,64,255,510,511,512,513 in sequence → `remaining`/`block`/offset-within-block (axis J) | `cfg_stralloc_length_matrix` | [x] |
| C36 | `stbds_stralloc` | > blocksize strings (dedicated-block path) mixed with small ones, on empty and populated arenas | `cfg_stralloc_oversize_mix` | [x] |
| C37 | `stbds_stralloc` | 4000 random small strings ⇒ `block` grows 0→22 and saturates at `1<<20` | `cfg_stralloc_block_saturation` | [x] |
| C38 | `stbds_stralloc`+`stbds_strreset` | alloc chain, reset (all fields zeroed), re-alloc after reset | `cfg_strreset_and_reuse` | [x] |
| C39 | `strkey` | n ∈ {0,±1,±9,±10,99,100,±12345,INT_MAX,INT_MIN,INT_MAX-1,INT_MIN+1, random×256} | `cfg_strkey_matrix` | [x] |
| C40 | `helxo` | letter ∈ {'A','z','0',' ',0x7f, 0, -1, -128, 127, 200-as-char, '\n', '%', '\t', random×64}; stdout captured in a re-executed child process and compared byte-for-byte (composed pipeline: 6 × `shput` + `shlen` iteration + struct-by-value `printf` + `shfree`) | `cfg_helxo_letters`, `cfg_helxo_seed_independent` | [x] |
| C41 | `helxo` | 8 calls in one process from 5 different start seeds → each call advances the private `stbds_hash_seed`; the streams must stay identical | `cfg_helxo_repeated` | [x] |
| C42 | `stbds_hmput_key` | mode 0 with keys that are **all equal** (single slot, `used_count` stays 1) | `cfg_all_equal_keys` | [x] |
| C43 | `stbds_hmput_key` | mode 0 with keysize = 8 and keys `0x00..00` / `0xff..ff` / `0x8000000000000000` (hash-value extremes, `hash < 2` fixup path) | `cfg_extreme_key_values` | [x] |
| C44 | `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` | mode ∈ {2, 3, 4, 44, 1000, INT_MAX} (out-of-enum "string") and {-1, -2, -1000, INT_MIN} (binary) on a fresh map | `err_mode_matrix_binary`, `err_mode_out_of_range_string_side`, `err_mode_negative_is_binary`, `err_hmput_key_mode_selects_string_mode` (`tests/errors_diff.rs`) | [x] |
| C45 | `stbds_hmfree_func` | on a map with `hash_table == NULL` (created via `hmget_key`), on a `SH_STRDUP` map, on a `SH_ARENA` map, on `NULL` | `cfg_hmfree_all_shapes` | [x] |
