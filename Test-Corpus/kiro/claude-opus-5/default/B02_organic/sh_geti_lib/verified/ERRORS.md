# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from `grep -n 'ASSERT\|return\|assert' c_src/src/lib.c` plus
every explicit `NULL` check, range check and min/max constant in `c_src/src/lib.c`.

This library is `stb_ds.h`: it has **no error enum and no error return codes**.
Every rejection is one of

* a **sentinel return value** (`-1` = `STBDS_INDEX_EMPTY`, `-2` =
  `STBDS_INDEX_DELETED`, `NULL`/`0`, or "returns the input pointer unchanged"),
* an **out-parameter sentinel** (`*temp = -1`),
* an **`assert()` abort** (`STBDS_ASSERT` is `#define`d to `assert`, and the
  library is compiled without `NDEBUG`, so a failed assert calls
  `__assert_fail` → `abort()`; the Rust translation calls `abort()`).

Rows marked *(unreachable from the public ABI)* are documented for completeness;
they cannot be triggered without first corrupting internal state, so there is no
differential test for them — noted explicitly rather than silently dropped.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` | `min_cap <= arrcap(a)` and `arrlen(a)+addlen <= min_cap`, i.e. request already satisfied (`a=NULL, elemsize=X, addlen=0, min_cap=0`) | returns `a` **unchanged** (same pointer, incl. `NULL`) — no allocation | `errors.rs::e01_arrgrowf_noop` |
| 2 | `stbds_arrgrowf` | `a == NULL` (no existing header to read) | allocates fresh block, header `length=0, hash_table=NULL, temp=0`, `capacity=max(addlen,min_cap,4)` | `errors.rs::e02_arrgrowf_null_input` |
| 3 | `stbds_arrgrowf` | `addlen`/`min_cap` so large that `elemsize*min_cap + 32` wraps (`min_cap = SIZE_MAX`, `elemsize = SIZE_MAX`, …) → an undersized `realloc` that the C then writes a 32-byte header into | identical wait status in both (run in a forked child) | `errors_crash.rs::e03_arrgrowf_size_overflow` |
| 4 | `stbds_arrfreef` | `a == NULL` — the C code has **no** null check, so it calls `free((char *) NULL - 32)` | identical fatal signal in both (verified: not a clean exit) | `errors_crash.rs::e04_arrfreef_null` |
| 5 | `stbds_hmfree_func` | `a == NULL` | early `return;` — no free, no crash | `errors.rs::e05_hmfree_null` |
| 6 | `stbds_hmfree_func` | `a != NULL` but `hdr(a)->hash_table == NULL` (array built by `arrgrowf` only) | skips the strdup-key loop and `strreset`, still `free`s `hash_table` (NULL) and the header | `errors.rs::e06_hmfree_no_table` |
| 7 | `stbds_hm_find_slot` (via get/del) | probe reaches a bucket slot with `hash == STBDS_HASH_EMPTY` (0) before a match — first inner loop | returns `-1` | `errors.rs::e07_find_slot_miss` |
| 8 | `stbds_hm_find_slot` (via get/del) | same, detected in the second (wrap-around) inner loop `i < pos & MASK` | returns `-1` | `errors.rs::e08_find_slot_miss_wrap` |
| 9 | `stbds_hmget_key_ts` | `a == NULL` | allocates a 1-element array, `length=1`, element zeroed, `*temp = -1`, returns `arr+elemsize` (non-NULL) | `errors.rs::e09_hmget_ts_null` |
| 10 | `stbds_hmget_key_ts` | `a != NULL` but `hdr(hash_to_arr(a))->hash_table == NULL` | `*temp = -1`, returns `a` unchanged; **key/keysize never read** (so a NULL key is accepted here) | `errors.rs::e10_hmget_ts_no_table` |
| 11 | `stbds_hmget_key_ts` | key absent from a populated table (`slot < 0`) | `*temp = -1` (`STBDS_INDEX_EMPTY`), returns `a` | `errors.rs::e11_hmget_ts_absent` |
| 12 | `stbds_hmget_key` | any of rows 9–11 | same as `hmget_key_ts`, **and** `hdr(hash_to_arr(p))->temp` is set to the same sentinel (`-1`) | `errors.rs::e12_hmget_key_sentinel` |
| 13 | `stbds_hmdel_key` | `a == NULL` | returns `0` (`NULL`) — the *only* NULL return in the library | `errors.rs::e13_hmdel_null` |
| 14 | `stbds_hmdel_key` | `a != NULL`, `hash_table == NULL` | sets `hdr(raw_a)->temp = 0`, returns `a` (no delete) | `errors.rs::e14_hmdel_no_table` |
| 15 | `stbds_hmdel_key` | key not present (`slot < 0`) | `temp` stays `0`, returns `a`, `length`/`used_count` unchanged | `errors.rs::e15_hmdel_absent` |
| 16 | `stbds_hmdel_key` | deleting the *same* key twice | 2nd call takes row 15 (`temp == 0`) — first call returns `temp == 1` | `errors.rs::e16_hmdel_twice` |
| 17 | `stbds_hmdel_key` | `assert(slot < (ptrdiff_t) table->slot_count)` | abort *(unreachable: `find_slot` masks `pos` with `slot_count-1`, so the returned slot is always `< slot_count`)* | *(documented, not run)* |
| 18 | `stbds_hmdel_key` | `assert(table->used_count >= 0)` | `used_count` is `size_t`, so this assert can never fire | *(vacuous)* |
| 19 | `stbds_hmdel_key` | `assert(slot >= 0)`: the re-find of the tail element moved into the hole fails. Reachable **through the public ABI**: `stbds_hmdel_key` picks the re-find with the exact test `mode == STBDS_HM_STRING`, while `stbds_hm_find_slot` dispatches on `mode >= STBDS_HM_STRING`, so `mode = 2` on a string table re-finds using the element's ADDRESS instead of the stored `char *` | **SIGABRT** (`__assert_fail`); the Rust `abort()`s | `errors_crash.rs::e19b_hmdel_refind_assert_via_public_abi` (+ `e19_hmdel_refind_assert` via deliberate identical bucket corruption) |
| 20 | `stbds_hmdel_key` | `assert(b->index[i] == final_index)` | abort *(unreachable while row 19 holds: if the re-find succeeds it can only have matched the moved element's own key, whose recorded index is `final_index`)* | *(documented, not run)* |
| 21 | `stbds_hmput_key` | `assert((size_t) i+1 <= arrcap(a))` after the grow | abort *(unreachable: the preceding `arrgrowf(a, elemsize, 1, 0)` guarantees `capacity >= length+1`)* | *(documented, not run)* |
| 22 | `stbds_make_hash_index` | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` — fails for `slot_count <= 2` | abort *(unreachable: `static`, only called with `8`, `slot_count*2`, or `slot_count>>1` guarded by `slot_count > 8`)* | *(documented, not run)* |
| 23 | `stbds_stralloc` | `assert(len <= a->remaining)` | abort *(unreachable: the preceding `if` either returns early or sets `remaining = blocksize >= len`)* | *(documented, not run)* |
| 24 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` (huge string) → big-block path | returns `sb->storage` of a **freshly chained** block; when `a->storage != NULL` the new block is spliced in *after* the head and `a->remaining` is **left unchanged**; when `a->storage == NULL` head is set and `remaining = 0` | `errors.rs::e24_stralloc_oversize` |
| 25 | `stbds_stralloc` | empty string `""` (`len == 1`) into a fresh arena (`remaining == 0`) | `1 > 0` so a 512-byte block is allocated, `block` → 1, `remaining` → 511 | `errors.rs::e25_stralloc_empty_string` |
| 26 | `stbds_stralloc` | `a->block` already saturated: `512 << (block>>1) >= 1<<20` | `a->block` is **not** incremented any more (max blocksize `1<<20`) | `errors.rs::e26_stralloc_block_saturation` |
| 27 | `stbds_strreset` | `a->storage == NULL` (fresh/empty arena) | while-loop body never runs; arena is fully zeroed (`storage=NULL, remaining=0, block=0, mode=0`) | `errors.rs::e27_strreset_empty` |
| 28 | `stbds_strreset` | called twice in a row | second call is a no-op (row 27) | `errors.rs::e28_strreset_twice` |
| 29 | `stbds_hash_bytes` | `len == 0` | still runs the finalisation rounds; result is `f(0, seed)`, **not** 0 | `errors.rs::e29_hash_bytes_zero_len` |
| 30 | `stbds_hash_bytes` | `len == 0` **and** `p == NULL` | never dereferences `p` (loop bound `i+8 <= 0` false, `switch(0)` → `case 0: break`) → returns a value, no crash | `errors.rs::e30_hash_bytes_null_zero_len` |
| 31 | `stbds_hash_string` | empty string `""` | `while (*str)` never runs; hashes the seed only | `errors.rs::e31_hash_string_empty` |
| 32 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` / `stbds_hm_find_slot` | `mode` is an **out-of-range enum int** (`STBDS_HM_BINARY=0`, `STBDS_HM_STRING=1` are the only defined values). Every dispatch is `mode >= STBDS_HM_STRING`, so `mode = 2, 7, 1000, INT_MAX` behave as **STRING**; `mode = -1, -7, INT_MIN` behave as **BINARY** | binary/string selected by `mode >= 1`; **but** `stbds_hmdel_key`'s two extra checks use `mode == STBDS_HM_STRING` (exact `== 1`), so `mode = 2` deletes *without* freeing the strdup'd key (verified by reading the key buffer after the delete) and re-finds the slot via the **binary** path | `errors.rs::e32_mode_out_of_range_string_side`, `e32_mode_out_of_range_binary_side`, `e32_mode_out_of_range_del_exact_string_check`; `errors_crash.rs::e19b_*` |
| 33 | `stbds_shmode_func` | `mode` out of `{0,1,2,3}` (e.g. `4`, `-1`, `256`, `INT_MAX`) | stored as `(unsigned char) mode`, i.e. **truncated mod 256**; `string.mode = 256` → `0` (`STBDS_SH_NONE`), `-1` → `255` | `errors.rs::e33_shmode_out_of_range` |
| 34 | `stbds_shmode_func` | `elemsize == 0` | `arrgrowf(0,0,0,1)` → `realloc(NULL, 32)`, `memset(a,0,0)`, `length=1`; returns `arr + 0` (== the array pointer itself) | `errors.rs::e34_shmode_zero_elemsize` |
| 35 | `stbds_hmput_default` | `a == NULL` | allocates, `length = 1`, element zeroed, returns `arr+elemsize` | `errors.rs::e35_hmput_default_null` |
| 36 | `stbds_hmput_default` | `a != NULL` and `hdr(hash_to_arr(a))->length == 0` | re-grows in place and bumps `length` to 1 | `errors.rs::e36_hmput_default_zero_len` |
| 37 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged**, does **not** re-zero the default element | `errors.rs::e37_hmput_default_idempotent` |
| 38 | `stbds_hmput_key` | `a == NULL` | bootstraps a 1-element array first, then inserts; result index (`temp`) is `0` for the first real key | `errors.rs::e38_hmput_null_bootstrap` |
| 39 | `stbds_hmput_key` | re-putting an **existing** key | returns the existing index in `temp`, `used_count` and `length` unchanged, and (first inner loop only) sets `table->temp_key` to the **stored** key pointer, not the caller's | `errors.rs::e39_hmput_existing_key` |
| 40 | `stbds_hmput_key` | `keysize == 0` in BINARY mode | `memcmp(...,0)` == 0 always → **every key compares equal**, so the table degenerates to a single entry | `errors.rs::e40_hmput_zero_keysize` |
| 41 | `sh_geti` | `num <= 0` (`0`, `-1`, `INT_MIN`) | every `for` loop body is skipped; the function still creates/destroys the two string maps and asserts `shgeti(...,"foo") == -1`; prints nothing | `errors.rs::e41_sh_geti_nonpositive` |
| 42 | `strkey` | `n` negative / `INT_MIN` | `sprintf(buffer, "test_%d", n)` → `"test_-2147483648"`; returns the shared `static` buffer (same pointer every call) | `errors.rs::e42_strkey_negative` |

## Generic FFI-boundary boundaries also covered

| trigger | covered by |
|---------|-----------|
| NULL `a` into every pointer-taking export | rows 4, 5, 9, 13, 35, 38 |
| NULL `p` with `len == 0` into `stbds_hash_bytes` | row 30 |
| NULL key where the C never reads it (accepted) | `errors_crash.rs::e_null_key_accepted_where_c_does_not_read_it` |
| NULL key where the C *does* read it (must fault identically) | `errors_crash.rs::e_null_key_crashes_where_c_reads_it` |
| zero length / zero size (`len=0`, `keysize=0`, `elemsize=0`, `addlen=0`) | rows 1, 29, 34, 40 |
| out-of-range enum ints across FFI (`mode`) | rows 32, 33 |
| one step past a documented range (`STBDS_SH_ARENA+1 = 4`) | row 33 |
| empty C strings | rows 25, 31 |
| oversized inputs (string longer than the arena block max) | row 24 |
| non-zero `keyoffset` (a public parameter the convenience macros always pass as 0) | `hashmap.rs::cfg_extra_del_nonzero_keyoffset` |

## Mutation adequacy

The differential harness was validated by injecting 14 deliberate single-line
faults into `src/lib.rs`, rebuilding, and re-running the whole suite. **13 of 14
were detected.** The one that was not:

* `if hash < 2 { hash += 2 }` → `if hash < 1 { ... }` in `stbds_hm_find_slot`
  and `stbds_hmput_key`. This is only distinguishable on a key whose
  siphash/string-hash output is exactly `1` — a 1-in-2^64 input that cannot be
  produced without inverting the hash. Recorded here as a known,
  input-unreachable equivalence rather than a coverage gap.

Faults that *were* detected include: `hash_string` multiplier and rotate
constants, removal of the siphash tail sign-extension, the siphash D-round
count, `used_count_shrink_threshold` / `tombstone_count_threshold` formulas, the
`temp`/`bucket->index` off-by-one in `hmput_key`, `stbds_shmode_func`'s `u8`
truncation, `strreset`'s memset length, `hmput_default`'s idempotence guard, the
arena `blocksize < MAX` comparison, the `strkey` format string, and both
`mode == STBDS_HM_STRING` exact comparisons inside `stbds_hmdel_key`
(strdup-free and re-find), plus adding **or** removing the `temp_key` write in
either inner loop of `stbds_hmput_key`.
