# CONFIGS.md — Configuration / valid-input surface (Phase A, gate for Phase B)

Derived mechanically from the branch structure of `c_src/src/lib.c`.
The library has **no runtime "options" struct**; its behaviour is steered by

* **`mode`** (`int`) passed to `stbds_hmput_key` / `stbds_hmget_key[_ts]` /
  `stbds_hmdel_key` → `stbds_is_key_equal` (`mode >= STBDS_HM_STRING`) and the
  hash choice (`stbds_hash_string` vs `stbds_hash_bytes`).
  Values the macros use: `0 = STBDS_HM_BINARY`, `1 = STBDS_HM_STRING`,
  `2 = STBDS_HM_PTR_TO_STRING`.
* **`string.mode`** (`unsigned char`) inside the hash index → the `switch` at
  lib.c:785 and the frees at lib.c:575 / lib.c:836.
  `0 = SH_NONE`, `1 = SH_DEFAULT`, `2 = SH_STRDUP`, `3 = SH_ARENA`.
  Set implicitly by `stbds_hmput_key` (0 or 1) or explicitly by
  `stbds_shmode_func` (any byte).
* **`elemsize` / `keysize` / `keyoffset`** — the shape of the caller's element.
* **the global hash seed** (`stbds_rand_seed`) — and its LCG advance on every
  fresh `stbds_make_hash_index`.
* **input shape**: array length/capacity, map load factor (which drives
  `slot_count` 8→16→32→…, tombstone rebuild and shrink), byte-buffer length
  (the `switch (len - i)` tail 0..7 and the 8-byte block loop), string length,
  and arena block counter (`512 << (block>>1)`, saturating at `1<<20`).

Both C and Rust are driven **only** through the 16 exported symbols loaded with
`libloading`, including the lowest-level ones (`stbds_arrgrowf`,
`stbds_hash_bytes`, `stbds_stralloc`), not just the composed helpers.

Every row is exercised with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, `tests/common/mod.rs::Rng`), and both libraries' results
are compared byte-for-byte via the canonical dumps in
`tests/common/mod.rs` (`dump_map`, `dump_arena`).

## A. `stbds_hash_bytes` / `stbds_siphash_bytes` (lowest level)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p = NULL` and `p = valid` (no block, `case 0: break`) | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` — every `switch (len-i)` fall-through case, random bytes | [x] |
| 3 | `stbds_hash_bytes` | `len = 1..7` with `byte[3] >= 0x80` and `byte[6] >= 0x80` (the `int`-promotion / sign-extension arms `case 4` and `case 7`) | [x] |
| 4 | `stbds_hash_bytes` | `len = 8, 16, 24` — exact multiples of `sizeof(size_t)`, tail `case 0` | [x] |
| 5 | `stbds_hash_bytes` | `len = 9..64` — block loop + every tail, random bytes | [x] |
| 6 | `stbds_hash_bytes` | block loop with `d[3] >= 0x80` and `d[7] >= 0x80` (sign-extension inside the loop, `<<16<<16`) | [x] |
| 7 | `stbds_hash_bytes` | `seed ∈ {0, 1, SIZE_MAX, 0x31415926, random}` × `len ∈ {0,7,8,15,32}` | [x] |

## B. `stbds_hash_string` / `stbds_rand_seed`

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 8 | `stbds_hash_string` | `""` (loop never taken) | [x] |
| 9 | `stbds_hash_string` | 1, 2, 8, 9, 100-byte ASCII strings, random | [x] |
| 10 | `stbds_hash_string` | strings containing bytes `0x80..0xFF` (`(unsigned char)*str` promotion) | [x] |
| 11 | `stbds_hash_string` | `seed ∈ {0, 1, SIZE_MAX, 0x31415926, random}` | [x] |
| 12 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to `{0, 1, SIZE_MAX, random}`, then observe the seed stored in the new index **and** the LCG-advanced global seed via a second index | [x] |

## C. `stbds_arrgrowf` / `stbds_arrfreef` (lowest level)

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 13 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap ∈ {1,2,3}` → `min_cap` bumped to 4 | [x] |
| 14 | `stbds_arrgrowf` | `a = NULL`, `addlen = 0`, `min_cap ∈ {4,5,17,1000}` → used as-is | [x] |
| 15 | `stbds_arrgrowf` | `a = NULL`, `addlen ∈ {1,3,7,64}`, `min_cap = 0` → `min_len` wins | [x] |
| 16 | `stbds_arrgrowf` | existing array, `min_cap <= cap` → returns unchanged (no realloc) | [x] |
| 17 | `stbds_arrgrowf` | existing array, `min_cap` between `cap+1` and `2*cap` → doubling wins | [x] |
| 18 | `stbds_arrgrowf` | existing array, `min_cap > 2*cap` → `min_cap` wins | [x] |
| 19 | `stbds_arrgrowf` | `elemsize ∈ {1,2,3,4,8,12,16,17,64}` × the above (payload preservation across realloc) | [x] |
| 20 | `stbds_arrgrowf` + `stbds_arrfreef` | grow → write payload → grow again ×N (random schedule) → free; compare header + payload at every step | [x] |
| 21 | `arr_del` (macro pipeline over `arrgrowf`) | `num ∈ {0, ±1, INT_MIN, INT_MAX, random}` — exercises `arrput`/`arrdel`/`arrdelswap`/`arrfree` for `i = 0..3` | [x] |

## D. `stbds_stralloc` / `stbds_strreset` (arena, lowest level)

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 22 | `stbds_stralloc` | fresh arena (`{0,0,0,0}`), short string → `len > remaining`, `len <= 512` → first block, `block` 0→1 | [x] |
| 23 | `stbds_stralloc` | subsequent strings while `len <= remaining` → carved from the same block, `block` unchanged | [x] |
| 24 | `stbds_stralloc` | string longer than the computed `blocksize` **with `storage == NULL`** → dedicated block becomes head, `remaining = 0` | [x] |
| 25 | `stbds_stralloc` | string longer than `blocksize` **with `storage != NULL`** → dedicated block spliced *after* the head, `remaining` untouched | [x] |
| 26 | `stbds_stralloc` | 40 random ~450-530-byte strings, each forcing a new block, walking `a->block` up through the `blocksize = 512<<(block>>1)` progression | [x] |
| 27 | `stbds_stralloc` | forged `block ∈ {0,1,2,3,4,5,20,21,22,23,24,110,111,126,127,128,129,254,255}` → saturation at `1<<20` (block 22+) and the shift-count-mod-64 wrap that makes `blocksize == 0` (block 110+). Values 25..109 are skipped on purpose: they ask for 4 MB–8 EB blocks. | [x] |
| 28 | `stbds_stralloc` | `""` (len = 1) repeatedly, incl. exactly exhausting `remaining` | [x] |
| 29 | `stbds_strreset` | arena with 0, 1, 2, N blocks (mixed normal + oversized) → all freed, struct zeroed | [x] |
| 30 | `stbds_stralloc` (via map) | `SH_ARENA` map: keys of length 1, 511, 512, 513, 4096 driving the same paths through `stbds_hmput_key` | [x] |

## E. `stbds_hmput_default` / `stbds_shmode_func`

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 31 | `stbds_hmput_default` | `a = NULL`, `elemsize ∈ {8,12,16,20,32}` | [x] |
| 32 | `stbds_hmput_default` | on a map that already has `length >= 1` → unchanged | [x] |
| 33 | `stbds_hmput_default` | on an array whose `length` was forced to 0 → re-init path | [x] |
| 34 | `stbds_hmput_default` | called **after** puts, then a default value written at `t[-1]`, then `hmget` of a missing key returns index `-1` → `t[-1].value` is the default (the `hmdefault` idiom) | [x] |
| 35 | `stbds_shmode_func` | `mode ∈ {0,1,2,3}` × `elemsize ∈ {8,16,24}` → `string.mode` byte + fully initialised 8-slot index | [x] |
| 36 | `stbds_shmode_func` | `mode ∈ {4,5,255,256,257,512,0x10001,-1,-2,INT_MIN,INT_MAX}` (out-of-enum-range ints) → `(unsigned char)` truncation | [x] |

## F. Binary-keyed maps — `stbds_hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key` / `hmfree_func`

`mode = 0` (`STBDS_HM_BINARY`), `string.mode` ends up `SH_NONE`.

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 37 | put/get | `elemsize=8, keysize=4` (`{int key; int value;}`), 1 key | [x] |
| 38 | put/get | `elemsize=8, keysize=4`, 5 keys (below `used_count_threshold=6`) | [x] |
| 39 | put/get | `elemsize=8, keysize=4`, 6 keys → first growth to 16 slots | [x] |
| 40 | put/get | `elemsize=8, keysize=4`, 12 → 24 → 48 keys → growth to 32/64/128 slots (nested rehash) | [x] |
| 41 | put/get | `elemsize=16, keysize=8` (`{size_t key; size_t value;}`), 0..64 random keys | [x] |
| 42 | put/get | `elemsize=20, keysize=8` (`{int key[2]; int b,c,d;}` — unaligned tail), 0..64 random keys | [x] |
| 43 | put/get | `keysize ∈ {1,2,3,5,6,7,9,16}` (odd widths → all `siphash` tails) with `elemsize = keysize` rounded up | [x] |
| 44 | put | duplicate keys (re-put the same key) — no growth, `temp` = existing index | [x] |
| 45 | put | keys chosen to collide in the same bucket (same `hash & (slot_count-1)`) → wrap-around probe loop, `pos += step` path | [x] |
| 46 | get | key present / key absent / after growth / on an empty (`NULL`) map | [x] |
| 47 | `hmget_key_ts` | explicit `temp` out-param, present + absent, on `NULL` map, on map without index | [x] |
| 48 | del | delete existing key, `old_index == final_index` (last element) | [x] |
| 49 | del | delete existing key, `old_index != final_index` (swap-with-last + index fix-up) | [x] |
| 50 | del | delete then re-put the same key → tombstone reuse (`tombstone >= 0` branch, `--tombstone_count`) | [x] |
| 51 | del | enough deletes to trip `tombstone_count > tombstone_count_threshold` → rebuild at same `slot_count` | [x] |
| 52 | del | enough deletes to trip `used_count < used_count_shrink_threshold && slot_count > 8` → shrink | [x] |
| 53 | del | delete every key one by one down to empty (`length` 1) | [x] |
| 54 | put/get/del | **randomized mixed workload** (300 ops, random put/get/del/put-dup) against a Rust-side reference model, `elemsize=16, keysize=8` | [x] |
| 55 | del | `keyoffset ∈ {0, 1, 4, 7, 8}` on an `elemsize=16, keysize=8` map (non-zero offset ⇒ comparison at the wrong place) — `cfg55_del_with_nonzero_keyoffset` + `err_b3_keyoffset_one_past_the_key` | [x] |
| 56 | `hmfree_func` | on `SH_NONE` map with index, on map without index, on `NULL` | [x] |

## G. String-keyed maps — all three `string.mode`s

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 57 | put/get, `mode=1` | implicit `SH_DEFAULT` (map created by `hmput_key` itself), `elemsize=16, keysize=8`, 1/5/6/20/50 random keys | [x] |
| 58 | put/get, `mode=1` | `SH_STRDUP` map (`shmode_func(_,2)`), same key counts → `stbds_strdup`ed keys | [x] |
| 59 | put/get, `mode=1` | `SH_ARENA` map (`shmode_func(_,3)`), same key counts → arena-allocated keys | [x] |
| 60 | put, `mode=1` | `SH_NONE` map (`shmode_func(_,0)`) with `mode=1` → the `default:` arm `memcpy`s the first `keysize` **bytes of the string** into the element (not the pointer). Only puts of distinct keys are well-defined afterwards, since any later lookup would `strcmp` through those bytes as a pointer. | [x] |
| 61 | put, `mode=1` | duplicate key → the first probe loop updates `temp_key`, the wrapped loop does not | [x] |
| 62 | put/get, `mode=1` | key strings of length 0, 1, 7, 8, 15, 16, 63, 512, 513 (arena block boundaries) | [x] |
| 63 | del, `mode=1` | `SH_DEFAULT` / `SH_STRDUP` / `SH_ARENA` × (`old_index == final_index`, `!=`) | [x] |
| 64 | del, `mode=2` | `SH_STRDUP` map deleted with `mode = 2` → the `mode == STBDS_HM_STRING` equality test fails, key is **not** freed (leak preserved) | [x] |
| 65 | put/get/del, `mode=2` | `STBDS_HM_PTR_TO_STRING`: `mode=2 >= 1` so string hashing/compare, but `string.mode` init in `hmput_key` is `SH_DEFAULT` | [x] |
| 66 | put/get, `mode ∈ {3, 99, INT_MAX}` | out-of-enum-range `mode` — still `>= 1`, so string path | [x] |
| 67 | put/get, `mode ∈ {-1, INT_MIN}` | out-of-enum-range negative `mode` — `< 1`, so binary path | [x] |
| 68 | put/get/del, `mode=1` | **randomized mixed workload** (300 ops) × each of `SH_DEFAULT/STRDUP/ARENA` | [x] |
| 69 | `hmfree_func` | `SH_STRDUP` map (frees each key), `SH_ARENA` map (`strreset` frees blocks), `SH_DEFAULT` map | [x] |
| 70 | put/get | string keys crafted to share a bucket (`hash_string` collisions modulo `slot_count`) | [x] |

## H. Top-level helpers

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 71 | `strkey` | `n ∈ {0, 1, 9, 10, 99, 100, -1, -9, -10, INT_MIN, INT_MAX, random}` → full 256-byte buffer compared | [x] |
| 72 | `arr_del` | `num` random + extremes (covered by row 21; asserted not to crash and to leave no state) | [x] |

## Test-name index (all rows verified)

| rows | test file | tests |
|------|-----------|-------|
| 1–7   | `tests/diff_lowlevel.rs` | `cfg01_hash_bytes_zero_len` … `cfg07_hash_bytes_seed_matrix` |
| 8–12  | `tests/diff_lowlevel.rs` | `cfg08_hash_string_empty` … `cfg12_rand_seed_and_lcg_advance` |
| 13–21 | `tests/diff_lowlevel.rs` | `cfg13_arrgrowf_min_cap_below_4` … `cfg21_arr_del_all_inputs` |
| 22–29 | `tests/diff_lowlevel.rs` | `cfg22_stralloc_first_block` … `cfg29_strreset_various_chain_lengths` |
| 30    | `tests/diff_map.rs`      | `cfg62_30_string_key_lengths_and_arena_blocks` |
| 31–36 | `tests/diff_lowlevel.rs` | `cfg31_hmput_default_from_null` … `cfg36_shmode_func_out_of_range_modes` |
| 37–56 | `tests/diff_map.rs`      | `cfg37_38_39_binary_small_counts` … `cfg56_hmfree_variants` |
| 57–70 | `tests/diff_map.rs`      | `cfg57_string_sh_default_implicit` … `cfg70_string_bucket_collisions` |
| 71    | `tests/diff_lowlevel.rs` | `cfg71_strkey` |
| 72    | `tests/diff_lowlevel.rs` | `cfg21_arr_del_all_inputs` |

## How the comparison is made

Absolute addresses differ between the two libraries, so every assertion is on a
**canonical serialisation** (`tests/common/mod.rs`) rather than on raw pointers:

* `dump_array` / `dump_map`: `length`, `capacity`, `temp`, every live element,
  and the *entire* hash index — `slot_count`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, the embedded arena's
  shape, and **every bucket's `hash[8]` and `index[8]`**.
* string keys are compared by *content* (the pointer differs for
  `SH_STRDUP`/`SH_ARENA`), binary keys byte-for-byte.
* `canon_elements` / `Map::fill_value` overwrite the element bytes the C
  deliberately leaves indeterminate (everything past `keysize`, straight out of
  `realloc`) with a deterministic pattern, so the raw comparison is meaningful.
* `stralloc_class` classifies where `stbds_stralloc` placed the string
  (carved out of the head block / head block base / block spliced behind the
  head) without depending on addresses.
* `temp_key` is compared explicitly (by content) only where the C has actually
  written it — `stbds_make_hash_index` never initialises the field.
* Both libraries are re-seeded with `stbds_rand_seed` at the start of every
  scenario, and every test serialises on one mutex, so the two process-global
  `stbds_hash_seed` LCGs stay in lock-step.
* `stbds_hmdel_key`'s reachable `assert` is compared as **abort parity** in a
  child process (`err32_del_mode2_refind_aborts`).

## Harness sensitivity (mutation check)

To prove the suite is not vacuously green, 21 mutations were injected into
`src/` one at a time and the suite re-run:

| mutation | caught |
|----------|--------|
| `hash_string` avalanche shift `18` → `17` | yes |
| `hash_string` final `+ seed` dropped | yes |
| siphash tail `case 4` sign-extension → zero-extension | yes |
| siphash finalisation `v2 ^= 0xff` → `0xfe` | yes |
| `arrgrowf` minimum-capacity floor `4` → `5` | yes |
| `stbds_log2` loop bound off-by-one | yes |
| `make_hash_index` `tombstone_count_threshold` shift | yes |
| `make_hash_index` rehash `used_count` carry-over | yes |
| `is_key_equal` `mode >= 1` → `mode > 1` | yes |
| `hmput_key` duplicate-hit `temp_key` update removed | yes |
| `hmput_key` stored bucket index `i-1` → `i` | yes |
| `hmget_key_ts` sentinel `-1` → `-2` | yes |
| `hmdel_key` success sentinel `1` → `2` | yes |
| `hmdel_key` shrink test `<` → `<=` | yes |
| `stralloc` saturation test `<` → `<=` | yes |
| `stralloc` oversize head/splice branch inverted | yes (test binary aborts) |
| `strkey` NUL terminator not copied | yes |
| `strkey` `(n as i64).unsigned_abs()` → `n.wrapping_neg()` | equivalent mutant (same value for every `i32`) |
| `hmdel_key` `null_mut()` → `a` on the `a == NULL` path | equivalent mutant (`a` *is* NULL there) |

19 distinct mutations: 17 detected, 2 provably equivalent (no behavioural
change), 0 real escapes.
