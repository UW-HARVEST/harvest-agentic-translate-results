# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the
`if` / `switch` / ternary branches in `c_src/src/lib.c`.

## Axes the C actually branches on

**A1 — hash mode (`int mode` parameter of `hmget_key`/`hmget_key_ts`/`hmput_key`/`hmdel_key`)**
- `mode < STBDS_HM_STRING` (i.e. `<= 0`) → `stbds_hash_bytes(key, keysize, seed)` + `memcmp`
- `mode >= STBDS_HM_STRING` (i.e. `>= 1`) → `stbds_hash_string(*key, seed)` + `strcmp`
- `hmdel_key` additionally splits on `mode == STBDS_HM_STRING` **exactly** (L836, L841):
  `mode == 1` frees the strdup'd key and re-finds via `*(char**)`; `mode >= 2`
  does neither, re-finding via the raw key bytes.

**A2 — `table->string.mode` (the `switch` at L783-788 in `hmput_key`)**
- `STBDS_SH_NONE` (0) → `default:` `memcpy(elem, key, keysize)`
- `STBDS_SH_DEFAULT` (1) → store the caller's `char*` verbatim
- `STBDS_SH_STRDUP` (2) → `stbds_strdup`, and `hmfree_func`/`hmdel_key` free it
- `STBDS_SH_ARENA` (3) → `stbds_stralloc` into `table->string`
- set implicitly by `hmput_key` on first table creation (`mode>=1 ? SH_DEFAULT : 0`)
  or explicitly by `stbds_shmode_func`.

**A3 — table lifecycle / occupancy** (`hmput_key` L698, `hmdel_key` L855-861)
- `table == NULL` → create with `slot_count = 8`
- `used_count >= used_count_threshold` → grow to `slot_count*2` + rehash (for
  `slot_count=8`, threshold is 6)
- delete → `used_count < used_count_shrink_threshold && slot_count > 8` → shrink
  to `slot_count>>1` + rehash
- else `tombstone_count > tombstone_count_threshold` → rebuild at same size
- tombstone reuse: `found_empty_slot` with `tombstone >= 0` → reuse the tombstone

**A4 — probe path halves** (`hm_find_slot`, `hmput_key`, `make_hash_index` rehash)
- first half `i = pos & 7 .. 8`, wrap-around half `i = 0 .. pos & 7`, and the
  multi-bucket `pos += step; step += 8` continuation.

**A5 — `stbds_hash_bytes` input shape** (`switch (len - i)` L536-544 + the block loop)
- `len` = 0 (case 0), 1..7 (each fall-through case), 8 (one full block, case 0),
  9..15, 16, 17..64; byte values with the high bit set in position 3 and 7
  (the signed-`int` overflow quirk in `data = ... | (d[3] << 24)`).

**A6 — `stbds_hash_string` input shape**
- empty, 1 char, ≥8 chars, bytes ≥ 0x80 (`(unsigned char) *str++`), long strings.

**A7 — global seed state** (`static size_t stbds_hash_seed`, mutated at L410)
- default `0x31415926`, explicitly set via `stbds_rand_seed`, and the
  *sequence* — each `make_hash_index(·, NULL)` advances the global seed, so the
  Nth table created gets a different seed. Order-dependent.

**A8 — `stbds_arrgrowf` shape** (L294-320)
- `a == NULL` vs existing; `addlen` = 0 / 1 / many; `min_cap` = 0 / < len /
  > len; the `min_cap < 2*cap` doubling branch vs the `min_cap < 4` floor branch
  vs neither.

**A9 — `stbds_stralloc` / string arena shape** (L893-925)
- `len <= remaining` (fast path), `len > remaining` with `len <= blocksize`
  (new block), `len > blocksize` (dedicated oversize block) with
  `a->storage == NULL` vs `!= NULL` (two different link orders!),
  `a->block` progression `512 << (block>>1)` and the
  `blocksize < 1<<20` increment cutoff.

**A10 — `keysize` / `elemsize` / `keyoffset` shapes**
- `keysize` = 1, 2, 4, 8, 16, and non-power-of-two (3, 5, 12);
  `elemsize` = keysize..keysize+16 (padding); `keyoffset` = 0 (all get/put paths
  hardcode 0) and non-zero (only `hmdel_key` takes it from the caller).

**A11 — element count**
- 0, 1, 2, 5 (below grow), 6 (grow trigger at slot_count 8), 7, 8, 20, 100,
  500 (multiple grows).

**A12 — `strkey` / `intput` scalar domain**
- `strkey`: 0, ±1, small, large, `INT_MIN`, `INT_MAX`.
- `intput`: any `num ∉ {9, 11}` (those abort — see `ERRORS.md` rows 25-26).

## Configuration rows

One row per combination the C treats differently. Every row is driven through
the `.so` exports of **both** libraries and compared byte-for-byte, with many
randomized inputs per row (fixed seed `0xC0FFEE` unless noted).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | A5: `len = 0` | [x] |
| 2 | `stbds_hash_bytes` | A5: `len = 1..7`, randomized bytes (hits every `switch` fall-through case) | [x] |
| 3 | `stbds_hash_bytes` | A5: `len = 8` (exactly one full block, `switch` case 0) | [x] |
| 4 | `stbds_hash_bytes` | A5: `len = 9..15` (one block + partial tail) | [x] |
| 5 | `stbds_hash_bytes` | A5: `len = 16..64` (multiple blocks, all tails) | [x] |
| 6 | `stbds_hash_bytes` | A5 quirk: bytes with high bit set at offsets 3 and 7 (signed `int` overflow in the block gather) | [x] |
| 7 | `stbds_hash_bytes` | A7: same bytes, randomized `seed` incl. 0, `usize::MAX`, `!0>>1` | [x] |
| 8 | `stbds_hash_string` | A6: `""` | [x] |
| 9 | `stbds_hash_string` | A6: 1..64 ASCII chars, randomized | [x] |
| 10 | `stbds_hash_string` | A6: bytes ≥ 0x80 (unsigned-char promotion) | [x] |
| 11 | `stbds_hash_string` | A7: randomized seeds incl. extremes | [x] |
| 12 | `stbds_rand_seed` + `stbds_shmode_func` | A7 sequence: `rand_seed(s)`, then create N tables, observe each table's derived seed indirectly via `hmput_key` slot placement | [x] |
| 13 | `stbds_arrgrowf` | A8: `a=NULL, addlen=0, min_cap=0` → cap 4 | [x] |
| 14 | `stbds_arrgrowf` | A8: `a=NULL, addlen=0, min_cap=1` → cap 4 (`min_cap<4` floor) | [x] |
| 15 | `stbds_arrgrowf` | A8: `a=NULL`, randomized `addlen`/`min_cap` in 0..1000, randomized `elemsize` 1..64 | [x] |
| 16 | `stbds_arrgrowf` | A8: existing array, `min_cap <= cap` → returns unchanged | [x] |
| 17 | `stbds_arrgrowf` | A8: existing array, `min_cap < 2*cap` → doubling branch | [x] |
| 18 | `stbds_arrgrowf` | A8: existing array, `min_cap >= 2*cap` → exact `min_cap` | [x] |
| 19 | `stbds_arrgrowf` + `stbds_arrfreef` | A8/A11: repeated growth chain (0→4→8→16→…) preserving length/temp/hash_table, then free | [x] |
| 20 | `stbds_hmput_default` | `a=NULL` | [x] |
| 21 | `stbds_hmput_default` | `a!=NULL, length==0` | [x] |
| 22 | `stbds_hmput_default` | `a!=NULL, length!=0` (unchanged) | [x] |
| 23 | `stbds_hmput_key` (binary) | A1 `mode=0`, A2 `SH_NONE`, A10 `keysize=4/elemsize=8`, A11 1 element | [x] |
| 24 | `stbds_hmput_key` (binary) | A1 `mode=0`, A11 = 5 elements (no grow) | [x] |
| 25 | `stbds_hmput_key` (binary) | A1 `mode=0`, A11 = 6 elements (A3 grow trigger 8→16 + rehash) | [x] |
| 26 | `stbds_hmput_key` (binary) | A1 `mode=0`, A11 = 100 randomized keys (A3 multiple grows, A4 multi-bucket probing) | [x] |
| 27 | `stbds_hmput_key` (binary) | A1 `mode=0`, A11 = 500 randomized keys with duplicates (update-in-place path L731-740) | [x] |
| 28 | `stbds_hmput_key` (binary) | A10: `keysize ∈ {1,2,3,4,5,8,12,16}` × `elemsize = keysize + pad ∈ {0,4,8}`, randomized keys | [x] |
| 29 | `stbds_hmput_key` (binary) | A1: `mode ∈ {-1, INT_MIN, 0}` all take the binary path — must be identical | [x] |
| 30 | `stbds_hmput_key` (string) | A1 `mode=1`, A2 `SH_DEFAULT` (implicit on first put), randomized C strings | [x] |
| 31 | `stbds_hmput_key` (string) | A1 `mode=1`, A2 `SH_STRDUP` via `shmode_func(elemsize, 2)`, randomized strings | [x] |
| 32 | `stbds_hmput_key` (string) | A1 `mode=1`, A2 `SH_ARENA` via `shmode_func(elemsize, 3)`, randomized strings (drives A9 block progression) | [x] |
| 33 | `stbds_hmput_key` (string) | A1 `mode=2` (out-of-enum, still `>= HM_STRING`) with each A2 mode | [x] |
| 34 | `stbds_hmput_key` (string) | A2 `SH_NONE` but `mode=1`: string hashing with the `default:` memcpy branch (`shmode_func(elemsize, 0)`) | [x] |
| 35 | `stbds_hmput_key` (string) | A11 = 6 / 100 / 500 strings (A3 grow + rehash under string mode) | [x] |
| 36 | `stbds_hmput_key` (string) | duplicate string keys → update path incl. the `temp_key` write (L734-735) | [x] |
| 37 | `stbds_hmget_key` | A1 binary, key present / absent, on tables of size 0..100 | [x] |
| 38 | `stbds_hmget_key` | A1 string, key present / absent, each A2 mode | [x] |
| 39 | `stbds_hmget_key_ts` | same as rows 37-38 but observing the `*temp` out-param instead of the header field | [x] |
| 40 | `stbds_hmget_key_ts` | `a == NULL` (allocating branch, returns non-NULL) | [x] |
| 41 | `stbds_hmdel_key` (binary) | A1 `mode=0`, delete an existing non-last element (triggers the `memmove` + slot re-find at L838-850) | [x] |
| 42 | `stbds_hmdel_key` (binary) | A1 `mode=0`, delete the last element (`old_index == final_index`, no memmove) | [x] |
| 43 | `stbds_hmdel_key` (binary) | A1 `mode=0`, delete-all in insertion order and in reverse order, 20 elements | [x] |
| 44 | `stbds_hmdel_key` (binary) | A3 shrink: grow to slot_count 32+ then delete below `used_count_shrink_threshold` | [x] |
| 45 | `stbds_hmdel_key` (binary) | A3 tombstone rebuild: interleave put/delete so `tombstone_count > threshold` at constant `slot_count` | [x] |
| 46 | `stbds_hmdel_key` (binary) | A3 tombstone *reuse*: delete then re-put a key that probes onto the tombstone | [x] |
| 47 | `stbds_hmdel_key` (string) | A1 `mode=1` + A2 `SH_DEFAULT`, delete existing/absent | [x] |
| 48 | `stbds_hmdel_key` (string) | A1 `mode=1` + A2 `SH_STRDUP` (frees the key at L836) | [x] |
| 49 | `stbds_hmdel_key` (string) | A1 `mode=1` + A2 `SH_ARENA` (does **not** free) | [x] |
| 50 | `stbds_hmdel_key` (string) | A1 `mode=2`: `>= HM_STRING` for hashing but `!= HM_STRING` for the free + re-find split | [x] |
| 51 | `stbds_hmdel_key` | A10 `keyoffset != 0` with a struct whose key is not the first field, binary mode | [x] |
| 52 | `stbds_hmfree_func` | A2 `SH_NONE` populated table | [x] |
| 53 | `stbds_hmfree_func` | A2 `SH_STRDUP` populated table (frees each key, L577-580) | [x] |
| 54 | `stbds_hmfree_func` | A2 `SH_ARENA` populated table (`strreset` frees blocks) | [x] |
| 55 | `stbds_hmfree_func` | A2 `SH_DEFAULT` populated table (does not free caller strings) | [x] |
| 56 | `stbds_shmode_func` | A2: `mode ∈ {0,1,2,3}` — check `string.mode` byte + header fields of the returned table | [x] |
| 57 | `stbds_shmode_func` | A2 out-of-range: `mode ∈ {-1, 4, 255, 256, 259, INT_MIN, INT_MAX}` → truncating `(unsigned char)` cast | [x] |
| 58 | `stbds_stralloc` | A9: fresh arena (`storage=NULL, block=0, remaining=0`), short string → new 512 block | [x] |
| 59 | `stbds_stralloc` | A9: `len <= remaining` fast path, many sequential allocations from one block | [x] |
| 60 | `stbds_stralloc` | A9: `len > blocksize` with `storage == NULL` (`sb->next=0; a->storage=sb; a->remaining=0`) | [x] |
| 61 | `stbds_stralloc` | A9: `len > blocksize` with `storage != NULL` (splices `sb` in **after** the head, keeps `remaining`) | [x] |
| 62 | `stbds_stralloc` | A9: `a->block` progression driven from 0 up to 6+ so `512 << (block>>1)` grows; check `block` increment stops at the `1<<20` cutoff | [x] |
| 63 | `stbds_stralloc` + `stbds_strreset` | A9: allocate a mixed chain of normal + oversize blocks, then `strreset` and confirm the arena is zeroed | [x] |
| 64 | `stbds_strreset` | A9: empty arena (`storage=NULL`) → just zeroes | [x] |
| 65 | `strkey` | A12: `n ∈ {0, 1, -1, 9, 11, 12345, -12345, INT_MAX, INT_MIN}` + 200 randomized `i32` | [x] |
| 66 | `intput` | A12: 200 randomized `num ∉ {9,11}` + `{0,1,7,8,10,12,-1,INT_MIN,INT_MAX}` — must return normally in both | [x] |
| 67 | full pipeline (binary) | randomized op stream: 2000 interleaved put / get / del / free ops over 8 `keysize`×`elemsize` shapes, comparing the entire array payload + every header field after **every** op | [x] |
| 68 | full pipeline (string) | same randomized op stream in string mode × each A2 mode | [x] |
| 69 | seed sensitivity | rows 67-68 repeated with `stbds_rand_seed(s)` for `s ∈ {0, 1, 0x31415926, usize::MAX}` | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` has `crate-type =
["cdylib"]` with no `[[bin]]`. **The project builds no driver binary**, so the
"compare C and Rust stdout" item of the completion gate is not applicable.
Verified with `grep -n add_executable c_src/CMakeLists.txt` (no match) and
`grep -n '\[\[bin\]\]' translation/Cargo.toml` (no match).

## Row → test mapping (all in `translation/tests/`)

| CONFIGS row(s) | test |
|---|---|
| 1 | `phase_b_low::row01_hash_bytes_len0` |
| 2-5 | `phase_b_low::row02_05_hash_bytes_all_lengths` |
| 5-7 (long inputs) | `phase_b_low::row05_07_hash_long_inputs` (len 65..=320, many blocks) |
| 6 | `phase_b_low::row06_hash_bytes_signed_overflow_quirk` |
| 7 | `phase_b_low::row07_hash_bytes_seed_sweep` |
| 8 | `phase_b_low::row08_hash_string_empty` |
| 9, 11 | `phase_b_low::row09_11_hash_string_ascii` |
| 10 | `phase_b_low::row10_hash_string_high_bytes` |
| 12 | `phase_b_map::row12_global_seed_sequence` |
| 13, 14 | `phase_b_low::row13_14_arrgrowf_fresh_small` |
| 15 | `phase_b_low::row15_arrgrowf_fresh_random` |
| 16-18 | `phase_b_low::row16_18_arrgrowf_existing_branches` |
| 19 | `phase_b_low::row19_arrgrowf_chain` |
| 20-22 | `phase_b_map::row20_22_hmput_default` |
| 23-25 | `phase_b_map::row23_25_binary_small_and_grow` |
| 26, 27 | `phase_b_map::row26_27_binary_many_and_duplicates` |
| 26, 44 (large) | `phase_b_map::row26_44_large_map_stress` (2000 keys, slot_count ≥ 4096) |
| 28 | `phase_b_map::row28_keysize_elemsize_matrix` |
| 29 | `phase_b_map::row29_binary_mode_aliases` |
| 30-33, 35, 36 | `phase_b_map::row30_36_string_modes` |
| 34 | `phase_b_map::row34_string_mode_with_sh_none` (puts only — see note) |
| 36 (`temp_key`) | `phase_b_map::row36_temp_key_parity` |
| 32, 62 | `phase_b_map::row32_62_arena_block_progression`, `phase_b_arena::row62_stralloc_block_progression` |
| 37-40 | `phase_b_map::row37_40_get_paths` |
| 41-43 | `phase_b_map::row41_43_binary_delete` |
| 44 | `phase_b_map::row44_delete_shrink` |
| 45, 46 | `phase_b_map::row45_46_tombstones` |
| 47-49 | `phase_b_map::row47_50_string_delete` |
| 50 | `phase_c_errors::err20_21_hmdel_mode2_abort` (see note) |
| 51 | `phase_c_errors::err_generic_keyoffset_mismatch` |
| 52-55 | `phase_b_map::row52_55_hmfree_all_modes` |
| 56, 57 | `phase_b_map::row56_57_shmode_func`, `phase_c_errors::err28_shmode_out_of_range` |
| 58, 59 | `phase_b_arena::row58_59_stralloc_fresh_and_fast_path` |
| 60 | `phase_b_arena::row60_stralloc_oversize_null_storage` |
| 61 | `phase_b_arena::row61_stralloc_oversize_with_existing_head` |
| 63, 64 | `phase_b_arena::row63_64_strreset` |
| 65 | `phase_b_low::row65_strkey` |
| 66 | `phase_b_arena::row66_intput_non_aborting` |
| 67, 69 | `phase_b_map::row67_69_random_pipeline_binary` |
| 68, 69 | `phase_b_map::row68_69_random_pipeline_string` |

Every map/arena row is checked with a **full state comparison after every single
operation** (array header `length`/`capacity`/`temp`, the entire
`stbds_hash_index` — `slot_count`, all four thresholds, `used_count`,
`tombstone_count`, `seed`, `slot_count_log2`, the whole embedded
`stbds_string_arena` — every bucket's `hash[]` and `index[]` arrays, and the full
element payload with pointer-typed keys replaced by the strings they point at),
not just the returned value.

## Notes on rows that touch C undefined behaviour

- **Row 34** (`string.mode == SH_NONE` with a string `mode`): `hmput_key` stores
  the key inline via the `default:` memcpy, so `is_key_equal` would dereference
  those bytes as a `char *`. The row is therefore driven with *puts of distinct
  keys into a fresh table only*, which is the sub-path where every probe
  terminates on an EMPTY slot and `is_key_equal` is never called.
- **Row 50** (`hmdel_key` with `mode >= 2`): `hmdel_key` splits on
  `mode == STBDS_HM_STRING` *exactly*, so the post-`memmove` slot re-find passes
  the raw element bytes instead of the stored `char *`, the lookup fails, and the
  C aborts on `STBDS_ASSERT(slot >= 0)`. Verified as termination parity in a
  subprocess (`ERRORS.md` rows 20-21) rather than in-process.
- **`stbds_stralloc` with `a->block >= 128`**: `512 << (block>>1)` has a shift
  count ≥ 64, which is C undefined behaviour. Row 62 tests `block` in `0..=20`
  (well-defined, moderate allocation sizes) and `110..=127` (well-defined shift
  counts 55..63, where the block size wraps to 0 and every request takes the
  oversize branch). `block >= 128` is not reachable through the public API
  because `++a->block` stops once the block size reaches `1 << 20`.
- **`temp_key`**: `stbds_make_hash_index` never initialises the `temp_key` field,
  so a put that both grows the table *and* finds an existing key leaves it
  indeterminate in the C original too. Row 36's test keeps `used_count` strictly
  below `used_count_threshold` before every duplicate put so no grow can coincide
  with a found-key exit, and asserts the table pointer did not change.
- **Uninitialised element bytes**: `stbds_arrgrowf` never zeroes the payload, so
  element bytes that no `memcpy` covered (struct padding, or everything when
  `keysize == 0`) are indeterminate. The `MapPair` driver writes a deterministic
  fill over the non-key part of each element after every put, so only bytes the
  library actually defines are ever compared.
