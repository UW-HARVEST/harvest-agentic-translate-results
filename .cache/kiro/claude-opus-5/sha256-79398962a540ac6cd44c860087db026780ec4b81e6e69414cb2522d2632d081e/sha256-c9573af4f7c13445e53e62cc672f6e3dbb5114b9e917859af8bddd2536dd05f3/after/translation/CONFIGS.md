# CONFIGS.md — configuration / valid-input surface table

Axes derived mechanically from the branches in `c_src/src/lib.c`.

## Axis 1 — `int mode` argument (`stbds_hmput_key` / `stbds_hmget_key[_ts]` / `stbds_hmdel_key`)

Tested with `mode >= STBDS_HM_STRING` (l.713, l.598, l.564) and, in
`stbds_hmdel_key` only, with `mode == STBDS_HM_STRING` (l.838):

| value | `>= 1`? | `== 1`? | effect |
|-------|---------|---------|--------|
| `0` (`STBDS_HM_BINARY`) | no | no | `stbds_hash_bytes` + `memcmp` |
| `1` (`STBDS_HM_STRING`) | yes | yes | `stbds_hash_string` + `strcmp`; del frees strdup'd key |
| `2` (old `HM_PTR_TO_STRING`) | yes | no | string hash/compare, but del does **not** free |
| `-1`, `INT_MIN` | no | no | binary |
| `7`, `INT_MAX` | yes | no | string, del does not free |

## Axis 2 — table string mode (`stbds_shmode_func(elemsize, mode)` → `(unsigned char)mode`, consumed by the `switch` on l.784)

| value | switch arm | key storage |
|-------|-----------|-------------|
| `1` `STBDS_SH_DEFAULT` | `case SH_DEFAULT` | stores the caller's `char*` verbatim |
| `2` `STBDS_SH_STRDUP` | `case SH_STRDUP` | `stbds_strdup`, freed by `hmfree`/`hmdel` |
| `3` `STBDS_SH_ARENA` | `case SH_ARENA` | `stbds_stralloc` into `table->string` |
| `0` `STBDS_SH_NONE`, `4`, `255`, `256`(→0), `-1`(→255) | `default` | `memcpy(elem, key, keysize)` — binary |
| *not called at all* | set by l.706 to `SH_DEFAULT` if `mode>=1` else `0` | implicit mode from the first `hmput_key` |

## Axis 3 — element / key shape

`elemsize` and `keysize` are caller-supplied and independent. Shapes the code
distinguishes: the siphash 8-byte main loop plus its 8-way tail `switch`
(l.535), `memcmp(keysize)` vs `strcmp`, and `keyoffset`.

- keysize `0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 24, 33` (covers every
  `len % 8` tail case and multi-iteration bodies)
- elemsize `8, 12, 16, 24, 32, 40` (incl. cases with padding and with
  `elemsize > keysize`)
- `keyoffset` `0` (the `STBDS_OFFSETOF(t,key)` value for a key-first struct)
  and non-zero
- key byte values including `>= 0x80` in positions 3 and 7 (exercises the
  sign-extension in the siphash loader and in `case 4`)

## Axis 4 — entry-count / growth boundaries

`slot_count` starts at `STBDS_BUCKET_LENGTH == 8`; `used_count_threshold` is
`sc - sc/4`, `tombstone_count_threshold` is `sc/8 + sc/16`,
`used_count_shrink_threshold` is `sc/4` (forced to 0 when `sc <= 8`).

Counts: `0, 1, 2, 5, 6, 7, 8, 9, 12, 13, 24, 25, 48, 100, 500, 2000`
— straddling every rebuild point (6→16, 12→32, 24→64, 48→128, 96→256, …).

## Axis 5 — arena / block sizes (`stbds_stralloc`)

`blocksize = 512 << (block>>1)`, capped by `block` freezing at 22.
Shapes: `len == 1` (`""`), `len < 512`, `len == 512`, `len == 513`
(`len > blocksize` dedicated-block path), `len` in the multi-MiB range,
first-alloc-on-empty-arena vs alloc-with-existing-storage.

---

## Configuration rows

Every row is exercised with **many randomized inputs driven by a fixed-seed
xorshift PRNG** (`tests/common/mod.rs::Rng`), calling both `.so`s through
`libloading` and comparing byte-for-byte (element bytes, array header, the whole
`stbds_hash_index` struct, and every `stbds_hash_bucket`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | `len = 0`, `p = NULL` | [x] |
| C2 | `stbds_hash_bytes` | `len = 1..7` (each tail `switch` case), random bytes incl. `>= 0x80` | [x] |
| C3 | `stbds_hash_bytes` | `len = 8` exactly (one main-loop iteration, empty tail) | [x] |
| C4 | `stbds_hash_bytes` | `len = 9..64`, all residues mod 8, random bytes | [x] |
| C5 | `stbds_hash_bytes` | `len` large (65..4096), random bytes | [x] |
| C6 | `stbds_hash_bytes` | bytes chosen so `d[3] >= 0x80` and `d[7] >= 0x80` (sign-extension path) | [x] |
| C7 | `stbds_hash_bytes` | random `seed` incl. `0`, `usize::MAX`, `1` | [x] |
| C8 | `stbds_hash_string` | `""`, 1 char, ASCII, bytes `>= 0x80`, long strings; random `seed` | [x] |
| C9 | `stbds_rand_seed` + `stbds_shmode_func`/`stbds_hmput_key` | seed `0`, `1`, `0x31415926`, `usize::MAX`, random → verify the `seed = seed*a+b` LCG advance is identical (observed via `table->seed`) | [x] |
| C10 | `strkey` | `n = 0, 1, 9, 10, 99, 100, -1, -9, -10, INT_MIN, INT_MAX`, random | [x] |
| C11 | `stbds_arrgrowf` | `a = NULL`, `elemsize` ∈ {1,4,8,16,24}, `addlen`/`min_cap` ∈ {0,1,2,3,4,5,8,100} (incl. the `min_cap<4 → 4` clamp and the NULL/0 no-op) | [x] |
| C12 | `stbds_arrgrowf` | repeated growth on an existing array (the `2*cap` doubling clamp), 1..64 successive grows | [x] |
| C13 | `stbds_arrgrowf` | `min_cap` already `<= cap` → returns `a` unchanged | [x] |
| C14 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free; header `length`/`capacity`/`temp`/`hash_table` state | [x] |
| C15 | `stbds_hmput_default` | `a = NULL` | [x] |
| C16 | `stbds_hmput_default` | `a` from `stbds_shmode_func` (length already 1) → no-op | [x] |
| C17 | `stbds_hmput_default` | called twice in a row | [x] |
| C18 | `stbds_hmput_key` (implicit mode, no `shmode_func`) | `mode = 0` (binary), keysize 8, 1..N random `u64` keys, N over every Axis-4 count | [x] |
| C19 | `stbds_hmput_key` (implicit) | `mode = 0`, keysize ∈ {1,2,3,4,5,6,7,9,15,16,24,33}, random keys, N ∈ {1,7,8,9,50} | [x] |
| C20 | `stbds_hmput_key` (implicit) | `mode = 0` with `elemsize > keysize` (padding bytes must stay zero/uninit-identical) | [x] |
| C21 | `stbds_hmput_key` (implicit) | `mode = 1` (string) with **no** `shmode_func` → l.706 sets `string.mode = SH_DEFAULT`, keys stored as caller pointers | [x] |
| C22 | `stbds_hmput_key` (implicit) | `mode = 2` / `7` / `INT_MAX` (string branch, `!= HM_STRING`) | [x] |
| C23 | `stbds_hmput_key` (implicit) | `mode = -1` / `INT_MIN` (binary branch) | [x] |
| C24 | `stbds_shmode_func(SH_STRDUP)` + `stbds_hmput_key(mode=1)` | strdup key storage, N random strings over Axis-4 counts | [x] |
| C25 | `stbds_shmode_func(SH_ARENA)` + `stbds_hmput_key(mode=1)` | arena key storage, N random strings, incl. strings > 512 bytes | [x] |
| C26 | `stbds_shmode_func(SH_DEFAULT)` + `stbds_hmput_key(mode=1)` | caller-pointer key storage | [x] |
| C27 | `stbds_shmode_func(SH_NONE=0)` + `stbds_hmput_key(mode=1)` | `switch` `default:` → binary `memcpy` of `keysize` bytes even though `mode` says string; find_slot still uses `strcmp` | [x] |
| C28 | `stbds_shmode_func(4 / 255 / 256 / -1 / INT_MIN / INT_MAX)` + `stbds_hmput_key` | out-of-range shmode → `default:` arm | [x] |
| C29 | `stbds_hmput_key` | overwrite: put the **same** key repeatedly (existing-key branch l.729, incl. the `temp_key` write only on the first inner loop) | [x] |
| C30 | `stbds_hmput_key` | interleaved put / re-put across a rebuild boundary (6, 12, 24, 48 entries) | [x] |
| C31 | `stbds_hmget_key` | binary mode, hits and misses, over table sizes 8..512 | [x] |
| C32 | `stbds_hmget_key` | string mode (all three shmodes), hits and misses | [x] |
| C33 | `stbds_hmget_key_ts` | same as C31/C32 but reading `*temp` and checking the header `temp` is **not** written (the `_ts` variant's whole point) | [x] |
| C34 | `stbds_hmget_key_ts` | `a = NULL` bootstrap path | [x] |
| C35 | `stbds_hmdel_key` | binary mode, delete the last entry (`old_index == final_index`) | [x] |
| C36 | `stbds_hmdel_key` | binary mode, delete a middle entry (move-and-repatch) | [x] |
| C37 | `stbds_hmdel_key` | binary mode, delete every entry one by one, in random order (drives shrink + tombstone rebuild) | [x] |
| C38 | `stbds_hmdel_key` | string mode + `SH_STRDUP` (key is freed), random order | [x] |
| C39 | `stbds_hmdel_key` | string mode + `SH_ARENA` (key not freed) | [x] |
| C40 | `stbds_hmdel_key` | string mode + `SH_DEFAULT` | [x] |
| C41 | `stbds_hmdel_key` | `mode = 2` on a `SH_STRDUP` table (`==` vs `>=` asymmetry, no free) | [x] |
| C42 | `stbds_hmdel_key` | `keyoffset != 0` (key is the 2nd member), binary mode | [x] |
| C43 | `stbds_hmdel_key` | delete-then-reinsert (tombstone reuse at `found_empty_slot`, l.767) | [x] |
| C44 | `stbds_hmdel_key` | enough deletes to cross `used_count_shrink_threshold` from `slot_count` 512→8 | [x] |
| C45 | `stbds_hmdel_key` | delete/insert churn that crosses `tombstone_count_threshold` without shrinking | [x] |
| C46 | `stbds_hmfree_func` | `SH_STRDUP` table with N entries (frees N-1 keys, then the arena, table, header) | [x] |
| C47 | `stbds_hmfree_func` | `SH_ARENA` table (arena reset), and `SH_DEFAULT`, and a hash-table-less array | [x] |
| C48 | `stbds_stralloc` | fresh arena, `len` ∈ {1,2,16,511,512,513}; check returned string, `remaining`, `block` | [x] |
| C49 | `stbds_stralloc` | many small strings until `block` saturates at 22 (checks the `blocksize < MAX` guard) | [x] |
| C50 | `stbds_stralloc` | `len > blocksize` dedicated-block path, on an empty arena and on a non-empty arena | [x] |
| C51 | `stbds_stralloc` | randomized mix of sizes; every returned string compared, plus `remaining`/`block` after each call | [x] |
| C52 | `stbds_strreset` | after C48–C51, and on a zeroed arena, and twice in a row | [x] |
| C53 | `sh_puts` | `num` ∈ {0, 1, 2, 3, 8, 100, 1000, -1, INT_MIN, INT_MAX-ish} — stdout captured and compared byte-for-byte | [x] |
| C54 | full pipeline, binary | `rand_seed(s)` → N random puts → gets (hit+miss) → random deletes → re-puts → `hmfree_func`, N over Axis-4, random seeds | [x] |
| C55 | full pipeline, `SH_STRDUP` | same as C54 with random strings | [x] |
| C56 | full pipeline, `SH_ARENA` | same as C54 with random strings incl. long ones | [x] |
| C57 | full pipeline, `SH_DEFAULT` | same as C54 | [x] |
| C58 | full pipeline, implicit string mode | no `shmode_func`; `mode=1` puts only | [x] |
| C59 | `stbds_hmput_key` | keys engineered to collide in the same bucket (same `hash & (slot_count-1)`) to force the `pos += step; step += 8` quadratic probe and the second (`limit`) inner loop | [x] |
| C60 | `stbds_hmput_key`/`hmget`/`hmdel` | `elemsize == keysize == 8` minimal, and `elemsize = 40 > keysize = 33` maximal, across all four shmodes | [x] |

---

## Notes discovered while checking these rows off

- **`(unsigned char)` aliasing (C28).** `stbds_shmode_func` narrows `mode` to
  `unsigned char`, so `257/258/259` alias `SH_DEFAULT/SH_STRDUP/SH_ARENA` and
  take the *pointer-storage* arms, while `256/-256/INT_MIN → 0`,
  `-1/INT_MAX → 255` and `1000 → 232` take the `default:` (binary) arm. Rows are
  split accordingly.
- **`mode > STBDS_HM_STRING` + non-final delete aborts (C22, C41).**
  `stbds_hmdel_key`'s key-reload tests `mode == STBDS_HM_STRING` while
  `stbds_hm_find_slot` tests `mode >= STBDS_HM_STRING`, so `mode == 2` makes the
  post-move re-lookup hash the raw pointer bytes, miss, and trip the live
  `STBDS_ASSERT(slot >= 0)`. These rows therefore delete tail-first
  (`old_index == final_index` skips the block); the abort itself is verified in a
  child process by `errors.rs::e18_mode2_nonlast_delete_aborts_in_both`.
- **String lookups against the `default:` arm are C UB (C27, C28b).**
  `default:` `memcpy`s the *key string's bytes* into the element, so a subsequent
  `mode >= 1` comparison reinterprets them as a `char *`. C27 therefore drives
  that storage mode with `mode = 0`; C28b covers `mode = 1` up to the insert only.

## Row → test mapping

| rows | test |
|------|------|
| C1–C7 | `leaf.rs::c1_hash_bytes_len0_null`, `leaf.rs::c2_c7_hash_bytes_all_lengths_and_seeds`, `errors.rs::e31_hash_bytes_every_tail_case` |
| C8 | `leaf.rs::c8_hash_string` |
| C9 | `leaf.rs::c9_rand_seed_lcg_advance` |
| C10 | `leaf.rs::c10_strkey` |
| C11–C14 | `leaf.rs::c11_arrgrowf_from_null`, `c12_arrgrowf_repeated_growth`, `c14_arrgrowf_payload_preserved`, `errors.rs::e26_e29_arrgrowf_edges` |
| C15–C17 | `maps.rs::c15_c17_hmput_default` |
| C18 | `maps.rs::c18_binary_counts` |
| C19 | `maps.rs::c19_binary_key_widths` |
| C20 | `maps.rs::c20_binary_elemsize_gt_keysize` |
| C21 | `maps.rs::c21_implicit_string_mode` |
| C22 | `maps.rs::c22_mode_above_one` |
| C23 | `maps.rs::c23_negative_mode` |
| C24 | `maps.rs::c24_sh_strdup` |
| C25 | `maps.rs::c25_sh_arena` |
| C26 | `maps.rs::c26_sh_default` |
| C27 | `maps.rs::c27_sh_none_binary` |
| C28 | `maps.rs::c28a_out_of_range_shmode_binary`, `maps.rs::c28b_out_of_range_shmode_string_single_insert` |
| C29 | `maps.rs::c29_repeated_same_key` |
| C30 | `maps.rs::c30_reput_across_rebuilds` |
| C31, C33 | `maps.rs::c31_c33_lookups_across_table_sizes` |
| C32 | `maps.rs::c32_string_lookups_all_shmodes` |
| C34 | `maps.rs::c34_get_from_null` |
| C35, C36 | `maps.rs::c35_c36_delete_last_and_middle` |
| C37, C44, C45 | `maps.rs::c37_c44_c45_delete_all_random_order`, `errors.rs::e34_e36_threshold_transitions` |
| C38–C40 | `maps.rs::c38_c40_string_deletes` |
| C41 | `maps.rs::c41_mode2_on_strdup_table` |
| C42 | `maps.rs::c42_keyoffset`, `errors.rs::e43_nonzero_keyoffset` |
| C43 | `maps.rs::c43_tombstone_reuse` |
| C46, C47 | `maps.rs::c46_c47_hmfree` |
| C48–C52 | `leaf.rs::c48_stralloc_boundary_lengths`, `c49_stralloc_block_saturation`, `c50_stralloc_dedicated_block`, `c51_stralloc_random_mix`, `c52_strreset_idempotent`, `errors.rs::e20/e22/e23/e24` |
| C53 | `leaf.rs::c53_sh_puts_stdout`, `errors.rs::e21_e44_sh_puts_edge_nums` |
| C54–C58 | `maps.rs::c54_c58_full_pipelines` |
| C59 | `maps.rs::c59_engineered_collisions` |
| C60 | `maps.rs::c60_extreme_geometries` |

## What "matches" means in these tests

After **every** operation the harness (`tests/common/mod.rs::snapshot`) compares:

- the array header: `length`, `capacity`, `temp`;
- every `stbds_hash_index` field except the three pointers
  (`temp_key`, `storage`, and the array's `hash_table`), i.e. `slot_count`,
  `used_count`, `used_count_threshold`, `used_count_shrink_threshold`,
  `tombstone_count`, `tombstone_count_threshold`, `seed`, `slot_count_log2`,
  and the embedded arena's `remaining` / `block` / `mode`;
- every `stbds_hash_bucket`'s `hash[8]` and `index[8]` — the full probe layout;
- every initialised byte of every element in `0..length` (uninitialised `realloc`
  padding is excluded by construction, never by masking a mismatch);
- the NUL-terminated key string behind each key pointer, for pointer-storage modes;
- and the return values / `*temp` / reported indices of every call.
