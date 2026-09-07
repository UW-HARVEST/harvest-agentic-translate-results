# CONFIGS.md — configuration surface (valid inputs)

Derived mechanically from the branches the C source takes on its *valid*
inputs. Axes found in `c_src/src/lib.c`:

* **`mode` argument** (`stbds_hmput_key` / `hmget_key` / `hmget_key_ts` /
  `hmdel_key`): the code only ever tests `mode >= STBDS_HM_STRING` (1) and, in
  `hmdel_key`, `mode == STBDS_HM_STRING` exactly. So the distinguished classes
  are `mode < 1` (binary), `mode == 1` (string), `mode > 1` (string for
  hashing/compare, **binary** for the strdup-free in `hmdel_key`).
* **`table->string.mode`** (`STBDS_SH_NONE 0`, `STBDS_SH_DEFAULT 1`,
  `STBDS_SH_STRDUP 2`, `STBDS_SH_ARENA 3`) — the `switch` in `hmput_key`
  (`lib.c:786-791`), the strdup-free in `hmfree_func` (`lib.c:576`) and in
  `hmdel_key` (`lib.c:837`). Set implicitly by `hmput_key` on a fresh table, or
  explicitly by `stbds_shmode_func`.
* **input shape**: `elemsize`, `keysize`, `keyoffset`, entry count (growth
  thresholds at `used_count >= slot_count - slot_count/4`), delete pattern
  (move-last / shrink / tombstone-rebuild), `len % 8` for `hash_bytes`, string
  length for `hash_string`/`stralloc`, `seed`.
* **entry points**: all 16 exported symbols, driven directly — including the
  lowest-level ones (`stbds_arrgrowf`, `stbds_make_hash_index` via
  `shmode_func`, `stbds_hash_bytes`) — not only `hm_geti`.

Every row is exercised with many randomized inputs from a fixed-seed PRNG
(`xorshift64*`, seed `0x2545F4914F6CDD1D`), and both libraries are compared on
the **full observable state**: returned `temp`, `stbds_array_header`
(`length`/`capacity`/`hash_table != NULL`), every element byte
`0 .. length*elemsize`, and every `stbds_hash_index` field plus every
`stbds_hash_bucket` `hash[]`/`index[]` slot.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, random seeds | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (all short-tail `switch` cases), random bytes incl. high-bit (`>= 0x80`) values, random seeds | [x] |
| 3 | `stbds_hash_bytes` | `len = 8` exactly (one main-loop iteration, empty tail) | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..15` (one main iteration + each tail case) | [x] |
| 5 | `stbds_hash_bytes` | `len = 16, 24, 64, 256, 1024` (many main iterations, no tail) | [x] |
| 6 | `stbds_hash_bytes` | `len` random in `1..1024` (mixed tail), random buffer, random seed | [x] |
| 7 | `stbds_hash_bytes` | `seed = 0`, `seed = 1`, `seed = SIZE_MAX`, `seed = 0x31415926`. (The C `stbds_siphash_bytes` XORs `seed` in twice, so it cancels and the result is in fact seed-independent — a quirk both libraries reproduce and that `harness_selftest` pins.) | [x] |
| 8 | `stbds_hash_string` | empty string `""`, random seeds | [x] |
| 9 | `stbds_hash_string` | length 1..64 random ASCII, random seeds | [x] |
| 10 | `stbds_hash_string` | length 1..512 random bytes `0x01..0xFF` (high-bit chars go through `(unsigned char)`) | [x] |
| 11 | `stbds_hash_string` | `seed = 0` / `SIZE_MAX` boundary seeds | [x] |
| 12 | `stbds_rand_seed` + `stbds_shmode_func` | seed reset then table creation: verifies the `stbds_hash_seed = seed*a + b` LCG advance and the per-table `seed` field, for seeds `0, 1, 0x31415926, SIZE_MAX, random` | [x] |
| 13 | `stbds_rand_seed` + `stbds_hmput_key` | 8 consecutive fresh tables after one `rand_seed`, checking the LCG chain advances identically in both libraries | [x] |
| 14 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 1` (the call `hmput_key`/`hmget_key_ts`/`shmode_func` make) | [x] |
| 15 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap = 0` → returns `NULL` | [x] |
| 16 | `stbds_arrgrowf` | `a = NULL`, `addlen ∈ {1,2,3,4,5,100}`, `min_cap = 0` → `min_cap < 4` clamp to 4 | [x] |
| 17 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → early return, header untouched | [x] |
| 18 | `stbds_arrgrowf` | existing array, `min_cap < 2*cap` → doubling branch | [x] |
| 19 | `stbds_arrgrowf` | existing array, `min_cap >= 2*cap` → exact `min_cap` branch | [x] |
| 20 | `stbds_arrgrowf` | repeated `addlen = 1` growth chain (4→8→16→…→1024), `elemsize ∈ {1,4,8,12,16,24}` | [x] |
| 21 | `stbds_arrgrowf` | `elemsize = 0` (degenerate, header-only) | [x] |
| 22 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free a valid array (no leak / no crash), `elemsize ∈ {8,16}` | [x] |
| 23 | `stbds_shmode_func` | `mode = STBDS_SH_NONE (0)`, `elemsize = 16` — full `stbds_hash_index` + bucket compare | [x] |
| 24 | `stbds_shmode_func` | `mode = STBDS_SH_DEFAULT (1)`, `elemsize = 16` | [x] |
| 25 | `stbds_shmode_func` | `mode = STBDS_SH_STRDUP (2)`, `elemsize = 16` | [x] |
| 26 | `stbds_shmode_func` | `mode = STBDS_SH_ARENA (3)`, `elemsize = 16` | [x] |
| 27 | `stbds_shmode_func` | `elemsize ∈ {8,12,16,24,32}` × `mode ∈ {0,1,2,3}` | [x] |
| 28 | `stbds_hmput_default` | `a = NULL` → creates the 1-element default slot | [x] |
| 29 | `stbds_hmput_default` | already-created handle (`length == 1`) → returns unchanged; and the second arm of the same `if`, `length == 0` on a non-NULL handle → grows and re-initialises element 0 | [x] |
| 30 | `stbds_hmput_default` | handle from `hmget_key(NULL,…)` then `hmput_default` (`length == 1` path) | [x] |
| 31 | `stbds_hmget_key` | `a = NULL` → fresh handle, `temp = -1` | [x] |
| 32 | `stbds_hmget_key_ts` | `a = NULL` → fresh handle, `*temp = -1`, header `temp` **not** written (differs from `hmget_key`) | [x] |
| 33 | `hmput_key`/`hmget_key`, binary | `mode = 0`, `elemsize = 8`, `keysize = 4` (`int` key + `int` value), 1 entry | [x] |
| 34 | `hmput_key`/`hmget_key`, binary | `mode = 0`, `elemsize = 8`, `keysize = 4`, 5 entries (below the grow threshold of 6) | [x] |
| 35 | `hmput_key`/`hmget_key`, binary | `mode = 0`, `elemsize = 8`, `keysize = 4`, 6 entries → first grow (8→16 slots) | [x] |
| 36 | `hmput_key`/`hmget_key`, binary | `mode = 0`, `elemsize = 8`, `keysize = 4`, 200 random keys → several grows, random lookups of present and absent keys | [x] |
| 37 | `hmput_key`/`hmget_key`, binary | `mode = 0`, `elemsize = 8`, `keysize = 4`, 2000 random keys (5 grows) | [x] |
| 38 | `hmput_key`/`hmget_key`, binary | re-put of existing keys (update path, `is_key_equal` hit in the upper scan) | [x] |
| 39 | `hmput_key`/`hmget_key`, binary | keys chosen so the *wrapped* (`i = 0..limit`) scan is taken — dense random keys in a nearly-full table | [x] |
| 40 | `hmput_key`/`hmget_key`, binary | `keysize = 1` (`u8` keys, guaranteed collisions in a large table), `elemsize = 8` | [x] |
| 41 | `hmput_key`/`hmget_key`, binary | `keysize = 2`, `elemsize = 8` | [x] |
| 42 | `hmput_key`/`hmget_key`, binary | `keysize = 8` (`u64` keys), `elemsize = 16` | [x] |
| 43 | `hmput_key`/`hmget_key`, binary | `keysize = 16` (128-bit keys), `elemsize = 24` | [x] |
| 44 | `hmput_key`/`hmget_key`, binary | `keysize = 3` / `5` / `7` (non-power-of-two, exercises `hash_bytes` tail) | [x] |
| 45 | `hmput_key`/`hmget_key`, binary | `elemsize == keysize` (no value payload), `elemsize = 4, keysize = 4` | [x] |
| 46 | `hmput_key`/`hmget_key`, binary | `elemsize` much larger than `keysize` (`64` vs `4`) — large uncompared payload tail written by the caller | [x] |
| 47 | `hmput_key`/`hmget_key`, binary | `mode = -1` and `mode = INT_MIN` (out-of-range, still binary) | [x] |
| 48 | `hmput_key`/`hmget_key`, string | `mode = 1`, fresh table → `string.mode` becomes `STBDS_SH_DEFAULT`; caller-owned keys; 1/5/6/50/500 entries | [x] |
| 49 | `hmput_key`/`hmget_key`, string | `mode = 1`, `string.mode = STBDS_SH_STRDUP` (table from `shmode_func(…,2)`); keys freed by the library — verify the *stored* strings match | [x] |
| 50 | `hmput_key`/`hmget_key`, string | `mode = 1`, `string.mode = STBDS_SH_ARENA` (table from `shmode_func(…,3)`); short keys (arena-packed) | [x] |
| 51 | `hmput_key`/`hmget_key`, string | `mode = 1`, `STBDS_SH_ARENA`, long keys (> 512) forcing the oversize-block branch mixed with short keys | [x] |
| 52 | `hmput_key`/`hmget_key`, string | `mode = 1`, `STBDS_SH_ARENA`, enough keys to walk `block` 0→23 (blocksize 512 → 1 MiB cap) | [x] |
| 53 | `hmput_key`/`hmget_key`, string | `mode = 2` (out-of-range: string for hash/compare) on `STBDS_SH_DEFAULT` and `STBDS_SH_STRDUP` tables | [x] |
| 54 | `hmput_key`/`hmget_key`, string | `strkey(n)`-shaped keys (`"test_%d"`) — the exact key shape the C driver produces, `n ∈ {0,±1,±12345,INT_MIN,INT_MAX}` and random | [x] |
| 55 | `hmput_key`/`hmget_key`, string | duplicate string keys re-put (update path incl. `stbds_temp_key` write) | [x] |
| 56 | `hmdel_key`, binary | delete a key that is the **last** entry (`old_index == final_index`, no move) | [x] |
| 57 | `hmdel_key`, binary | delete a key in the **middle** (`old_index != final_index` → move + re-find) | [x] |
| 58 | `hmdel_key`, binary | delete every key of a 200-entry map in insertion order (repeated moves + shrinks) | [x] |
| 59 | `hmdel_key`, binary | delete every key in *reverse* insertion order | [x] |
| 60 | `hmdel_key`, binary | delete in random order, 500 entries (shrink + tombstone-rebuild interleaved) | [x] |
| 61 | `hmdel_key`, binary | delete/insert churn: 2000 random ops (put/get/del) against the same map | [x] |
| 62 | `hmdel_key`, binary | delete down to `used_count < used_count_shrink_threshold` with `slot_count = 16` → shrink to 8 | [x] |
| 63 | `hmdel_key`, binary | tombstone-rebuild path: put 6, delete/re-put the same keys until `tombstone_count > (slot_count>>3)+(slot_count>>4)` | [x] |
| 64 | `hmdel_key`, binary | `slot_count == 8` (shrink threshold forced to 0) — delete all 6 entries, index must never shrink | [x] |
| 65 | `hmdel_key`, string | `mode = 1`, `STBDS_SH_DEFAULT`, delete middle + last, 100 keys | [x] |
| 66 | `hmdel_key`, string | `mode = 1`, `STBDS_SH_STRDUP`, delete (frees the duped key) then re-insert the same text | [x] |
| 67 | `hmdel_key`, string | `mode = 1`, `STBDS_SH_ARENA`, delete (no free) then re-insert | [x] |
| 68 | `hmdel_key`, string | `mode = 2` on `STBDS_SH_STRDUP` / `STBDS_SH_DEFAULT` tables: hashes as string but **skips** the strdup free (`mode == STBDS_HM_STRING` is false). Only the `old_index == final_index` case is a valid path (delete in reverse insertion order); `old_index != final_index` always aborts in C and is `ERRORS.md` row 53 | [x] |
| 69 | `hmget_key_ts` | `temp` out-param on populated maps: hit, miss, and after deletes; binary and string modes | [x] |
| 70 | `hmfree_func` | binary table (`string.mode = 0`) with 0 / 1 / 200 entries | [x] |
| 71 | `hmfree_func` | `STBDS_SH_STRDUP` table with entries (frees every duped key) | [x] |
| 72 | `hmfree_func` | `STBDS_SH_ARENA` table with entries (`strreset` walks the block list) | [x] |
| 73 | `hmfree_func` | handle with `hash_table == NULL` (from `hmput_default` / `hmget_key(NULL)`) | [x] |
| 74 | `stbds_stralloc` | fresh arena, single short string (`len <= 512`) | [x] |
| 75 | `stbds_stralloc` | fresh arena, `len > 512` but `< 1<<20` → oversize-block-vs-blocksize comparison at `block = 0` | [x] |
| 76 | `stbds_stralloc` | many short strings filling one block then spilling to the next (`remaining` bookkeeping, `block` increment) | [x] |
| 77 | `stbds_stralloc` | interleaved short / oversize strings (oversize block spliced after the head, `remaining` preserved) | [x] |
| 78 | `stbds_stralloc` | oversize string as the *very first* allocation (`storage == NULL` → head install, `remaining = 0`) | [x] |
| 79 | `stbds_stralloc` | 4000 random strings of random length 0..3000 into one arena (walks `block` to saturation) | [x] |
| 80 | `stbds_stralloc` + `stbds_strreset` | allocate then reset then re-allocate (arena reuse) | [x] |
| 81 | `stbds_strreset` | zeroed arena (no-op), single-block arena, multi-block arena | [x] |
| 82 | `strkey` | `n ∈ {0, 1, -1, 9, 10, 99, 100, 12345, -12345, INT_MAX, INT_MIN}` + 1000 random `int`s — full 256-byte buffer compared | [x] |
| 83 | `hm_geti` | `num ∈ {0, -1, INT_MIN}` (all loops skipped) | [x] |
| 84 | `hm_geti` | `num ∈ {1,2,3,4,5,6,7,8,9,10,11,12,16,17}` (growth + delete boundaries) | [x] |
| 85 | `hm_geti` | `num ∈ {31,32,33,64,100,127,128,256,1000}` (multi-grow, shrink, tombstone paths) | [x] |
| 86 | `hm_geti` | `num` after `stbds_rand_seed(s)` for `s ∈ {0,1,SIZE_MAX,random}` — the per-table `seed` field and the global LCG chain differ. (Probe orders do *not* change, because `hm_geti` uses binary keys and `stbds_hash_bytes` is seed-independent; the string-keyed rows 48-55 are where the seed actually changes the layout.) | [x] |
| 87 | mixed pipeline | `shmode_func(STRDUP)` → 300 random `strkey`-style puts → random gets → random deletes → `hmfree_func`, full state compared after **every** step | [x] |
| 88 | mixed pipeline | `hmget_key(NULL)` → `hmput_default` → 300 binary puts → interleaved `hmget_key_ts` / `hmdel_key` → `hmfree_func`, full state compared after every step | [x] |
| 89 | mixed pipeline | `shmode_func(ARENA)` → puts with mixed short/long keys → deletes → re-puts → `hmfree_func` | [x] |
| 90 | mixed pipeline | binary map with `keysize = 0` (every key collides) → puts/gets/dels; and `elemsize = 0, keysize = 0` (fully degenerate) | [x] |
| 91 | `hmput_key` (string) | `table->temp_key` (`stbds_temp_key`): new-insert path for `STBDS_SH_DEFAULT` / `STBDS_SH_STRDUP` / `STBDS_SH_ARENA`, plus the update path where only the *upper* bucket scan writes it (the wrapped scan's omission is a preserved quirk). Put-only workloads, since `hmdel_key` in STRDUP mode frees the key and leaves `temp_key` dangling in both libraries | [x] |

| 93 | `hmput_key`/`hmget_key`/`hmdel_key`, string | 12 different global seeds (`0, 1, 2, 0x31415926, SIZE_MAX, SIZE_MAX-1` + 6 random) × `STBDS_SH_DEFAULT` / `STBDS_SH_STRDUP` / `STBDS_SH_ARENA`, 150 keys each with hits, misses and deletes. This is the only place the seed changes the bucket layout, since `stbds_hash_bytes` ignores it | [x] |
| 92 | harness self-test | negative controls: the snapshot must be sensitive to table seed, bucket layout, element bytes, `temp` and `length`; `same()` must panic on a mismatch; `in_child` must distinguish clean exit / `SIGSEGV` / `SIGABRT`; the arena snapshot must separate the short and oversize paths | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configuration is the default one (`--no-default-features` is equivalent).
This is verified by a script that parses `Cargo.toml` and loops over the
(single) combination, running `cargo check` and the full test suite for each.
There is no `[[bin]]` target and `c_src/CMakeLists.txt` builds only
`add_library(... SHARED ...)` — no driver executable exists on either side, so
there is no stdout comparison to make.

## Row → test mapping

| rows | test file :: test |
|------|-------------------|
| 1-7 | `phase_b_hash.rs :: cfg_01_07_hash_bytes_all_length_classes` |
| 8-11 | `phase_b_hash.rs :: cfg_08_11_hash_string` |
| 12 | `phase_b_hash.rs :: cfg_12_rand_seed_then_table_seed` |
| 13 | `phase_b_hash.rs :: cfg_13_seed_lcg_chain` |
| 14, 15, 16, 19, 21 | `phase_b_arr.rs :: cfg_14_16_19_21_arrgrowf_fresh` |
| 17 | `phase_b_arr.rs :: cfg_17_arrgrowf_early_return` |
| 18, 19, 20 | `phase_b_arr.rs :: cfg_18_20_arrgrowf_growth_chain` |
| 22 | `phase_b_arr.rs :: cfg_22_arrgrowf_payload_survives` |
| 23-27 | `phase_b_map.rs :: cfg_23_27_shmode_func` |
| 28-30 | `phase_b_map.rs :: cfg_28_30_hmput_default` |
| 31, 32 | `phase_b_map.rs :: cfg_31_32_lookup_on_null` |
| 33-39, 47 | `phase_b_map.rs :: cfg_33_39_47_binary_growth` |
| 40-46, 49-51 | `phase_b_map.rs :: cfg_40_46_key_and_elem_sizes` |
| 48, 53-55 | `phase_b_map.rs :: cfg_48_53_55_string_default_mode` |
| 49-53 | `phase_b_map.rs :: cfg_49_52_strdup_and_arena_modes` |
| 56-64 | `phase_b_map.rs :: cfg_56_64_binary_deletes` |
| 65-68 | `phase_b_map.rs :: cfg_65_68_string_deletes` |
| 69 | `phase_b_map.rs :: cfg_69_hmget_key_ts` |
| 70-73 | `phase_b_map.rs :: cfg_70_73_hmfree_func` |
| 74-76 | `phase_b_strings.rs :: cfg_74_76_stralloc_short` |
| 75, 77, 78 | `phase_b_strings.rs :: cfg_75_77_78_stralloc_oversize` |
| 79 | `phase_b_strings.rs :: cfg_79_stralloc_block_saturation` |
| 80, 81 | `phase_b_strings.rs :: cfg_80_81_strreset` |
| 82 | `phase_b_hash.rs :: cfg_82_strkey` |
| 83 | `phase_b_driver.rs :: cfg_83_hm_geti_non_positive` |
| 84 | `phase_b_driver.rs :: cfg_84_hm_geti_small` |
| 85 | `phase_b_driver.rs :: cfg_85_hm_geti_large` |
| 86 | `phase_b_driver.rs :: cfg_86_hm_geti_seeded` |
| 87-90 | `phase_b_map.rs :: cfg_87_90_pipelines` |
| 91 | `phase_b_map.rs :: cfg_91_temp_key` |
| 92 | `harness_selftest.rs` (all 5 tests) |
| 93 | `phase_b_map.rs :: cfg_93_string_maps_under_varied_seeds` |
