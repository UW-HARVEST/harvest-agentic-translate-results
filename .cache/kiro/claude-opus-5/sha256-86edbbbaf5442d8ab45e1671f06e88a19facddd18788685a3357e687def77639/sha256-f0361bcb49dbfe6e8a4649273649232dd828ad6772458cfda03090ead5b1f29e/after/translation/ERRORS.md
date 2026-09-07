# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `STBDS_ASSERT`,
every `return 0 / return -1 / return NULL`, every `== NULL` / `== 0` guard, and
every sentinel written into `*temp` / `bucket->index[]`. Line numbers refer to
`c_src/src/lib.c`.

`STBDS_ASSERT` is `assert` and the C library is built with no `NDEBUG`
(`CMakeLists.txt` sets no build type / no `-DNDEBUG`, confirmed: the `.so` has
an undefined reference to `__assert_fail`). A failing assert therefore calls
`__assert_fail` → `abort()` → **SIGABRT (signal 6)**. The Rust side maps this to
`std::process::abort()`, also SIGABRT. Differential tests for abort rows run the
call in a forked child and compare the wait status of both libraries.

Legend for "expected C result": `NULL` / `= a` = returned pointer; `temp=-1` =
value written through the out-param or into the array header's `temp` field;
`SIGABRT` = process aborts via failed assert.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_hmfree_func` (L573) | `a == NULL` | returns immediately, no-op, no crash | [x] |
| 2 | `stbds_hmfree_func` (L574) | `a != NULL` but `stbds_header(a)->hash_table == NULL` (raw array from `arrgrowf`, never put into) | skips string-arena cleanup; frees `hash_table` (NULL) and header | [x] |
| 3 | `stbds_hmdel_key` (L809-810) | `a == NULL` | returns `0` (NULL) — the *only* function that returns a NULL sentinel | [x] |
| 4 | `stbds_hmdel_key` (L816) | `a != NULL` but `hash_table == 0` (table created by `hmput_default`, no `hmput_key` yet) | sets `stbds_temp(raw_a)=0`, returns `a` unchanged, length unchanged | [x] |
| 5 | `stbds_hmdel_key` (via `stbds_hm_find_slot` L610/L621 → `slot < 0`) | key not present in a populated table | sets `stbds_temp(raw_a)=0`, returns `a`, length unchanged, `used_count` unchanged | [x] |
| 6 | `stbds_hmget_key_ts` (L638) | `a == NULL` | allocates 1-elem zeroed array, `*temp = STBDS_INDEX_EMPTY` (-1), returns non-NULL hash ptr | [x] |
| 7 | `stbds_hmget_key_ts` (L645) | `a != NULL`, `hash_table == 0` | `*temp = -1`, returns `a` unchanged | [x] |
| 8 | `stbds_hmget_key_ts` (L649) | key absent from a populated table (`stbds_hm_find_slot` returns -1) | `*temp = STBDS_INDEX_EMPTY` (-1) | [x] |
| 9 | `stbds_hmget_key` (L659-663) | same three cases as rows 6–8 | writes the same `-1` into `stbds_header(raw_a)->temp` | [x] |
| 10 | `stbds_hm_find_slot` (L610) | probe hits `STBDS_HASH_EMPTY` in the `i = pos&MASK .. BUCKET_LENGTH` half | returns `-1` | [x] |
| 11 | `stbds_hm_find_slot` (L621) | probe hits `STBDS_HASH_EMPTY` in the wrap-around `i = 0 .. limit` half | returns `-1` | [x] |
| 12 | `stbds_arrgrowf` (L307) | `min_cap <= stbds_arrcap(a)` (e.g. `addlen=0, min_cap=0` on an existing array) | returns `a` **unchanged**, no realloc, capacity/length untouched | [x] |
| 13 | `stbds_arrgrowf` (L307) | `a == NULL` with `addlen=0, min_cap=0` | `min_len = 0`, so `min_cap (0) <= stbds_arrcap(NULL) (0)` is TRUE and the function takes the early `return a` — i.e. it **returns NULL** and allocates nothing. (Verified against the C `.so`; the `min_cap < 4` floor is never reached in this case.) | [x] |
| 13a | `stbds_arrgrowf` (L310-313) | `a == NULL` with `min_cap ∈ {1,2,3}` or `addlen ∈ {1,2,3}` | `min_cap < 2*0` is false, `min_cap < 4` is true ⇒ capacity forced to **4**; fresh header `length=0, temp=0, hash_table=NULL` | [x] |
| 14 | `stbds_hmput_default` (L669) | `a != NULL` but `stbds_header(hash_to_arr(a))->length == 0` | re-grows and re-zeroes, `length` becomes 1 | [x] |
| 15 | `stbds_hmput_default` (L669) | `a != NULL` and `length != 0` | returns `a` **unchanged** (no allocation) | [x] |
| 16 | `stbds_make_hash_index` (L401) `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | `slot_count <= 2`. Not reachable through any public entry point (`hmput_key` uses 8 or `slot_count*2`; `hmdel_key` shrinks only while `slot_count > 8`, so min is 8; `shmode_func` uses 8). | unreachable — documented as such, no test possible without calling a `static` fn | n/a |
| 17 | `stbds_hmput_key` (L778) `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | unreachable: the preceding `if ((size_t) i+1 > stbds_arrcap(a)) arrgrowf(...)` guarantees it | unreachable | n/a |
| 18 | `stbds_hmdel_key` (L828) `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | unreachable: `stbds_hm_find_slot` only ever returns a slot `< slot_count` | unreachable | n/a |
| 19 | `stbds_hmdel_key` (L832) `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t`, so the comparison is tautologically true in C — the assert can never fire | unreachable (Rust must also never fire) | n/a |
| 20 | `stbds_hmdel_key` (L846) `STBDS_ASSERT(slot >= 0)` | the last element's key is not findable after the `memmove`. Reachable by deleting with a **`mode` mismatch**: insert with `mode=0` (binary) then delete with `mode=1` (string) — the re-find dereferences the key bytes as a `char*` | SIGABRT (or SIGSEGV first, from the bogus `char*` deref) | [x] |
| 21 | `stbds_hmdel_key` (L849) `STBDS_ASSERT(b->index[i] == final_index)` | same class as row 20 (re-find lands on a different slot) | SIGABRT | [x] |
| 22 | `stbds_stralloc` (L913) `STBDS_ASSERT(len <= a->remaining)` | **unreachable**: the assert is only reached when either `len <= a->remaining` already held, or the `else` branch just set `a->remaining = blocksize` with `blocksize >= len` (the `len > blocksize` case returns early). Confirmed by inspecting all three exits of the `if (len > a->remaining)` block. | unreachable | n/a |
| 23 | `intput` (L953) `STBDS_ASSERT(hmget(intmap, 9) == num)` | cannot fire: `hmput(intmap, 9, num)` is the last put, so slot 9 always holds `num` | unreachable | n/a |
| 24 | `intput` (L954) `STBDS_ASSERT(hmget(intmap, 11) == 3)` | `num == 9`? no. Fires when `num == 11`? no — `hmput(intmap,11,3)` runs after `hmput(intmap,num,7)`, so key 11 is 3 unless... `num==11` → put(11,7) then put(11,3) → 3, still passes. Not reachable. | unreachable | n/a |
| 25 | `intput` (L955) `STBDS_ASSERT(hmget(intmap, num) == 7)` | `num == 11` → key 11 was overwritten to 3, `3 != 7`; `num == 9` → key 9 was overwritten to `num=9`, `9 != 7` | **SIGABRT** for `num ∈ {9, 11}` | [x] |
| 26 | `intput` | `num == 7`: passes (7 is a value, not a key). Included to prove only 9 and 11 abort. | returns normally | [x] |
| 27 | out-of-range enum: `mode` | `mode` is a plain `int`; the C only ever tests `mode >= STBDS_HM_STRING` (1) and, in `hmdel_key`, `mode == STBDS_HM_STRING` exactly. Values `-1`, `INT_MIN`, `2`, `3`, `999`, `INT_MAX` are all legal ints with no enum variant. `mode<1` ⇒ binary; `mode>1` ⇒ string hashing/compare **but** `hmdel_key`'s strdup-free and key-re-find take the `!= 1` path. | must match exactly per value, incl. the `mode==2` vs `mode==1` split in `hmdel_key` | [x] |
| 28 | out-of-range enum: `stbds_shmode_func` `mode` | `h->string.mode = (unsigned char) mode` — truncating cast. `mode = -1` → 255, `256` → 0, `259` → 3 (aliases `STBDS_SH_ARENA`), `INT_MIN` → 0, `INT_MAX` → 255. `hmput_key`'s `switch (table->string.mode)` then hits `default` (memcpy branch) for anything not in 1..3. | identical `string.mode` byte and identical subsequent `hmput_key` behaviour | [x] |
| 29 | zero length | `stbds_hash_bytes(p, 0, seed)` — the `switch (len - i)` `case 0: break;` path, `data = 0 << 56` | a well-defined hash of the empty input | [x] |
| 30 | zero length | `stbds_hash_string(p, seed)` on `""` — loop body never runs | hash of the seed alone | [x] |
| 31 | oversized length | `stbds_hash_bytes` with `len` far larger than the buffer would read OOB — **not testable** (UB). Instead: `len` = 1..64 over a 64-byte buffer covers every `switch` case and every full 8-byte block count. | n/a (documented) | n/a |
| 32 | `stbds_arrgrowf` overflow | `elemsize * min_cap + sizeof(header)` overflows / `realloc` returns NULL → the C then writes through `b = NULL + 32`. Segfault, UB, not differentially testable. Documented, not tested. | n/a | n/a |
| 33 | null pointers into `stbds_arrfreef` / `stbds_strreset` / `stbds_hash_string` | `stbds_arrfreef(NULL)` → `free((char*)NULL - 32)`; `stbds_strreset(NULL)` → NULL deref; `stbds_hash_string(NULL,·)` → NULL deref. All UB / crash in C, deliberately not "fixed" in Rust. Verified only that Rust crashes the same way is not meaningful; documented as UB and excluded. | n/a (UB) | n/a |
| 34 | `strkey` boundary values | `n = INT_MIN` (`-2147483648`, `sprintf %d` of the most-negative int), `n = INT_MAX`, `n = 0`, `n = -1` | exact `test_%d` bytes incl. the `-` sign | [x] |

Rows marked `n/a` are unreachable-through-the-public-API or genuine C
undefined behaviour; they are recorded so the surface is complete, and the
reasoning for excluding each one is given inline. Every other row has a
differential test in `translation/tests/`.

## Row → test mapping (all in `translation/tests/`)

| ERRORS row(s) | test |
|---|---|
| 1 | `phase_c_errors::err01_hmfree_null` |
| 2 | `phase_c_errors::err02_hmfree_no_table` |
| 3 | `phase_c_errors::err03_hmdel_null_returns_null` |
| 4 | `phase_c_errors::err04_hmdel_no_table` |
| 5 | `phase_c_errors::err05_hmdel_absent_key` |
| 6, 7, 8, 9 | `phase_c_errors::err06_09_get_sentinels` |
| 10, 11 | `phase_c_errors::err10_11_find_slot_both_halves` |
| 12, 13, 13a | `phase_c_errors::err12_13_arrgrowf_boundaries`, `phase_b_low::row13_14_arrgrowf_fresh_small` |
| 14, 15 | `phase_c_errors::err14_15_hmput_default_branches` |
| 20, 21 | `phase_c_errors::err20_21_hmdel_mode2_abort` (subprocess termination parity) |
| 25 | `phase_c_errors::err25_intput_aborts_on_9_and_11` (subprocess, both SIGABRT) |
| 26 | `phase_c_errors::err26_intput_ok_on_7` (subprocess, both exit 0) |
| 27 | `phase_c_errors::err27_out_of_range_mode_enum` |
| 28 | `phase_c_errors::err28_shmode_out_of_range` |
| 29, 30 | `phase_c_errors::err29_30_zero_length` |
| 34 | `phase_c_errors::err34_strkey_boundaries`, `phase_b_low::row65_strkey` |
| generic: `keysize == 0`, `key == NULL` | `phase_c_errors::err_generic_keysize_zero` |
| generic: mismatched `keyoffset` | `phase_c_errors::err_generic_keyoffset_mismatch` |

Rows 16-19, 22-24, 31-33 are the `n/a` rows: unreachable asserts, tautological
asserts, or genuine C undefined behaviour. Each row states its exclusion reason
inline.

## Corrections made while testing

Two rows in the first draft of this table were derived wrongly and were fixed
after checking the actual C `.so`:

- **Row 13** originally claimed `arrgrowf(NULL, e, 0, 0)` bumps the capacity to
  4. It does not: `min_len` is 0, so `min_cap (0) <= stbds_arrcap(NULL) (0)`
  holds and the function takes the early `return a`, returning **NULL**. The
  capacity floor of 4 applies only for `min_cap`/`addlen` in 1..3 (row 13a).
- **Row 22** originally claimed `STBDS_ASSERT(len <= a->remaining)` in
  `stbds_stralloc` was reachable by wrapping the block size to 0. It is not: the
  `len > blocksize` case returns before the assert, and the `else` case sets
  `remaining = blocksize >= len`. The row is now marked unreachable.
