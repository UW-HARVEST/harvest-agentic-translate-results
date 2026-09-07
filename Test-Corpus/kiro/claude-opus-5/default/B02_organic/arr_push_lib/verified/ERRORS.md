# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `STBDS_ASSERT`,
every early `return`, every `== NULL` / `== 0` guard, every sentinel constant
(`STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`, `STBDS_HASH_EMPTY = 0`,
`STBDS_HASH_DELETED = 1`) and every min/max constant
(`STBDS_STRING_ARENA_BLOCKSIZE_MIN/MAX`, `STBDS_BUCKET_LENGTH`).

Note: this library has **no error-code return convention**. Its rejections take
three forms: (a) a NULL / unchanged-pointer early return, (b) an out-of-band
sentinel written into `header->temp` or returned from an internal probe, and
(c) `assert()` → `SIGABRT`. All three are covered below.

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap` (nothing to do) | returns `a` **unchanged** (same pointer, capacity untouched) | `err_01_arrgrowf_noop` | [x] |
| 2 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` (degenerate empty request) | `min_cap (0) <= stbds_arrcap(NULL) (0)` is true, so it **returns `a` — i.e. NULL — without allocating**. Any other `(addlen, min_cap)` allocates with `min_cap` forced up to at least 4. | `err_02_arrgrowf_zero_zero` | [x] |
| 3 | `stbds_arrgrowf` | `elemsize == 0` | for `(0,0)` returns NULL as in row 2; otherwise allocates only the 32-byte header with `capacity = max(min_len, min_cap, 4)` | `err_03_arrgrowf_elemsize_zero` | [x] |
| 4 | `stbds_arrgrowf` | `addlen` so large that `arrlen+addlen` overflows / `elemsize*min_cap` overflows `size_t` | C wraps (unsigned) and passes the wrapped size to `realloc`, which returns NULL; `b = NULL + 32` then dereferenced → SIGSEGV | `err_04_arrgrowf_overflow` (forked child, both must die identically) | [x] |
| 5 | `stbds_hmfree_func` | `a == NULL` | `return;` — no free, no crash | `err_05_hmfree_null` | [x] |
| 6 | `stbds_hm_find_slot` (via `stbds_hmget_key_ts`) | key absent, probe lands on `bucket->hash[i] == STBDS_HASH_EMPTY (0)` | internal `-1`, surfaced as `*temp = STBDS_INDEX_EMPTY (-1)` | `err_06_get_missing_key` | [x] |
| 7 | `stbds_hmget_key_ts` | `a == NULL` | creates a fresh 1-element array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash pointer | `err_07_get_ts_null_a` | [x] |
| 8 | `stbds_hmget_key_ts` | `a != NULL` but `header->hash_table == 0` (array made by `stbds_hmput_default`/`arrgrowf`, never `hmput_key`) | `*temp = -1`, returns `a` unchanged | `err_08_get_ts_no_table` | [x] |
| 9 | `stbds_hmget_key` | same as #6/#7/#8 | additionally stores the sentinel into `stbds_header(raw)->temp` (= -1) | `err_09_get_key_sentinel` | [x] |
| 10 | `stbds_hmput_default` | `a == NULL` | grows a 1-element array, zeroes it, `length = 1` | `err_10_put_default_null` | [x] |
| 11 | `stbds_hmput_default` | `a != NULL` and `header(raw)->length == 0` (hand-forged zero-length array) | takes the grow branch anyway, `length` becomes 1 | `err_11_put_default_len0` | [x] |
| 12 | `stbds_hmput_key` | `a == NULL` | bootstraps a 1-element array before inserting | `err_12_put_null_a` | [x] |
| 13 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` (= `slot_count - slot_count/4`; 6 for 8 slots). The test runs on *entry*, so the 6th insert fills the 8-slot table to the threshold and the **7th** insert is the one that rehashes. | rehashes into a table of `slot_count*2` | `err_13_put_grow_threshold` | [x] |
| 14 | `stbds_hmput_key` | duplicate key (already present) | does **not** append; sets `header->temp` to the existing index and returns; `length` unchanged | `err_14_put_duplicate` | [x] |
| 15 | `stbds_hmput_key` | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | unreachable without corrupting the header (grow just above guarantees it); asserted equal in both libs by checking `capacity >= length` invariant after every insert | `err_15_put_capacity_invariant` | [x] |
| 16 | `stbds_hmput_key` | `keysize == 0`, binary mode | `memcmp(...,0)==0` → the *first* probed entry always compares equal, so the 2nd and later puts all collide with entry 1 | `err_16_put_keysize_zero` | [x] |
| 17 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` | `mode` out of enum range, negative (e.g. `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING` is **false** → binary/`memcmp` path; `nt->string.mode = 0` | `err_17_mode_negative` | [x] |
| 18 | `stbds_hmput_key` / `stbds_hmget_key` | `mode` out of enum range, `> 1` (e.g. `2`, `7`, `INT_MAX`) | `mode >= STBDS_HM_STRING` is **true** → string/`strcmp` path, `string.mode = STBDS_SH_DEFAULT` | `err_18_mode_gt_one` | [x] |
| 19 | `stbds_hmdel_key` | `mode == 2` (string-ish but not exactly `STBDS_HM_STRING`) | find_slot uses the *string* path (`>=`), but the strdup-free and the re-find use `mode == STBDS_HM_STRING` (`==`) → re-find takes the **binary** branch and compares raw pointer bytes. Replicated verbatim. | `err_19_del_mode_two` | [x] |
| 20 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` (`0`), writes nothing | `err_20_del_null_a` | [x] |
| 21 | `stbds_hmdel_key` | `a != NULL`, `header->hash_table == 0` | sets `header(raw)->temp = 0`, returns `a` unchanged | `err_21_del_no_table` | [x] |
| 22 | `stbds_hmdel_key` | key absent (`stbds_hm_find_slot < 0`) | `header(raw)->temp == 0`, returns `a`, `length` unchanged, `used_count`/`tombstone_count` unchanged | `err_22_del_missing_key` | [x] |
| 23 | `stbds_hmdel_key` | key present | `header(raw)->temp == 1`, slot becomes `hash=STBDS_HASH_DELETED(1)` / `index=STBDS_INDEX_DELETED(-2)`, `length -= 1` | `err_23_del_present_sentinels` | [x] |
| 24 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | unreachable via API (find_slot masks `pos` with `slot_count-1`); verified as an invariant on every delete of a 3000-op storm | `err_24_25_26_del_invariants` | [x] |
| 25 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` on the swapped-in element's re-find | reachable only if the moved element is not in the table (see row 19, which does reach it); with `mode == 1` it is verified as an invariant across a randomized delete storm, and both libraries must survive it | `err_24_25_26_del_invariants` | [x] |
| 26 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` | same invariant; the storm additionally asserts `used_count == #in-use slots`, `length == used_count+1` and `0 <= index < length` for every slot after every op | `err_24_25_26_del_invariants` | [x] |
| 27 | `stbds_hmdel_key` | after delete, `used_count < used_count_shrink_threshold && slot_count > 8` | rebuilds the table at `slot_count >> 1` | `err_27_del_shrink` | [x] |
| 28 | `stbds_hmdel_key` | after delete, `tombstone_count > tombstone_count_threshold` (`slot_count/8 + slot_count/16`) | rebuilds at the same `slot_count`, tombstones reset to 0 | `err_28_del_tombstone_rebuild` | [x] |
| 29 | `stbds_hmdel_key` | delete the *last* element (`old_index == final_index`) | skips the memmove + re-find entirely | `err_29_del_last_element` | [x] |
| 30 | `stbds_hmdel_key` | `keyoffset` non-zero but past the element (`keyoffset >= elemsize`) | reads out of the element, still deterministic; C and Rust must agree byte-for-byte | `err_30_del_keyoffset_oob` | [x] |
| 31 | `stbds_hm_find_slot` | probe encounters `hash == STBDS_HASH_DELETED (1)` (tombstone) | does **not** stop — keeps probing past it | `err_31_find_probes_past_tombstone` | [x] |
| 32 | `stbds_hmput_key` / `stbds_hm_find_slot` | a key whose raw hash is `0` or `1` | `if (hash < 2) hash += 2;` — this exists so that a live slot can never be confused with `HASH_EMPTY (0)` or `HASH_DELETED (1)`. A key that actually hashes below 2 cannot be found by search (the space is 2^64), so the rule is verified as the invariant it maintains: over 600 randomized inserts × both key kinds, every slot with `index >= 0` must hold `hash >= 2` in both libraries. | `err_32_hash_lt_two` | [x] |
| 33 | `stbds_hash_bytes` | `len == 0` | tail `switch(0)` → `data = 0 << 56`; returns the fixed "empty" digest for that seed | `err_33_hash_bytes_len0` | [x] |
| 34 | `stbds_hash_bytes` | `len == 0` **and** `p == NULL` | never dereferences `p` (loop and switch both skip) → must return the same value as #33, not crash | `err_34_hash_bytes_null_len0` | [x] |
| 35 | `stbds_hash_bytes` | `len` with high bit set in the last byte of a full 8-byte word (`d[3] >= 0x80` / `d[7] >= 0x80`) | `d[3] << 24` is a *signed int* expression → sign-extends into the top 32 bits of `data` | `err_35_hash_sign_extension` | [x] |
| 36 | `stbds_hash_bytes` | tail lengths 1..7 (all seven `switch` fall-through arms) | each arm ORs a differently-shifted byte; arm 4 also sign-extends | `err_36_hash_tail_all_arms` | [x] |
| 37 | `stbds_hash_bytes` | `len == SIZE_MAX`, `SIZE_MAX-1`, `1<<40` (oversized length) | the `i + 8 <= len` loop walks off the end of the buffer until it hits an unmapped page → fatal signal; both libraries must die identically | `err_37_hash_bytes_oversized_len` (forked child) | [x] |
| 38 | `stbds_hash_string` | empty string `""` | loop body never runs; digest is `seed`-derived only | `err_38_hash_string_empty` | [x] |
| 39 | `stbds_hash_string` | bytes `>= 0x80` in the string | `(unsigned char) *str++` — zero-extends, does **not** sign-extend (unlike `hash_bytes`) | `err_39_hash_string_high_bytes` | [x] |
| 40 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` | unreachable on a well-formed arena; reachable only on a forged arena (`storage == NULL, remaining > 0`) which NULL-derefs first. Both libs abort/segv identically. | `err_40_stralloc_forged_arena` (forked child) | [x] |
| 41 | `stbds_stralloc` | `len > blocksize` (string longer than the current block size, i.e. `> 512 << (block>>1)`), tested at `block` 0..4 and at the exact `len == blocksize` boundary | takes the "oversized single block" path: block spliced *after* `a->storage`, `remaining` untouched unless `storage == NULL` (then `remaining = 0`) | `err_41_stralloc_oversized`, `cfg_54_stralloc_oversized_fresh`, `cfg_55_stralloc_oversized_spliced` | [x] |
| 42 | `stbds_stralloc` | first call on a zeroed arena (`storage == NULL`, `remaining == 0`) with a short string (`strlen` 0..511) | `len > 0 == remaining` → allocates a 512-byte block, `remaining = 512 - len`, `block` bumped to 1 | `err_42_stralloc_first` | [x] |
| 43 | `stbds_stralloc` | `block` grown until `512 << (block>>1) >= 1<<20` (`STBDS_STRING_ARENA_BLOCKSIZE_MAX`) | `++a->block` stops; `block` saturates at 24 | `err_43_stralloc_block_saturate` | [x] |
| 44 | `stbds_stralloc` | empty string `""` (`len == 1`) | still consumes 1 byte of `remaining` | `err_44_stralloc_empty_string` | [x] |
| 45 | `stbds_strreset` | already-empty / zeroed arena (`storage == NULL`) | frees nothing, zeroes the 24-byte arena | `err_45_strreset_empty` | [x] |
| 46 | `stbds_shmode_func` | `mode` out of enum range (`4`, `255`, `256`, `-1`) | `(unsigned char) mode` truncates: `256 -> 0`, `-1 -> 255`; the later `switch (table->string.mode)` then falls to `default:` → raw `memcpy` of the key instead of a string mode | `err_46_shmode_out_of_range` | [x] |
| 47 | `stbds_shmode_func` | `elemsize == 0` | `memset(a,0,0)`, `length = 1`, table allocated; subsequent `hmput_key` writes at offset 0 | `err_47_shmode_elemsize_zero` | [x] |
| 48 | `stbds_is_key_equal` | string mode with a NULL stored key pointer (reachable via `STBDS_SH_DEFAULT` + `key == NULL`) | `strcmp(NULL, ...)` → SIGSEGV in both | `err_48_string_null_key` (forked child) | [x] |
| 49 | `arr_push` | `num <= 0` (`0`, `-1`, `INT_MIN`) | outer loop never runs; no allocation, no crash | `err_49_arr_push_nonpositive` | [x] |
| 50 | `arr_push` | `num` large enough that `i += 50` overflows `int` | Not reachable in practice: the inner loop pushes `i` elements per outer step, so any `num` near `INT_MAX` requires ~10^15 pushes. Verified over the largest tractable values (`4999`..`20000`, plus `5000`/`5001` around the `i += 50` stride) — both libraries exit 0 with identical behaviour. The wrapping arithmetic itself is byte-identical (`i` is `c_int` in both). | `err_50_arr_push_large`, `cfg_18_arr_push_range` | [x] |
| 51 | `strkey` | negative `n` (`-1`, `INT_MIN`) | `sprintf(buffer,"test_%d",n)` → `"test_-2147483648"` (16 chars, fits the 256-byte buffer) | `err_51_strkey_negative` | [x] |
| 52 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | only reachable with `slot_count < 8`; `slot_count` is always a power of two `>= 8` via the public API, so unreachable. Invariant asserted on every table observed. | `err_52_hash_index_invariant` | [x] |
