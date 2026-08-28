# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes.  The
axes below are the *runtime options* and *input shapes* the C code
distinguishes; the table is their (pruned) cross-product.

## Axes

| axis | values the C code distinguishes | where it branches |
|------|--------------------------------|-------------------|
| `mode` (int, not a real enum) | `INT_MIN`, `-1`, `0` (`STBDS_HM_BINARY`), `1` (`STBDS_HM_STRING`), `2`, `INT_MAX` | `mode >= STBDS_HM_STRING` (`lib.c:560,590,713`), `mode == STBDS_HM_STRING` (`lib.c:836,842`) |
| `table->string.mode` | `0` `SH_NONE`, `1` `SH_DEFAULT`, `2` `SH_STRDUP`, `3` `SH_ARENA`, out-of-range (`4`,`255`) | `switch` (`lib.c:785`), `== SH_STRDUP` (`lib.c:575,836`) |
| how the table is created | implicitly by `stbds_hmput_key` (table==NULL) vs explicitly by `stbds_shmode_func` | `lib.c:698`, `lib.c:796` |
| `elemsize` | `8`, `12`, `16` (the `sh_geti` layout), `24`, `32`, `40`, `64` | all pointer arithmetic |
| `keysize` | `0`, `1`, `2`, `3`, `4`, `8`, `16`, `== elemsize` | `memcmp`/`memcpy` (`lib.c:563,789`) |
| `keyoffset` (only `stbds_hmdel_key` exposes it) | `0`, `4`, `8` | `lib.c:561,563,837,843,845` |
| element count | `0`, `1`, `2`, `5`, `6` (= `used_count_threshold` for 8 slots), `7` (first grow), `8`, `12`, `13`, `24`, `50`, `200`, `1000` | `used_count >= used_count_threshold` (`lib.c:698`) |
| `slot_count` reached | `8`, `16`, `32`, `64`, `128`, `256`, `1024`, `2048` | `slot_count*2` (`lib.c:702`), `>>1` (`lib.c:855`) |
| delete shape | last element (`old_index == final_index`), interior (swap-with-last), absent key, delete-all, delete-then-reinsert (tombstone reuse), delete enough to shrink, delete enough to rebuild | `lib.c:839,854,858` |
| `stbds_hmput_default` position | before any put, after puts, twice in a row, on `length==0` array | `lib.c:669` |
| `stbds_arrgrowf` shape | `a` NULL / non-NULL × `addlen` `0`/`1`/`n` × `min_cap` `0`/`1`/`<cap`/`==cap`/`cap+1`/`2*cap`/`huge` | `lib.c:283,286,289,291` |
| hash-byte length | `0`..`80` (covers `switch` cases 0–7 **and** ≥1 full 8-byte blocks), incl. `8`,`16`,`64` | `lib.c:522,532` |
| hash-byte content | all-zero, all-`0xFF`, high-bit-set top byte (sign-extension quirk), random | `lib.c:523,524,536` |
| seed | `0`, `1`, `0x31415926` (default), `usize::MAX`, random | `stbds_rand_seed` |
| string length | `0`, `1`, `7`, `8`, `9`, `15`, `16`, `63`, `64`, `255`, `510`, `511`, `512`, `513`, `1024`, `4096` | `len > a->remaining`, `len > blocksize` (`lib.c:885,893`) |
| string content | ASCII, bytes ≥ `0x80`, embedded `0x7f`, repeated | `(unsigned char) *str++` (`lib.c:481`) |
| arena state | fresh (`{0}`), after `strreset`, mid-block, exactly-exhausted, oversized-block-first, oversized-block-after, `block` saturated at 22 | `lib.c:885-911` |
| `sh_geti(num)` | `INT_MIN`, `-1`, `0`, `1`, `2`, `3`, `4`, `5`, `7`, `8`, `9`, `13`, `16`, `17`, `31`, `32`, `33`, `63`, `64`, `100`, `127`, `128`, `200`, `256`, `500`, `1000` | every loop bound in `sh_geti` |

Every row is driven with **many randomized inputs** (fixed seed
`0x5DEECE66D`-style LCG in `tests/common/mod.rs`, so runs are reproducible), not
a single hand-picked value.

`[x]` = row passes byte-identically against both `.so`s.

## Rows

### `stbds_hash_bytes` (lowest level, no state)

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p = NULL`; seeds `{0,1,default,MAX,random×64}` | `hash_fns.rs::c01_hash_bytes_len_zero` | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (every `switch` tail case) × random bytes × random seeds | `hash_fns.rs::c02_hash_bytes_tail_cases` | [x] |
| 3 | `stbds_hash_bytes` | `len = 1..7` with top byte `>= 0x80` (forces the `int` sign-extension in `case 4`) | `hash_fns.rs::c03_hash_bytes_tail_sign_extension` | [x] |
| 4 | `stbds_hash_bytes` | `len = 8,16,24,32,64,80` (whole blocks, no tail) × random content | `hash_fns.rs::c04_hash_bytes_whole_blocks` | [x] |
| 5 | `stbds_hash_bytes` | `len = 9..79` (blocks **plus** tail) × random content × random seeds | `hash_fns.rs::c05_hash_bytes_blocks_plus_tail` | [x] |
| 6 | `stbds_hash_bytes` | content all-`0x00` and all-`0xFF` for `len = 0..40` | `hash_fns.rs::c06_hash_bytes_extreme_content` | [x] |
| 7 | `stbds_hash_bytes` | unaligned buffer start (`p = buf+1 .. buf+7`) | `hash_fns.rs::c07_hash_bytes_unaligned` | [x] |
| 8 | `stbds_hash_bytes` | `len` much larger than the buffer is *not* tested (OOB read); instead `len = 4096` with a 4096-byte random buffer | `hash_fns.rs::c08_hash_bytes_large` | [x] |

### `stbds_hash_string`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 9 | `stbds_hash_string` | `""` (empty) × seeds `{0,1,default,MAX,random×64}` | `hash_fns.rs::c09_hash_string_empty` | [x] |
| 10 | `stbds_hash_string` | lengths `1..64` of random printable ASCII × random seeds | `hash_fns.rs::c10_hash_string_ascii` | [x] |
| 11 | `stbds_hash_string` | lengths `1..64` of random bytes in `0x80..=0xFF` (unsigned-char promotion) | `hash_fns.rs::c11_hash_string_high_bytes` | [x] |
| 12 | `stbds_hash_string` | long strings (`255`, `1024`, `4096` bytes) | `hash_fns.rs::c12_hash_string_long` | [x] |
| 13 | `stbds_hash_string` | `"test_%d"`-shaped keys (`strkey` outputs) for `n = 0..2000` | `hash_fns.rs::c13_hash_string_strkey_shapes` | [x] |

### `stbds_rand_seed` + seed evolution

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 14 | `stbds_rand_seed` → `stbds_shmode_func` | seeds `{0,1,default,MAX,random×32}`; check the *table* seed and the evolved global seed after 1,2,3,…,16 table creations | `hashmap.rs::c14_table_seed_and_evolution` | [x] |
| 15 | `stbds_rand_seed` | seed reset mid-sequence; verify `stbds_hash_seed = seed*a + b` LCG step matches | `hashmap.rs::c15_reseed_mid_sequence` | [x] |

### `strkey`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 16 | `strkey` | `n ∈ {0,1,-1,9,10,99,100,999,1000,INT_MAX,INT_MIN}` + 256 random `i32` | `hash_fns.rs::c16_strkey_values` | [x] |
| 17 | `strkey` | repeated calls reuse the same static buffer (pointer stability + overwrite semantics) | `hash_fns.rs::c17_strkey_static_buffer_semantics` | [x] |

### `stbds_arrgrowf` / `stbds_arrfreef`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 18 | `stbds_arrgrowf` | `a=NULL` × `elemsize ∈ {1,2,4,8,12,16,24,32,64}` × `addlen ∈ {0,1,2,7,100}` × `min_cap ∈ {0,1,2,4,5,100}` (full cross product).  Note `addlen==0 && min_cap==0` returns **NULL** (see ERRORS.md #4). | `arrays.rs::c18_arrgrowf_from_null_cross_product` | [x] |
| 19 | `stbds_arrgrowf` | non-NULL `a`, repeated growth (`addlen=1`, `min_cap=0`) 64 times → doubling sequence 4,8,16,… | `arrays.rs::c19_arrgrowf_repeated_doubling` | [x] |
| 20 | `stbds_arrgrowf` | non-NULL `a`, `min_cap` `<cap`, `==cap`, `cap+1`, `2*cap`, `2*cap+1`, `4*cap` (the `min_cap < 2*cap` branch) | `arrays.rs::c20_arrgrowf_min_cap_branches` | [x] |
| 21 | `stbds_arrgrowf` | `elemsize=0` (header-only allocation) × `min_cap ∈ {0,1,4,100}` | `arrays.rs::c21_arrgrowf_zero_elemsize` | [x] |
| 22 | `stbds_arrgrowf` | data preservation: fill `length` elements with a random pattern, grow, verify bytes survive | `arrays.rs::c22_arrgrowf_preserves_payload` | [x] |
| 23 | `stbds_arrgrowf` + `stbds_arrfreef` | grow → write → grow → free round-trip, 256 randomized sequences | `arrays.rs::c23_arrgrowf_randomized_sequences` | [x] |
| 24 | `stbds_arrgrowf` | `addlen` large enough that `min_len > min_cap` dominates (`min_cap=0, addlen=1000`) | `arrays.rs::c24_arrgrowf_addlen_dominates` | [x] |

### `stbds_stralloc` / `stbds_strreset`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 25 | `stbds_stralloc` | fresh arena `{0}` + one string, lengths `0,1,7,8,63,255,510,511` (fits the first 512-byte block) | `arena.rs::c25_stralloc_fresh_small` | [x] |
| 26 | `stbds_stralloc` | fresh arena + one string with `len > 512` (`512,513,1024,4096`) → oversized-block path with `storage == NULL` | `arena.rs::c26_stralloc_oversized_first` | [x] |
| 27 | `stbds_stralloc` | many small strings until several blocks are chained (`block` 0→8), verify each returned string and the arena fields after every call | `arena.rs::c27_stralloc_many_small` | [x] |
| 28 | `stbds_stralloc` | mixed small/oversized strings (oversized-after-existing-storage path, `remaining` untouched) | `arena.rs::c28_stralloc_mixed_oversized_after` | [x] |
| 29 | `stbds_stralloc` | exactly-exhausting a block (`len == remaining`), then one more byte | `arena.rs::c29_stralloc_exact_exhaustion` | [x] |
| 30 | `stbds_stralloc` | drive `a->block` to saturation (blocksize clamps at `1<<20`) using 4096-byte strings | `arena.rs::c30_stralloc_block_saturation` | [x] |
| 31 | `stbds_stralloc` | strings containing bytes `>= 0x80` and `0x7f` (content fidelity) | `arena.rs::c31_stralloc_high_bytes` | [x] |
| 32 | `stbds_strreset` | on `{0}`, on 1 block, on N blocks, on a chain with an oversized block; then reuse the arena | `arena.rs::c32_strreset_shapes` | [x] |
| 33 | `stbds_stralloc` | 512 randomized (length, content) sequences on one arena, comparing every returned string and every arena field | `arena.rs::c33_stralloc_randomized` | [x] |

### `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmget_key_ts` — BINARY mode

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 34 | `stbds_hmput_key` | `mode=0`, implicit table, `elemsize=8, keysize=4`, `n ∈ {1,2,5,6,7,8}` random keys (crosses the first grow) | `hashmap.rs::c34_binary_small_counts` | [x] |
| 35 | `stbds_hmput_key` | `mode=0`, `elemsize=16, keysize=8`, `n = 200` random keys → `slot_count` 8→256 | `hashmap.rs::c35_binary_two_hundred` | [x] |
| 36 | `stbds_hmput_key` | `mode=0`, `elemsize ∈ {8,12,16,24,32,40,64}` × `keysize ∈ {1,2,3,4,8,16}` (keysize ≤ elemsize), `n = 40` random keys | `hashmap.rs::c36_binary_elemsize_keysize_cross_product` | [x] |
| 37 | `stbds_hmput_key` | `mode=0`, duplicate keys interleaved with new keys (find-or-insert path) | `hashmap.rs::c37_binary_duplicates` | [x] |
| 38 | `stbds_hmput_key` | `mode=0`, `keysize=0` (degenerate all-keys-equal) | `hashmap.rs::c38_binary_zero_keysize`, `errors.rs::e25_zero_keysize_binary` | [x] |
| 39 | `stbds_hmput_key` | `mode ∈ {INT_MIN,-1}` (negative ⇒ binary branch) with `elemsize=16,keysize=8` | `hashmap.rs::c39_negative_modes_are_binary` | [x] |
| 40 | `stbds_hmget_key` | `mode=0`, lookups of present *and* absent keys after every insert | `hashmap.rs::c40_c41_get_present_and_absent` | [x] |
| 41 | `stbds_hmget_key_ts` | `mode=0`, same as #40 but the index comes back through `*temp`; also confirm `header->temp` is **not** written | `hashmap.rs::c40_c41_get_present_and_absent` | [x] |
| 42 | `stbds_hmget_key` / `_ts` | `mode=0` on a map created by `stbds_arrgrowf` only (`hash_table == NULL`) | `hashmap.rs::c42_get_on_table_less_map` | [x] |
| 43 | `stbds_hmput_key` | `mode=0`, `keysize == elemsize` (whole element is the key) | `hashmap.rs::c43_keysize_equals_elemsize` | [x] |

### `stbds_hmput_key` / `stbds_hmget_key` — STRING modes

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 44 | `stbds_hmput_key` | `mode=1`, implicit table ⇒ `string.mode = SH_DEFAULT`; `elemsize=16,keysize=8`; `n ∈ {1,6,7,50}` random strings | `hashmap.rs::c44_string_implicit_default_mode` | [x] |
| 45 | `stbds_shmode_func`+`stbds_hmput_key` | `string.mode = SH_STRDUP (2)`, `mode=1`, `n = 100` random strings (keys are `strdup`ed) | `hashmap.rs::c45_string_strdup_mode` | [x] |
| 46 | `stbds_shmode_func`+`stbds_hmput_key` | `string.mode = SH_ARENA (3)`, `mode=1`, `n = 100` random strings (keys go through `stbds_stralloc`, incl. oversized) | `hashmap.rs::c46_string_arena_mode` | [x] |
| 47 | `stbds_shmode_func`+`stbds_hmput_key` | `string.mode = SH_DEFAULT (1)`, `mode=1` — key pointers stored verbatim | `hashmap.rs::c47_string_default_mode_explicit` | [x] |
| 48 | `stbds_shmode_func`+`stbds_hmput_key` | `string.mode = SH_NONE (0)` but `mode ∈ {1,2,9,INT_MAX}` — hashing is by string, storage is `memcpy` of `keysize` bytes (mismatched pair the C allows).  **Insert-only**: a look-up would reinterpret the raw key bytes as a `char *`, which is UB in the C original. | `hashmap.rs::c48_c49_string_hash_with_memcpy_storage` | [x] |
| 49 | `stbds_shmode_func`+`stbds_hmput_key` | `string.mode ∈ {4,200,255}` (out-of-range) ⇒ `default:` memcpy branch (insert-only, same UB caveat as #48) | `hashmap.rs::c48_c49_string_hash_with_memcpy_storage`, `fuzz.rs::c77b_string_memcpy_storage_sequences`, `errors.rs::e24_put_default_switch_branch`/`e26` | [x] |
| 50 | `stbds_hmput_key` | `mode ∈ {2, INT_MAX}` (out-of-range but `>= STBDS_HM_STRING`) with each `string.mode` | `hashmap.rs::c50_mode_two_behaves_like_string` | [x] |
| 51 | `stbds_hmput_key` | `mode=1`, duplicate string keys (`temp_key` written on the upper-half hit) | `hashmap.rs::c51_string_duplicate_keys` | [x] |
| 52 | `stbds_hmput_key` | `mode=1`, keys that are prefixes of one another and keys differing only in the last byte | `hashmap.rs::c52_string_prefix_and_last_byte_keys` | [x] |
| 53 | `stbds_hmput_key` | `mode=1`, `strkey(i)` keys for `i = 0..500` — the exact key shape `sh_geti` uses | `hashmap.rs::c53_string_strkey_shaped_keys` | [x] |
| 54 | `stbds_hmput_key` | `mode=1`, empty-string key `""` | `hashmap.rs::c54_string_empty_key` | [x] |
| 55 | `stbds_hmput_key` | `mode=1`, `elemsize ∈ {8,16,24,32,64}` (key pointer at offset 0, value area varies) | `hashmap.rs::c55_string_elemsize_variants` | [x] |

### `stbds_hmput_default`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 56 | `stbds_hmput_default` | `a = NULL`, `elemsize ∈ {8,12,16,24,32,64}` | `hashmap.rs::c56_c57_put_default_null_and_twice` | [x] |
| 57 | `stbds_hmput_default` | called twice in a row (second is a no-op) | `hashmap.rs::c56_c57_put_default_null_and_twice` | [x] |
| 58 | `stbds_hmput_default` | after `n` puts (no-op, `length != 0`), both binary and string modes | `hashmap.rs::c58_put_default_after_puts` | [x] |
| 59 | `stbds_hmput_default` | on an array produced by `stbds_arrgrowf` with `length == 0` | `hashmap.rs::c59_put_default_on_len0_array` | [x] |
| 60 | `stbds_hmput_default` → `stbds_hmget_key` | default element value returned for absent keys (the `t[-1]` idiom) | `hashmap.rs::c60_default_element_is_returned_for_misses` | [x] |

### `stbds_hmdel_key`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 61 | `stbds_hmdel_key` | `mode=0`, delete the **last** element (`old_index == final_index`, no memmove) | `hashmap.rs::c61_delete_last_element` | [x] |
| 62 | `stbds_hmdel_key` | `mode=0`, delete an **interior** element (swap-with-last + slot re-point) | `hashmap.rs::c62_delete_interior_element` | [x] |
| 63 | `stbds_hmdel_key` | `mode=0`, delete **all** keys in insertion order / reverse / random order, `n ∈ {1,2,8,50,200}` | `hashmap.rs::c63_delete_all_orders` | [x] |
| 64 | `stbds_hmdel_key` | `mode=0`, delete-then-reinsert (tombstone reuse), 256 randomized op sequences | `hashmap.rs::c64_randomized_delete_reinsert` | [x] |
| 65 | `stbds_hmdel_key` | `mode=0`, enough deletes at `slot_count ≥ 16` to trigger the **shrink** rebuild | `hashmap.rs::c65_delete_triggers_shrink` | [x] |
| 66 | `stbds_hmdel_key` | `mode=0`, enough tombstones (delete+reinsert churn) to trigger the **tombstone rebuild** at constant `slot_count` | `hashmap.rs::c66_delete_triggers_tombstone_rebuild` | [x] |
| 67 | `stbds_hmdel_key` | `mode=1`, `string.mode=SH_STRDUP` (key `free`d) — delete-all then reinsert | `hashmap.rs::c67_c68_c69_delete_string_modes` | [x] |
| 68 | `stbds_hmdel_key` | `mode=1`, `string.mode=SH_ARENA` (key left in the arena) | `hashmap.rs::c67_c68_c69_delete_string_modes` | [x] |
| 69 | `stbds_hmdel_key` | `mode=1`, `string.mode=SH_DEFAULT` | `hashmap.rs::c67_c68_c69_delete_string_modes` | [x] |
| 70 | `stbds_hmdel_key` | `mode ∈ {2,7,INT_MAX}` on a `SH_STRDUP` map: skips the `free` (`lib.c:836`) **and** takes the wrong re-lookup branch (`lib.c:842`).  Deleting last-to-first (`old_index == final_index`) completes; any swap-with-last makes `STBDS_ASSERT(slot >= 0)` fire in BOTH implementations (verified in a subprocess). | `hashmap.rs::c70_delete_mode_two_skips_the_free`, `errors.rs::e33_hmdel_mode_two_no_free` | [x] |
| 71 | `stbds_hmdel_key` | `keyoffset ∈ {0,4,8}` with `elemsize=24, keysize=8`, binary mode | `hashmap.rs::c71_delete_keyoffset_variants` | [x] |
| 72 | `stbds_hmdel_key` | absent key / empty map / no-table map | `hashmap.rs::c72_delete_edge_cases`, `errors.rs::e27_hmdel_null_map`/`e28`/`e29` | [x] |

### `stbds_hmfree_func`

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 73 | `stbds_hmfree_func` | `string.mode ∈ {SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 4, 255}` × `length ∈ {1, 2, 8, 100}` | `hashmap.rs::c73_hmfree_all_string_modes` | [x] |
| 74 | `stbds_hmfree_func` | array with `hash_table == NULL` | `hashmap.rs::c74_c75_hmfree_null_and_table_less`, `errors.rs::e07_hmfree_no_table` | [x] |
| 75 | `stbds_hmfree_func` | `a == NULL` | `hashmap.rs::c74_c75_hmfree_null_and_table_less`, `errors.rs::e06_hmfree_null` | [x] |

### Full randomized op-sequence fuzz (all axes at once)

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 76 | `stbds_hmput_key` + `stbds_hmget_key` + `stbds_hmget_key_ts` + `stbds_hmdel_key` + `stbds_hmput_default` + `stbds_hmfree_func` | 3000 randomized sequences of 200 ops each, `mode=0`, random `elemsize`/`keysize`, comparing the **entire** state (header, every element, hash index fields, every bucket `hash[]`/`index[]`) after **every** op | `fuzz.rs::c76_binary_op_sequences` | [x] |
| 77 | same, string modes | 3000 randomized sequences (120 ops each) × `string.mode ∈ {DEFAULT,STRDUP,ARENA}` with `mode = STBDS_HM_STRING`, random string keys (duplicates, `""`, ≥0x80 bytes, >512 bytes to hit the arena's oversized-block path).  `mode ∈ {2,INT_MAX}` is covered separately by rows 50 and 70 because a swap-delete with `mode != 1` aborts. | `fuzz.rs::c77_string_op_sequences` | [x] |
| 78 | `stbds_arrgrowf` + `stbds_arrfreef` | 3000 randomized grow sequences (40 steps each) with random `elemsize`, comparing header + payload after every step | `fuzz.rs::c78_array_op_sequences` | [x] |
| 79 | `stbds_stralloc` + `stbds_strreset` | 3000 randomized arena sequences (30 steps each), comparing every returned string and every arena field | `fuzz.rs::c79_arena_op_sequences` | [x] |

### `sh_geti` — the top-level driver (stdout compared byte-for-byte)

| # | entry point(s) | configuration | test | [ ] |
|---|----------------|---------------|------|-----|
| 80 | `sh_geti` | `num ∈ {0,1,2,3,4,5,6,7,8,9}` | `sh_geti.rs::c80_sh_geti_small` | [x] |
| 81 | `sh_geti` | `num ∈ {10..40}` (crosses the 8→16→32 table growth and the arena's first block) | `sh_geti.rs::c81_sh_geti_mid` | [x] |
| 82 | `sh_geti` | `num ∈ {63,64,65,127,128,129}` (power-of-two boundaries) | `sh_geti.rs::c82_sh_geti_power_of_two_boundaries` | [x] |
| 83 | `sh_geti` | `num ∈ {200,255,256,257,500,1000,2000,2048,4096}` (multi-block arena, `slot_count` up to 4096); the exact canonical text is asserted as well | `sh_geti.rs::c83_sh_geti_large` | [x] |
| 84 | `sh_geti` | `num ∈ {-1,-2,-1000,INT_MIN}` (all loops skipped) | `sh_geti.rs::c84_sh_geti_non_positive`, `errors.rs::e52_sh_geti_non_positive` | [x] |
| 85 | `sh_geti` | repeated calls in one process (static `buffer`, global `stbds_hash_seed` evolution) — 30 calls with a random `num` each, stdout compared cumulatively | `sh_geti.rs::c85_sh_geti_repeated_calls_share_globals` | [x] |
| 86 | `sh_geti` | after `stbds_rand_seed(s)` for `s ∈ {0,1,2,MAX,DEFAULT,random×8}` — the seed changes the bucket layout but **not** the printed order (the print loop walks the array, not the table), so the output must be *identical* for every seed and equal to the canonical `test_i i*3` text | `sh_geti.rs::c86_sh_geti_seed_dependence`, `sh_geti.rs::c80b_sh_geti_exact_output` | [x] |
