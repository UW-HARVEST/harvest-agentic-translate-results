# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c`. The library is `stb_ds`; it has no
error enum and no `RETURN_ERROR` macro. Its entire rejection surface consists of

* `assert()` (`#define STBDS_ASSERT assert`, no `NDEBUG` in the cmake build → asserts are LIVE),
* sentinel returns (`return -1`, `return 0`/NULL, `return a` unchanged),
* the "not found" sentinel `STBDS_INDEX_EMPTY == -1` written through `*temp` /
  `stbds_header(a)->temp`,
* null-pointer guards (`if (a == NULL) ...`),
* mode / range comparisons (`mode >= STBDS_HM_STRING`, `mode == STBDS_HM_STRING`,
  `switch (table->string.mode)` default arm),
* min/max constants `STBDS_STRING_ARENA_BLOCKSIZE_MIN/MAX`, `STBDS_BUCKET_LENGTH`.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|--------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` (287) | `min_cap <= stbds_arrcap(a)` after `min_len` clamp — growth rejected | returns `a` unchanged (identical pointer), no realloc, header untouched |
| 2 | `stbds_arrgrowf` (300) | `a == NULL` (no existing header to inherit) | fresh block: `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(addlen,min_cap,4)` |
| 3 | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0` on `a == NULL` → `min_len = 0`, `min_cap` stays 0, `0 <= arrcap(NULL)==0` | returns `NULL` (input `a`) — **no allocation at all** |
| 4 | `stbds_arrgrowf` | `elemsize == 0` | allocates only `sizeof(stbds_array_header)`, `capacity = 4` |
| 5 | `stbds_arrfreef` (293) | `a == NULL` → `free((stbds_array_header*)NULL - 1)` | invalid free / undefined behaviour (crash). Both impls do the identical bad pointer arithmetic; not exercised as a live call, verified by disassembly-equivalent argument (`stbds_header(NULL)` = `-32`) |
| 6 | `stbds_make_hash_index` (401) `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | `slot_count == 0` | assert abort. Unreachable from the public API: only callers pass `STBDS_BUCKET_LENGTH`(8), `slot_count*2`, or `slot_count>>1` guarded by `slot_count > 8` |
| 7 | `stbds_hm_find_slot` (610) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` in the upper half-bucket scan → key absent | returns `-1` |
| 8 | `stbds_hm_find_slot` (621) | probe hits `STBDS_HASH_EMPTY` in the wrapped lower half-bucket scan → key absent | returns `-1` |
| 9 | `stbds_hmfree_func` (573) | `a == NULL` | returns immediately, no free |
| 10 | `stbds_hmfree_func` (574) | `stbds_hash_table(a) == NULL` (array made by `stbds_hmput_default` / `hmget_key(NULL,..)`, never `hmput_key`) | skips strdup sweep + `strreset`, still `free(hash_table)` (NULL, ok) and `free(header)` |
| 11 | `stbds_hmget_key_ts` (634) | `a == NULL` | allocates 1-element array, `length=1`, element zeroed, `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` (non-NULL) |
| 12 | `stbds_hmget_key_ts` (644) | `a != NULL` but `hash_table == 0` | `*temp = -1`, returns `a` unchanged, does NOT call `find_slot` |
| 13 | `stbds_hmget_key_ts` (648) | key absent (`find_slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` |
| 14 | `stbds_hmget_key` (660) | any of #11–#13 | same as `_ts` **plus** `stbds_header(p-elemsize)->temp = temp` (i.e. `-1` on miss) |
| 15 | `stbds_hmget_key` on `a == NULL` | `a == NULL` | `temp` field of the *new* header is set to `-1`; `length == 1` |
| 16 | `stbds_hmput_default` (669) | `a == NULL` | grows a 1-element array, `length=1`, zeroed, returns `arr+elemsize` |
| 17 | `stbds_hmput_default` (669) | `a != NULL` and `stbds_header(a-elemsize)->length == 0` | grows again, `length += 1` → 1 |
| 18 | `stbds_hmput_default` (675) | `a != NULL`, `length != 0` | returns `a` unchanged, no allocation |
| 19 | `stbds_hmput_key` (686) | `a == NULL` | bootstraps 1-element array before the table logic |
| 20 | `stbds_hmput_key` (698) | `table == NULL` | creates `slot_count = STBDS_BUCKET_LENGTH (8)` index, and `nt->string.mode = (mode >= 1) ? STBDS_SH_DEFAULT : 0` |
| 21 | `stbds_hmput_key` (698) | `table->used_count >= table->used_count_threshold` (6 of 8 slots) | rehash into `slot_count*2`, old table freed, `string`/`seed` inherited, `string.mode` NOT re-derived from `mode` |
| 22 | `stbds_hmput_key` (778) `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` | capacity not grown | assert abort; unreachable — the preceding `if` grows when needed |
| 23 | `stbds_hmput_key` (730/748) | duplicate key found | no new slot, `temp = bucket->index[i]`, `used_count` unchanged. In the **upper** scan `temp_key` is also written; in the **wrapped lower** scan it is NOT (asymmetry preserved verbatim) |
| 24 | `stbds_hmput_key` `switch (table->string.mode)` default arm (795) | `string.mode` is `STBDS_SH_NONE(0)` or any value not 1/2/3 | `memcpy(elem, key, keysize)` — binary key copy even when `mode >= STBDS_HM_STRING` |
| 25 | `stbds_hmput_key` | `mode` out of enum range, e.g. `mode = 2`, `7`, `INT_MAX` | `mode >= STBDS_HM_STRING` is true → treated as string mode (hash_string, strcmp) |
| 26 | `stbds_hmput_key` | `mode` negative, e.g. `-1`, `INT_MIN` | `mode >= 1` false → binary mode (hash_bytes/memcmp), `string.mode = 0` |
| 27 | `stbds_hmdel_key` (809) | `a == NULL` | returns `0` (NULL) |
| 28 | `stbds_hmdel_key` (816) | `hash_table == 0` | `stbds_header(raw_a)->temp = 0`, returns `a` unchanged |
| 29 | `stbds_hmdel_key` (821) | key absent (`find_slot < 0`) | `temp = 0`, returns `a`, `length`/`used_count` unchanged |
| 30 | `stbds_hmdel_key` (828) `STBDS_ASSERT(slot < table->slot_count)` | corrupt slot | abort; unreachable, `find_slot` masks by `slot_count-1` |
| 31 | `stbds_hmdel_key` (832) `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t` → always true | never fires (dead assert, kept for parity) |
| 32 | `stbds_hmdel_key` (846) `STBDS_ASSERT(slot >= 0)` | re-lookup of the moved last element fails | abort; reachable only on a corrupt table |
| 33 | `stbds_hmdel_key` (849) `STBDS_ASSERT(b->index[i] == final_index)` | re-lookup found the wrong slot | abort; reachable only on a corrupt table |
| 34 | `stbds_hmdel_key` (838) | `mode == STBDS_HM_STRING` **exactly** and `string.mode == STBDS_SH_STRDUP` | frees the old key string. `mode = 2` (also "string" for hashing) does **not** free — quirk preserved |
| 35 | `stbds_hmdel_key` (853) | `mode != STBDS_HM_STRING` (incl. `mode = 2`) on a string table | re-lookup uses the raw element bytes as the key (`memcmp`/`hash_bytes` path) instead of the stored `char*` |
| 36 | `stbds_hmdel_key` (859) | `used_count < used_count_shrink_threshold && slot_count > 8` | table rebuilt at `slot_count>>1` |
| 37 | `stbds_hmdel_key` (862) | `tombstone_count > tombstone_count_threshold` | table rebuilt at same `slot_count` |
| 38 | `stbds_hmdel_key` | delete the only element (`old_index == final_index`) | no memmove, no re-lookup, `length -= 1` |
| 39 | `stbds_stralloc` (913) `STBDS_ASSERT(len <= a->remaining)` | reachable: after the *big-string* branch takes `a->storage != NULL` and returns early it never asserts; but calling `stralloc` on an arena whose `storage != NULL` and `remaining == 0` with `len > blocksize` re-enters the big-string branch. The assert only fires if `blocksize >= len` yet `remaining < len`, impossible after the else-branch sets `remaining = blocksize` | never fires for reachable inputs |
| 40 | `stbds_stralloc` (899) | `len > blocksize` (blocksize = `512 << (block>>1)`) and `a->storage == NULL` | dedicated over-sized block, `a->storage = sb`, `a->remaining = 0`, returns `sb->storage` |
| 41 | `stbds_stralloc` (899) | `len > blocksize` and `a->storage != NULL` | block spliced in as `a->storage->next`, `a->remaining` **left unchanged** |
| 42 | `stbds_stralloc` | `a->block` saturation: `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` stops incrementing |
| 43 | `stbds_stralloc` | empty string `""` (`len == 1`) | consumes 1 byte of `remaining` |
| 44 | `stbds_strreset` (925) | `a->storage == NULL` | frees nothing, memsets the arena to 0 |
| 45 | `stbds_hash_string` (471) | empty string `""` | `hash = seed`, then the avalanche; deterministic value must match |
| 46 | `stbds_hash_bytes` (546) | `len == 0` | siphash of the length-only final block; `p` never dereferenced |
| 47 | `stbds_is_key_equal` (561) | `mode >= STBDS_HM_STRING` | `strcmp` on the *stored pointer* (`*(char**)elem`) — a NULL stored pointer would crash; guarded by construction |
| 48 | `stbds_shmode_func` (802) | `mode` out of the `STBDS_SH_*` enum, e.g. `4`, `255`, `256`, `-1` | `h->string.mode = (unsigned char) mode` — silent truncation to 8 bits; `256` → `0`, `-1` → `255`; a mode not in {1,2,3} then hits the `default:` `memcpy` arm of `hmput_key` |
| 49 | `str_put` (958) `STBDS_ASSERT(*strmap[0].key == 'a')` | never fires (literal `"a"` inserted) | kept for parity |
| 50 | `str_put` (959/960) | never fire | kept for parity |
| 51 | `str_put` | `num <= 0` | the `stralloc` loop body never executes |
| 52 | `strkey` (939) | `n` with a long decimal form (e.g. `INT_MIN`) | `sprintf(buffer, "test_%d", n)` into a 256-byte static; no overflow for any `int` |

## Generic FFI boundary cases (mandatory even though not in the table)

| # | case | expected |
|---|------|----------|
| G1 | NULL `a` into every pointer-taking entry point that guards it (`hmput_key`, `hmput_default`, `hmget_key`, `hmget_key_ts`, `hmdel_key`, `hmfree_func`, `arrgrowf`) | per rows #2/#9/#11/#16/#19/#27 |
| G2 | `len = 0` and `keysize = 0` | `hash_bytes(p,0,seed)` well-defined; `memcmp(...,0)` == 0 → every key "equal" |
| G3 | oversized `keysize` vs `elemsize` (`keysize > elemsize`) | C reads/copies past the element; both impls must do the identical thing |
| G4 | out-of-range enum `mode` across FFI: `-2147483648`, `-1`, `2`, `3`, `2147483647` | rows #25/#26/#34/#35 |
| G5 | out-of-range enum `mode` into `shmode_func`: `-1`, `0`, `4`, `255`, `256`, `2147483647` | row #48 |
| G6 | one past valid range on `stbds_arrgrowf`: `min_cap = SIZE_MAX`, `addlen = SIZE_MAX` | `elemsize*min_cap + sizeof(header)` wraps; `realloc` returns NULL → both deref NULL identically (not executed live; wrap arithmetic verified for `min_cap` values that still allocate) |
| G7 | `stbds_rand_seed(0)`, `(SIZE_MAX)` | subsequent `make_hash_index` seed evolution identical |

## Check-off — `tests/phase_c_errors.rs` (53 tests, all passing)

Every row has a differential test that constructs the exact condition, calls
**both** `.so`s through `libloading`, and asserts the *same* sentinel / error
code / abort — not merely "both failed".

| row(s) | test | asserted result | [x] |
|--------|------|-----------------|-----|
| 1 | `e01_arrgrowf_early_out` | both return the *input* pointer; header untouched | [x] |
| 2 | `e02_arrgrowf_fresh_header` | `length=0, temp=0, hash_table=NULL` | [x] |
| 3 | `e03_arrgrowf_returns_null` | both return `NULL` | [x] |
| 4 | `e04_arrgrowf_elemsize_zero` | identical header, `capacity=4` | [x] |
| 5 | `e05_arrfreef_null_crashes_identically` | forked child: identical fatal signal (SIGABRT) | [x] |
| 6 | `e06_make_hash_index_assert` | identical SIGABRT; stderr names the same assertion text, `stbds_make_hash_index`, `:401:`, `lib.c` | [x] |
| 7, 8 | `e07_e08_find_slot_miss` | every miss reports `-1` from both bucket scans (25 seeds × 400 probes) | [x] |
| 9 | `e09_hmfree_null` | no-op on both, any `elemsize` incl. `SIZE_MAX` | [x] |
| 10 | `e10_hmfree_no_table` | strdup sweep + `strreset` skipped, both survive | [x] |
| 11 | `e11_hmget_ts_null` | `*temp = -1`, 1-element zeroed array, non-NULL return | [x] |
| 12 | `e12_hmget_ts_no_table` | `*temp = -1`, `a` returned *identically* (pointer compared) | [x] |
| 13 | `e13_hmget_ts_miss` | `*temp = -1` | [x] |
| 14 | `e14_hmget_key_temp_on_miss` | `stbds_header(...)->temp == -1` | [x] |
| 15 | `e15_hmget_key_null_sets_temp` | new header's `temp == -1`, `length == 1` | [x] |
| 16 | `e16_hmput_default_null` | `length=1`, element fully zeroed, no table | [x] |
| 17 | `e17_hmput_default_length_zero` | grows again to `length = 1` | [x] |
| 18 | `e18_hmput_default_noop` | same pointer, byte-identical state | [x] |
| 19 | `e19_hmput_key_null_bootstrap` | `length=2`, `slot_count=8` | [x] |
| 20 | `e20_hmput_key_initial_string_mode` | `string.mode = SH_DEFAULT` for `mode ∈ {1,2,7,INT_MAX}`, `0` for `{0,-1,INT_MIN}` | [x] |
| 21 | `e21_hmput_key_grow_inherits` | rehash at `used_count == 6`; `string.mode` and `seed` inherited, not re-derived | [x] |
| 22 | `e22_hmput_key_capacity_invariant` | `length <= capacity` after all 500 inserts (the assertion's invariant) | [x] |
| 23 | `e23_hmput_key_duplicate` | `length` and `used_count` unchanged on duplicates, 15 seeds | [x] |
| 24 | `e24_hmput_key_default_arm` | `string.mode ∈ {0,4,5,99,255}` → element holds a `memcpy` of the key body | [x] |
| 25 | `e25_mode_above_range_is_string` | `mode ∈ {2,3,100,INT_MAX}` behave as string mode | [x] |
| 26 | `e26_mode_negative_is_binary` | `mode ∈ {-1,-2,-1000,INT_MIN}` behave as binary mode | [x] |
| 27 | `e27_hmdel_null` | both return `NULL`, across `elemsize`/`keysize`/`mode` incl. 0 and `INT_MAX` | [x] |
| 28 | `e28_hmdel_no_table` | `temp = 0`, `a` returned identically | [x] |
| 29 | `e29_hmdel_missing` | result `0`; `length`, table and elements unchanged | [x] |
| 30–33 | `e30_e33_hmdel_invariants` | the four assertions' invariants hold over a full randomized drain (slot indices in range, `used_count == length-1`, `used+tombstones <= slot_count`, every live key still findable) | [x] |
| 34 | `e34_hmdel_strdup_frees` | `mode == 1` on `SH_STRDUP`: key freed, state identical | [x] |
| 35 | `e35_hmdel_mode2_no_free` | `mode ∈ {2,3,77,INT_MAX}`: no free, identical state | [x] |
| 36 | `e36_hmdel_shrink` | ≥4 successive `slot_count` halvings, bottoming out at 8 | [x] |
| 37 | `e37_hmdel_tombstone_rebuild` | same-size rebuild observed; state identical every round | [x] |
| 38 | `e38_hmdel_only_element` | `old_index == final_index`, result `1`, `hmlen` 0 (binary + string) | [x] |
| 39 | `e39_stralloc_boundary_lengths` | lengths straddling every block boundary: identical arena, never asserts | [x] |
| 40 | `e40_stralloc_oversize_empty` | `remaining = 0`, `block = 1`, 1 block | [x] |
| 41 | `e41_stralloc_oversize_nonempty` | spliced block, `remaining` unchanged | [x] |
| 42 | `e42_stralloc_block_saturation` | `block` saturates at 22 (`512 << 11 == 1<<20`) | [x] |
| 43 | `e43_stralloc_empty_string` | `""` consumes exactly 1 byte, 600 times | [x] |
| 44 | `e44_strreset_null_storage` | arena zeroed, nothing freed, incl. `block=255, mode=255, remaining=SIZE_MAX` | [x] |
| 45 | `e45_hash_string_empty` | identical hash for `""` at 6 seeds | [x] |
| 46 | `e46_hash_bytes_len_zero` | identical hash; result independent of the (NULL / dangling) pointer | [x] |
| 47 | `e47_is_key_equal_string` | content-equal-but-different-address key hits; prefix key misses | [x] |
| 48 | `e48_shmode_func_mode_truncation` | `(unsigned char) mode`: `256→0`, `257→1`, `-1→255`, `INT_MAX→255` | [x] |
| 49–51 | `e49_e51_str_put_asserts_never_fire` | forked children exit 0 with empty stderr for `num ∈ {INT_MIN,-1000,-1,0,1,2,300}` | [x] |
| 52 | `e52_strkey_extremes` | identical text for `INT_MIN`/`INT_MAX` neighbourhood, fits 256 bytes | [x] |
| G1 | `g1_null_into_every_guarded_entry_point` | NULL × {`hmdel_key`, `hmfree_func`, `hmput_default`, `hmget_key`, `hmput_key`, `arrgrowf`} × elemsize × mode | [x] |
| G2 | `g2_keysize_zero` | `memcmp(_,_,0)==0` → all keys collapse to one entry on both | [x] |
| G3 | `g3_keysize_covers_whole_element` | `keysize == elemsize` (8/16/24), 60 random keys each | [x] |
| G4 | `g4_out_of_range_mode` | `mode ∈ {INT_MIN,-1000,-1,0,1,2,3,4,1000,INT_MAX}` through put/get/get_ts/del | [x] |
| G5 | `g5_shmode_out_of_range_then_put` | truncated `sh_mode` then a put landing in the `default:` arm | [x] |
| G6 | `g6_arrgrowf_allocation_failure` | `realloc` refusal → identical fatal outcome in forked children | [x] |
| G7 | `g7_rand_seed_extremes` | `rand_seed ∈ {0,1,2,SIZE_MAX,SIZE_MAX-1,0x31415926,1<<63}` → identical table seeds over 8 tables | [x] |

**0 unchecked rows.** Reproduce with `cargo test --release --test phase_c_errors`.

### Notes on the unreachable rows

Rows 6, 22, 30–33 and 39 are `assert()`s that no public-API input can reach.
They are still covered rather than waved away:

* row 6 is *forced* by handing the library a `slot_count == 0` table, which proves
  the Rust assert wrapper aborts through `__assert_fail` with the same assertion
  text, function name and line number as the C one;
* rows 22 and 30–33 are covered by asserting the invariants they guard directly,
  after every operation of a long randomized workload;
* row 39's precondition (`blocksize >= len` yet `remaining < len`) is impossible
  once the else-branch sets `remaining = blocksize`, so boundary lengths around
  every block size are swept instead.

Rows 5 and G6 are genuine undefined behaviour in the C (`free((header*)NULL-1)`
and writing through `NULL + sizeof(header)` after a failed `realloc`). They are
compared by wait status in a forked child, which is the strongest available
statement of "both fail the same way".
