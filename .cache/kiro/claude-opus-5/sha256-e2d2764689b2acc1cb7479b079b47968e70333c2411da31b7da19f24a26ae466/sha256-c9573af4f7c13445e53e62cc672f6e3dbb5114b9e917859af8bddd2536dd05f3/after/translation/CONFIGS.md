# CONFIGS.md — configuration surface table (valid inputs)

Mechanically derived from the axes `c_src/src/lib.c` actually branches on.

## Axes the C code distinguishes

| axis | values the C treats differently | where |
|------|---------------------------------|-------|
| `mode` (int param of `hmget_key`/`hmget_key_ts`/`hmput_key`/`hmdel_key`) | `mode < 1` (binary: `memcmp` + `stbds_hash_bytes`); `mode == 1` (string: `strcmp` + `stbds_hash_string`, and the *only* value that enables strdup-free / char\*\* re-lookup in `hmdel_key`); `mode >= 2` (string compare/hash but **not** `mode == 1`) | `stbds_is_key_equal` L561, `hmput_key` L686/728, `hm_find_slot` L603, `hmdel_key` L838/843 |
| `table->string.mode` (set by `stbds_shmode_func`, or implicitly by `hmput_key`) | `STBDS_SH_NONE(0)`/default → `memcpy` key by value; `STBDS_SH_DEFAULT(1)` → store caller pointer; `STBDS_SH_STRDUP(2)` → `stbds_strdup`; `STBDS_SH_ARENA(3)` → `stbds_stralloc`; out-of-range → `default` arm | `hmput_key` L786 switch, `hmfree_func` L575, `hmdel_key` L838 |
| `elemsize` | any; drives `HASH_TO_ARR`/`ARR_TO_HASH` offset and element stride. `elemsize < sizeof(char*)` in string modes overlaps neighbouring elements | everywhere |
| `keysize` | `0`, `1..7`, `8`, `>8` — feeds `stbds_hash_bytes` (the siphash 8-byte-block loop + the 7-case fall-through tail) and `memcmp` | `siphash_bytes` L520/533 |
| `keyoffset` | `0` (all `hmput_key`/`hmget_key*` paths hard-code 0) vs. caller-supplied non-zero (only `hmdel_key` takes it) | `hmdel_key` param, `is_key_equal` |
| `seed` | default `0x31415926`; any value via `stbds_rand_seed`. Also the *per-table* seed advances via the LCG `seed = seed*a + b` on every fresh `stbds_make_hash_index(_, NULL)` — so table identity depends on global call order | `stbds_rand_seed` L327, `make_hash_index` L410 |
| byte values | bytes with the high bit set change `stbds_hash_bytes` (the `d[3] << 24` / `d[7] << 24` `int` expressions sign-extend into the upper 32 bits of `size_t`) | `siphash_bytes` L521/522, L537 |
| `slot_count` | `8` (initial, and `used_count_shrink_threshold == 0`); `16`, `32`, `64`, … after growth; shrink back at `used_count < slot_count>>2` | `make_hash_index` L392-400, `hmput_key` L703, `hmdel_key` L855 |
| table lifecycle state | no table (`hash_table == NULL`); table below `used_count_threshold`; at/over threshold (grow ×2); with tombstones below `tombstone_count_threshold`; over it (rebuild); below `used_count_shrink_threshold` (shrink ÷2) | `hmput_key` L700, `hmdel_key` L855/858 |
| array state (`stbds_arrgrowf`) | `a == NULL` (fresh: length/hash_table/temp initialised) vs `a != NULL` (only capacity written); `min_cap <= cap` (no-op early return); `min_cap < 2*cap`; `min_cap < 4` | `arrgrowf` L275-300 |
| string length (`stbds_hash_string`) | `0`, `1`, … — the rotate/add loop is per byte | `hash_string` L478 |
| arena state (`stbds_stralloc`) | `len <= remaining` (bump-allocate from the current block); `len > remaining` with `len <= blocksize` (fresh block, LIFO push); `len > remaining` with `len > blocksize` (dedicated oversized block, spliced *behind* head, or becomes head with `remaining = 0`); `a->block` 0..255 driving `512 << (block>>1)` and the `< 1<<20` cap | `stralloc` L898-921 |
| `arr_push(num)` | `num <= 0` (no-op); `num` in `1..50` (one iteration, inner loop 0 times); `num > 50` (multiple realloc/grow/free cycles, exercising `arrgrowf` `a != NULL` path repeatedly) | `arr_push` L945 |
| `strkey(n)` | `n >= 0`, `n < 0`, `n == INT_MIN`, `n == INT_MAX` — `sprintf("test_%d")` | `strkey` L940 |

## Rows (each = one differential test, randomised inputs, fixed seed)

Legend for "entry point(s)": `L` = called directly as a low-level export;
`M` = driven through the caller-side re-implementation of the C macro
(`hmput`/`hmget`/`hmdel`/`shput`/`shget`/`shdel`/`arrput`/…), i.e. the full
composed pipeline a real consumer runs.

| # | entry point(s) | configuration (options set + input shape) | test (`tests/phase_b.rs`) | [x] |
|---|----------------|--------------------------------------------|---------------------------|-----|
| 1 | `stbds_hash_bytes` (L) | `len = 0`, random seeds | `row01_hash_bytes_len0` | [x] |
| 2 | `stbds_hash_bytes` (L) | `len = 1..7` (every tail `case` of the fall-through switch), random bytes incl. high-bit | `row02_hash_bytes_tail_1_to_7` | [x] |
| 3 | `stbds_hash_bytes` (L) | `len = 8` exactly (one block, empty tail `case 0`) | `row03_hash_bytes_len8` | [x] |
| 4 | `stbds_hash_bytes` (L) | `len = 9..64` (block loop + every tail remainder), random bytes | `row04_hash_bytes_9_to_64` | [x] |
| 5 | `stbds_hash_bytes` (L) | `len` 1..64 with **all bytes ≥ 0x80** (forces the `d[3]<<24` / `d[7]<<24` sign-extension paths) | `row05_hash_bytes_high_bit / row05b_hash_unaligned_buffers` | [x] |
| 6 | `stbds_hash_bytes` (L) | large `len` (256..4096), random seed | `row06_hash_bytes_large` | [x] |
| 7 | `stbds_hash_string` (L) | empty string; length 1..64 random ASCII; bytes ≥ 0x80 (`(unsigned char)*str`); random seeds incl. 0 and `SIZE_MAX` | `row07_hash_string / row05b_hash_unaligned_buffers` | [x] |
| 8 | `stbds_rand_seed` + `stbds_shmode_func` (L) | seed set explicitly, then a fresh table — checks the global LCG advance `seed*a+b` is byte-identical (compare `table->seed` observed through hashing behaviour) | `row08_rand_seed_lcg_lockstep` | [x] |
| 9 | `stbds_arrgrowf` (L) | `a = NULL`, `addlen = 0`, `min_cap = 0` → early-return `NULL` | `row09_arrgrowf_null_noop` | [x] |
| 10 | `stbds_arrgrowf` (L) | `a = NULL`, random `elemsize ∈ {1,2,4,8,16,24,32}`, random `addlen`/`min_cap` → header `length/capacity/hash_table/temp` compared field-by-field | `row10_arrgrowf_fresh_random` | [x] |
| 11 | `stbds_arrgrowf` (L) | `a != NULL`, repeated growth (`min_cap < 2*cap` doubling path, and `min_cap >= 2*cap` explicit path), capacity sequence compared | `row11_arrgrowf_repeated_growth` | [x] |
| 12 | `stbds_arrgrowf` (L) | `a != NULL`, `min_cap <= cap` → early return, header untouched | `row12_arrgrowf_early_return` | [x] |
| 13 | `stbds_arrgrowf` + `stbds_arrfreef` (L) | grow then free, no leak/crash; repeated 1000× | `row13_arrgrowf_grow_free_cycles` | [x] |
| 14 | `arrput` pipeline (M) | `stbds_arrmaybegrow`+`length++` loop for 0..500 elements of `elemsize = 4`; full element payload compared | `row14_arrput_pipeline` | [x] |
| 15 | `arrput`/`arrdel`/`arrins`/`arrdelswap`/`arrpop`/`arrsetlen`/`arrsetcap` pipeline (M) | randomised op stream, `elemsize = 4` and `= 16`, contents + header compared after each op | `row15_array_op_stream` | [x] |
| 16 | `arr_push` (L) | `num = 0`, `1`, `49`, `50`, `51`, `100`, `1000`, `5000` (no observable output; verified to not abort and to leave the process usable) | `row16_arr_push` | [x] |
| 17 | `strkey` (L) | `n = 0, 1, -1, 42, -42, INT_MAX, INT_MIN`, plus randoms — returned C string compared byte-for-byte | `row17_strkey` | [x] |
| 18 | `stbds_hmput_default` (L) | `a = NULL`, random `elemsize`; header + zeroed payload compared | `row18_hmput_default_from_null` | [x] |
| 19 | `stbds_hmput_default` (L) | `a` from a previous `hmput_default` (length already 1 → no-op path) | `row19_hmput_default_idempotent` | [x] |
| 20 | `stbds_hmput_default` (L) | `a` whose `length == 0` (forced) → regrow path | `row20_hmput_default_zero_length` | [x] |
| 21 | `hmput` pipeline (M), `mode = STBDS_HM_BINARY` | `keysize = 4`, `elemsize = 8` (`{int key; int value;}`), 1 insert | `row21_bin_single_insert` | [x] |
| 22 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | `keysize = 4`, `elemsize = 8`, N = 1, 2, 6 (below `used_count_threshold` for 8 slots), random keys | `row22_bin_below_threshold` | [x] |
| 23 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | N = 7..64 → crosses `used_count_threshold` → grow to 16/32/64/128 slots; every key looked up, `temp` index compared | `row23_bin_crosses_growth` | [x] |
| 24 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | N = 300 random keys with duplicates (update-existing path L742), values overwritten | `row24_bin_with_duplicates` | [x] |
| 25 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | `keysize = 8`, `elemsize = 16` (`{size_t key; size_t value;}`) | `row25_bin_keysize8_elem16` | [x] |
| 26 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | `keysize = 8`, `elemsize = 24`, keys are a 2-`int` struct (`stbds_struct2`-shaped) | `row26_bin_keysize8_elem24` | [x] |
| 27 | `hmput`+`hmget` pipeline (M), `mode = BINARY` | `keysize = 1` and `= 3` (odd sizes, exercise siphash tail inside the map) | `row27_bin_odd_keysizes` | [x] |
| 28 | `hmput`+`hmdel`+`hmget` pipeline (M), `mode = BINARY` | randomised insert/delete/lookup stream, 2000 ops — exercises tombstones, `tombstone_count_threshold` rebuild, `used_count_shrink_threshold` shrink, and the last-element-moves-into-hole swap | `row28_bin_insert_delete_stream` | [x] |
| 29 | `hmdel` pipeline (M), `mode = BINARY` | delete the **last** element (`old_index == final_index`, no memmove/re-lookup) | `row29_bin_delete_last` | [x] |
| 30 | `hmdel` pipeline (M), `mode = BINARY` | delete a **middle** element (`old_index != final_index` → memmove + re-lookup + `b->index[i] = old_index`) | `row30_bin_delete_middle` | [x] |
| 31 | `hmdel` pipeline (M), `mode = BINARY` | delete a key never inserted → `temp == 0`, length unchanged | `row31_bin_delete_absent` | [x] |
| 32 | `hmdel` then re-insert (M), `mode = BINARY` | insert into a reclaimed tombstone (`tombstone >= 0` path L768, `--tombstone_count`) | `row32_bin_tombstone_reuse` | [x] |
| 33 | `hmget_key_ts` (L), `mode = BINARY` | explicit `temp` out-param instead of the header field; hit and miss; `a == NULL` first call | `row33_hmget_key_ts` | [x] |
| 34 | `hmget_key` vs `hmget_key_ts` (L) | same table, both APIs, header `temp` vs out-param compared | `row34_hmget_key_vs_ts` | [x] |
| 35 | `shput`+`shget` pipeline (M), `mode = STBDS_HM_STRING`, `string.mode = SH_DEFAULT` (implicit) | `elemsize = 16` (`{char *key; int value;}`), N = 1..64 keys from `strkey`, keys stored **by pointer** | `row35_str_sh_default_implicit / row35c_temp_key` | [x] |
| 36 | `shput`+`shget` pipeline (M), `mode = STRING`, `string.mode = SH_STRDUP` via `stbds_shmode_func(elemsize, 2)` | keys duplicated with `stbds_strdup`; stored pointer differs from input; `temp_key` written | `row36_str_sh_strdup` | [x] |
| 37 | `shput`+`shget` pipeline (M), `mode = STRING`, `string.mode = SH_ARENA` via `stbds_shmode_func(elemsize, 3)` | keys allocated in the arena; short keys (bump path) and long keys (> blocksize dedicated block); `a->block` growth | `row37_str_sh_arena / row37b_str_sh_arena_long_keys` | [x] |
| 38 | `shput`+`shdel` pipeline (M), `mode = STRING`, `SH_STRDUP` | delete frees the strdup'd key (`hmdel_key` L838), then re-lookup misses | `row38_str_strdup_delete` | [x] |
| 39 | `shput`+`shdel` pipeline (M), `mode = STRING`, `SH_ARENA` | delete does **not** free (arena owns), re-lookup misses | `row39_str_arena_delete` | [x] |
| 40 | `shput`+`shdel` pipeline (M), `mode = STRING`, `SH_DEFAULT` | delete of middle element → `char**` re-lookup path (L843 `mode == STBDS_HM_STRING` true) | `row40_str_default_delete` | [x] |
| 41 | randomised string-map op stream (M), `mode = STRING`, each of `SH_DEFAULT`/`SH_STRDUP`/`SH_ARENA` | 1000 ops, keys drawn from a pool with collisions; length + every element compared | `row41_str_op_stream` | [x] |
| 42 | `stbds_shmode_func` (L) | `mode = 0,1,2,3` and out-of-range `4, 255, 256, -1, INT_MAX, INT_MIN` (stored as `(unsigned char)mode`); resulting header + `string.mode` compared | `row42_shmode_func_all_modes` | [x] |
| 43 | `stbds_hmput_key` (L), `mode = 2` and `mode = INT_MAX` | out-of-range mode: string hash/compare but **not** `mode == 1` — key slot treated as `char*` by `is_key_equal`, `string.mode = SH_DEFAULT` | `row43_mode_out_of_range_positive` | [x] |
| 44 | `stbds_hmput_key`/`hmget_key` (L), `mode = -1` / `INT_MIN` | negative mode → binary path (`mode >= 1` false) | `row44_mode_negative_is_binary` | [x] |
| 45 | `stbds_hmdel_key` (L), non-zero `keyoffset` | binary mode with `keyoffset = 4` inside a 16-byte element | `row45_hmdel_keyoffset_nonzero` | [x] |
| 46 | `stbds_hmfree_func` (L) | table with `string.mode = SH_NONE` / `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA`; free after N inserts; and `a == NULL` | `row46_hmfree_func_all_modes` | [x] |
| 47 | `stbds_stralloc` (L) | fresh zeroed arena, `len ∈ 1..64` repeatedly until a new block is needed; returned bytes + arena fields (`remaining`, `block`, `storage != NULL`) compared | `row47_stralloc_bump_path` | [x] |
| 48 | `stbds_stralloc` (L) | first string longer than `512` (i.e. `len > blocksize` on a fresh arena, `a->storage == NULL` → becomes head with `remaining = 0`) | `row48_stralloc_oversized_first` | [x] |
| 49 | `stbds_stralloc` (L) | oversized string **after** a normal block exists (`a->storage != NULL` → spliced behind head, `remaining` untouched) | `row49_stralloc_oversized_after_block` | [x] |
| 50 | `stbds_stralloc` (L) | drive `a->block` from 0 up through the `blocksize < 1<<20` cap (blocksizes 512, 512, 1024, 1024, …, 1<<20) and past it | `row50_stralloc_block_counter_sweep` | [x] |
| 51 | `stbds_stralloc` + `stbds_strreset` (L) | allocate a multi-block chain, reset, verify arena is fully zeroed and reusable; also `strreset` on an already-zero arena | `row51_strreset_chain_and_empty` | [x] |
| 52 | full stress, mixed (M) | interleaved binary map + string map + dynamic array + arena operations under one fixed-seed op stream (5000 ops), all four data structures live simultaneously; every byte of every structure compared | `row52_mixed_stress` | [x] |
| 53 | `stbds_rand_seed` (L) determinism | set the same seed on both libraries before each row so the per-table `seed` LCG stays in lock-step; verified by rows 22-41 producing identical `temp` indices and identical element order | `row08_rand_seed_lcg_lockstep + rows 22-41 (all seeded)` | [x] |

All 53 rows pass, with randomized inputs (fixed seeds) per row, under every
profile (debug + release) and every feature configuration (default and
`--no-default-features`; the crate declares no features). Reproduce with
`./verify.sh`.
