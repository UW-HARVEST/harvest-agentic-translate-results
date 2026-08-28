# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`.  Every `STBDS_ASSERT` (== glibc
`assert`), every early `return` that signals "nothing to do / not found", every
NULL check, every explicit range/threshold comparison, and every min/max
constant produces one row.

Legend for "expected C result":

* `-1` / `0` / `NULL` — literal returned sentinel value
* `abort` — glibc `assert` fires → `SIGABRT` (Rust mirrors this with `abort()`)
* `identity` — the input pointer is returned completely unchanged
* `n/a (unreachable)` — the check exists in the source but cannot be triggered
  through any public entry point; documented and reasoned about, tested where a
  synthetic input can reach it

`[x]` = a differential test exists and passes on both `.so`s.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [ ] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `min_len <= min_cap` → growth rejected (`lib.c:286`) | returns `a` unchanged (`identity`), header untouched | `errors.rs::e01_arrgrowf_no_grow` | [x] |
| 2 | `stbds_arrgrowf` | `a == NULL` (`lib.c:300`) — no previous allocation, with `max(addlen,min_cap) > 0` | fresh block, `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(addlen,min_cap,4)` | `errors.rs::e02_arrgrowf_null_input` | [x] |
| 3 | `stbds_arrgrowf` | `elemsize == 0` (with `min_cap > 0`) | allocates only the 32-byte header, `capacity=max(min_cap,4)` | `errors.rs::e03_arrgrowf_zero_elemsize` | [x] |
| 4 | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0` on a NULL array → `min_len == 0`, so `min_cap <= stbds_arrcap(NULL) == 0` matches FIRST (`lib.c:286`) and the `min_cap < 4` branch at `lib.c:291` is never reached | returns the input pointer, i.e. **`NULL`** — *not* a 4-element array | `errors.rs::e04_arrgrowf_zero_zero` | [x] |
| 5 | `stbds_arrgrowf` | `elemsize * min_cap` overflows `size_t` (e.g. `elemsize=1<<63, min_cap=4`) | wrapping multiply → `realloc` of a small/absurd size; both must agree on the *computed* size and the resulting `capacity` field | `errors.rs::e05_arrgrowf_overflow_capacity` | [x] |
| 6 | `stbds_hmfree_func` | `a == NULL` (`lib.c:573`) | returns immediately, no free, no crash | `errors.rs::e06_hmfree_null` | [x] |
| 7 | `stbds_hmfree_func` | `stbds_header(a)->hash_table == NULL` (`lib.c:574`) — array never got a table | skips key/arena cleanup, frees header only | `errors.rs::e07_hmfree_no_table` | [x] |
| 8 | `stbds_hm_find_slot` | probe reaches a slot with `hash == STBDS_HASH_EMPTY (0)` in the *upper* half of the bucket (`lib.c:610`) — key absent | `-1` | `errors.rs::e08_find_slot_miss_upper` | [x] |
| 9 | `stbds_hm_find_slot` | probe reaches `hash == 0` in the *wrapped* (lower) half of the bucket (`lib.c:621`) | `-1` | `errors.rs::e09_find_slot_miss_wrapped` | [x] |
| 10 | `stbds_hmget_key_ts` | `a == NULL` (`lib.c:634`) | `*temp = STBDS_INDEX_EMPTY (-1)`, brand-new 1-element array returned | `errors.rs::e10_hmget_ts_null_map` | [x] |
| 11 | `stbds_hmget_key_ts` | `hash_table == 0`, i.e. array exists but no hash index yet (`lib.c:644`) | `*temp = -1`, `a` returned unchanged | `errors.rs::e11_hmget_ts_no_table` | [x] |
| 12 | `stbds_hmget_key_ts` | key not present, `slot < 0` (`lib.c:648`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `errors.rs::e12_hmget_ts_missing_key` | [x] |
| 13 | `stbds_hmget_key` | same three cases as #10–#12, but the sentinel is written into `stbds_header(p-elemsize)->temp` | `header->temp == -1` | `errors.rs::e13_hmget_key_missing` | [x] |
| 14 | `stbds_hmput_default` | `a == NULL` (`lib.c:669`) | new 1-element array, `length==1` | `errors.rs::e14_hmput_default_null` | [x] |
| 15 | `stbds_hmput_default` | `length == 0` on an existing array (`lib.c:669`) | grows by the default element, `length==1` | `errors.rs::e15_hmput_default_len0` | [x] |
| 16 | `stbds_hmput_default` | `length != 0` → rejected, nothing done (`lib.c:675`) | returns `a` unchanged (`identity`) | `errors.rs::e16_hmput_default_noop` | [x] |
| 17 | `stbds_hmput_key` | `a == NULL` (`lib.c:686`) | allocates array + default element before inserting | `errors.rs::e17_hmput_key_null_map` | [x] |
| 18 | `stbds_hmput_key` | `table == NULL` (`lib.c:698`) — first insert | creates an 8-slot index; `string.mode = (mode >= 1 ? SH_DEFAULT : 0)` | `errors.rs::e18_hmput_key_first_table` | [x] |
| 19 | `stbds_hmput_key` | `used_count >= used_count_threshold` (`lib.c:698`) — table full | rebuild at `slot_count*2`, old table freed | `errors.rs::e19_hmput_key_growth` | [x] |
| 20 | `stbds_hmput_key` | duplicate key found in the upper bucket half (`lib.c:730`) | no insert; `header->temp = existing index`; `table->temp_key` updated for `mode>=1` | `errors.rs::e20_e21_hmput_duplicate_paths` | [x] |
| 21 | `stbds_hmput_key` | duplicate key found in the wrapped bucket half (`lib.c:748`) | no insert; `header->temp = existing index`; `temp_key` **not** written | `errors.rs::e20_e21_hmput_duplicate_paths` | [x] |
| 22 | `stbds_hmput_key` | a tombstone (`index == STBDS_INDEX_DELETED`) was seen before the empty slot (`lib.c:766`) | insert reuses the tombstone slot, `--tombstone_count` | `errors.rs::e22_hmput_reuse_tombstone` | [x] |
| 23 | `stbds_hmput_key` | `(size_t)i+1 > stbds_arrcap(a)` → array must grow (`lib.c:774`) then `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` (`lib.c:778`) | assert holds; array capacity doubles | `errors.rs::e23_hmput_array_growth_assert` | [x] |
| 24 | `stbds_hmput_key` | `table->string.mode` is **not** one of `SH_STRDUP/SH_ARENA/SH_DEFAULT` (`default:` at `lib.c:789`) — reached via `stbds_shmode_func` with an out-of-range mode | `memcpy(elem, key, keysize)` — raw key bytes, *not* a pointer | `errors.rs::e24_put_default_switch_branch` | [x] |
| 25 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` | `keysize == 0` with `mode == STBDS_HM_BINARY` → `memcmp(...,0)` always returns 0, so *every* key with the same hash compares equal | first key wins; `memcpy` of 0 bytes | `errors.rs::e25_zero_keysize_binary` | [x] |
| 26 | `stbds_shmode_func` | `mode` out of the `{0,1,2,3}` enum range: `4`, `255`, `-1` (→ `0xFF`), `256` (→ `0`), `INT_MIN` (→ `0`) — `(unsigned char) mode` truncation at `lib.c:803` | `string.mode` = low byte of `mode`; subsequent puts take the `default:` memcpy branch | `errors.rs::e26_shmode_out_of_range` | [x] |
| 27 | `stbds_hmdel_key` | `a == NULL` (`lib.c:809`) | returns `0` (`NULL`) | `errors.rs::e27_hmdel_null_map` | [x] |
| 28 | `stbds_hmdel_key` | `hash_table == 0` (`lib.c:816`) | `header->temp = 0`, returns `a` unchanged | `errors.rs::e28_hmdel_no_table` | [x] |
| 29 | `stbds_hmdel_key` | key absent, `slot < 0` (`lib.c:821`) | `header->temp = 0`, returns `a` unchanged, `length` unchanged | `errors.rs::e29_hmdel_missing_key` | [x] |
| 30 | `stbds_hmdel_key` | successful delete → `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` (`lib.c:828`) | assert holds | `errors.rs::e30_e31_hmdel_slot_and_used_count_invariants` | [x] |
| 31 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` (`lib.c:832`) — `used_count` is `size_t`, so this is **always true**; it cannot fire even when `used_count` wraps | `n/a (unreachable)`; the wrap is still observable in `used_count` | `errors.rs::e30_e31_hmdel_slot_and_used_count_invariants` | [x] |
| 32 | `stbds_hmdel_key` | `old_index != final_index` → swap-with-last, then `STBDS_ASSERT(slot >= 0)` (`lib.c:846`) and `STBDS_ASSERT(b->index[i] == final_index)` (`lib.c:849`) | for `mode == 0` and `mode == 1` the asserts hold and the moved element's slot is re-pointed to `old_index` | `errors.rs::e32_hmdel_swap_asserts` | [x] |
| 33 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **and** `string.mode == SH_STRDUP` (`lib.c:836`) → key `free`d.  With `mode == 2` / `INT_MAX` (out-of-range but `>= STBDS_HM_STRING`) the `free` is **skipped**, **and** the swap-with-last re-lookup at `lib.c:842` takes the `else` branch, passing the *address of the element* instead of the stored `char *`; `stbds_hm_find_slot` then hashes the pointer bytes, finds nothing, and `STBDS_ASSERT(slot >= 0)` **fires** | `abort` (SIGABRT) when `old_index != final_index`; clean no-`free` delete when `old_index == final_index` | `errors.rs::e33_hmdel_mode_two_no_free` (subprocess for the abort) + `hashmap.rs::c70_delete_mode_two_skips_the_free` | [x] |
| 34 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` (`lib.c:854`) | table rebuilt at `slot_count>>1` | `errors.rs::e34_hmdel_shrink` | [x] |
| 35 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (`lib.c:858`) | table rebuilt at the same `slot_count` | `errors.rs::e35_hmdel_tombstone_rebuild` | [x] |
| 36 | `stbds_hmdel_key` | `slot_count == 8` (min) → shrink branch suppressed because `used_count_shrink_threshold` was forced to `0` (`lib.c:399`) | no shrink ever below 8 slots | `errors.rs::e36_hmdel_no_shrink_at_min` | [x] |
| 37 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` (`lib.c:401`) | holds for every reachable `slot_count` (8,16,32,…): 7<8, 15<16, 30<32, 60<64, 120<128 | `errors.rs::e37_make_hash_index_threshold_invariant` | [x] |
| 38 | `stbds_hm_find_slot` / `stbds_hmput_key` | `hash < 2` (i.e. computed hash collides with `STBDS_HASH_EMPTY=0` or `STBDS_HASH_DELETED=1`) → `hash += 2` (`lib.c:596`, `lib.c:719`) | the sentinel values are never stored as a real hash | `errors.rs::e38_hash_below_two_bias` | [x] |
| 39 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` (`lib.c:913`) | holds on every path (the `len > blocksize` path returns early) | `errors.rs::e39_stralloc_remaining_assert` | [x] |
| 40 | `stbds_stralloc` | `len > blocksize` — oversized string gets its own block; when `a->storage == NULL` the arena is left with `remaining = 0` (`lib.c:896-903`) | returns `sb->storage`; arena `remaining == 0`, `block` incremented | `errors.rs::e40_stralloc_oversized_first` | [x] |
| 41 | `stbds_stralloc` | `len > blocksize` with an existing `a->storage` → new block spliced in *after* the head, `remaining` **not** reset (`lib.c:896-899`) | returns `sb->storage`; `remaining` keeps its old value | `errors.rs::e41_stralloc_oversized_after` | [x] |
| 42 | `stbds_stralloc` | empty string `""` → `len == 1` | 1 byte consumed from the block | `errors.rs::e42_stralloc_empty_string` | [x] |
| 43 | `stbds_stralloc` | `blocksize` clamps at `STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`; `a->block` stops incrementing at 22 (`lib.c:890`) | block counter saturates, blocksize stays `1<<20` | `errors.rs::e43_stralloc_block_saturation` | [x] |
| 44 | `stbds_strreset` | all-zero arena (`storage == NULL`) | no-op, arena zeroed | `errors.rs::e44_strreset_empty` | [x] |
| 45 | `stbds_strreset` | arena with a multi-block chain, including an oversized block | whole chain freed, arena zeroed (`block`/`remaining`/`mode` all 0) | `errors.rs::e45_strreset_chain` | [x] |
| 46 | `stbds_hash_bytes` | `len == 0` (with `p == NULL`) — no byte is dereferenced, `switch(0)` hits `case 0: break` | pure seed-derived hash, no crash | `errors.rs::e46_hash_bytes_zero_len` | [x] |
| 47 | `stbds_hash_bytes` | `len` in `1..=7` — every `switch` fall-through case (`lib.c:533-540`), including `case 4` where `d[3] << 24` becomes a **negative `int`** that sign-extends into `size_t` | identical 64-bit hash | `errors.rs::e47_hash_bytes_tail_cases` | [x] |
| 48 | `stbds_hash_string` | empty string `""` | `hash = seed`, then the mix | `errors.rs::e48_hash_string_empty` | [x] |
| 49 | `stbds_hash_string` | bytes `>= 0x80` — `(unsigned char) *str++` must **not** sign-extend | identical hash | `errors.rs::e49_hash_string_high_bytes` | [x] |
| 50 | `stbds_is_key_equal` | `mode < STBDS_HM_STRING` (including negative `mode`, e.g. `-1`, `INT_MIN`) → `memcmp` branch; `mode >= 1` (including `2`, `7`, `INT_MAX`) → `strcmp` branch (`lib.c:560`) | branch selected purely by `mode >= 1` | `errors.rs::e50_mode_out_of_range_enum` | [x] |
| 51 | `strkey` | `n = 0`, `-1`, `INT_MIN`, `INT_MAX` — `sprintf` into the shared 256-byte static | identical NUL-terminated string, identical (stable) returned pointer across calls | `errors.rs::e51_strkey_extremes` | [x] |
| 52 | `sh_geti` | `num <= 0` (`0`, `-1`, `INT_MIN`) — all `for` bodies skipped, the three `shgeti(...) == -1` asserts still run | no output, no abort | `errors.rs::e52_sh_geti_non_positive` | [x] |
| 53 | `sh_geti` | `STBDS_ASSERT(shgeti(strmap,"foo") == -1)` ×3 (`lib.c:956,961,963`) — before creation, after `sh_new_*`, after `shdefault` | all three hold | `errors.rs::e53_sh_geti_asserts_hold` + every `sh_geti.rs` test | [x] |
| 54 | `sh_geti` | `STBDS_ASSERT(shget(...) == -2)` / `== i*3` (`lib.c:971-981`) — the default-element sentinel `-2` must be returned for absent keys | all hold; process exits normally | `sh_geti.rs::*` | [x] |
| 55 | `stbds_arrfreef` | called on a pointer whose header is valid | frees; calling it with `NULL` would `free(NULL-32)` → UB in **both** implementations, therefore deliberately not exercised | documented | `errors.rs::e55_arrfreef_valid_only` | [x] |
| 56 | `stbds_hmdel_key` | `keyoffset != 0` (the API parameter that `hmput`/`hmget` hard-code to `0`) — the comparison reads the wrong bytes, so the key is never found | `header->temp = 0`, `a` returned unchanged | `errors.rs::e56_hmdel_bad_keyoffset` | [x] |
| 57 | `stbds_hmget_key` / `stbds_hmput_key` | oversized `keysize` for `mode == STBDS_HM_BINARY` — `keysize` up to `elemsize` (e.g. `keysize == elemsize`) | whole element is the key; identical behaviour | `errors.rs::e57_keysize_equals_elemsize` | [x] |
| 58 | `stbds_hmfree_func` | `string.mode == SH_STRDUP` with `length == 1` (only the default element) — the `for (i=1; i<length; ++i)` loop body never runs | nothing freed except table/header | `errors.rs::e58_hmfree_strdup_empty` | [x] |
| 59 | `stbds_hmfree_func` | `string.mode == SH_ARENA` — `stbds_strreset` on the table's arena, keys **not** individually freed | no double free | `errors.rs::e59_hmfree_arena` | [x] |
| 60 | `stbds_hmfree_func` | `string.mode == SH_DEFAULT` — caller-owned keys, neither freed nor reset | caller memory still readable afterwards | `errors.rs::e60_hmfree_default_mode` | [x] |
| 61 | `stbds_stralloc` | hand-crafted `a->block` (an `unsigned char`, so the shift count `a->block >> 1` can reach 127): `(size_t)512u << k` with `k >= 64` is UB in C but on x86-64 the count is masked to 6 bits, and `k` in `55..63` makes the blocksize wrap to **0** so every string takes the oversized-block path | identical returned string + identical `remaining`/`block`/`mode`/`storage` for `block ∈ 0..=23 ∪ 110..=127 ∪ 238..=255` | `errors.rs::e61_stralloc_handcrafted_block_counter` | [x] |
| 62 | `stbds_shmode_func` / `stbds_hmput_default` / `stbds_hmget_key_ts` / `stbds_hmdel_key` / `stbds_hmfree_func` | `elemsize == 0` — `STBDS_ARR_TO_HASH`/`STBDS_HASH_TO_ARR` become the identity and `memset(a,0,0)` is a no-op | `length == 1`, table created normally, `*temp == -1`, `header->temp == 0`, clean free | `errors.rs::e62_zero_elemsize_map_lifecycle` | [x] |
| 63 | `stbds_hash_bytes` / `stbds_hash_string` | **exhaustive** single-byte inputs: every value `0x00..=0xFF` in every tail position `0..7`, for every `len` `1..=8`, × seeds `{0,1,default,MAX}` — pins down the `int` sign-extension of `case 4: d[3] << 24` and the `(unsigned char)` promotion in `hash_string` | identical 64-bit hash for all 4·256·8·8 combinations | `errors.rs::e63_hash_exhaustive_small_inputs` | [x] |
| 64 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmget_key_ts` | `keysize > elemsize` (one step past the element) — `memcmp`/`memcpy` run past the element boundary into the next element | both implementations make the same read and reach the same found/not-found conclusion | `errors.rs::e64_keysize_one_past_element` | [x] |
| 65 | `stbds_make_hash_index` (via `stbds_shmode_func`) | struct-layout boundary: `t->storage = STBDS_ALIGN_FWD((size_t)(t+1), 64)` — a wrong `sizeof(stbds_hash_index)` in the Rust translation would place the bucket array at a different offset | the `(base % 64) -> (storage - base)` function is identical in both `.so`s and equals `ALIGN_FWD(base+104,64)-base`; `sizeof == 104` on LP64 | `errors.rs::e65_hash_index_layout_parity` | [x] |
| 66 | `stbds_arrgrowf` / `stbds_stralloc` | struct-layout boundary: `sizeof(stbds_array_header) == 32` (so the returned payload is 16-byte aligned) and `offsetof(stbds_string_block, storage) == 8` (so the first string in a fresh 512-byte block sits at `block + 8 + 512 - len`) | both hold for both `.so`s | `errors.rs::e66_header_and_block_layout_parity` | [x] |

## Corrections found while testing (the C is ground truth)

* Row 4 originally assumed `stbds_arrgrowf(NULL, es, 0, 0)` produced a 4-element
  array.  The C returns **`NULL`**, because `min_cap <= stbds_arrcap(a)` is
  checked *before* the `min_cap < 4` clamp and `0 <= 0` holds.  Both
  implementations agree.
* Row 33 originally assumed an out-of-range `mode` merely leaked the key.  It
  also makes `STBDS_ASSERT(slot >= 0)` fire whenever the delete has to
  swap-with-last, so the process dies with `SIGABRT`.  Verified in a subprocess
  for both `.so`s (`mode = 2` and `mode = INT_MAX`).
* `table->temp_key` is **never initialised** by `stbds_make_hash_index`, and
  `stbds_hmput_key` only writes it on the insert path and on the
  "duplicate found in the *upper* half of the bucket" path (`lib.c:733`) — not on
  the wrapped-half path (`lib.c:748`) and not in the `default:` (memcpy) storage
  mode.  It is therefore only an observable value under those conditions; the
  tests zero it in both libraries first (`MapPair::zero_temp_key`) before
  comparing (rows 20/21).
* `sh_geti` prints by walking the *array* in insertion order, not the hash
  table, so its stdout is independent of `stbds_hash_seed` even though the
  bucket layout is not (`sh_geti.rs::c86_sh_geti_seed_dependence`).

## Notes on non-testable / UB rows

* Row 55 (`stbds_arrfreef(NULL)`) is undefined behaviour in the C original
  (`free()` on `NULL - 32`).  Both implementations do the *same* arithmetic and
  the *same* `free`, so they are identical by construction; actually executing it
  would corrupt the heap and is therefore only verified by inspection.
* Row 31's assert is dead code (`size_t >= 0`).  The test verifies the
  *observable* consequence instead: `used_count` wrapping is impossible via the
  public API because a delete requires a found slot, which requires
  `used_count >= 1`.
* `stbds_stralloc` with `remaining > 0` but `storage == NULL` dereferences NULL
  in both implementations — same UB, not exercised.
* Mixing a *string* `table->string.mode` with a *binary* `mode` (or a
  `SH_NONE`/out-of-range `string.mode` with `mode >= 1`) makes look-ups
  reinterpret raw key bytes as a `char *`.  That is UB in the C original, so
  those combinations are exercised **insert-only**
  (`hashmap.rs::c48_c49_string_hash_with_memcpy_storage`,
  `fuzz.rs::c77b_string_memcpy_storage_sequences`).
* `stbds_hmdel_key` with a non-zero `keyoffset` on a *string* map would `strcmp`
  a wild pointer; only the binary-mode variant is exercised
  (`errors.rs::e56_hmdel_bad_keyoffset`).
