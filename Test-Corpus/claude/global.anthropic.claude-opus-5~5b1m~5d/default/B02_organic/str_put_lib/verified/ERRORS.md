# ERRORS.md — Phase C error-surface table

Every distinct rejection / error / sentinel / assert / range-check / boundary
constant found by grepping `c_src/src/lib.c`. Rows are derived from what the C
**actually checks**, one row per distinct branch.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| E1  | `stbds_arrgrowf` (L286) | `min_cap <= stbds_arrcap(a)` after `min_cap = max(min_cap, arrlen+addlen)` — i.e. growth request already satisfied | returns the **same pointer `a` unchanged**, no realloc, header untouched | `err_e1_arrgrowf_no_growth_needed` | [x] |
| E2  | `stbds_arrgrowf` (L283/L291) | `addlen == 0 && min_cap == 0` on a NULL array (degenerate zero request) | `min_len=0`, `min_cap=0`; `0 <= arrcap(NULL)=0` ⇒ returns `NULL` (no allocation) | `err_e2_arrgrowf_zero_request_null` | [x] |
| E3  | `stbds_arrgrowf` (L291) | `min_cap` in `1..3` on a fresh (NULL) array | `min_cap` clamped **up to 4** ⇒ `capacity == 4`, `length == 0` | `err_e3_arrgrowf_min_cap_clamped_to_4` | [x] |
| E4  | `stbds_arrgrowf` (L297) | `elemsize * min_cap` overflows `size_t` (e.g. `elemsize = SIZE_MAX/2`, `min_cap = 4`) | the product is computed in wrapping `size_t`; when the result is huge `realloc` fails and the C ignores the NULL, writing the header at `(char*)NULL+32` ⇒ **SIGSEGV**. When the product wraps to a *small* value (`SIZE_MAX/4 * 8 + 32 == 24`) `realloc` succeeds and the call returns normally. Both outcomes must match | `err_e4_arrgrowf_size_overflow` (forked children; signal **and** stderr compared) | [x] |
| E5  | `stbds_hmfree_func` (L573) | `a == NULL` | returns immediately, no free, no crash | `err_e5_hmfree_null` | [x] |
| E6  | `stbds_hmfree_func` (L574) | `stbds_hash_table(a) == NULL` (array built by `stbds_arrgrowf` only, never `hmput`) | skips the strdup/arena teardown, still `free`s `hash_table` (NULL) and the header | `err_e6_hmfree_no_hash_table` | [x] |
| E7  | `stbds_hm_find_slot` (L610, L621) | probed bucket slot has `hash == STBDS_HASH_EMPTY` (0) ⇒ key absent | returns `-1` | covered by E8/E9/E10/E11 | [x] |
| E8  | `stbds_hmget_key_ts` (L634) | `a == NULL` | **allocates** a default element (`cap==4`, `length==1`, elem zeroed), writes `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` (non-NULL) | `err_e8_hmget_key_ts_null_map` | [x] |
| E9  | `stbds_hmget_key_ts` (L644) | `a != NULL` but `header->hash_table == 0` (e.g. right after `stbds_hmput_default`) | `*temp = -1`, returns `a` unchanged | `err_e9_hmget_key_ts_no_table` | [x] |
| E10 | `stbds_hmget_key_ts` (L648) | key not present in a populated table (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `err_e10_hmget_missing_key` | [x] |
| E11 | `stbds_hmget_key` (L659) | same three sub-cases, via the non-`_ts` wrapper | header `temp` field set to `-1`; return value as in E8/E9/E10 | `err_e11_hmget_key_wrapper_missing` | [x] |
| E12 | `stbds_hmdel_key` (L809) | `a == NULL` | returns `0` (**NULL**) — the only entry point that returns NULL | `err_e12_hmdel_null_map` | [x] |
| E13 | `stbds_hmdel_key` (L816) | `a != NULL`, `hash_table == 0` | sets header `temp = 0`, returns `a` unchanged | `err_e13_hmdel_no_table` | [x] |
| E14 | `stbds_hmdel_key` (L821) | key absent (`slot < 0`) | header `temp = 0`, returns `a`, `length`/`used_count`/`tombstone_count` all unchanged | `err_e14_hmdel_missing_key` | [x] |
| E15 | `stbds_hmdel_key` (L831) | key present | header `temp = 1` (the "deleted" flag), `used_count--`, `tombstone_count++`, `length--` | `err_e15_hmdel_present_sets_temp1` | [x] |
| E16 | `stbds_hmput_default` (L669) | `a == NULL` | allocates default element, `length = 1`, returns `arr+elemsize` | `err_e16_hmput_default_null` | [x] |
| E17 | `stbds_hmput_default` (L669) | `a != NULL` **and** `header(arr)->length == 0` (array grown but empty) | re-grows / bumps `length` to 1 and zeroes elem 0 | `err_e17_hmput_default_zero_len` | [x] |
| E18 | `stbds_hmput_default` (L675) | `a != NULL` and `length != 0` | returns `a` **unchanged** (no allocation, no zeroing) | `err_e18_hmput_default_noop` | [x] |
| E19 | `stbds_hash_string` (L480) | `str` points at `'\0'` (empty string) | loop body never runs; still returns the full avalanche of `seed` | `err_e19_hash_string_empty` | [x] |
| E20 | `stbds_hash_bytes` (L522/L532) | `len == 0` | main loop skipped, `switch(0)` hits `case 0: break` ⇒ hash of `data = 0` only. **`p` is never dereferenced**, so even `p == NULL` is accepted | `err_e20_hash_bytes_zero_len` / `err_e20b_hash_bytes_null_zero_len` | [x] |
| E21 | `stbds_hash_bytes` (L596) / `stbds_hmput_key` (L719) | computed hash `< 2` (i.e. collides with `STBDS_HASH_EMPTY=0` or `STBDS_HASH_DELETED=1`) | `hash += 2` before probing, so a bucket's `hash` field is never `0`/`1` | `err_e21_hash_lt_2_bump` — a 64-bit hash of `0` or `1` needs ~2^63 trials and is **not constructible**, so the test instead pins down the mapping the guard implements: for every sampled key the value stored in the bucket equals `max(hash, hash<2 ? hash+2 : hash)` as returned by the *exported* hash function, in both libraries, and no stored bucket hash is ever `0`/`1` | [x] |
| E22 | `stbds_stralloc` (L885) | `len > a->remaining` and `len > blocksize` (huge string, e.g. 4 KiB with `block==0`) | takes the "oversized block" path: allocates exactly `len+8`, **splices behind** `a->storage` (or sets `remaining = 0` when `storage == NULL`) and returns `sb->storage`; `a->remaining` is *not* decremented | `err_e22_stralloc_oversized` | [x] |
| E23 | `stbds_stralloc` (L890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (`a->block >= 22`) | `a->block` is **not** incremented (saturates at 22) | `err_e23_stralloc_block_saturates` | [x] |
| E24 | `stbds_stralloc` (L888) | caller-supplied `a->block` in `23..=255` ⇒ `512 << (block>>1)` shifts by `>= 64` (**C UT / x86 masks the count**) | shift count masked to 6 bits ⇒ `blocksize` is `0` or a wrapped power of two; behaviour must match the compiled C bit-for-bit | `err_e24_stralloc_shift_overflow` | [x] |
| E25 | `stbds_stralloc` (L913) | `STBDS_ASSERT(len <= a->remaining)` | **unreachable**: either `len <= remaining` on entry, or a block of `blocksize >= len` was just installed, or the oversized path returned early. Proven by exhaustive `block`×`len` sweep in the test | `err_e25_stralloc_assert_unreachable` | [x] |
| E26 | `stbds_strreset` (L920) | all-zero / already-reset arena (`storage == NULL`) | while-loop body never runs; arena memset to 0; idempotent, no crash | `err_e26_strreset_empty` | [x] |
| E27 | `stbds_shmode_func` (L803) | `mode` outside the `STBDS_SH_*` enum (`-1`, `4`, `255`, `256`, `INT_MIN`, `INT_MAX`) | `(unsigned char) mode` **truncates** ⇒ `string.mode = mode & 0xff`; a later `hmput_key` `switch` falls to `default:` (raw `memcpy` of the key) for any truncated value not in `{2,3,1}` | `err_e27_shmode_out_of_range_enum` | [x] |
| E28 | `stbds_hmput_key` (L707/L713/L732) | `mode` outside `{0,1}`: the code tests `mode >= STBDS_HM_STRING` (**not** `== 1`) | any `mode >= 1` (`2`, `7`, `INT_MAX`) behaves as STRING; any `mode <= 0` (`-1`, `INT_MIN`) behaves as BINARY | `err_e28_hmput_out_of_range_mode` | [x] |
| E29 | `stbds_hmdel_key` (L836/L842) | `mode` outside `{0,1}`: these two sites test `mode == STBDS_HM_STRING` **exactly** | `mode == 2` hashes/compares as a *string* (`mode>=1` in `find_slot`) but takes the *binary* re-index branch — the key bytes (a `char*`) are re-hashed with `stbds_hash_string` over the pointer's own bytes. Must match exactly | `err_e29_hmdel_mode_2_asymmetry` | [x] |
| E30 | `stbds_is_key_equal` (L560) | `mode >= STBDS_HM_STRING` while `table->string.mode` is `STBDS_SH_NONE`: element offset 0 holds raw key **bytes**, so `strcmp(key, *(char**)elem)` follows a wild pointer | dies (SIGSEGV) or, if the bytes happen to be readable, compares garbage — whatever it does, both libraries must do it identically | `err_e30_is_key_equal_wild_pointer` (forked children, signal+stderr compared) | [x] |
| E31 | `stbds_hmput_key` (L698) | `used_count >= used_count_threshold` (6 for an 8-slot table) — i.e. the 7th distinct insert | table is rebuilt at `slot_count*2`; `seed` is **inherited** from the old table (global `stbds_hash_seed` *not* advanced) | `err_e31_grow_at_threshold` | [x] |
| E32 | `stbds_hmdel_key` (L854) | `used_count < used_count_shrink_threshold && slot_count > 8` | table shrinks to `slot_count>>1` | `err_e32_shrink_threshold` | [x] |
| E33 | `stbds_hmdel_key` (L858) | `tombstone_count > tombstone_count_threshold` (`(sc>>3)+(sc>>4)`) | table rebuilt at the same `slot_count`, tombstones cleared | `err_e33_tombstone_rebuild` | [x] |
| E34 | `stbds_make_hash_index` (L399) | `slot_count <= STBDS_BUCKET_LENGTH` (8) | `used_count_shrink_threshold` forced to **0** (so an 8-slot table never shrinks) | `err_e34_no_shrink_at_8_slots` | [x] |
| E35 | `stbds_make_hash_index` (L401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | **unreachable**: only `slot_count ∈ {8,16,32,...}` is ever produced; `sc-(sc>>2) + (sc>>3)+(sc>>4) < sc` holds for all of them (verified numerically in the test) | `err_e35_make_hash_index_assert_unreachable` | [x] |
| E36 | `stbds_hmput_key` (L778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | **unreachable**: preceded by `if (i+1 > arrcap) arrgrowf(a,elemsize,1,0)` which guarantees `arrcap >= i+1` | `err_e36_hmput_capacity_assert` | [x] |
| E37 | `stbds_hmdel_key` (L828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | **unreachable**: `find_slot` masks `pos &= slot_count-1` | `err_e37_hmdel_slot_range_assert` | [x] |
| E38 | `stbds_hmdel_key` (L832) | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t` ⇒ the condition is a tautology, **never** fires (even after the `--` underflows) | `err_e38_hmdel_used_count_tautology` | [x] |
| E39 | `stbds_hmdel_key` (L846/L849) | `STBDS_ASSERT(slot >= 0)` / `STBDS_ASSERT(b->index[i] == final_index)` after relocating the last element | reachable **only** with inconsistent `mode`/`keyoffset` between put and delete (see E29). With consistent usage the moved element is always found | `err_e39_hmdel_relocate_asserts` | [x] |
| E40 | `str_put` (L958-960) | the three `STBDS_ASSERT`s on `strmap[0]` | always hold (`strmap` is `arr+elemsize`, and `temp==0` after the first `shputs`) — verified for many `num` | `err_e40_str_put_asserts_hold` | [x] |
| E41 | `str_put` (L951) | `num <= 0` (`0`, `-1`, `INT_MIN`) | the `stralloc` loop is skipped entirely; still prints exactly one line `"a <num>\n"` | `err_e41_str_put_non_positive` | [x] |
| E42 | `strkey` (L939) | `n` negative / `INT_MIN` / `INT_MAX` | `sprintf(buffer, "test_%d", n)` — at most 16 bytes into a 256-byte buffer, **never overflows**; returns the shared static buffer | `err_e42_strkey_extremes` | [x] |
| E43 | `stbds_hmget_key` (L663) | `stbds_temp(STBDS_HASH_TO_ARR(p,elemsize)) = temp` on the freshly-allocated map from E8 | writes `-1` into the brand-new header's `temp` | `err_e43_hmget_key_null_writes_temp` | [x] |
| E44 | `stbds_arrfreef` (L312) | `a == NULL` | computes `stbds_header(NULL)` = `(void*)-32` and calls `free` on it ⇒ **glibc abort / SIGSEGV** (identical in both) | `err_e44_arrfreef_null` (subprocess, signal compared) | [x] |
| E45 | `stbds_hmput_key` (L785) | `table->string.mode` not in `{1,2,3}` (e.g. `0`, `4`, `255` via `stbds_shmode_func`) | `switch` `default:` ⇒ `memcpy(elem, key, keysize)` (raw key bytes, no pointer store, `temp_key` **not** written) | `err_e45_string_mode_default_branch` | [x] |
| E46 | `stbds_hmput_key` (L746-758) | duplicate key found in the **wrap-around** inner loop (`i < limit`) | `stbds_temp` is set but `stbds_temp_key` is **NOT** updated (asymmetry vs. the first inner loop) — a genuine upstream quirk that must be reproduced | `err_e46_wraparound_no_temp_key` | [x] |
| E47 | `stbds_hmget_key_ts` / `stbds_hmput_key` | `keysize == 0` with `mode == STBDS_HM_BINARY` | `memcmp(...,0)` returns 0 ⇒ **every** key "matches"; `hash_bytes(key,0,seed)` is key-independent; `memcpy(...,0)` writes nothing | `err_e47_zero_keysize_binary` | [x] |
| E48 | `stbds_hmput_key` | `elemsize == 0` | `arrgrowf(0,0,0,1)` allocates only the header (`cap=4`); every element aliases offset 0; `STBDS_ARR_TO_HASH(a,0) == a` so the "hash" and "arr" pointers coincide | `err_e48_zero_elemsize` | [x] |
| E49 | `stbds_hmdel_key` (L843) | `keyoffset != 0` (the real `STBDS_OFFSETOF(t,key)` for a struct whose key is not first) | key is read at `elem + keyoffset`; `find_slot`/`is_key_equal` must use the same offset. `hmput_key` **hardcodes `keyoffset = 0`** — this asymmetry must be reproduced | `err_e49_nonzero_keyoffset` | [x] |
| E50 | `stbds_stralloc` (L896) | `a->storage != NULL` on the oversized path | new block is spliced in as `a->storage->next` (second position), **not** at the head, and `a->remaining` is left alone | `err_e50_stralloc_oversized_splice` | [x] |

## Generic FFI boundary cases (covered even though not a distinct C branch)

| # | case | covered by |
|---|------|------------|
| G1 | NULL map pointer into every `void*`-taking entry point | E5, E8, E12, E16, E44 |
| G2 | zero lengths (`keysize=0`, `elemsize=0`, `len=0`) | E20, E47, E48 |
| G3 | oversized lengths (`elemsize` near `SIZE_MAX`) | E4 |
| G4 | one step past a valid range (`mode = 2`, `mode = -1`, `string.mode = 4`) | E27, E28, E29, E45 |
| G5 | out-of-range **enum** ints across FFI (`STBDS_SH_*` / `STBDS_HM_*`) | E27, E28, E29, E45 |
| G6 | `INT_MIN` / `INT_MAX` for the `int` parameters (`mode`, `num`, `n`) | E27, E28, E41, E42 |
| G7 | NULL `char*` to the string entry points | E30 (documented UB, identical) |
