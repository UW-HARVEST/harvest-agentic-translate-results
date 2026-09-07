# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the branches in `c_src/src/lib.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `mode` (hash/compare mode) | `mode >= STBDS_HM_STRING(1)` ⇒ `strcmp`+`stbds_hash_string`; else `memcmp`+`stbds_hash_bytes`. Also `mode == STBDS_HM_STRING` **exactly** in `hmdel_key`'s strdup-free. | L560, L590, L713, L836 |
| `table->string.mode` (key ownership) | `STBDS_SH_NONE(0)`/other ⇒ `memcpy`; `STBDS_SH_DEFAULT(1)` ⇒ store caller pointer; `STBDS_SH_STRDUP(2)` ⇒ `strdup`; `STBDS_SH_ARENA(3)` ⇒ `stralloc` | L785–790 |
| how the map is created | (a) implicitly by `hmput_key(NULL,…)`; (b) by `hmput_default(NULL,…)` (**no** hash table); (c) by `shmode_func(elemsize, mode)` (table pre-made, chosen string mode) | L686, L669, L796 |
| `elemsize` / `keysize` | key widths 1,2,3,4,5,7,8,9,16 bytes (drives the `siphash` main-loop vs. `switch(len-i)` tail cases 0..7) and elemsize with/without padding | L522–541 |
| table growth | `table == NULL` ⇒ 8 slots; `used_count >= used_count_threshold` ⇒ `slot_count*2` and rehash of every in-use entry | L698–710, L426–469 |
| table shrink / rebuild | `used_count < used_count_shrink_threshold && slot_count > 8` ⇒ `slot_count>>1`; else `tombstone_count > tombstone_count_threshold` ⇒ same size rebuild | L854–862 |
| probe path | slot found in the `i = pos&MASK .. BUCKET_LENGTH` loop vs. the wrap-around `i = 0 .. limit` loop vs. quadratic re-probe (`pos += step; step += 8`) | L604–628, L728–764 |
| tombstone reuse on insert | `tombstone >= 0` at `found_empty_slot` ⇒ reuse slot, `--tombstone_count` | L766–769 |
| delete position | `old_index == final_index` (deleting the last element) vs. `!=` (memmove + re-find + re-index) | L839–851 |
| `arrgrowf` growth branch | `min_cap <= cap` (no-op) / `min_cap < 2*cap` / `min_cap < 4` / plain; `a == NULL` (init header) vs. `a != NULL` (preserve header) | L283–307 |
| `stbds_hash_seed` global | advances on every `make_hash_index(_, NULL)`; `stbds_rand_seed(s)` resets it → every hash & slot layout depends on it | L353–358, L410–412 |
| arena block growth | `len <= remaining` (bump) / `len > remaining && len <= blocksize` (new 512<<(block>>1) block) / `len > blocksize` (dedicated block, empty-vs-nonempty arena splice) | L881–918 |
| `sh_geti(num)` | `num` = 0,1,2,3,4,5,8,9,16,17,32,63,64,100,257 — drives grow/shrink/rebuild + both `j` iterations (STRDUP then ARENA) and the stdout `printf` | L945–986 |

## Rows (each = one differential test, randomized inputs, fixed seed)

| #  | entry point(s) | configuration (options set + input shape) | test | ✔ |
|----|----------------|-------------------------------------------|------|---|
| 1  | `stbds_rand_seed` + `stbds_hash_bytes` | seeds {0, 1, 0x31415926, SIZE_MAX, random}; `len` ∈ 0..64 (covers main loop 0/1/2/…×8 and every `switch(len-i)` tail case 0..7); random bytes incl. ≥0x80 | `cfg_01_hash_bytes_all_lengths` | [x] |
| 2  | `stbds_hash_bytes` | 512 random (len, seed, buffer) triples, len 0..256 | `cfg_02_hash_bytes_random` | [x] |
| 3  | `stbds_hash_string` | strings of length 0..64, ASCII + bytes 0x80..0xFF, random seeds | `cfg_03_hash_string_random` | [x] |
| 4  | `stbds_rand_seed` | set seed then create a table (`shmode_func`) twice; assert the *advanced* global seed produces the same second table seed | `cfg_04_rand_seed_advance` | [x] |
| 5  | `stbds_arrgrowf` | `a == NULL`, cross product of `elemsize` ∈ {1,4,8,12,16,24}, `addlen` ∈ {0,1,2,7,100}, `min_cap` ∈ {0,1,3,4,5,64} | `cfg_05_arrgrowf_fresh` | [x] |
| 6  | `stbds_arrgrowf` | grow an existing array repeatedly (`addlen=1, min_cap=0`) 200× — exercises `min_cap < 2*cap` doubling; compare `length`/`capacity` each step | `cfg_06_arrgrowf_regrow` | [x] |
| 7  | `stbds_arrgrowf` + `stbds_arrfreef` | build array, write payload, grow, verify payload preserved, then free | `cfg_07_arrgrowf_payload_arrfree` | [x] |
| 8  | `stbds_hmput_key`/`stbds_hmget_key`/`stbds_hmget_key_ts` | **binary** mode (`mode=0`), keysize ∈ {1,2,3,4,5,7,8,9,16}, elemsize = keysize+4 (no padding, so every element byte is deterministic), N ∈ {1,2,7,8,9,50,300} random keys; full insert-then-lookup-all | `cfg_08_binary_map_widths` | [x] |
| 9  | same | binary mode, N=300 keys ⇒ forces table growth 8→16→…→512 and rehashing | `cfg_09_binary_map_growth` | [x] |
| 10 | same | binary mode with **duplicate** keys interleaved (replace path, `temp` = existing index, length unchanged) | `cfg_10_binary_map_duplicates` | [x] |
| 11 | `stbds_hmdel_key` | binary mode, delete in insertion order (each delete hits `old_index != final_index` except the last) | `cfg_11_binary_del_forward` | [x] |
| 12 | `stbds_hmdel_key` | binary mode, delete in **reverse** order (always `old_index == final_index`) | `cfg_12_binary_del_reverse` | [x] |
| 13 | `stbds_hmdel_key` | binary mode, delete in **random** order; drives shrink (`used_count < shrink_threshold`) and rebuild (`tombstone_count > threshold`) | `cfg_13_binary_del_random` | [x] |
| 14 | `stbds_hmput_key` after deletes | binary mode: insert 64, delete 32, insert 32 more ⇒ **tombstone reuse** at `found_empty_slot` | `cfg_14_binary_tombstone_reuse` | [x] |
| 15 | `stbds_hmput_default` + `stbds_hmget_key` | default-only map (no hash table) then lookup ⇒ `temp = -1`; then `hmput_key` on it creates the table | `cfg_15_hmput_default_then_put` | [x] |
| 16 | `stbds_shmode_func(STBDS_SH_NONE)` + `hmput_key(mode ∈ {1,2,9})` | table `string.mode = 0` ⇒ the `switch` takes **`default:` memcpy**, copying the first `keysize` *bytes of the string* into the key slot even though `mode` says "string".  Exactly one insert is compared (a second insert could `strcmp` those bytes as a pointer, which crashes in C too) | `cfg_16_shmode_none_string_mode` | [x] |
| 17 | `stbds_shmode_func(STBDS_SH_DEFAULT)` + string map | keys are caller-owned pointers stored verbatim | `cfg_17_shmode_default` | [x] |
| 18 | `stbds_shmode_func(STBDS_SH_STRDUP)` + string map | `strdup` path; put/get/del of N ∈ {1,2,8,9,64,257} random strings, then `hmfree_func` | `cfg_18_shmode_strdup` | [x] |
| 19 | `stbds_shmode_func(STBDS_SH_ARENA)` + string map | `stralloc` path; same N set; verifies arena `block`/`remaining` evolution | `cfg_19_shmode_arena` | [x] |
| 20 | implicit string map: `hmput_key(NULL, …, mode=1)` | `nt->string.mode` set to `STBDS_SH_DEFAULT`; put/get/del | `cfg_20_implicit_string_map` | [x] |
| 21 | implicit binary map: `hmput_key(NULL, …, mode=0)` | `nt->string.mode` set to `0`; put/get/del | `cfg_21_implicit_binary_map` | [x] |
| 22 | `stbds_hmget_key_ts` (low-level, `temp` out-param) | string + binary map, hit / miss / NULL-`a` / no-table; `temp` compared and the array `temp` field NOT written (contrast with `hmget_key`) | `cfg_22_hmget_key_ts_lowlevel` | [x] |
| 23 | `stbds_hmdel_key` with `keyoffset != 0` | `stbds_hmput_key` hardcodes `keyoffset = 0`, so only `hmdel_key` can be given a non-zero one.  Element layout `[key(4) | mirror(4) | value(4)]`, `keyoffset = 4`, binary mode; the test mirrors the key into offset 4 after every insert so `hm_find_slot`/`is_key_equal` at offset 4 stay consistent.  n ∈ {1,2,9,64,150}, shuffled deletes | `cfg_23_keyoffset_nonzero` | [x] |
| 24 | `stbds_stralloc` | fresh arena; strings of length 1..600 in random order ⇒ bump / new-block / oversized-block paths; verify contents + `block`/`remaining` after each | `cfg_24_stralloc_mixed_lengths` | [x] |
| 25 | `stbds_stralloc` | 5000 short strings ⇒ block index climbs 0→…→22, where the `blocksize < 1<<20` gate freezes it (blocksize 512→1 MiB) | `cfg_25_stralloc_block_growth` | [x] |
| 26 | `stbds_stralloc` + `stbds_strreset` | allocate across many blocks, reset, re-allocate ⇒ arena reusable, fields zeroed | `cfg_26_stralloc_strreset_cycle` | [x] |
| 27 | `stbds_hmfree_func` | free a map in each of the 4 `string.mode`s and with `hash_table == NULL` | `cfg_27_hmfree_all_modes` | [x] |
| 28 | `strkey` | `n` ∈ {0,1,9,10,99,100,12345, -1, -12345, INT_MAX, INT_MIN} ⇒ byte-identical static buffer contents | `cfg_28_strkey_values` | [x] |
| 29 | `sh_geti` (top-level driver) | `num` ∈ {0,1,2,3,4,5,6,7,8,9,15,16,17,31,32,33,63,64,65,100,127,128,255,257} ⇒ **stdout compared byte-for-byte** | `cfg_29_sh_geti_stdout` | [x] |
| 30 | `sh_geti` after `stbds_rand_seed` | seeds {0,1,7,42,0xdeadbeef,SIZE_MAX} × `num` ∈ {16,64,100} ⇒ different slot layouts ⇒ different print order; stdout compared | `cfg_30_sh_geti_seeded_stdout` | [x] |
| 31 | full pipeline, binary | randomized op stream (put/get/del/getp) 4000 ops over a 512-key space, `mode=0`, keysize 4 — deep state compare (header, every bucket `hash[]`/`index[]`, every element) after every op | `cfg_31_fuzz_binary_pipeline` | [x] |
| 29b | `sh_geti` repeatedly, **without** re-seeding | the global `stbds_hash_seed` is advanced by every table creation inside `sh_geti`, so 12 consecutive calls must stay in lockstep across the two libraries; stdout compared each round | `cfg_29b_sh_geti_consecutive_no_reseed` | [x] |
| 32 | full pipeline, string STRDUP | randomized op stream 4000 ops, `shmode_func(STBDS_SH_STRDUP)` — deep state compare after every op | `cfg_32_fuzz_string_strdup_pipeline` | [x] |
| 33 | full pipeline, string ARENA | randomized op stream 4000 ops, `shmode_func(STBDS_SH_ARENA)` — deep state compare after every op | `cfg_33_fuzz_string_arena_pipeline` | [x] |
| 34 | full pipeline, string DEFAULT | randomized op stream 4000 ops, `shmode_func(STBDS_SH_DEFAULT)` — deep state compare after every op | `cfg_34_fuzz_string_default_pipeline` | [x] |
| 35 | full pipeline, binary keysize 8/16 | randomized op stream, keysize 8 and 16 (multi-word siphash main loop inside the map) | `cfg_35_fuzz_binary_wide_keys` | [x] |

| 36 | full pipeline, out-of-range `mode` | randomized put/get/geti_ts stream (1500 ops × 3 modes × 2 `sh` modes) with `mode ∈ {2, 5, 1000}` — all select the "string" branch.  Deletes are excluded on purpose: for `mode != 1` exactly, `hmdel_key` aborts by design (Phase C `err_29`) | `cfg_32b_fuzz_string_out_of_range_mode` | [x] |

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section** ⇒ the only build
configuration is the default (no features, no optional deps).  Verified with:

```sh
grep -n '\[features\]' translation/Cargo.toml   # -> no match
```

Therefore "every feature combination" = the single default configuration, plus
the `dev` and `release` profiles (both `panic = "abort"`), both of which are
exercised: `cargo build --release` produces the `.so` under test, and
`cargo test` builds the harness in `dev`.

## Result

37 tests across `tests/phase_b_low.rs` (11), `tests/phase_b_maps.rs` (23) and
`tests/phase_b_driver.rs` (3), all passing.  Every map row compares the *full*
structural state after every single operation — array header
(`length`/`capacity`/`temp`), the whole hash index (`slot_count`, `used_count`,
all four thresholds, `tombstone_count`, `seed`, `slot_count_log2`, the arena
`remaining`/`block`/`mode`), every `hash[]`/`index[]` entry of every bucket, and
every element's key and value — not just the return value.
