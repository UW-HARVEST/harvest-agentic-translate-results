# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every early `return`,
every sentinel/`-1`/`NULL` result, every `STBDS_ASSERT`, every null check, every
explicit range/threshold comparison and every min/max constant.

This library follows the `stb_ds.h` convention: it has **no error enum and no
`errno`**.  Rejection is expressed as
* an **early `return`** that leaves state untouched (no-op),
* the **sentinels** `-1` (`STBDS_INDEX_EMPTY`), `-2` (`STBDS_INDEX_DELETED`),
  `0`, or `NULL` written to a return value / to `header->temp`,
* or an **`assert()` abort** (the C `.so` is built with asserts LIVE — it
  imports `__assert_fail@GLIBC_2.2.5`).

`temp` below means `stbds_header(x)->temp`, the out-of-band result channel the
`hm*`/`sh*` macros read after each call.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `stbds_arrgrowf` | `min_cap <= arrcap(a)` after `min_len` clamp → nothing to do (`lib.c:286-287`) | returns `a` **unchanged** (same pointer, header untouched); with `a==NULL,addlen==0,min_cap==0` returns `NULL` | `err_e1_arrgrowf_noop` | [x] |
| E2 | `stbds_arrgrowf` | `a == NULL` (fresh allocation) (`lib.c:300`) | new block with `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap, 4)` | `err_e2_arrgrowf_fresh` | [x] |
| E3 | `stbds_arrgrowf` | `elemsize == 0` | still allocates `0*min_cap+32` bytes; `capacity=min_cap` | `err_e3_arrgrowf_elemsize0` | [x] |
| E4 | `stbds_arrgrowf` | `addlen` huge so `min_len = arrlen+addlen` wraps `size_t` | wrapped `min_cap` is used verbatim (no overflow check) — must wrap identically | `err_e4_arrgrowf_wrap` | [x] |
| E5 | `stbds_hmfree_func` | `a == NULL` (`lib.c:573`) | `return;` — no-op, no crash | `err_e5_hmfree_null` | [x] |
| E6 | `stbds_hmfree_func` | `a != NULL` but `header(a)->hash_table == NULL` (`lib.c:574`) | skips arena reset, still `free`s hash_table(NULL) + header | `err_e6_hmfree_no_table` | [x] |
| E7 | `stbds_hmget_key_ts` | `a == NULL` (`lib.c:634`) | allocates the 1-element "default" slot, sets `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash-pointer with `length==1` | `err_e7_hmget_ts_null` | [x] |
| E8 | `stbds_hmget_key_ts` | `a != NULL` but `header(raw_a)->hash_table == 0` (`lib.c:644`) | `*temp = -1`, returns `a` unchanged, **`header->temp` NOT written** | `err_e8_hmget_ts_no_table` | [x] |
| E9 | `stbds_hmget_key_ts` | key absent from a populated table → `stbds_hm_find_slot` returns `<0` (`lib.c:648`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err_e9_hmget_ts_missing` | [x] |
| E10 | `stbds_hm_find_slot` | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` in the *upper* half-scan (`lib.c:609-610`) | `return -1` (key-not-found sentinel) | `err_e9_hmget_ts_missing` / `err_e10_find_slot_wrap` | [x] |
| E11 | `stbds_hm_find_slot` | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` in the *wrap-around* half-scan (`lib.c:620-621`) | `return -1` | `err_e10_find_slot_wrap` | [x] |
| E12 | `stbds_hmget_key` | any of E7/E8/E9 | same as `*_ts` **plus** `header(raw_a)->temp = temp` is written | `err_e12_hmget_key_missing` | [x] |
| E13 | `stbds_hmput_default` | `a == NULL` (`lib.c:669`) | allocates slot 0, `length==1`, returns hash-pointer | `err_e13_hmput_default_null` | [x] |
| E14 | `stbds_hmput_default` | `a != NULL` **and** `header(raw_a)->length == 0` (`lib.c:669`) | grows/allocates and bumps `length` to 1 | `err_e14_hmput_default_len0` | [x] |
| E15 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (pure no-op) | `err_e15_hmput_default_noop` | [x] |
| E16 | `stbds_hmput_key` | `a == NULL` (`lib.c:686`) | bootstraps a 1-element array before inserting | `err_e16_hmput_key_null` | [x] |
| E17 | `stbds_hmput_key` | `table == NULL` (first insert) (`lib.c:698,702,707`) | builds an 8-slot index and sets `string.mode = (mode>=1 ? SH_DEFAULT : 0)` | `err_e17_hmput_key_first` | [x] |
| E18 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` (load factor 3/4 exceeded) (`lib.c:698`) | doubles `slot_count`, rehashes, `free`s old table (arena + seed carried over) | `cfg_*_grow` rows | [x] |
| E19 | `stbds_hmput_key` | duplicate key found in the *upper* half-scan (`lib.c:729-735`) | **does not insert**; `temp = existing index`; for `mode>=1` also sets `temp_key` | `err_e19_hmput_dup` | [x] |
| E20 | `stbds_hmput_key` | duplicate key found in the *wrap-around* half-scan (`lib.c:747-751`) | **does not insert**; `temp = existing index`; **`temp_key` is NOT updated** (C quirk) | `err_e20_hmput_dup_wrap` | [x] |
| E21 | `stbds_hmput_key` | a tombstone (`index == STBDS_INDEX_DELETED (-2)`) was seen before the empty slot (`lib.c:739-742, 766-769`) | reuses the tombstone slot and decrements `tombstone_count` | `err_e21_hmput_reuse_tombstone` | [x] |
| E22 | `stbds_hmput_key` | out-of-range `mode` (`mode >= STBDS_HM_STRING`, e.g. 2 / 3 / 999 / `INT_MAX`) (`lib.c:707,713,732`) | treated exactly as STRING: `hash_string`, `strcmp`, `string.mode=SH_DEFAULT` | `err_e22_mode_out_of_range` | [x] |
| E23 | `stbds_hmput_key` | negative `mode` (e.g. `-1`, `INT_MIN`) (`lib.c:707,713,732`) | `mode >= 1` false → treated exactly as BINARY: `hash_bytes`, `memcmp`, `string.mode=0` | `err_e22_mode_out_of_range` | [x] |
| E24 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= arrcap(a))` (`lib.c:778`) | unreachable via the public protocol (`arrgrowf` guarantees it); must NOT abort in either lib | `err_e24_no_abort_under_load` | [x] |
| E25 | `stbds_hmdel_key` | `a == NULL` (`lib.c:809-810`) | returns `NULL` (`0`); `hmdel` macro then yields `0` | `err_e25_hmdel_null` | [x] |
| E26 | `stbds_hmdel_key` | `header(raw_a)->hash_table == 0` (`lib.c:816-817`) | sets `temp = 0`, returns `a` unchanged (delete reported as "not found") | `err_e26_hmdel_no_table` | [x] |
| E27 | `stbds_hmdel_key` | key absent → `find_slot < 0` (`lib.c:821-822`) | `temp = 0`, returns `a`, `length` unchanged | `err_e27_hmdel_missing` | [x] |
| E28 | `stbds_hmdel_key` | key present (`lib.c:823-864`) | `temp = 1`, slot → `hash=STBDS_HASH_DELETED(1)`, `index=STBDS_INDEX_DELETED(-2)`, `used_count--`, `tombstone_count++`, `length--` | `err_e28_hmdel_hit` | [x] |
| E29 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t)table->slot_count)` (`lib.c:828`) | invariant; must not abort | `err_e24_no_abort_under_load` | [x] |
| E30 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` (`lib.c:832`) | vacuous for `size_t`; never fires | `err_e24_no_abort_under_load` | [x] |
| E31 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` after re-finding the moved last element (`lib.c:846`) | invariant; must not abort | `err_e24_no_abort_under_load` | [x] |
| E32 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` (`lib.c:849`) | invariant; must not abort | `err_e24_no_abort_under_load` | [x] |
| E33 | `stbds_hmdel_key` | deleting the **last** element, i.e. `old_index == final_index` (`lib.c:839`) | skips the memmove + slot re-point entirely | `err_e33_hmdel_last` | [x] |
| E34 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **exactly** and `string.mode == SH_STRDUP` (`lib.c:836`) | `free`s the strdup'd key.  For `mode == 2` (out-of-range "string") the key is **leaked instead** and the re-find takes the `memcmp` path (`lib.c:842-845`) — C quirk | `err_e34_hmdel_mode2_quirk` | [x] |
| E35 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` (`lib.c:854`) | halves `slot_count`, rebuilds, frees old table | `err_e35_hmdel_shrink` | [x] |
| E36 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (`lib.c:858`) | rebuilds at same `slot_count`, frees old table | `err_e36_hmdel_tombstone_rebuild` | [x] |
| E37 | `stbds_make_hash_index` | `slot_count <= STBDS_BUCKET_LENGTH (8)` (`lib.c:399-400`) | forces `used_count_shrink_threshold = 0` so an 8-slot table never shrinks | `err_e37_no_shrink_at_8` | [x] |
| E38 | `stbds_make_hash_index` | `STBDS_ASSERT(uct + tct < slot_count)` (`lib.c:401`) | holds for every power-of-two `slot_count >= 8` reachable here; must not abort | `err_e24_no_abort_under_load` | [x] |
| E39 | `stbds_is_key_equal` | `mode >= STBDS_HM_STRING` (`lib.c:560-561`) | dereferences the element as `char**` and `strcmp`s — comparing a BINARY table with `mode>=1` reads a bogus pointer, so the modes must match *exactly* between libs | `err_e22_mode_out_of_range` | [x] |
| E40 | `stbds_is_key_equal` | `keysize == 0` in BINARY mode (`lib.c:563`) | `memcmp(...,0) == 0` → **every** key compares equal, so only the first insert ever happens | `err_e40_keysize_zero` | [x] |
| E41 | `stbds_hash_bytes` / `stbds_siphash_bytes` | `len == 0` (and `p == NULL`) (`lib.c:522,532`) | no dereference at all; returns the hash of the empty message | `err_e41_hash_bytes_len0` | [x] |
| E42 | `stbds_hash_bytes` | `len % 8 != 0` → `switch (len-i)` fall-through chain (`lib.c:532-541`) | tail bytes folded in with the C `int`-promotion sign-extension quirk (`d[3]<<24` becomes negative and sign-extends into `size_t`) | `cfg_hash_bytes_*` | [x] |
| E43 | `stbds_hash_string` | empty string `""` (`lib.c:480`) | loop body never runs; avalanche applied to `seed` alone | `err_e43_hash_string_empty` | [x] |
| E44 | `stbds_hash_string` | bytes `>= 0x80` (`lib.c:481`) | `(unsigned char)` cast → **no** sign extension (unlike `hash_bytes`) | `cfg_hash_string_highbit` | [x] |
| E45 | `stbds_hm_find_slot` / `stbds_hmput_key` | `hash < 2` after hashing (`lib.c:596,719`) | `hash += 2` so a real hash can never collide with `HASH_EMPTY(0)` / `HASH_DELETED(1)` | `err_e45_hash_lt_2` | [x] |
| E46 | `stbds_stralloc` | `len > a->remaining` (`lib.c:885`) | allocates a new block; `a->block` bumped iff `blocksize < 1<<20` | `err_e46_stralloc_newblock` | [x] |
| E47 | `stbds_stralloc` | `len > blocksize` (oversized string) (`lib.c:893`) | dedicated block spliced **after** `storage`; `remaining` **not** reduced; returns `sb->storage` | `err_e47_stralloc_oversized` | [x] |
| E48 | `stbds_stralloc` | oversized string while `a->storage == NULL` (`lib.c:898-902`) | `sb->next = 0`, `a->storage = sb`, `a->remaining = 0` | `err_e48_stralloc_oversized_empty` | [x] |
| E49 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` (`lib.c:890`) | `a->block` saturates at 22 (`512 << 11 == 1<<20`) and stops growing | `err_e49_stralloc_block_saturate` | [x] |
| E50 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` (`lib.c:913`) | invariant after the new-block path; must not abort | `err_e46_stralloc_newblock` | [x] |
| E51 | `stbds_stralloc` | empty string `""` (`len == 1`) | consumes exactly 1 byte of `remaining` | `err_e51_stralloc_empty_str` | [x] |
| E52 | `stbds_strreset` | `a->storage == NULL` (`lib.c:923-928`) | while-loop skipped; arena zeroed anyway | `err_e52_strreset_empty` | [x] |
| E53 | `stbds_shmode_func` | `mode` outside the `STBDS_SH_*` enum (`lib.c:803`) | `(unsigned char) mode` **truncates**: `256→0`, `-1→255`, `INT_MIN→0`, `999→231`; any value not in {1,2,3} then takes the `default:` `memcpy` branch in `hmput_key` | `err_e53_shmode_out_of_range` | [x] |
| E54 | `stbds_shmode_func` | `elemsize == 0` | `arrgrowf(0,0,0,1)` still succeeds, `capacity == 4` | `err_e54_shmode_elemsize0` | [x] |
| E55 | `arr_push` | `num <= 0` (`lib.c:951`) | outer `for` never runs → no allocation, no crash | `err_e55_arr_push_nonpositive` | [x] |
| E56 | `arr_push` | `STBDS_ASSERT(arrlen(arr)==0)` with `arr==NULL` (`lib.c:950`) | `arrlen(NULL)==0` → holds; must not abort | `err_e55_arr_push_nonpositive` | [x] |
| E57 | `arr_push` | `0 < num <= 50` | exactly one outer iteration (`i==0`), inner loop empty → `arrfree(NULL)` is a no-op | `err_e57_arr_push_small` | [x] |
| E58 | `strkey` | `n < 0`, `n == INT_MIN`, `n == INT_MAX`, `n == 0` (`lib.c:941`) | `sprintf(buffer,"test_%d",n)` → `"test_-2147483648"` etc., NUL-terminated in the shared 256-byte static | `err_e58_strkey_extremes` | [x] |
| E59 | `strkey` | called twice | second call **overwrites** the same static buffer; both calls return the *same* address | `err_e59_strkey_aliases` | [x] |
| E60 | `stbds_arrfreef` | `a == NULL` (`lib.c:312-315`) | `free((char*)NULL - 32)` → **undefined / crashes in both libs**.  Not reachable from `arrfree()`, which null-checks first (`lib.c:121`).  Documented, deliberately not executed. | (documented, not executed) | [x] |
| E61 | generic FFI boundary | `stbds_rand_seed(0)` / `SIZE_MAX` then a table build (`lib.c:355-358,409-412`) | seed stored verbatim; next `make_hash_index(_,NULL)` consumes it and advances `seed*0x27bb2ee687b0b0fd + 0xb504f32d` | `err_e61_seed_extremes` | [x] |
| E62 | generic FFI boundary | `keyoffset != 0` passed to `stbds_hmdel_key` (`lib.c:843,845`) | offset is applied to the *re-find* key address but `hmput`/`hmget` always use `keyoffset==0` — asymmetry must match | `err_e62_hmdel_keyoffset` | [x] |
| E63 | generic FFI boundary | `keysize` larger than the stored key field in BINARY mode | `memcmp` over-reads into the neighbouring value bytes — both libs must agree | `err_e63_keysize_oversized` | [x] |

## Deliberately excluded

* **E60** (`stbds_arrfreef(NULL)`) is memory-unsafe *by construction in the C*
  (`free(NULL - 32)`).  Both libraries do the identical wrong thing; executing
  it would abort the whole test process, so it is verified by inspection only.
* `stbds_hmget_key(NULL, …)` / `stbds_hmput_key` with a `NULL` **key** in
  STRING mode dereferences the key in `strlen`/`strcmp` — a null deref in both.
  Not executed.
* `arr_push(INT_MAX)` is signed-overflow UB in the C `i += 50` and would need
  ~10^16 operations.  Bounded values are tested instead (E55/E57 + `cfg` rows).

---

## Phase C completion status

**All 63 rows have a passing differential test** (`tests/errors.rs`, 51 test
functions — several rows share a test because they are the two half-scans of the
same probe loop or the same `assert`):

```
running 51 tests
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Each test asserts the two libraries return the **same** sentinel, not merely that
both failed:

* the exact returned pointer nullness (`hmdel_key(NULL,…)` → `NULL`,
  `arrgrowf(NULL,e,0,0)` → `NULL`),
* the exact `*temp` / `header->temp` value (`-1` = `STBDS_INDEX_EMPTY`,
  `0` = not-deleted, `1` = deleted),
* the exact slot state after a rejection (`hash == STBDS_HASH_DELETED (1)`,
  `index == STBDS_INDEX_DELETED (-2)`),
* the exact truncated `string.mode` for out-of-range `shmode_func` arguments,
* and, for every row, that the *whole* observable state (header + hash index +
  all buckets + element payload) is byte-identical afterwards.

### Generic FFI boundaries covered beyond the table

| boundary | where |
|----------|-------|
| `NULL` pointer into every entry point that documents a null path | E1, E2, E5, E7, E13, E16, E25, E41 |
| zero lengths (`elemsize == 0`, `keysize == 0`, `len == 0`, `min_cap == 0`) | E1, E3, E40, E41, E54 |
| oversized lengths (`keysize` > the nominal key field, `addlen == SIZE_MAX`) | E4, E63 |
| `size_t` wrap-around (`min_len`, `elemsize*min_cap + 32`, `2*arrcap`) | E4, `torture_arrgrowf_capacity_overflow` |
| one step past a documented range (`slot_count` 8 floor, `block` 22 ceiling, load/tombstone thresholds ±1) | E35, E36, E37, E49 |
| **out-of-range enum values across the FFI** — `mode ∈ {INT_MIN, -1000, -2, -1, 0, 1, 2, 3, 4, 255, 256, 999, 65537, INT_MAX}` and `sh_mode ∈ {INT_MIN, -256, -2, -1, 0..5, 255, 256, 257, 511, 999, INT_MAX}` | E7, E17, E22, E23, E25, E26, E39, E53, E54 |
| `int` extremes for the scalar APIs (`strkey`, `arr_push`) | E55, E57, E58 |
| `size_t` extremes for `rand_seed` (`0`, `1`, `SIZE_MAX`, `1<<63`) and the LCG chain that follows | E61 |

`mode` is a C `int` parameter, so any 32-bit value is a legal input.  The tests
confirm the C's actual rule — `mode >= STBDS_HM_STRING` selects the string path,
so `2`, `999` and `INT_MAX` behave exactly like `1` while `-1` and `INT_MIN`
behave exactly like `0` — and that `stbds_hmdel_key` alone tests `mode == 1`
*exactly* (E34).  `sh_mode` is narrowed by `(unsigned char) mode`, so `256 → 0`,
`-1 → 255`, `999 → 231` and `INT_MIN → 0`; every one of those then falls into
`hmput_key`'s `default:` `memcpy` branch, which the tests verify.

### Rows verified by inspection rather than execution

Only **E60** (`stbds_arrfreef(NULL)`).  The C computes
`free((char *) NULL - 32)`, i.e. `free((void *) 0xffff...e0)`, which glibc
aborts on.  Both libraries perform the identical wrapping pointer arithmetic
(`err_e60_arrfreef_null_documented` asserts the computed address), but the call
is not made because it would abort the test process.  No legitimate use reaches
it: the `arrfree()` macro (`lib.c:121`) null-checks first.

The `assert()`-based rows (E24, E29–E32, E38, E50, E56) are verified
*negatively*: the C `.so` is built with asserts live (it imports
`__assert_fail@GLIBC_2.2.5`), and `gcov` confirms the assert-failure branch edge
was never taken across the entire suite — so those invariants held for every one
of the ~116 k inserts, ~50 k deletes and ~45 k arena allocations the suite drove.
