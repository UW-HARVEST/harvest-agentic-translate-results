# CONFIGS.md — Phase A: the CONFIGURATION-SURFACE TABLE

The mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes:

```sh
grep -n 'if \|else\|switch\|case \|for (\|while (\|?' c_src/src/lib.c
```

## The axes the C code branches on

**Ax1 — entry point / level in the call hierarchy** (all 16 exported symbols;
the low-level ones are driven directly, not only through `sh_puts`):

| level | entry points |
|-------|--------------|
| L0 pure functions | `stbds_rand_seed`, `stbds_hash_bytes`, `stbds_hash_string`, `strkey` |
| L1 dynamic array | `stbds_arrgrowf`, `stbds_arrfreef` |
| L2 string arena | `stbds_stralloc`, `stbds_strreset` |
| L3 hash map | `stbds_shmode_func`, `stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func` |
| L4 composed pipeline | `sh_puts` (arena loop → `strreset` → `sh_new_arena` → `shputs` → `shlen` → `printf` → `shfree`) |

**Ax2 — `mode` (the `STBDS_HM_*` "enum")**, tested by `mode >= STBDS_HM_STRING`
(hash/compare selection, `lib.c:560,590,713`) *and separately* by
`mode == STBDS_HM_STRING` (`lib.c:836,842`, strdup-free + re-lookup key form):
`mode < 1` (binary) · `mode == 1` (string) · `mode >= 2` (string-hash but not
"==STRING").

**Ax3 — `table->string.mode` (the `STBDS_SH_*` enum)**, selecting the key-storage
`switch` in `stbds_hmput_key` (`lib.c:785`): `STBDS_SH_NONE(0)`/default →
`memcpy` · `STBDS_SH_DEFAULT(1)` → alias the caller's pointer ·
`STBDS_SH_STRDUP(2)` → `malloc`+copy · `STBDS_SH_ARENA(3)` → arena copy. Set
either implicitly (`hmput_key` on a NULL table: `mode>=1 ? SH_DEFAULT : 0`) or
explicitly by `stbds_shmode_func`.

**Ax4 — table state at entry**: `hash_table == NULL` · `used_count <
used_count_threshold` · `used_count >= used_count_threshold` (grow) ·
`tombstone_count > tombstone_count_threshold` (rebuild) · `used_count <
used_count_shrink_threshold && slot_count > 8` (shrink).

**Ax5 — element/key geometry**: `elemsize` ∈ {8, 16, 20, 24, 32, 64} ·
`keysize` ∈ {1, 2, 4, 8, 16, 0} · `keyoffset` ∈ {0, non-zero}.

**Ax6 — element count / probe shape**: 0, 1, 2, many; counts that straddle every
threshold (6/7 → sc 8→16, 12/13 → 16→32, 24/25 → 32→64, 48/49 → 64→128);
probe-wrap (`pos & 7 != 0` so the second `for (i=0;i<limit;++i)` scan runs);
duplicate-key re-put; delete-last vs delete-middle (relocation `memmove`).

**Ax7 — byte-string shape** for the hash functions: `len` 0,1,…,9,15,16,17,31,
32,33,64 (the `i+sizeof(size_t)<=len` block loop plus every `switch (len-i)`
fall-through case 7…0); byte values with the **high bit set** at `d[3]`/`d[7]`
(the `d[3] << 24` sign-extension-into-`size_t` path, `lib.c:523,536`);
unaligned `p`; `seed` ∈ {0, 1, default 0x31415926, `SIZE_MAX`, random}.

**Ax8 — arena shape**: `remaining` 0 / ≥len / <len · `block` 0…22 (the
`512 << (block>>1)` ladder), 23…254, 255 (shift ≥ 64) · `storage` NULL /
non-NULL · `len` ≤ / > `blocksize` (dedicated over-sized block).

**Ax9 — `sh_puts(num)`**: `num` ≤ 0 · 1 · small · large enough to cross several
arena block sizes (each `strkey(i)` is ≤ 16 bytes, so 512-byte block ⇒ ~32
strings/block; the block ladder is 512,512,1024,1024,2048,… ).

---

## The table (one row per combination the C treats differently)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| **L0 — pure functions** | | | |
| C1 | `stbds_hash_bytes` | `len == 0`, `p == NULL`; seeds {0,1,default,SIZE_MAX} | [x] |
| C2 | `stbds_hash_bytes` | `len` 1..7 (every `switch (len-i)` fall-through case), random bytes, random seeds | [x] |
| C3 | `stbds_hash_bytes` | `len` 1..7 with `d[3] >= 0x80` (forces the `(d[3]<<24)` negative-`int` → sign-extended `size_t` path) | [x] |
| C4 | `stbds_hash_bytes` | `len == 8` exactly (one block-loop iteration, `len-i == 0`) | [x] |
| C5 | `stbds_hash_bytes` | `len` 9..15 (one block + tail 1..7) | [x] |
| C6 | `stbds_hash_bytes` | `len` 16,17,31,32,33,64,255 (multi-block + tail) | [x] |
| C7 | `stbds_hash_bytes` | block-loop bytes with `d[3] >= 0x80` and/or `d[7] >= 0x80` (both sign-extension sites inside the loop) | [x] |
| C8 | `stbds_hash_bytes` | unaligned `p` (offset 1..7 into a buffer) | [x] |
| C9 | `stbds_hash_bytes` | `len << (64-8)` truncation: `len` with bits above 8 set (e.g. 256+1, 1000) | [x] |
| C10 | `stbds_hash_string` | `""`, 1, 7, 8, 9, 64, 1000-char strings; bytes ≥ 0x80 (the `(unsigned char)*str` cast) | [x] |
| C11 | `stbds_hash_string` | seeds {0,1,default,SIZE_MAX,random}; verifies the `hash ^= hash ^ ROTR(...)` idioms | [x] |
| C12 | `stbds_rand_seed` + `stbds_shmode_func` | seed is a *global* consumed and advanced (`seed*0x27bb2ee687b0b0fd + 0xb504f32d`) by each fresh `stbds_make_hash_index`; drive N consecutive fresh tables and compare `table->seed` each time | [x] |
| C13 | `strkey` | `n` = 0, 1, 9, 10, 99, 100, −1, −9, −10, `INT_MAX`, `INT_MIN`, 1000 randoms | [x] |
| **L1 — dynamic array (`stbds_arrgrowf` / `stbds_arrfreef`)** | | | |
| C14 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` → returns NULL, no alloc | [x] |
| C15 | `stbds_arrgrowf` | `a == NULL`, `min_cap` 1,2,3 → `min_cap < 4` → bumped to 4 | [x] |
| C16 | `stbds_arrgrowf` | `a == NULL`, `min_cap` 4,5,17,1000 → used verbatim | [x] |
| C17 | `stbds_arrgrowf` | `a == NULL`, `addlen` > `min_cap` → `min_cap = min_len` | [x] |
| C18 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → identical pointer, no realloc (early-out) | [x] |
| C19 | `stbds_arrgrowf` | existing array, `min_cap < 2*cap` → doubling wins (`cap = 2*cap`) | [x] |
| C20 | `stbds_arrgrowf` | existing array, `min_cap >= 2*cap` → `min_cap` wins | [x] |
| C21 | `stbds_arrgrowf` | repeated growth chain (append 1 at a time, 0→64 elements) for `elemsize` ∈ {1,4,8,16,20,24,32,64}; compare `length`/`capacity`/`temp`/`hash_table` after every step | [x] |
| C22 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (heap round-trip, non-NULL `a`) | [x] |
| C23 | `stbds_arrgrowf` | `elemsize == 0` (degenerate but legal: `0*min_cap + 32`) | [x] |
| **L2 — string arena (`stbds_stralloc` / `stbds_strreset`)** | | | |
| C24 | `stbds_stralloc` | fresh arena `{0,0,0,0}`, one short string → first 512-byte block, `remaining = 512 - len` | [x] |
| C25 | `stbds_stralloc` | many short strings until the block is exhausted → `block` ladder 0→1→2→…, block sizes 512,512,1024,1024,2048,… ; compare returned strings, `remaining`, `block` at each step | [x] |
| C26 | `stbds_stralloc` | string of length exactly `remaining` (`len == remaining` → no new block) | [x] |
| C27 | `stbds_stralloc` | string of length `remaining + 1` (→ new block) | [x] |
| C28 | `stbds_stralloc` | `len == blocksize` exactly (`len > blocksize` false → normal block, `remaining` becomes 0) | [x] |
| C29 | `stbds_stralloc` | `len == blocksize + 1` on a **fresh** arena → over-sized block, `storage = sb`, `remaining = 0` (R26) | [x] |
| C30 | `stbds_stralloc` | `len > blocksize` on a **non-empty** arena → over-sized block spliced as `storage->next`, `remaining` preserved (R25) | [x] |
| C31 | `stbds_stralloc` | pre-set `block` = 0,1,2,10,11,21,22 (ladder incl. the 1 MiB ceiling where `block` stops incrementing, R28) | [x] |
| C32 | `stbds_stralloc` | pre-set `block` = 110…127 and 238…255, where `(block>>1) & 63 >= 55` so `512 << (block>>1)` shifts every bit out → `blocksize == 0` and the over-sized-block path is always taken (R27). `block` values whose `blocksize` lands between 2 GiB and 8 EiB (e.g. 64 → 2 TiB) are excluded: the C then `realloc`s a multi-terabyte block, gets `NULL` and dereferences it — undefined behaviour that crashes both libraries identically. | [x] |
| C33 | `stbds_stralloc` | empty string `""` (`len == 1`) | [x] |
| C34 | `stbds_strreset` | arena with 0 / 1 / many blocks, incl. one spliced over-sized block; then re-use the arena afterwards | [x] |
| **L3 — hash map, binary keys (`mode = 0`)** | | | |
| C35 | `stbds_hmput_key` | `a == NULL`, `mode = 0` → fresh 1-element array + `slot_count 8` table, `string.mode = 0` → `memcpy` key path | [x] |
| C36 | `stbds_hmput_key` + `stbds_hmget_key` | 1..7 distinct keys (crosses `used_count_threshold 6` → `slot_count 8→16`), `elemsize/keysize` = 8/4 | [x] |
| C37 | `stbds_hmput_key` + `stbds_hmget_key` | 1..64 distinct keys (crosses 8→16→32→64→128); compare every `temp`, `length`, `capacity`, and the whole bucket array | [x] |
| C38 | `stbds_hmput_key` | re-put of an **existing** key (first scan hit) → `temp` = existing index, no growth | [x] |
| C39 | `stbds_hmput_key` | re-put whose probe hits in the **second** (wrap-around) scan `for (i=0;i<limit;++i)` — note the C deliberately does **not** set `temp_key` there | [x] |
| C40 | `stbds_hmget_key_ts` | absent key / present key / `a == NULL` / `hash_table == NULL`, `*temp` compared | [x] |
| C41 | `stbds_hmget_key` | same as C40 but through the non-`_ts` wrapper (also writes `header->temp`) | [x] |
| C42 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`, no relocation) | [x] |
| C43 | `stbds_hmdel_key` | delete a **middle/first** element (`old_index != final_index` → `memmove` + re-find + index patch) | [x] |
| C44 | `stbds_hmdel_key` | delete until `tombstone_count > tombstone_count_threshold` → rebuild at the same `slot_count` (sc 8: 2 deletes) | [x] |
| C45 | `stbds_hmdel_key` | delete until `used_count < used_count_shrink_threshold && slot_count > 8` → shrink (sc 16 → 8, sc 32 → 16) | [x] |
| C46 | `stbds_hmdel_key` | delete **every** element, then re-insert (tombstone reuse, `pos = tombstone`, `--tombstone_count`, R36) | [x] |
| C47 | `stbds_hmput_key`/`get`/`del` | `keysize` ∈ {1,2,4,8,16} with matching `elemsize` ∈ {8,16,20,24,32,64} | [x] |
| C48 | `stbds_hmdel_key` | `keyoffset` = 0 (normal) and non-zero (R15) | [x] |
| C49 | `stbds_hmput_default` | on `NULL`, on a fresh `hmput_key` map, twice in a row, then `hmget_key` (`length != 0` → returns unchanged) | [x] |
| C50 | `stbds_hmfree_func` | map with `string.mode` 0 / 1 / 2 (STRDUP sweep) / 3 (arena reset); and a map that never got a table | [x] |
| **L3 — hash map, string keys** | | | |
| C51 | `stbds_hmput_key` | `a == NULL`, `mode = 1` → `string.mode` auto-set to `STBDS_SH_DEFAULT` → key pointer **aliased**, `temp_key` set | [x] |
| C52 | `stbds_shmode_func(elemsize, STBDS_SH_STRDUP)` + `hmput_key(mode=1)` | key `strdup`ed; `temp_key` = the new copy; free sweep in `hmfree_func`; `hmdel_key(mode==1)` frees the copy | [x] |
| C53 | `stbds_shmode_func(elemsize, STBDS_SH_ARENA)` + `hmput_key(mode=1)` | key arena-copied; `table->string.{remaining,block}` advance; `strreset` on free | [x] |
| C54 | `stbds_shmode_func(elemsize, STBDS_SH_NONE)` + `hmput_key(mode=1)` | `switch` hits `default:` → raw `memcpy` of `keysize` bytes **of the pointer** (R24) | [x] |
| C55 | `stbds_shmode_func(elemsize, STBDS_SH_DEFAULT)` + `hmput_key(mode=1)` | explicit `SH_DEFAULT` | [x] |
| C56 | string map, all 4 `SH_*` modes | 1..64 distinct string keys (grow chain), keys of length 0,1,7,8,9,600 (arena over-sized block inside the map) | [x] |
| C57 | string map | `hmget_key`/`hmget_key_ts` for present + absent keys; `strcmp`-vs-hash collisions | [x] |
| C58 | string map | duplicate `hmput_key` of the same key → `temp` reuse + `temp_key` propagation (`shputs` relies on it) | [x] |
| C59 | string map, `SH_STRDUP` | `hmdel_key` with `mode == 1` (frees the copy) vs relocation delete | [x] |
| C60 | string map, `SH_ARENA` | `hmdel_key`, `tombstone` rebuild, shrink | [x] |
| **Out-of-range enum values (valid at the ABI, no enumerator)** | | | |
| C61 | `stbds_hmput_key`/`hmget_key`/`hmget_key_ts` | `mode` ∈ {2, 3, 5, 255, 256, `INT_MAX`} → string path (`mode >= 1`) | [x] |
| C62 | `stbds_hmput_key`/`hmget_key`/`hmget_key_ts` | `mode` ∈ {−1, −2, `INT_MIN`} → binary path | [x] |
| C63 | `stbds_hmdel_key` | `mode` ∈ {2, 5, `INT_MAX`}: string hash but `mode != 1` → no strdup free, binary re-lookup form (delete-last only, see A5) | [x] |
| C64 | `stbds_hmdel_key` | `mode` ∈ {−1, `INT_MIN`} → fully binary | [x] |
| C65 | `stbds_shmode_func` | `mode` ∈ {0,1,2,3,4,5,255,256,−1,`INT_MIN`,`INT_MAX`} → `(unsigned char) mode` truncation, then the `switch` in `hmput_key` | [x] |
| **Cross-library interop (proves identical memory layout)** | | | |
| C66 | C `.so` → Rust `.so` | build the map with the **C** functions, then `hmget_key`/`hmdel_key`/`hmfree_func` it with the **Rust** functions, and vice versa; binary + all 4 string modes | [x] |
| C67 | C `.so` ↔ Rust `.so` | arena built by C, `stralloc`/`strreset` continued by Rust and vice versa | [x] |
| C68 | C `.so` ↔ Rust `.so` | array grown by C, grown further + freed by Rust and vice versa | [x] |
| **L4 — composed pipeline (`sh_puts`)** | | | |
| C69 | `sh_puts` | `num` = 0, 1, 2, 3 → stdout captured byte-for-byte | [x] |
| C70 | `sh_puts` | `num` = 31, 32, 33 (first 512-byte arena block boundary: `strkey` gives 6..8-byte strings) | [x] |
| C71 | `sh_puts` | `num` = 100, 200, 1000, 5000 (several arena block-ladder steps) | [x] |
| C72 | `sh_puts` | `num` < 0 (`-1`, `INT_MIN`) → loop skipped | [x] |
| C73 | `sh_puts` | called repeatedly / interleaved between the two libraries with the seed re-synced (global-state independence) | [x] |
| C74 | `sh_puts` | the ABI quirk `printf("%s %d\n", strmap[z], strmap[z].value)` — the 16-byte struct occupies two INTEGER argument registers, so `%s` consumes `.key` and `%d` consumes `.value`; verified by exact stdout match | [x] |

---

## Row → test mapping (every checkbox is auditable)

Every test drives BOTH `.so`s through their exported symbols and, after **each
individual operation**, compares the complete observable state:

* `stbds_array_header`: `length`, `capacity`, `hash_table` nullness, `temp`;
* `stbds_hash_index`: `slot_count`, `used_count`, `used_count_threshold`,
  `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, the `storage`
  alignment invariant, and the embedded `stbds_string_arena`;
* every bucket's full `hash[8]` / `index[8]` array;
* every live element's bytes (string keys compared by content, since heap
  addresses legitimately differ).

| rows | test |
|------|------|
| C1 | `b_pure::c1_hash_bytes_len0_null` |
| C2, C3 | `b_pure::c2_c3_hash_bytes_tail_1_to_7` (~25 000 vectors × 9 seeds) |
| C4, C5, C6, C7 | `b_pure::c4_c7_hash_bytes_block_loop` (25 lengths × 120 vectors × 9 seeds) |
| C8 | `b_pure::c8_hash_bytes_unaligned` (16 offsets × 64 lengths × 3 seeds) |
| C9 | `b_pure::c9_hash_bytes_len_shift_truncation` |
| C10, C11 | `b_pure::c10_c11_hash_string` (fixed corners + 640 random strings × 9 seeds + 2000 random seeds) |
| C12 | `b_pure::c12_global_seed_ladder` (5 start seeds × 40 steps, also checked against the closed-form LCG) |
| C13 | `b_pure::c13_strkey` (18 corner values + 3000 random `int`s) |
| C14 | `b_array::c14_null_zero_zero_returns_null` |
| C15, C16, C17, C23 | `b_array::c15_c16_c17_c23_fresh_capacity_rules` (1200 + 500 random cases) |
| C18 | `b_array::c18_early_out_returns_same_pointer` |
| C19, C20 | `b_array::c19_c20_growth_rules_on_existing_array` |
| C21, C22 | `b_array::c21_c22_append_one_at_a_time`, `b_array::c21_randomized_growth_walks` |
| C24, C33 | `b_arena::c24_c33_fresh_arena_single_string` |
| C25 | `b_arena::c25_block_ladder_many_short_strings` (5 × 3000 strings + 20 mixed trials) |
| C26–C30 | `b_arena::c26_c30_boundaries` |
| C31, C32 | `b_arena::c31_c32_preset_block_field` |
| C34 | `b_arena::c34_strreset_shapes`, `b_arena::c24_c34_randomized_arena_walks` |
| C35 | `b_map_binary::c35_first_insert_from_null` (11 geometries) |
| C36, C37 | `b_map_binary::c36_c37_growth_chain` (11 geometries × 4 seeds × 60 keys) |
| C38, C39 | `b_map_binary::c38_c39_reput_existing_key`, `c_errors::coverage_hmput_key_reaches_all_four_probe_outcomes` (proves first/second × empty/match all occur), `c_errors::coverage_hmput_key_second_scan_match_skips_temp_key` |
| C40, C41 | `b_map_binary::c40_c41_lookup_states` |
| C42, C43 | `b_map_binary::c42_c43_delete_last_and_middle` (24 seeds × 5 geometries, both orders) |
| C44, C45 | `b_map_binary::c44_c45_rebuild_and_shrink`, `d_rehash::make_hash_index_rehash_matches_an_independent_model` (517 grows / 237 shrinks / 484 rebuilds verified against an independent model) |
| C46 | `b_map_binary::c46_tombstone_reuse`, `c_errors::r36_insertion_reuses_a_tombstone` |
| C47, C48 | `b_map_binary::c47_c48_geometry_and_keyoffset` |
| C49 | `b_map_binary::c49_hmput_default` |
| C50 | `b_map_binary::c50_hmfree_states` |
| C51, C55 | `b_map_string::c51_c55_string_default_mode`, `b_map_string::c51_randomized_walks_from_null` |
| C52, C53 | `b_map_string::c52_c53_strdup_and_arena` |
| C54 | `b_map_string::c54_sh_none_with_string_mode` |
| C56 | `b_map_string::c56_all_modes_grow_chain_and_key_lengths` (key lengths 0…1500, all 3 pointer modes) |
| C57 | `b_map_string::c57_string_lookups` |
| C58 | `b_map_string::c58_temp_key_propagation`, `b_map_string::c58_shputs_macro_shape` |
| C59, C60 | `b_map_string::c59_c60_string_deletes`, `d_rehash::rehash_preserves_seed_and_arena_across_all_modes` |
| C61 | `b_enums::c61_stringish_modes_put_get`, `c_errors::r20_out_of_range_mode_takes_the_string_path` |
| C62 | `b_enums::c62_binaryish_modes_put_get`, `c_errors::r21_negative_mode_takes_the_binary_path` |
| C63 | `b_enums::c63_stringish_delete_last_only`, `c_errors::r22_hmdel_stringish_mode_delete_last` |
| C64 | `b_enums::c64_binaryish_delete_all` |
| C65 | `b_enums::c65_shmode_func_truncation`, `c_errors::r23_shmode_func_truncates_mode_to_unsigned_char` |
| C61–C65 | `b_enums::c61_c65_randomized_enum_sweep` (200 randomized (mode, sh, elemsize, key-set) trials) |
| C66 | `b_interop::c66_map_interop_binary`, `b_interop::c66_map_interop_string_all_modes` (5 interleavings × 4 geometries / 4 modes) |
| C67 | `b_interop::c67_arena_interop` |
| C68 | `b_interop::c68_array_interop` |
| C69, C74 | `b_shputs::c69_c74_small_num` |
| C70 | `b_shputs::c70_arena_block_boundary`, `b_shputs::c70_strkey_widths_inside_shputs_range` |
| C71 | `b_shputs::c71_many_arena_blocks` |
| C72 | `b_shputs::c72_negative_num` |
| C73 | `b_shputs::c73_repeated_and_interleaved` |
| C69–C72 | `b_shputs::c69_c72_randomized_num` (400 randomized values) |

### Supporting tests that make the above meaningful

| test | what it establishes |
|------|---------------------|
| `a_symbols::nm_symbol_diff_is_empty` | the C and Rust `.so`s export exactly the same 16 symbols |
| `a_symbols::rust_so_has_no_unresolved_non_libc_symbols` | nothing but libc / unwinder is left undefined |
| `a_symbols::every_symbol_loads_from_both_libraries` | every symbol is `dlsym`-able from both |
| `a_symbols::libraries_do_not_interpose` | each `.so` binds its internal calls to its *own* definitions (different `stbds_hash_seed` statics prove it) — without this, every "differential" test would be comparing a library with itself |
| `a_symbols::mirror_struct_sizes`, `a_symbols::hash_index_storage_alignment_matches` | the harness's `#[repr(C)]` mirrors match the C compiler's layout |
| `b_interop::*` | a structure built by one library can be grown, queried, mutated and freed by the other — the strongest available proof of byte-identical layout |
| `b_interop::c66_c68_hash_agreement_under_shared_tables` | 5000 random (key, seed) pairs hash identically in both libraries |

## Feature combinations

`Cargo.toml` declares no `[features]`, so the default build is the only
buildable configuration. `check_all_features.sh` proves this mechanically and
runs the *entire* suite under:

* the default profile,
* `--no-default-features`,
* `--all-features`,
* and, additionally, against the **debug** artifact
  (`RUST_TRANSLATION_SO=target/debug/libsh_puts_lib.so`), which is compiled with
  integer-overflow checks **on** and `panic = unwind` — a different code path for
  every arithmetic operation in the translation.

Result: `ALL CONFIGURATIONS PASS`, symbol diff empty in each.
