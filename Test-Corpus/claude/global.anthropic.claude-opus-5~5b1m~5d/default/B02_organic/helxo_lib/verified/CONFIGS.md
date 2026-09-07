# CONFIGS.md — Phase B configuration surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**A. Entry point** (all 16 exported symbols, including the lowest-level ones —
`stbds_arrgrowf`, `stbds_hash_bytes`, `stbds_hash_string`, `stbds_stralloc`,
`stbds_strreset`, `stbds_shmode_func` — not just the map wrappers).

**B. `mode` argument** (`lib.c:560,590,713,836,842`) — the code tests
`mode >= STBDS_HM_STRING(1)` in `is_key_equal` / `hm_find_slot` / `hmput_key`,
but tests `mode == STBDS_HM_STRING` exactly in `hmdel_key`. Distinct classes:
`mode < 0`, `mode == 0` (BINARY), `mode == 1` (STRING), `mode >= 2`.

**C. `table->string.mode`** (`lib.c:785`, set by `shmode_func` or defaulted by
`hmput_key`): `SH_NONE(0)` → raw `memcpy` of the key, `SH_DEFAULT(1)` → store
caller's pointer, `SH_STRDUP(2)` → `malloc`+copy per key (and per-key `free` on
delete/free), `SH_ARENA(3)` → `stralloc` from the arena. Plus out-of-range
values truncated by `(unsigned char)`.

**D. `elemsize` / `keysize` shape** — `keysize` 1/2/4/8/16 bytes (`char`,
`short`, `int`, `long`, `int key[2]`), `elemsize` from 8 to 40, `elemsize == 0`,
`keysize < elemsize` vs `keysize == elemsize`, key sizes that are and are not a
multiple of `sizeof(size_t)` (drives the siphash tail `switch`).

**E. `keyoffset`** (only `hmdel_key` takes it; `hmput_key`/`hmget_key` hard-code
0): 0 and non-zero.

**F. Table-size / occupancy shape** — `slot_count` starts at
`STBDS_BUCKET_LENGTH == 8`. Boundaries the code special-cases:
`used_count_threshold = slot_count - slot_count/4` (6 at 8 slots → grow),
`used_count_shrink_threshold = slot_count/4` forced to 0 when
`slot_count <= 8`, `tombstone_count_threshold = (slot_count>>3)+(slot_count>>4)`
(1 at 8 slots → rebuild). Element counts to exercise: 0, 1, 2, 5, 6, 7, 8, 9,
16, 17, 100, 1000.

**G. Hash-index provenance** — `make_hash_index(n, NULL)` (advances the global
`stbds_hash_seed`) vs `make_hash_index(n, ot)` (inherits seed+arena). Reached
via first insert / grow / shrink / tombstone-rebuild.

**H. Global seed** (`stbds_rand_seed`) — default `0x31415926`, `0`, `1`,
`usize::MAX`, random. Because a table's seed is drawn from the global and the
global is then advanced, **both `.so`s must be re-seeded identically before
every scenario**; every test below does so.

**I. `stbds_hash_bytes` length shape** — `len` 0..24 covering all 8 tail
`switch` cases plus 1, 2 and 3 whole `size_t` body iterations; byte values
`< 0x80` vs `>= 0x80` (the `d[3] << 24` sign-extension path).

**J. `stbds_arrgrowf` shape** — `a` NULL vs existing; `addlen` 0/1/3/1000;
`min_cap` 0/1/4/5/exactly-cap/cap+1/huge; the doubling-vs-min_cap-vs-4 ladder.

**K. String-arena shape** (`stbds_stralloc`) — `len <= remaining`;
`len > remaining` with `len <= blocksize` (new 512<<(block/2) block);
`len > blocksize` (dedicated oversized block, two sub-cases: `storage == NULL`
and `storage != NULL`); `block` counter progression 0,1,2,…; empty string.

**L. Operation sequence** — insert-only; insert+lookup; insert+delete+reinsert
(tombstone reuse); delete-all; interleaved random op streams; duplicate-key
overwrite; delete of the last vs a middle element (the `memmove` fix-up path).

## Rows (cross-product pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_string` | seed = default / 0 / 1 / `usize::MAX` / random; str = `""` | [x] |
| 2 | `stbds_hash_string` | random ASCII strings, len 1..64, 4 seeds | [x] |
| 3 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (unsigned-char path), len 1..64 | [x] |
| 4 | `stbds_hash_bytes` | `len == 0` (pointer unread), 4 seeds | [x] |
| 5 | `stbds_hash_bytes` | `len == 1..7` (each tail `switch` case), random bytes `< 0x80` | [x] |
| 6 | `stbds_hash_bytes` | `len == 1..7`, random bytes with high bit set (`d[3]` sign-extension in tail) | [x] |
| 7 | `stbds_hash_bytes` | `len == 8` / `16` / `24` (whole body iterations, empty tail) | [x] |
| 8 | `stbds_hash_bytes` | `len == 9..23` (body + every tail remainder), random bytes full `0x00..0xFF` | [x] |
| 9 | `stbds_hash_bytes` | `len` 0..64 random, seed random, 512 iterations (property sweep) | [x] |
| 10 | `stbds_rand_seed` + `stbds_hash_bytes`/`_string` | seed set explicitly, then hash — confirms the global is only read by index creation, not by the hash fns | [x] |
| 11 | `stbds_arrgrowf` | `a=NULL`, `elemsize` ∈ {1,4,8,16,40}, `addlen=0`, `min_cap=0` → returns NULL | [x] |
| 12 | `stbds_arrgrowf` | `a=NULL`, `addlen=0`, `min_cap=1` → cap raised to 4; header initialised | [x] |
| 13 | `stbds_arrgrowf` | `a=NULL`, `addlen=1..1000`, `min_cap=0` → cap = max(min_len,4) | [x] |
| 14 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → identical pointer, no change | [x] |
| 15 | `stbds_arrgrowf` | existing array, `min_cap = cap+1` → doubling ladder (`min_cap = 2*cap`) | [x] |
| 16 | `stbds_arrgrowf` | existing array, `min_cap = 5*cap` → `min_cap` wins over doubling | [x] |
| 17 | `stbds_arrgrowf` | repeated growth chain (16 successive `arrgrowf` calls), tracking `length`/`capacity`/`temp`/`hash_table` after each | [x] |
| 18 | `stbds_arrgrowf` | `elemsize == 0` | [x] |
| 19 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (valgrind-clean round trip) | [x] |
| 20 | `stbds_hmput_default` | `a == NULL`, `elemsize` ∈ {8,16,24,40} | [x] |
| 21 | `stbds_hmput_default` | called twice — second call must return the *same* pointer | [x] |
| 22 | `stbds_hmput_default` then `stbds_hmget_key` | `hash_table == NULL` path (`*temp == -1`) | [x] |
| 23 | `stbds_hmput_key` BINARY (`mode=0`) | `keysize=4`, `elemsize=16`; 1 key | [x] |
| 24 | `stbds_hmput_key` BINARY | `keysize=4`, `elemsize=16`; 5 keys (below grow threshold) | [x] |
| 25 | `stbds_hmput_key` BINARY | 6 keys — hits `used_count >= used_count_threshold`, first table doubling | [x] |
| 26 | `stbds_hmput_key` BINARY | 100 keys — several doublings + rehash of a populated `ot` | [x] |
| 27 | `stbds_hmput_key` BINARY | 1000 random `i32` keys with duplicates, checking `stbds_temp` after every insert | [x] |
| 28 | `stbds_hmput_key` BINARY | `keysize=1` / `2` / `8` / `16` (siphash tail cases via the key) | [x] |
| 29 | `stbds_hmput_key` BINARY | `keysize == elemsize` (no value payload) | [x] |
| 30 | `stbds_hmput_key` BINARY | duplicate key re-put — `stbds_temp` = original index, length unchanged | [x] |
| 31 | `stbds_hmput_key` BINARY | random keys under 4 different global seeds (different probe orders) | [x] |
| 32 | `stbds_hmget_key` BINARY | lookup of present and absent keys after N inserts, all N | [x] |
| 33 | `stbds_hmget_key_ts` BINARY | same as 32 but through the `temp`-out-param low-level entry point | [x] |
| 34 | `stbds_hmdel_key` BINARY | delete the **last** element (`old_index == final_index`, no memmove) | [x] |
| 35 | `stbds_hmdel_key` BINARY | delete a **middle** element (memmove + re-find + index fix-up) | [x] |
| 36 | `stbds_hmdel_key` BINARY | delete absent key (`temp == 0`, length unchanged) | [x] |
| 37 | `stbds_hmdel_key` BINARY | delete until `tombstone_count > threshold` → same-size rebuild | [x] |
| 38 | `stbds_hmdel_key` BINARY | 40 inserts then delete down past `used_count_shrink_threshold` with `slot_count > 8` → shrink+rehash | [x] |
| 39 | `stbds_hmdel_key` BINARY | delete-all then re-insert (tombstone reuse in `hmput_key`) | [x] |
| 40 | `stbds_hmdel_key` BINARY | `keyoffset != 0` (key not the first field of the element) | [x] |
| 41 | full BINARY pipeline | 2000-op randomized stream (put/get/del/len) on `{i32 key; i32 pad; i64 value}`, full array + header + every lookup compared each step | [x] |
| 42 | full BINARY pipeline | same stream with `keysize=8`, `elemsize=24` | [x] |
| 43 | full BINARY pipeline | same stream with `keysize=16` (`int key[2]`), `elemsize=32` | [x] |
| 44 | `stbds_hmput_key` STRING, implicit `SH_DEFAULT` | `a=NULL`, `mode=1` → `string.mode` auto-set to `SH_DEFAULT`; caller's pointers stored | [x] |
| 45 | STRING/`SH_DEFAULT` | 1 / 5 / 6 / 9 / 100 distinct string keys (grow boundary) | [x] |
| 46 | STRING/`SH_DEFAULT` | duplicate string key — `stbds_temp_key` written, old pointer kept | [x] |
| 47 | STRING/`SH_DEFAULT` | `""` as a key | [x] |
| 48 | STRING/`SH_DEFAULT` | keys with high-bit bytes | [x] |
| 49 | `stbds_shmode_func(SH_STRDUP)` + put/get/del | 1 / 6 / 9 / 100 keys; per-key `strdup`; delete frees the dup | [x] |
| 50 | `stbds_shmode_func(SH_ARENA)` + put/get/del | 1 / 6 / 9 / 100 keys; keys copied into the arena | [x] |
| 51 | `stbds_shmode_func(SH_ARENA)` | long keys (> 512-byte first block) forcing the `len > blocksize` dedicated-block path *inside* the map | [x] |
| 52 | `stbds_shmode_func(SH_NONE)` | `mode` 0 → `hmput_key`'s `default:` memcpy branch even with `mode=STRING` at call time | [x] |
| 53 | `stbds_shmode_func(SH_DEFAULT)` | explicit `SH_DEFAULT` (vs the implicit one in row 44) | [x] |
| 54 | `shmode_func` + `hmput_key` | `mode >= 2` (out-of-range enum, e.g. 2, 7, 1000, `INT_MAX`) → string path in put/get, **binary** path in `hmdel_key` | [x] |
| 55 | `shmode_func` + `hmput_key` | `mode < 0` (`-1`, `INT_MIN`) → binary path everywhere | [x] |
| 56 | `shmode_func` | `mode` 4 / 256 / 1000 / -1 (`(unsigned char)` truncation) then a put | [x] |
| 57 | full STRING pipeline | 1000-op randomized stream over `SH_DEFAULT`, comparing array contents, header, temp, and every lookup | [x] |
| 58 | full STRING pipeline | same stream over `SH_STRDUP` (compares string *contents*, since pointers differ per library) | [x] |
| 59 | full STRING pipeline | same stream over `SH_ARENA` | [x] |
| 60 | `stbds_hmfree_func` | after a `SH_DEFAULT` map (no per-key frees) | [x] |
| 61 | `stbds_hmfree_func` | after a `SH_STRDUP` map with 100 keys (per-key frees + `strreset`) | [x] |
| 62 | `stbds_hmfree_func` | after a `SH_ARENA` map with 100 long keys (`strreset` frees the block chain) | [x] |
| 63 | `stbds_hmfree_func` | on an array with `hash_table == NULL` (from `hmput_default` only) | [x] |
| 64 | `stbds_stralloc` | fresh arena, short string (`len <= 512`) → first block, `block` 0→1, `remaining` bookkeeping | [x] |
| 65 | `stbds_stralloc` | repeated short strings until `remaining` exhausted → second block, `block` 1→2, blocksize `512<<(block/2)` progression | [x] |
| 66 | `stbds_stralloc` | 40 successive allocations of random lengths 1..300, recording every returned string + `remaining`/`block` after each | [x] |
| 67 | `stbds_stralloc` | `len > blocksize` with `storage == NULL` → dedicated block becomes storage, `remaining = 0` | [x] |
| 68 | `stbds_stralloc` | `len > blocksize` with `storage != NULL` → dedicated block spliced in as `storage->next` | [x] |
| 69 | `stbds_stralloc` | empty string `""` (`len == 1`) | [x] |
| 70 | `stbds_stralloc` | `block` driven to saturation region (blocksize `>= 1<<20`, `block` stops incrementing) | [x] |
| 71 | `stbds_strreset` | on a zeroed arena (no storage) — idempotent | [x] |
| 72 | `stbds_strreset` | on an arena with a multi-block chain, then reuse of the arena | [x] |
| 73 | `strkey` | `n` = 0, 1, -1, 42, `i32::MAX`, `i32::MIN`, plus 64 random ints | [x] |
| 74 | `strkey` | two calls in a row — returned pointer is the same static buffer both times | [x] |
| 75 | `helxo` | `letter` = `'w'`, `'A'`, `'0'`, `'\0'`, `'\n'`, `0x7f`, `-1` (`0xFF`), plus all 256 byte values; stdout captured and compared byte-for-byte | [x] |
| 76 | `helxo` | called repeatedly (16x) — stdout identical each time and independent of the global seed | [x] |
| 77 | cross-cutting | every one of rows 23–63 repeated under global seed ∈ {default, 0, 1, `usize::MAX`, 0xdeadbeef} | [x] |
| 78 | all map entry points + `stbds_arrgrowf` | FIVE maps of different shapes/arena modes (`SH_NONE` implicit, `SH_DEFAULT`, `SH_STRDUP`, `SH_ARENA`) plus two raw arrays all alive simultaneously, 1500 interleaved random ops; every map's full state re-compared after every step. Reaches seed values no single-map test can, because each fresh `stbds_hash_index` consumes and advances the one process-global `stbds_hash_seed` | [x] |

## Row → test mapping (all rows PASS; run `./run_matrix.sh`)

| rows | test |
|------|------|
| 1–3 | `tests/lowlevel.rs::row01_hash_string_empty_and_fixed_seeds`, `row02_hash_string_random_ascii`, `row03_hash_string_high_bit_bytes` |
| 4–9 | `tests/lowlevel.rs::row04_hash_bytes_len0`, `rows05_08_hash_bytes_every_length_0_to_24`, `row09_hash_bytes_property_sweep` |
| 10 | `tests/lowlevel.rs::row10_rand_seed_does_not_affect_hash_fns` |
| 11–13, 18 | `tests/lowlevel.rs::rows11_13_18_arrgrowf_fresh` |
| 14–17 | `tests/lowlevel.rs::rows14_17_arrgrowf_growth_ladder` |
| 19 | `tests/lowlevel.rs::row19_arrgrowf_randomised_chains` |
| 20–22 | `tests/hashmap.rs::rows20_22_hmput_default` |
| 23–26, 28, 29, 32, 33 | `tests/hashmap.rs::rows23_28_binary_counts_and_keysizes` |
| 27, 30, 31 | `tests/hashmap.rs::rows27_30_31_binary_duplicates_and_seeds` |
| 34–36, 39 | `tests/hashmap.rs::rows34_36_39_delete_last_middle_absent` |
| 37, 38, 43 | `tests/hashmap.rs::rows37_38_41_43_tombstone_rebuild_and_shrink` |
| 40 | `tests/hashmap.rs::row40_hmdel_nonzero_keyoffset` |
| 41–43 | `tests/hashmap.rs::rows41_43_random_binary_streams` |
| 44–48, 53 | `tests/hashmap.rs::rows44_48_string_default_mode` |
| 49 | `tests/hashmap.rs::row49_sh_strdup` |
| 50, 51 | `tests/hashmap.rs::rows50_51_sh_arena` |
| 52, 56, 60 | `tests/hashmap.rs::rows52_56_shmode_out_of_range_and_none` |
| 54 | `tests/hashmap.rs::row54_mode_ge_2_is_string_in_put_but_binary_in_del` |
| 55 | `tests/hashmap.rs::row55_negative_mode_is_binary` |
| 57 | `tests/hashmap.rs::row57_random_stream_sh_default` |
| 58 | `tests/hashmap.rs::row58_random_stream_sh_strdup` |
| 59 | `tests/hashmap.rs::row59_random_stream_sh_arena` |
| 60–63 | `tests/hashmap.rs::rows60_63_hmfree_all_modes` |
| 64–66 | `tests/lowlevel.rs::rows64_66_stralloc_short_strings` |
| 67–69 | `tests/lowlevel.rs::rows67_69_stralloc_oversized_and_empty` |
| 70 | `tests/lowlevel.rs::row70_stralloc_block_counter_saturation` |
| 71, 72 | `tests/lowlevel.rs::rows71_72_strreset` |
| 73, 74 | `tests/lowlevel.rs::rows73_74_strkey` |
| 75, 76 | `tests/lowlevel.rs::rows75_76_helxo_stdout_all_bytes`, `row76_helxo_repeated_and_seed_independent` |
| 78 | `tests/hashmap.rs::row78_interleaved_multiple_maps_and_arrays` |
| 77 | every map test above loops over `SEED_SET = [0x31415926, 0, 1, usize::MAX, 0xdeadbeef]` and calls `stbds_rand_seed` on BOTH `.so`s before each scenario |

### What "passes" means here

For map rows the comparison is white-box and total, not just "the return value
matched". After **every single operation** the test compares, between the two
`.so`s:

* `stbds_array_header`: `length`, `capacity`, `temp` (and `hash_table`
  NULL-ness);
* every live element's bytes — with the key rendered as raw bytes (binary mode)
  or as the pointed-to C string (string modes, since the two heaps hand out
  different addresses);
* the whole `stbds_hash_index`: `slot_count`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, and the embedded
  arena's `remaining` / `block` / `mode` / storage-NULL-ness;
* **every one of the 8 `hash[]` and 8 `index[]` entries of every bucket** — so
  a divergence in probe order, tombstone placement, rehash order or the
  `if (hash < 2) hash += 2` guard is caught immediately, not just eventually;
* `stbds_temp_key`, at the one point the C actually defines it (right after a
  `hmput_key` that inserted a new element).

Heap addresses are never compared, since they legitimately differ.

### Non-comparable-by-construction values (deliberately excluded)

* `stbds_hash_index.temp_key` outside a fresh insert — `stbds_make_hash_index`
  never initialises it (`realloc` garbage), and the duplicate-key path in the
  wrap-around probe loop (`lib.c:746-751`) does not assign it.
* Struct padding / bytes past `keysize` in an element the caller never wrote —
  the real `stbds_hmput` macro leaves those undefined, so every test writes the
  full non-key remainder of each element itself.
* Bytes between `length` and `capacity` — uninitialised `realloc` memory.
