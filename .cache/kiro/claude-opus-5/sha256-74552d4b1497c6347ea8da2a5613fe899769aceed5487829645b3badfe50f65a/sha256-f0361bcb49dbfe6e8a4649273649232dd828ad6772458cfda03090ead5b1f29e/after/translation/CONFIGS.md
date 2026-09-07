# CONFIGS.md — Phase A configuration surface (valid inputs)

Axes derived mechanically from the branches the C actually takes.

## Axis inventory

**A1 — `mode` argument of `stbds_hmput_key` / `stbds_hmget_key{,_ts}` / `stbds_hmdel_key`**
Branch points: `mode >= STBDS_HM_STRING` (lines 560, 590, 713),
`mode == STBDS_HM_STRING` exactly (lines 836, 842).
Distinct values: `0` (BINARY), `1` (STRING), `2` (≥STRING but ≠STRING), `-1`, large ints.

**A2 — `stbds_string_arena.mode` = the `switch (table->string.mode)` in `hmput_key` (line 785)**
`STBDS_SH_DEFAULT (1)` / `STBDS_SH_STRDUP (2)` / `STBDS_SH_ARENA (3)` / `default` (0 or anything else).
Set either implicitly by `hmput_key` (line 707: `mode>=1 ? SH_DEFAULT : 0`) or
explicitly by `stbds_shmode_func` (line 803, truncating `(unsigned char) mode`).

**A3 — `elemsize` / `keysize` shapes**
`elemsize` is `sizeof *t` and `keysize` is `sizeof t->key`. Distinct shapes:
8/8 (pointer-key string map), 16/8 (`{char*;char}` — the `helxo` shape),
8/4 (`{int key; int value}`), 16/8 (`{int key[2]; int b,c,d}` = `stbds_struct2`, keysize 8),
24/8, 4/4, and `elemsize > keysize` vs `elemsize == keysize`.

**A4 — `keyoffset` of `stbds_hmdel_key`** — `0` (all macros pass `STBDS_OFFSETOF(t,key)` which is 0
for a leading `key` field) and non-zero (a struct whose key is not first).

**A5 — table population / growth stage** (drives `stbds_make_hash_index` slot_count and rehash)
`used_count_threshold = slot_count - slot_count/4` ⇒ growth at 6 (sc 8), 12 (16), 24 (32), 48 (64) …
Shapes: 0 elements, 1, 5 (just under first grow), 6 (at grow), 7, 12, 13, 24, 25, 100, 1000.

**A6 — delete patterns** (drive shrink / tombstone-rebuild in `hmdel_key`)
`used_count < used_count_shrink_threshold (slot_count/4) && slot_count > 8` ⇒ shrink;
`tombstone_count > (slot_count>>3)+(slot_count>>4)` ⇒ same-size rebuild;
delete-last (`old_index == final_index`) vs delete-middle (swap + re-find);
delete-then-reinsert (tombstone reuse); delete-absent.

**A7 — global seed (`stbds_hash_seed`, `stbds_rand_seed`)**
default `0x31415926`; `0`; `SIZE_MAX`; arbitrary. It also **advances** on every
fresh `stbds_make_hash_index` (line 412) so seed state is order-dependent.

**A8 — `stbds_hash_bytes` input shape** — `len % 8` ∈ {0..7} (the fall-through switch),
`len == 0`, `len` < 8, spanning multiple 8-byte blocks, bytes ≥ 0x80 (sign-extension in
`d[3] << 24`), NULL+0.

**A9 — `stbds_hash_string` input shape** — empty, 1 char, long, bytes ≥ 0x80, embedded NUL-terminated.

**A10 — `stbds_arrgrowf` shape** — `a == NULL` vs existing; `addlen` 0/1/n; `min_cap`
below/at/above cap; the `<4` clamp; the `2*cap` doubling clamp; `elemsize` 1/4/8/16/24.

**A11 — `stbds_stralloc` arena state** — fresh arena (`{0,0,0,mode}`); `remaining` ≥ len;
`remaining` < len with `len <= blocksize`; oversized string (`len > blocksize`) with
`storage == NULL` and with `storage != NULL`; `block` at 0/1/2/21/22/128/110.

**A12 — `helxo` argument** — every `char` value (it is the sole `include/lib.h` API).

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | randomized buffers, `len` 0..64 covering every `len%8`, bytes uniform 0..255, default seed | [x] |
| 2 | `stbds_hash_bytes` | same buffers, seed = 0 | [x] |
| 3 | `stbds_hash_bytes` | same buffers, seed = `SIZE_MAX` | [x] |
| 4 | `stbds_hash_bytes` | same buffers, randomized 64-bit seeds | [x] |
| 5 | `stbds_hash_bytes` | long buffers, `len` 65..600 (many siphash blocks) | [x] |
| 6 | `stbds_hash_bytes` | `p = NULL`, `len = 0`, several seeds | [x] |
| 7 | `stbds_hash_string` | randomized NUL-terminated strings, len 0..64, bytes 1..255 (incl. ≥0x80), default seed | [x] |
| 8 | `stbds_hash_string` | same strings, seeds {0, 1, SIZE_MAX, random} | [x] |
| 9 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to {0, 1, 0x31415926, SIZE_MAX, random}, then observe the seed baked into a fresh index and the advanced global seed via a second index | [x] |
| 10 | `stbds_arrgrowf` / `stbds_arrfreef` | `a=NULL`, elemsize ∈ {1,4,8,16,24}, addlen ∈ {0,1,3,7}, min_cap ∈ {0,1,2,3,4,5,9,100} → header length/capacity/hash_table/temp | [x] |
| 11 | `stbds_arrgrowf` | existing array, repeated growth (doubling chain), verify capacity sequence + preserved length/temp/hash_table | [x] |
| 12 | `stbds_arrgrowf` | existing array, `min_cap <= cap` (early return, identity) | [x] |
| 13 | `stbds_arrgrowf` | randomized `(elemsize, addlen, min_cap)` triples incl. huge `addlen`/`min_cap` (wrap) | [x] |
| 14 | `stbds_hmput_key` | mode=BINARY(0), elemsize=8 keysize=4 (int key), 0→1→5→6→7→13→25→100 inserts, default seed | [x] |
| 15 | `stbds_hmput_key` | mode=BINARY(0), elemsize=16 keysize=8 (`stbds_struct2`-shape), randomized keys, 1000 inserts | [x] |
| 16 | `stbds_hmput_key` | mode=BINARY(0), elemsize=keysize=8, randomized keys with **duplicates** (exercises both duplicate-found loops) | [x] |
| 17 | `stbds_hmput_key` | mode=BINARY(0), elemsize=24 keysize=8, keys crafted so `hash < 2` cannot be forced but many collisions occur (small key domain) | [x] |
| 18 | `stbds_hmput_key` | mode=STRING(1), `string.mode = SH_DEFAULT` (implicit, table==NULL path), randomized string keys, 1..200 inserts, elemsize 16/keysize 8 | [x] |
| 19 | `stbds_hmput_key` | mode=STRING(1) with duplicate string keys re-put (value overwrite path + `temp_key`) | [x] |
| 20 | `stbds_shmode_func`+`stbds_hmput_key` | `SH_STRDUP (2)`, mode=STRING, randomized keys, 1..200 inserts, then `hmfree_func` (frees the dups) | [x] |
| 21 | `stbds_shmode_func`+`stbds_hmput_key` | `SH_ARENA (3)`, mode=STRING, randomized keys incl. very long (> blocksize) ones, 1..200 inserts | [x] |
| 22 | `stbds_shmode_func`+`stbds_hmput_key` | `SH_NONE (0)` with mode=STRING → `default:` memcpy of the key **pointer bytes** | [x] |
| 23 | `stbds_shmode_func`+`stbds_hmput_key` | `SH_DEFAULT (1)` explicitly, mode=STRING | [x] |
| 24 | `stbds_shmode_func` | out-of-range `mode` ∈ {-1, 4, 5, 255, 256, 259, 1000, INT_MAX, INT_MIN} → truncated `string.mode` | [x] |
| 25 | `stbds_hmget_key_ts` | BINARY, populated table, mix of present/absent keys, randomized | [x] |
| 26 | `stbds_hmget_key_ts` | STRING, populated table, mix of present/absent keys, randomized | [x] |
| 27 | `stbds_hmget_key_ts` | `a == NULL` (bootstrap path) for several elemsizes | [x] |
| 28 | `stbds_hmget_key_ts` | array built by `hmput_default` only (hash_table == 0) | [x] |
| 29 | `stbds_hmget_key` | same as 25/26 but through the non-`_ts` wrapper (checks `stbds_header->temp` write) | [x] |
| 30 | `stbds_hmput_default` | `a == NULL`; then again on the returned pointer (length!=0 → identity); elemsize ∈ {4,8,16,24} | [x] |
| 31 | `stbds_hmput_default` + `hmput_key` | default-then-insert interleaving, BINARY and STRING | [x] |
| 32 | `stbds_hmdel_key` | BINARY, keyoffset=0, delete present middle key (swap-with-last + re-find) | [x] |
| 33 | `stbds_hmdel_key` | BINARY, keyoffset=0, delete the last element (`old_index == final_index`) | [x] |
| 34 | `stbds_hmdel_key` | BINARY, keyoffset=0, delete absent key (temp stays 0) | [x] |
| 35 | `stbds_hmdel_key` | BINARY, delete enough to cross `used_count_shrink_threshold` with `slot_count > 8` (shrink rebuild) | [x] |
| 36 | `stbds_hmdel_key` | BINARY, delete/reinsert pattern crossing `tombstone_count_threshold` (same-size rebuild + tombstone reuse) | [x] |
| 37 | `stbds_hmdel_key` | STRING mode=1, `SH_DEFAULT` arena, randomized delete order | [x] |
| 38 | `stbds_hmdel_key` | STRING mode=1, `SH_STRDUP` arena (frees the dup) | [x] |
| 39 | `stbds_hmdel_key` | STRING mode=1, `SH_ARENA` arena | [x] |
| 40 | `stbds_hmdel_key` | mode=2 (≥STRING but ≠STRING) on a string table — hashing via `strcmp`, re-find via raw bytes | [x] |
| 41 | `stbds_hmdel_key` | BINARY, non-zero `keyoffset` (key not the first field), elemsize 16 keysize 8 keyoffset 8 | [x] |
| 42 | `stbds_hmdel_key` | randomized full lifecycle: N random puts, random gets, random deletes, random re-puts (500 ops), BINARY | [x] |
| 43 | `stbds_hmdel_key` | randomized full lifecycle, STRING/`SH_STRDUP` | [x] |
| 44 | `stbds_hmfree_func` | on a `SH_STRDUP` table, `SH_ARENA` table, `SH_DEFAULT` table, and a table with `hash_table == 0` | [x] |
| 45 | `stbds_stralloc` | fresh arena `{0,0,0,mode}`, sequence of randomized strings (len 0..2000) — carve + new-block + oversized paths | [x] |
| 46 | `stbds_stralloc` | pre-set `block` ∈ {0,1,2,3,10,21,22,23,110,111,128,255}, single string | [x] |
| 47 | `stbds_stralloc` | pre-set `remaining` ∈ {0,1,len-1,len,len+1} with a live block | [x] |
| 48 | `stbds_stralloc` | oversized string with `storage == NULL` vs `storage != NULL` (different `next` chaining) | [x] |
| 49 | `stbds_strreset` | empty arena, 1-block arena, multi-block arena (after a stralloc sequence) | [x] |
| 50 | `strkey` | `n` ∈ randomized `i32` incl. 0, ±1, INT_MIN, INT_MAX; repeated calls (shared buffer) | [x] |
| 51 | `helxo` | all 256 `char` values, stdout captured and compared byte-for-byte | [x] |
| 52 | `helxo` | called repeatedly (global seed advances between calls → different iteration order) | [x] |
| 53 | end-to-end | replicate the C's own `helxo` pipeline manually via the low-level exports (`hmput_key`+`hmfree_func` with elemsize 16/keysize 8, mode STRING) and compare the whole structure | [x] |
| 54 | interleaving | two independent tables alive at once (shared global seed advances) — put/del interleaved across both | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)`; there is
no `add_executable`. Rust `Cargo.toml` declares only `crate-type = ["cdylib"]` and has
no `src/main.rs`. **No driver binary exists**, so the "compare stdout of the C and Rust
binaries" clause is satisfied vacuously; `helxo`'s stdout is compared instead
(rows 51–52) by running each `.so` in a forked child with stdout redirected.

## Row → test map

All rows are checked off, each driven with many randomized inputs from a fixed
seed (`common::Rng`, splitmix64). Every call in every test goes through
`libloading` into the `.so` exports — the Rust crate is never linked directly.

| rows | test |
|------|------|
| 1-6, 8, 9 | `phase_b_lowlevel.rs::row01_…` … `row06_…`, `row08_hash_bytes_high_bit_bytes_every_remainder` |
| 7-8 | `row07_hash_string_random`, `row08_hash_string_random_seeds_and_high_bytes` |
| 9 | `row09_rand_seed_and_seed_advance` |
| 10-13 | `row10_arrgrowf_from_null`, `row11_arrgrowf_growth_chain`, `row12_arrgrowf_early_return_identity`, `row13_arrgrowf_random_triples`, `row13b_arrgrowf_size_wraparound` |
| 14-17 | `phase_b_hashmap.rs::row14_…` … `row17_binary_elemsize24_collisions` |
| 18-19 | `row18_string_implicit_sh_default`, `row19_string_duplicates_and_temp_key` |
| 20-23 | `row20_sh_strdup`, `row21_sh_arena_incl_long_keys`, `row22_sh_none_with_string_mode_single_insert`, `row23_sh_default_explicit` |
| 24 | `row24_shmode_func_out_of_range_modes` |
| 25-29 | `row25_…`, `row26_…`, `row27_hmget_bootstrap_from_null`, `row28_hmget_on_hmput_default_only_array` |
| 30-31 | `row30_hmput_default_shapes`, `row31_hmput_default_then_insert` |
| 32-34 | `row32_row33_row34_del_middle_last_absent` |
| 35-36 | `row35_row36_del_shrink_and_tombstone_rebuild` |
| 37-39 | `row37_del_string_sh_default`, `row38_del_string_sh_strdup`, `row39_del_string_sh_arena` |
| 40 | `row40_del_mode_two_string_table` |
| 41 | `row41_del_nonzero_keyoffset` |
| 42-43 | `row42_random_lifecycle_binary`, `row43_random_lifecycle_string_strdup` |
| 44 | `row44_hmfree_all_arena_modes` |
| 45-49 | `phase_b_lowlevel.rs::row45_stralloc_random_sequence` … `row49_strreset_shapes` |
| 50 | `row50_strkey` |
| 51-52 | `phase_b_helxo.rs::row51_helxo_all_char_values`, `row51b_…`, `row52_…` |
| 53 | `phase_b_hashmap.rs::row53_helxo_pipeline_via_low_level_exports` |
| 54 | `row54_two_interleaved_tables` |

### Harness note

`stbds_hmput_key` only defines the first `keysize` bytes (raw-key `default:` arm)
or the first 8 bytes (the three pointer-storing arena arms) of a freshly inserted
element; the rest is whatever `realloc` returned, and the C and Rust libraries
have independent allocation histories. The harness therefore normalises those
bytes (`common::normalize_last_elem`, `Cfg::lib_writes`) exactly the way a real
consumer's `t[i].key = k; t[i].value = v;` would, before comparing element bytes.
The same applies to `table->temp_key`, which `stbds_make_hash_index` never
initialises — it is compared only while the harness knows a `stbds_temp_key`
assignment has happened on the current index object (`Map::tk_valid`).

## Verification run

`./run_all.sh` builds the C `.so`, enumerates the feature combinations from
`Cargo.toml`, and for each combination × profile rebuilds the Rust cdylib,
diffs `nm -D`, and runs the whole suite. Last run: **89 tests, 0 failures,
0 missing symbols, in all 4 configurations** (release/debug × default/
`--no-default-features`).
