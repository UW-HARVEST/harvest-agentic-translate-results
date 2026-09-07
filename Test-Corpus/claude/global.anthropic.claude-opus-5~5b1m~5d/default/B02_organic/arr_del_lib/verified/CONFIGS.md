# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c`: the axes below are exactly the ones
the C code branches on.

## Axes the C actually distinguishes

**A. `mode` argument** (`int`, tested as `mode >= STBDS_HM_STRING(1)` in
`stbds_is_key_equal`/`stbds_hm_find_slot`/`stbds_hmput_key`, and as
`mode == STBDS_HM_STRING` in `stbds_hmdel_key` line 836/842):
`STBDS_HM_BINARY(0)`, `STBDS_HM_STRING(1)`, `2`, negative.

**B. `table->string.mode`** — the `switch` at `stbds_hmput_key` line 785 and the
`== STBDS_SH_STRDUP` tests at lines 575 / 836:
`STBDS_SH_NONE(0)` / `default` → memcpy the key bytes;
`STBDS_SH_DEFAULT(1)` → store the caller's `char*`;
`STBDS_SH_STRDUP(2)` → `stbds_strdup`;
`STBDS_SH_ARENA(3)` → `stbds_stralloc`.
Set implicitly by `stbds_hmput_key` on a fresh table
(`mode >= 1 ? SH_DEFAULT : 0`, line 707) or explicitly by `stbds_shmode_func`.

**C. table growth / rehash state** (`stbds_hmput_key` line 698,
`stbds_hmdel_key` lines 854/858, `stbds_make_hash_index`'s `ot != NULL` rehash
loop): `slot_count` 8 → 16 → 32 → … ; grow at `used_count >= slot_count -
slot_count/4`; shrink at `used_count < slot_count/4` when `slot_count > 8`;
rebuild at `tombstone_count > slot_count/8 + slot_count/16`.

**D. probe path shape** (`stbds_hm_find_slot` / `stbds_hmput_key`): first
in-bucket loop `i = pos&7 .. 7`, wrap-around loop `i = 0 .. (pos&7)`, and the
quadratic outer step `pos += step; step += 8`. Reached only with collisions
inside a bucket, i.e. with many keys.

**E. tombstones** (`stbds_hmput_key` lines 739-742 / 755-758,
`found_empty_slot`'s `tombstone >= 0` reuse): requires delete-then-insert.

**F. `elemsize` / `keysize` shapes**: `keysize == elemsize` (key-only element),
`keysize < elemsize` (key + value), `keysize` 1/2/4/8/16 (SipHash block loop vs.
tail `switch`), `keysize` not a multiple of 8 (tail 1..7 bytes), `keysize == 0`.

**G. `stbds_arrgrowf` shapes**: `a == NULL` vs existing; `min_cap <= arrcap`
(early return); `min_cap < 2*arrcap` (double); `min_cap < 4` (bump to 4);
`min_cap >= 2*arrcap && >= 4` (use as-is); `addlen` 0 vs >0.

**H. `stbds_stralloc` / arena shapes**: `len <= remaining` (fast path);
`len > remaining && len <= blocksize` (new block); `len > blocksize` (oversize
block, spliced vs first); fresh arena vs arena with existing storage;
`a->block` growth 0..22 (blocksize 512 → 1 MiB saturation).

**I. `seed`**: `stbds_rand_seed` changes the global `stbds_hash_seed`, which
`stbds_make_hash_index` copies into each new table and then advances via
`seed = seed*a + b`. Per-`.so` global state ⇒ both libs must be re-seeded
identically before every comparison, and the *sequence* of tables must match.

**J. byte values in keys**: bytes `>= 0x80` in positions 3 and 7 of a SipHash
block, and in the tail `case 4`, trigger the C `int`-promotion sign-extension.

## Rows (each = one differential test over many randomized inputs)

| # | entry point(s) | configuration (options set + input shape) | test (`tests/phase_b_*.rs`) | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`; then `len = 1..=7` (every tail `switch` case), random bytes, random seeds | `cfg_01_hash_bytes_tail_lengths` | [x] |
| 2 | `stbds_hash_bytes` | `len = 8, 16, 24` (whole blocks, no tail) — exercises the block loop only | `cfg_02_hash_bytes_whole_blocks` | [x] |
| 3 | `stbds_hash_bytes` | `len = 9..=64` random (block loop **+** tail), random seeds | `cfg_03_hash_bytes_block_plus_tail` | [x] |
| 4 | `stbds_hash_bytes` | keys whose byte 3 and/or byte 7 of a block is `>= 0x80` (sign-extension path, axis J), plus tail `len == 4..7` with `d[3] >= 0x80` | `cfg_04_hash_bytes_sign_extension` | [x] |
| 5 | `stbds_hash_bytes` | `seed = 0`, `usize::MAX`, `1`, `0x31415926`, and random 64-bit seeds | `cfg_05_hash_bytes_seed_space` | [x] |
| 6 | `stbds_hash_string` | `""`, 1 char, ASCII, bytes `>= 0x80` (the `(unsigned char)` cast), length > 8, random strings; random seeds | `cfg_06_hash_string` | [x] |
| 7 | `stbds_rand_seed` + `stbds_hash_bytes`/`stbds_hash_string` | seeding does **not** affect the raw hash fns (they take `seed` explicitly) but does affect `make_hash_index`; verify the seed *sequence* by chaining table creations | `cfg_07_44_rand_seed_sequence` | [x] |
| 8 | `stbds_arrgrowf` | `a = NULL`, `(addlen, min_cap)` over `{0,1,2,3,4,5,7,8,100}²`, `elemsize` `{1,2,4,8,16,0}` — checks returned `length`/`capacity`/`temp`/`hash_table` and NULL-return edge (axis G) | `cfg_08_arrgrowf_from_null` | [x] |
| 9 | `stbds_arrgrowf` | existing array, repeated growth (doubling chain) with random `addlen`; compare capacity sequence | `cfg_09_arrgrowf_growth_chain` | [x] |
| 10 | `stbds_arrgrowf` + `stbds_arrfreef` | grow → write elements → grow again (data preserved by `realloc`) → free | `cfg_10_arrgrowf_preserves_data` | [x] |
| 11 | `stbds_hmput_default` | `a = NULL`; `a` with `length == 0`; `a` with `length > 0`; `elemsize` `{4,8,16}` | `cfg_11_hmput_default` | [x] |
| 12 | `stbds_hmput_key` | **binary** mode 0, `keysize == elemsize == 4` (int key only), 1 key | `cfg_12_binary_single_int_key` | [x] |
| 13 | `stbds_hmput_key` | binary mode 0, `keysize = 4 < elemsize = 16` (`struct {int key,b,c,d;}`), N random keys, N ∈ {1,2,5,6,7,8,20,100,1000} spanning every grow threshold (axis C) | `cfg_13_binary_struct_key_grow_thresholds` | [x] |
| 14 | `stbds_hmput_key` | binary mode 0, `keysize = 8 == elemsize`(u64 key) and `keysize = 8 < elemsize = 20` (`struct {int key[2],b,c,d;}`) | `cfg_14_binary_wide_keys` | [x] |
| 15 | `stbds_hmput_key` | binary mode 0, duplicate keys re-put (the "key already present" branch, lines 729-735) — must return the same index and not grow `length` | `cfg_15_binary_duplicate_puts` | [x] |
| 16 | `stbds_hmput_key` | binary mode 0, `keysize` 1, 2, 3, 5, 16 (odd/tail SipHash sizes, axis F) | `cfg_16_binary_odd_keysizes` | [x] |
| 17 | `stbds_hmput_key` | binary mode 0, `keysize == 0` (every key "equal") | `cfg_17_binary_zero_keysize` | [x] |
| 18 | `stbds_hmput_key` | **string** mode 1 on a fresh table → `string.mode` becomes `SH_DEFAULT` (line 707); caller-owned `char*` stored verbatim; N random strings | `cfg_18_string_default_mode_implicit + cfg_18b_temp_key_after_insert + cfg_18c_temp_key_wraparound_quirk` | [x] |
| 19 | `stbds_shmode_func(elemsize, STBDS_SH_STRDUP)` + `stbds_hmput_key(mode=1)` | `string.mode = 2` → keys `stbds_strdup`'d; verify stored pointer ≠ input pointer and contents equal; N strings across grow thresholds | `cfg_19_string_strdup_mode` | [x] |
| 20 | `stbds_shmode_func(elemsize, STBDS_SH_ARENA)` + `stbds_hmput_key(mode=1)` | `string.mode = 3` → keys arena-allocated via `stbds_stralloc`; many strings so the arena grows blocks; verify contents + arena `block`/`remaining` sequence | `cfg_20_string_arena_mode` | [x] |
| 21 | `stbds_shmode_func(elemsize, STBDS_SH_NONE)` + `stbds_hmput_key(mode=0)` | `string.mode = 0` → `switch` `default` → memcpy the key bytes, even though the table was made by `shmode_func` | `cfg_21_shmode_none_binary_puts` | [x] |
| 22 | `stbds_shmode_func(elemsize, STBDS_SH_DEFAULT)` + `stbds_hmput_key(mode=1)` | `string.mode = 1` explicitly via `shmode_func` (vs implicitly via `hmput_key`) | `cfg_22_shmode_default_explicit` | [x] |
| 23 | `stbds_hmget_key_ts` | binary mode 0: hits, misses, and `a == NULL`; check the `*temp` out-param and the returned pointer's relation to the input | `cfg_23_hmget_ts_binary` | [x] |
| 24 | `stbds_hmget_key_ts` | string mode 1 (`SH_DEFAULT`/`STRDUP`/`ARENA` tables): hits and misses by *content*-equal but pointer-different keys (exercises `strcmp`) | `cfg_24_hmget_ts_string_all_modes` | [x] |
| 25 | `stbds_hmget_key` | same as rows 23-24 but through the non-`_ts` wrapper, which additionally stores into `stbds_header(p-elemsize)->temp` | `cfg_25_hmget_key_writes_temp` | [x] |
| 26 | `stbds_hmdel_key` | binary mode 0, delete an **interior** element (`old_index != final_index`) → memmove of the last element + slot re-find (lines 839-851) | `cfg_26_hmdel_interior` | [x] |
| 27 | `stbds_hmdel_key` | binary mode 0, delete the **last** element (`old_index == final_index`) → no memmove | `cfg_27_hmdel_last_element` | [x] |
| 28 | `stbds_hmdel_key` | binary mode 0, delete enough to hit the **shrink** path (`used_count < slot_count/4 && slot_count > 8`, line 854) | `cfg_28_29_30_hmdel_shrink_rebuild_tombstone` | [x] |
| 29 | `stbds_hmdel_key` | binary mode 0, delete/insert pattern that hits the **rebuild** path (`tombstone_count > slot_count/8 + slot_count/16`, line 858) | `cfg_28_29_30_hmdel_shrink_rebuild_tombstone` | [x] |
| 30 | `stbds_hmdel_key` + `stbds_hmput_key` | delete then re-insert → the **tombstone reuse** branch (axis E, lines 739/766-769): `tombstone_count--`, slot reused | `cfg_28_29_30_hmdel_shrink_rebuild_tombstone` | [x] |
| 31 | `stbds_hmdel_key` | string mode 1 with `string.mode == SH_STRDUP` → the deleted key is `free`d (line 836) | `cfg_31_32_string_deletes` | [x] |
| 32 | `stbds_hmdel_key` | string mode 1 with `SH_DEFAULT` and with `SH_ARENA` (no free) | `cfg_31_32_string_deletes` | [x] |
| 33 | full pipeline, binary | random interleaved put/get/del/get over N ops (N = 2000) with a small key space so collisions, tombstones, grows, shrinks and rebuilds all occur; compare the *entire* element array + every returned index after every op (axes C+D+E together) | `cfg_33_pipeline_binary` + `cfg_33b_soak_multi_seed` (24 different global seeds) | [x] |
| 34 | full pipeline, string `SH_DEFAULT` | same randomized op stream with `char*` keys | `cfg_34_35_36_pipeline_string` | [x] |
| 35 | full pipeline, string `SH_STRDUP` | same randomized op stream; compare key *contents* (pointers differ by design) | `cfg_34_35_36_pipeline_string` | [x] |
| 36 | full pipeline, string `SH_ARENA` | same randomized op stream; compare key contents + final arena state | `cfg_34_35_36_pipeline_string` | [x] |
| 37 | `stbds_hmfree_func` | after each pipeline, for every `string.mode` (0/1/2/3) and for `hash_table == NULL`; plus `a == NULL` | `cfg_37_hmfree_all_shapes` | [x] |
| 38 | `stbds_stralloc` | fresh arena; strings of length 0, 1, 100, 511, 512, 513 (blocksize boundary `512`), and 5000 (`len > blocksize` oversize path) | `cfg_38_stralloc_boundaries` | [x] |
| 39 | `stbds_stralloc` | many random small strings on one arena → drives `a->block` 0→22 and the `512<<(block>>1)` saturation at 1 MiB (axis H); compare returned offsets, `remaining`, `block` after every call | `cfg_39_stralloc_block_growth + cfg_39b_stralloc_block_saturation` | [x] |
| 40 | `stbds_stralloc` | oversize string **after** the arena already has storage (the `sb->next = a->storage->next` splice branch) vs oversize on a **fresh** arena (`a->storage = sb; a->remaining = 0`) | `cfg_40_stralloc_oversize_splice` | [x] |
| 41 | `stbds_strreset` | empty arena; arena with 1 block; arena with many blocks; arena with a spliced oversize block. Verify the arena struct is fully zeroed. | `cfg_41_strreset_shapes` | [x] |
| 42 | `strkey` | `0`, `1`, `-1`, `9`, `10`, `99`, `100`, `INT_MAX`, `INT_MIN`, and random ints — compare the returned C string byte-for-byte | `cfg_42_strkey` | [x] |
| 43 | `arr_del` | `num` = 0, 1, -1, `INT_MAX`, `INT_MIN`, random — the function is `void`, so verify it completes without abort under ASan-free normal run for both libs (all 4 `i` values incl. the `i == 3` zero-length memmove) | `cfg_43_arr_del` | [x] |
| 44 | `stbds_rand_seed` sequence | seed the global, then create tables via repeated `stbds_hmput_key(NULL, …)` and read back each table's `seed` field, plus the resulting per-table slot assignment; confirms the `seed*a+b` LCG constants from `stbds_load_32_or_64` | `cfg_07_44_rand_seed_sequence` | [x] |
| 45 | binary: `keyoffset != 0` | `stbds_hmdel_key` takes an explicit `keyoffset` (the only entry point that does; `hmput`/`hmget` hardcode 0). Non-zero `keyoffset` with a struct whose key is not first. | `cfg_45_hmdel_keyoffset` | [x] |
| 46 | struct layout probe | after each operation, compare the raw `stbds_array_header` (length/capacity/temp/`hash_table != NULL`) and the whole `stbds_hash_index` scalar fields + all bucket `hash[]`/`index[]` arrays byte-for-byte between C and Rust | `cfg_46_struct_layout` (explicit size/offset/alignment pins) + `cfg_46b_allocation_sufficiency` (every block big enough for what is written into it) + every row — `assert_same_binary` / `assert_same_string` compare the header, all element bytes and every bucket; `cfg_33b_soak_multi_seed` repeats it under 24 global seeds | [x] |

All 46 rows verified: `cargo test` -> 79 passing differential tests across
`phase_b_low.rs` (18), `phase_b_map.rs` (24) and `phase_c_errors.rs` (37).

## Cross-cutting fuzz

`cfg_fuzz_random_configurations` (`tests/phase_b_map.rs`) randomises the
*configuration* as well as the data — `elemsize`, `keysize`, `sh_mode`, `mode`,
key space size and op mix — for 1200 independent cases under 1200 different
global seeds, so combinations no row above happens to name are still covered.
Two configurations are excluded because the **C itself faults** on them, leaving
nothing differential to observe (both are documented in `VERIFICATION.md`):

* `mode >= 2` with an *interior* delete → trips the live
  `STBDS_ASSERT(slot >= 0)` at `lib.c:846`;
* a string-keyed table with `string.mode == STBDS_SH_NONE` (only reachable via an
  explicit `stbds_shmode_func(es, STBDS_SH_NONE)`) → the `switch` `default:` arm
  memcpy's the key's *characters* into the element and the next
  `stbds_is_key_equal` dereferences them as a `char *`.

## Feature combinations

`Cargo.toml` has no `[features]` table ⇒ exactly one configuration; all rows are
verified under it (`cargo test --release` and `cargo test --no-default-features
--release` are the same build).

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — no
`add_executable`, and `translation/Cargo.toml` declares only a `cdylib` `[lib]`
with no `[[bin]]`. There is no driver binary, so the "compare stdout" gate is
vacuously satisfied.
