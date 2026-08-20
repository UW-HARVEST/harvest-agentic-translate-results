# CONFIGS.md — configuration-surface table (Phase A → gates Phase B)

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| `mode` (runtime flag, plain `int`) | `mode >= STBDS_HM_STRING(1)` ⇒ hash/compare via `stbds_hash_string`+`strcmp`; `mode <= 0` ⇒ `stbds_hash_bytes`+`memcmp`. `stbds_hmdel_key` additionally tests `mode == 1` **exactly** | `lib.c:560,590,713,836,842` |
| `table->string.mode` (per-table state) | `STBDS_SH_NONE(0)` / anything unlisted ⇒ `memcpy` raw key bytes; `STBDS_SH_DEFAULT(1)` ⇒ store caller's `char*`; `STBDS_SH_STRDUP(2)` ⇒ `stbds_strdup`; `STBDS_SH_ARENA(3)` ⇒ `stbds_stralloc` | `lib.c:785-790` |
| how the table is created | implicitly by `stbds_hmput_key` (⇒ `string.mode = DEFAULT` if `mode>=1` else `NONE`) **or** explicitly by `stbds_shmode_func(elemsize, mode)` (⇒ `string.mode = (u8)mode`) | `lib.c:707`, `lib.c:803` |
| `elemsize` / `keysize` shape | `keysize == elemsize`, `keysize < elemsize`, `keysize = 8` pointer key, `elemsize` 8/12/16/24/32; `keysize=0` | all `hm*` |
| `keyoffset` (`stbds_hmdel_key` only) | `0` (what the `hm*`/`sh*` macros pass) and non-zero (`STBDS_OFFSETOF` for a key that is not the first member) | `lib.c:807` |
| element **count** (drives table growth) | `0`, `1`, `2`, `5` (`used_count_threshold` for 8 slots is 6), `6` ⇒ grow to 16, `12` ⇒ grow to 32, `24` ⇒ 64, hundreds/thousands ⇒ repeated doubling | `lib.c:698,702` |
| bucket-probe shape | insert/lookup that resolves inside `pos&MASK … 8`; one that wraps into the `0 … limit` loop; one that needs ≥2 buckets (`pos += step; step += 8`) | `lib.c:728-764`, `600-628` |
| tombstones | `0`; `1..threshold` (`(n>>3)+(n>>4)`); `> threshold` ⇒ **rebuild** at same `slot_count`; `used_count < used_count_shrink_threshold (n>>2)` **and** `slot_count > 8` ⇒ **shrink** to `n>>1`; the `shrink_threshold` is forced to `0` when `slot_count <= 8` | `lib.c:395-400,854-862` |
| put-onto-tombstone | `tombstone >= 0` at `found_empty_slot` ⇒ reuse slot, `--tombstone_count` | `lib.c:766` |
| swap-with-last on delete | `old_index == final_index` (delete the last element) vs `old_index != final_index` (memmove + re-find + `b->index[i] = old_index`) | `lib.c:839` |
| array growth path (`stbds_arrgrowf`) | `min_cap <= arrcap` ⇒ no-op; `a == NULL` ⇒ fresh (init `length/hash_table/temp`); `min_cap < 2*arrcap` ⇒ double; `min_cap < 4` ⇒ floor 4; `min_cap` from `addlen` vs from explicit `min_cap` | `lib.c:283-307` |
| hash seed state | `stbds_hash_seed` starts at `0x31415926` and is advanced (`seed*a+b`) on **every** `make_hash_index(_, NULL)`; a rehash/shrink/grow passes `ot != NULL` and therefore **inherits** the seed and does **not** advance the global | `lib.c:353,403-413` |
| `stbds_hash_bytes` input shape | `len` 0..7 (each `switch` fall-through case), 8, 9..15, exact multiples of 8, ≥16; bytes with the **high bit set** in positions 3 and 7 (the sign-extension quirk); `seed` = 0, 1, `0x31415926`, `SIZE_MAX` | `lib.c:498-551` |
| `stbds_hash_string` input shape | empty string, 1 char, long string, bytes ≥ 0x80 (`(unsigned char)` cast), embedded high-bit UTF-8; `seed` extremes | `lib.c:477-491` |
| string-arena shape | `len <= remaining` (carve from current block); `len > remaining` & `len <= blocksize` (new 512·2^k block); `len > blocksize` with `storage==NULL`; `len > blocksize` with `storage!=NULL`; `block` counter walk 0→…→MAX(1<<20) saturation | `lib.c:881-918` |
| `stbds_strreset` | empty arena (`storage==NULL`), 1 block, many blocks | `lib.c:920-930` |
| `str_put` | `num` = 0, 1, 2, 3, 7, 8, 100, 1000, negative (`INT_MIN`) — drives the `stralloc`/`strkey` loop and the printed value. `num` near `INT_MAX` is excluded: the loop performs `num` allocations and OOMs identically in both | `lib.c:945-967` |

## Combination rows (Phase B checklist)

Each row is exercised with **many randomized inputs** (seeded xorshift, fixed
seeds ⇒ reproducible) and both libraries' full observable state is compared
byte-for-byte: array header (`length`, `capacity`, `temp`), all live element
bytes, the whole `stbds_hash_index` (`slot_count`, `used_count`, all four
thresholds, `tombstone_count`, `seed`, `slot_count_log2`, `string.{remaining,
block,mode}`) and **every** `hash[8]`/`index[8]` entry of **every** bucket.
Pointer-valued keys are compared by their pointed-to C string (heap addresses
legitimately differ between the two `.so`s).

| #  | entry point(s) | configuration (options set + input shape) | status + test |
|----|----------------|-------------------------------------------|-----|
|  1 | `stbds_hash_bytes` | `len = 0..64` × random bytes × seed ∈ {0, 1, 0x31415926, SIZE_MAX, random} | [x] `cfg_01_hash_bytes_len_sweep` |
|  2 | `stbds_hash_bytes` | `len % 8 ∈ 1..7` with byte[3] / byte[6] / byte[7] ≥ 0x80 — the `int` sign-extension `switch` cases | [x] `cfg_02_hash_bytes_tail_sign_extension` |
|  3 | `stbds_hash_bytes` | `len` exact multiples of 8 (8,16,…,256), bytes with high bit set in lanes 3 and 7 — main-loop sign extension | [x] `cfg_03_hash_bytes_main_loop_sign_extension` |
|  4 | `stbds_hash_string` | empty / 1 / 2 / 8 / 100-char strings, ASCII and bytes 0x80..0xFF, seed sweep | [x] `cfg_04_hash_string_sweep` |
|  5 | `stbds_rand_seed` + `stbds_hash_bytes`/`_string` | seed set to 0, 1, SIZE_MAX, random; verify the *global* seed feeds new tables and advances identically | [x] `cfg_05_global_seed_advance` |
|  6 | `stbds_arrgrowf` | `a=NULL`, random `elemsize ∈ 1..64`, `addlen ∈ 0..8`, `min_cap ∈ 0..40` — fresh-alloc + cap-floor-4 + no-op paths | [x] `cfg_06_arrgrowf_from_null` |
|  7 | `stbds_arrgrowf` | repeated growth on an existing array: doubling path (`min_cap < 2*arrcap`) vs explicit-`min_cap` path, then `stbds_arrfreef` | [x] `cfg_07_arrgrowf_repeated` |
|  8 | `stbds_arrgrowf` (as `arrput`) | `arrmaybegrow`+`length++` loop for 0,1,2,3,4,5,8,17,100,1000 elements, `elemsize` 1/2/4/8/16 | [x] `cfg_08_arrput_sequences` |
|  9 | `stbds_hmput_key` | `mode=0` (BINARY), table auto-created (`string.mode=NONE`), `elemsize=8/keysize=4` int keys, count 0,1,2,5,6,12,24 (crosses 8→16→32→64 slots) | [x] `cfg_09_binary_int_growth_boundaries` |
| 10 | `stbds_hmput_key` | `mode=0`, `elemsize=16/keysize=8` (`struct2 {int key[2]; …}`), random keys, count up to 1000 | [x] `cfg_10_binary_struct2_random` |
| 11 | `stbds_hmput_key` | `mode=0`, `keysize == elemsize` (no value area), and `keysize < elemsize` with the value area written by the caller (macro emulation) | [x] `cfg_11_keysize_shapes` |
| 12 | `stbds_hmput_key` | `mode=0`, **duplicate** keys re-put (hits both dup-found loops), value overwritten each time | [x] `cfg_12_binary_duplicates` |
| 13 | `stbds_hmput_key` | `mode=1` (STRING), table auto-created ⇒ `string.mode=DEFAULT`, caller-owned `char*` keys, count 0..500 | [x] `cfg_13_string_default_autotable` |
| 14 | `stbds_shmode_func`(STRDUP) + `stbds_hmput_key`(1) | `string.mode=STRDUP` ⇒ keys `strdup`'d; duplicate puts; `stbds_hmfree_func` frees every key | [x] `cfg_14_string_strdup` |
| 15 | `stbds_shmode_func`(ARENA) + `stbds_hmput_key`(1) | `string.mode=ARENA` ⇒ keys allocated from the table's `stbds_string_arena`; long keys force the oversized-block path *inside* the map | [x] `cfg_15_string_arena` |
| 16 | `stbds_shmode_func`(NONE=0) + `stbds_hmput_key`(1) | **mixed**: STRING hashing but `default:` `memcpy(elem,key,keysize)` — copies string *bytes* (row 26 of ERRORS.md as a valid config) | [x] `cfg_16_string_mode_on_none_table` |
| 17 | `stbds_shmode_func`(DEFAULT) + `stbds_hmput_key`(**0**) | **mixed**: BINARY hashing/compare of the *pointer bytes* but the stored key is the pointer (DEFAULT branch) | [x] `cfg_17_binary_mode_on_default_table` |
| 18 | `stbds_hmput_key` | `mode = 2 / 7 / 1000 / INT_MAX` (out-of-range enum ⇒ STRING) on DEFAULT and STRDUP tables | [x] `cfg_18_mode_out_of_range_string` |
| 19 | `stbds_hmput_key` | `mode = -1 / INT_MIN` (out-of-range ⇒ BINARY) | [x] `cfg_19_mode_out_of_range_binary` |
| 20 | `stbds_hmget_key_ts` | lowest-level getter, `temp` out-param, present / absent keys, both modes, table present & absent | [x] `cfg_20_hmget_key_ts_states` |
| 21 | `stbds_hmget_key` | wrapper that copies `temp` into the header, present / absent keys, both modes | [x] `cfg_21_hmget_key_wrapper` |
| 22 | `stbds_hmput_default` | `hmdefault` on a fresh (NULL) map, on a map with `length==0`, and on a populated map (no-op path) | [x] `cfg_22_hmput_default` |
| 23 | `stbds_hmdel_key` | `mode=0`, delete the **last** element (`old_index == final_index`) | [x] `cfg_23_del_last_element` |
| 24 | `stbds_hmdel_key` | `mode=0`, delete a **middle** element ⇒ memmove + binary re-find + `b->index[i]=old_index` | [x] `cfg_24_del_middle_binary` |
| 25 | `stbds_hmdel_key` | `mode=1`, delete middle ⇒ **string** re-find path (`*(char**)elem`) | [x] `cfg_25_del_middle_string_default` |
| 26 | `stbds_hmdel_key` | `mode=1` on a STRDUP table ⇒ the removed key is `free`d | [x] `cfg_26_del_strdup` |
| 27 | `stbds_hmdel_key` | delete-then-reinsert so `stbds_hmput_key` lands on a **tombstone** (`tombstone >= 0`, `--tombstone_count`) | [x] `cfg_27_put_onto_tombstone` |
| 28 | `stbds_hmdel_key` | enough deletes to exceed `tombstone_count_threshold = (n>>3)+(n>>4)` ⇒ **rebuild** at the same `slot_count` (ot != NULL rehash, seed inherited) | [x] `cfg_28_tombstone_rebuild` |
| 29 | `stbds_hmdel_key` | enough deletes so `used_count < (slot_count>>2)` with `slot_count > 8` ⇒ **shrink** to `slot_count>>1` | [x] `cfg_29_shrink` |
| 30 | `stbds_hmdel_key` | `slot_count == 8` boundary: `used_count_shrink_threshold` forced to `0` ⇒ never shrinks below 8 | [x] `cfg_30_no_shrink_below_8` |
| 31 | `stbds_hmdel_key` | non-zero `keyoffset` (key is not the first struct member), binary and string modes | [x] `cfg_31_keyoffset` |
| 32 | `stbds_hmdel_key` | delete **all** elements one by one, then re-put — full lifecycle across grow/shrink/rebuild | [x] `cfg_32_full_lifecycle` |
| 33 | full pipeline | randomized op stream (put / get / get_ts / del / default) of 2000 ops over `elemsize=8,keysize=4`, comparing state after **every** op | [x] `cfg_33_random_op_stream_binary` |
| 34 | full pipeline | randomized op stream over the STRING/DEFAULT map, 2000 ops, state compared after every op | [x] `cfg_34_random_op_stream_string_default` |
| 35 | full pipeline | randomized op stream over the STRDUP map, 2000 ops | [x] `cfg_35_random_op_stream_strdup` |
| 36 | full pipeline | randomized op stream over the ARENA map, 2000 ops (also stresses the arena block walk) | [x] `cfg_36_random_op_stream_arena` |
| 37 | `stbds_stralloc` | fresh arena, many short strings ⇒ carve from the current block; `block` counter walk 0,1,2,… and `remaining` bookkeeping | [x] `cfg_37_arena_short_strings` |
| 38 | `stbds_stralloc` | strings longer than the current `blocksize` with `storage == NULL` (very first call) ⇒ own block, `remaining = 0` | [x] `cfg_38_arena_oversize_first` |
| 39 | `stbds_stralloc` | strings longer than `blocksize` with `storage != NULL` ⇒ spliced own block, `remaining` untouched | [x] `cfg_39_arena_oversize_later` |
| 40 | `stbds_stralloc` | pre-set `a->block` = 0..7, 20..31, 110..145, 238..255 ⇒ the `512 << (block>>1)` MAX saturation, the wrap-to-0 for `block>>1 >= 55`, and the x86-64 shift-count masking that makes `block == 128` behave like `block == 0`. Starts whose blocksize is huge but non-zero (`block>>1` ≈ 19..54, and 166..237 after masking) make `realloc` fail and both libraries dereference NULL — covered by the Phase C subprocess test `err_38b_stralloc_huge_block_alloc_failure` | [x] `cfg_40_arena_block_counter` |
| 41 | `stbds_stralloc` + `stbds_strreset` | empty arena / 1 block / many blocks / mixture of normal and oversized blocks, then reset and re-use the arena | [x] `cfg_41_strreset` |
| 42 | `strkey` | `n` = 0,1,9,10,99,100,-1,-99, `INT_MIN`, `INT_MAX`, random — returned C string compared | [x] `cfg_42_strkey` |
| 43 | `str_put` | `num` = 0,1,2,3,4,7,8,9,16,63,64,100,1000,5000, -1,-2,-100,-1000, `INT_MIN`, + 40 random, x 4 hash seeds — **stdout captured** (`dup2` onto a temp file + `fflush(NULL)`) and compared byte-for-byte. (`num` near `INT_MAX` is excluded: the C loops `num` times doing a `stralloc`, i.e. ~2^31 allocations, so it OOMs identically in both and is not a useful comparison.) | [x] `cfg_43_str_put_stdout` |
| 44 | `str_put` | called repeatedly in one process (static `buffer` reuse + global seed advance) — output of a 20-call sequence compared | [x] `cfg_44_str_put_repeated` |
| 45 | `stbds_hmfree_func` | free a NONE / DEFAULT / STRDUP / ARENA table, `length` 1..N; then reuse the handle from scratch | [x] `cfg_45_hmfree_all_flavours` |
| 46 | probe-collision shape | keys hand-picked so `pos & MASK != 0` and the scan wraps into the `0 … limit` loop, plus keys forcing `pos += step` onto a second bucket (dense 8-slot table) | [x] `cfg_46_probe_shapes` |
| 46b | `stbds_hmput_key` + `stbds_hmget_key`/`_ts` + the rehash inside `stbds_make_hash_index` | >= 24 hand-picked keys that all hash into **bucket 0 of a 64-slot table**, inserted so the table grows 8→16→32→64. The 32→64 rehash then has to spill bucket 0 → bucket 1 → bucket 3, which is the only way the `pos += step; step += 8` walk takes its **second** advance. Also covers long lookup chains for present and absent keys, and deleting the whole colliding set | [x] `cfg_46b_rehash_multibucket_spill` |

## Row status

**47 / 47 rows (1..46 plus 46b) pass across their randomized inputs**, in both
the `dev` and the `release` profile, for the single valid feature combination
(Cargo.toml declares no `[features]`).

### Deliberately excluded, and why

These are configurations where the C's own result depends on a **heap address**,
so no two distinct `.so`s can agree and a byte-for-byte comparison is
meaningless. They are documented rather than tested:

| configuration | why it is not comparable |
|---------------|--------------------------|
| `stbds_hmdel_key` with `mode != 1` but `mode >= 1` on a **pointer-key** table (SH_DEFAULT / SH_STRDUP / SH_ARENA) *and* a delete that triggers the swap-with-last | the re-find key is `(char*)a + elemsize*old_index + keyoffset`, i.e. the *address of the pointer slot*; with `mode >= 1` the C then runs `stbds_hash_string` over the **bytes of the stored heap pointer**. Rows 18/35 therefore delete LIFO (`old_index == final_index`), which skips the re-find entirely. |
| `stbds_hmdel_key` with `mode = 0` on an SH_DEFAULT table plus a swapping delete | same reason via `stbds_hash_bytes(&stored_pointer, keysize)`. Row 17 uses put/get only. |
| lookups on an SH_NONE table in STRING mode where the hash actually matches | `stbds_is_key_equal` reinterprets the memcpy'd *string bytes* as a `char *` and dereferences it — identical UB in both libraries, but it is a wild pointer, so row 16 inserts distinct keys only. |
| `stbds_stralloc` with `a->block` in the "huge but non-zero blocksize" window | `realloc` fails, both libraries dereference NULL. Compared as a crash instead: `err_38b_stralloc_huge_block_alloc_failure`. |
| `str_put(num)` for `num` near `INT_MAX` | `num` arena allocations ⇒ OOM in both. |

### What "state compared" means, concretely

`tests/common/mod.rs::snapshot_map` renders, and `MapPair::check` diffs after
**every single operation**:

* `stbds_array_header`: `length`, `capacity`, `temp`, and whether `hash_table`
  is set (the pointer value itself legitimately differs);
* every byte of every live element (`0 .. length`), with `char*` fields rendered
  as their pointee bytes;
* the whole `stbds_hash_index`: `slot_count`, `slot_count_log2`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, and `string.{remaining, block, mode}`
  plus whether `string.storage` is set;
* **all** `hash[8]` and `index[8]` entries of **all** `slot_count/8` buckets.

`temp_key` is compared separately (`MapPair::assert_temp_key_matches`) and only
in put-only sequences: after a delete or a table rebuild the C leaves
`table->temp_key` stale or uninitialised, and for SH_STRDUP it can point at
freed memory.

## Mutation evidence (the checklist is not vacuous)

To confirm the differential harness actually observes the behaviour it claims
to, 22 deliberate bugs were injected into `src/lib.rs` one at a time; the C
source was never touched. **21 were caught** by the row indicated:

| mutant | caught by |
|--------|-----------|
| siphash tail `lo as i64` → `lo as u32` (drops the C `int` sign-extension) | `cfg_02` |
| siphash `STBDS_SIPHASH_D_ROUNDS` 4 → 3 | `cfg_01` |
| `stbds_hash_string` rotate 9 → 10 | `cfg_04` |
| `stbds_load_32_or_64` `v64_lo` of the seed multiplier changed | `cfg_05` |
| `stbds_load_32_or_64` `v64_hi` of the seed multiplier changed | `cfg_05` |
| `bucket->index[…] = i-1` → `i` | `cfg_09` |
| grow test `used_count >= threshold` → `>` | `cfg_09` |
| `arrgrowf` capacity floor 4 → 3 | `err_02` |
| `hmput_default` drops the `length == 0` case | `err_19` |
| `shmode_func` clamps instead of truncating `(unsigned char) mode` | `err_26` |
| `tombstone_count_threshold` `(n>>3)+(n>>4)` → `n>>3` | `cfg_28` |
| `hmdel_key` shrink/rebuild branch order swapped | `cfg_29` |
| rehash does not inherit `ot->seed` | `cfg_29` |
| rehash does not inherit `ot->string` (arena state) | `cfg_36` |
| rehash `step += 8` → `step += 16` | `cfg_46b` |
| rehash wrap-around (`0 .. limit`) scan removed | `cfg_46b` |
| `hmput_key` probe `step += 8` → `+= 16` | `cfg_46b` |
| `hm_find_slot` probe `step += 8` → `+= 16` | `cfg_46b` |
| `hm_find_slot` no early `-1` on an empty slot | `err_12` |
| `temp_key` *is* set in the second probe loop (C does not) | `err_24b` |
| `str_put`'s `*strmap[0].key == 'a'` assert made to fire | `cfg_43` (SIGABRT) |
| `stbds_stralloc` `wrapping_shl` → `checked_shl().unwrap_or(0)` | `cfg_40` |
| both `stbds_hmdel_key` asserts removed | `err_33`, `err_34` |

The single survivor is **equivalent, not a gap**: removing *only*
`assert(slot >= 0)` still aborts, because the very next statement is
`assert(b->index[i] == final_index)`, which fails on the garbage it reads —
so both libraries still die with `SIGABRT`. Removing both *is* caught.
(Likewise, mutating the `v32` argument of `stbds_load_32_or_64` is provably a
no-op on a 64-bit target: the macro computes
`var = (v64_hi<<32) ^ ((v64_lo ^ v32) ^ v32)`, in which `v32` cancels.)
