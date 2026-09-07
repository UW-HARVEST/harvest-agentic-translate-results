# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from every early-return, sentinel-return, `assert`, range
check and boundary constant in `c_src/src/lib.c`. There are no error *enums* and
no `RETURN_ERROR` macros in this library; rejection is expressed as

* a **sentinel return value** (`NULL`, `-1` / `STBDS_INDEX_EMPTY`, unchanged
  pointer), or
* an **`assert()` failure** (`#define STBDS_ASSERT assert`) → glibc
  `__assert_fail` → `SIGABRT` (exit signal 6).

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` (:286) | `min_cap <= stbds_arrcap(a)` after `min_cap = max(min_cap, arrlen+addlen)` — i.e. request already satisfied | returns `a` **unchanged** (identical pointer, no realloc, capacity untouched) | `err_01_arrgrowf_no_grow` [x] |
| 2 | `stbds_arrgrowf` (:283,:300) | `a == NULL` (no array yet) | allocates, sets `length=0, hash_table=NULL, temp=0`, `capacity=max(addlen,min_cap,4)` | `err_02_arrgrowf_null_input` [x] |
| 3 | `stbds_arrgrowf` (:291) | `min_cap` in `1..=3` with `a == NULL` (below the minimum capacity floor) | capacity is bumped to `4` | `err_03_arrgrowf_min_cap_floor` [x] |
| 4 | `stbds_arrgrowf` (:297) | `elemsize * min_cap` overflows `size_t` (e.g. `elemsize=SIZE_MAX`, `min_cap` large) | wrapping multiply then `realloc(huge)`; both must agree on the wrapped size request. Not exercised destructively; the wrapping arithmetic is asserted via `elemsize=0`. | `err_04_arrgrowf_elemsize_zero` [x] |
| 5 | `stbds_hmfree_func` (:573) | `a == NULL` | returns immediately, **no free, no crash** | `err_05_hmfree_null` [x] |
| 6 | `stbds_hmfree_func` (:574) | `stbds_hash_table(a) == NULL` (array made by `arrgrowf`, never a map) | skips strdup-key loop and `strreset`, still frees `hash_table` (NULL) + header | `err_06_hmfree_no_table` [x] |
| 7 | `stbds_hm_find_slot` (:610,:621) | probed bucket slot has `hash == STBDS_HASH_EMPTY` (0) → key absent | returns `-1` | covered by #8/#9/#12 [x] |
| 8 | `stbds_hmget_key_ts` (:634) | `a == NULL` | `*temp = STBDS_INDEX_EMPTY (-1)`; returns a **newly allocated** 1-element map (non-NULL) | `err_08_hmget_ts_null_map` [x] |
| 9 | `stbds_hmget_key_ts` (:644) | `a != NULL` but `header->hash_table == 0` (e.g. map created by `hmput_default` only) | `*temp = -1`; returns `a` unchanged | `err_09_hmget_ts_no_table` [x] |
| 10 | `stbds_hmget_key_ts` (:648) | key not present in a populated table | `*temp = STBDS_INDEX_EMPTY (-1)`; returns `a` unchanged | `err_10_hmget_ts_missing_key` [x] |
| 11 | `stbds_hmget_key` (:663) | same three triggers as #8/#9/#10 | writes the same `-1` into `stbds_header(arr)->temp` | `err_11_hmget_key_missing` [x] |
| 12 | `stbds_hmdel_key` (:809) | `a == NULL` | returns `0` (**NULL**) | `err_12_hmdel_null_map` [x] |
| 13 | `stbds_hmdel_key` (:816) | `header->hash_table == 0` | sets `temp = 0`, returns `a` unchanged, length unchanged | `err_13_hmdel_no_table` [x] |
| 14 | `stbds_hmdel_key` (:821) | key not found (`slot < 0`) | `temp = 0`, returns `a`, `used_count`/length unchanged | `err_14_hmdel_missing_key` [x] |
| 15 | `stbds_hmdel_key` (:828) | `assert(slot < (ptrdiff_t) table->slot_count)` | unreachable from the public API (`find_slot` masks `pos` with `slot_count-1`); asserted to be non-firing on a large randomized delete workload | `err_15_hmdel_slot_assert_never_fires` [x] |
| 16 | `stbds_hmdel_key` (:846) | `assert(slot >= 0)` — re-locating the moved last element fails | unreachable in a consistent table; a *deliberately corrupted* map (delete a key whose stored key bytes were overwritten behind the library's back) makes both C and Rust `SIGABRT` | `err_16_hmdel_corrupt_key_abort` [x] |
| 17 | `stbds_hmdel_key` (:849) | `assert(b->index[i] == final_index)` | same class as #16; both abort identically | covered by `err_16_hmdel_corrupt_key_abort` [x] |
| 18 | `stbds_hmdel_key` (:836) | `mode == STBDS_HM_STRING` **and** `string.mode == STBDS_SH_STRDUP` | frees the old key pointer; `mode == 2` (out-of-range "string" mode) must **not** free (`==` not `>=`) | `err_18_hmdel_mode_two_no_free` [x] |
| 19 | `stbds_hmput_key` (:778) | `assert((size_t) i+1 <= stbds_arrcap(a))` | unreachable after the preceding grow; asserted non-firing across the randomized insert workload | `err_19_hmput_cap_assert_never_fires` [x] |
| 20 | `stbds_make_hash_index` (:401) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)`. Fails for `slot_count ∈ {0,1,2}` (`0<0`, `1<1`, `2<2`). | unreachable: the only call sites pass `8`, `slot_count*2` or `slot_count>>1` (floored at 8 by the `slot_count > 8` guard at :854). Documented as unreachable; the shrink path is driven down to `slot_count == 8` and must not abort. | `err_20_shrink_floor_no_abort` [x] |
| 21 | `stbds_stralloc` (:913) | `assert(len <= a->remaining)` | unreachable: the preceding `if` guarantees either `len <= a->remaining` already, or a fresh block of `blocksize >= len`. Driven with hand-forged arenas (`block` 0..255, `remaining` 0) without ever firing. | `err_24_stralloc_block_ceiling` [x] |
| 22 | `stbds_stralloc` (:893) | `len > blocksize` (string longer than the current block size) **and** `a->storage == NULL` | allocates a dedicated block, `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`; returns `sb->storage`. Next call must therefore re-block. | `err_22_stralloc_oversize_first` [x] |
| 23 | `stbds_stralloc` (:893,:896) | `len > blocksize` **and** `a->storage != NULL` | splices the new block in as `a->storage->next`; `a->remaining` **unchanged** | `err_23_stralloc_oversize_splice` [x] |
| 24 | `stbds_stralloc` (:888,:890) | `a->block` at/over the `BLOCKSIZE_MAX (1<<20)` ceiling — `512 << (block>>1) >= 1<<20` ⇔ `block >= 22`; also `block` up to `255` where `512 << 127` is a UB/masked shift | `a->block` stops incrementing once `blocksize >= 1<<20`; the masked-shift value must match byte-for-byte | `err_24_stralloc_block_ceiling` [x] |
| 25 | `stbds_stralloc` (:884) | empty string `""` (`len == 1`, the minimum) | consumes exactly 1 byte of `remaining` | `err_25_stralloc_empty_string` [x] |
| 26 | `stbds_strreset` (:923) | arena already empty / all-zero (`storage == NULL`) | no frees, arena zeroed; idempotent | `err_26_strreset_empty_idempotent` [x] |
| 27 | `stbds_hash_bytes` / `stbds_siphash_bytes` (:522,:532) | `len == 0` (and `p` pointing at a zero-size buffer) | the block loop never runs, `switch(0)` falls to `case 0: break`; returns the length-only digest | `err_27_hash_bytes_zero_len` [x] |
| 28 | `stbds_hash_bytes` (:531) | `len` with the top byte set (`len << (SIZE_T_BITS-8)`), e.g. `len` huge — length is folded into `data` | not exercised with a real huge buffer; instead all `len` in `0..=64` are compared exhaustively (all 8 `switch` fall-through arms × multi-block) | `cfg_01`–`cfg_03` [x] |
| 29 | `stbds_hash_string` (:480) | empty string `""` | loop body never executes; still mixes/returns `seed`-derived value | `err_29_hash_string_empty` [x] |
| 30 | `stbds_hash_*` (:596,:719) | computed `hash < 2` (collides with `STBDS_HASH_EMPTY`/`STBDS_HASH_DELETED`) | `hash += 2` fix-up must be applied identically in `hm_find_slot` **and** `hmput_key` | `err_30_hash_below_two_fixup` [x] |
| 31 | `stbds_shmode_func` (:803) | `mode` outside the `STBDS_SH_*` enum: `-1`, `4`, `255`, `256`, `INT_MIN`, `INT_MAX` — C enums accept any `int`, and the value is narrowed with `(unsigned char) mode` | `string.mode = (unsigned char) mode`; later `switch (table->string.mode)` in `hmput_key` hits `default:` → **`memcpy` of `keysize` bytes** instead of any string handling | `err_31_shmode_out_of_range` [x] |
| 32 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` (:560) | `mode` outside `{0,1}`: `2`, `3`, `-1`, `INT_MIN`, `INT_MAX`. `stbds_is_key_equal` branches on `mode >= STBDS_HM_STRING`, so negative modes are **binary** and every `mode >= 1` is **string** | negative/`INT_MIN` mode ⇒ binary `memcmp` path; `mode >= 1` ⇒ `strcmp`/`hash_string` path. `hmdel_key`'s strdup-free uses `mode == 1` exactly. | `err_32_mode_out_of_range` [x] |
| 33 | `stbds_hmput_key` (:707) | first insert with `mode >= STBDS_HM_STRING` on a fresh map | `nt->string.mode = STBDS_SH_DEFAULT (1)` ⇒ the key **pointer** is stored, not copied; with `mode < 1` it is `0` ⇒ `memcpy` | `err_33_put_string_mode_default` [x] |
| 34 | `stbds_hmput_default` (:669) | `a == NULL` **or** `header(arr)->length == 0` | allocates/extends and zeroes element 0, returns the `+elemsize` hash pointer; second call is a no-op returning the same pointer | `err_34_hmput_default_twice` [x] |
| 35 | `stbds_hmput_key` (:789) | `keysize == 0` with binary mode | `memcpy(dst, key, 0)` — no key bytes stored; every key then compares equal (`memcmp(.,.,0)==0`) so the map degenerates to one entry | `err_35_keysize_zero` [x] |
| 36 | `stbds_hmput_key` (:713) | `keysize` larger than `elemsize` (oversized key) | hashes/copies `keysize` bytes into an `elemsize` slot — out-of-bounds write, but must be *identical* out-of-bounds behaviour. Compared with `keysize == elemsize` (the boundary one step below overflow). | `err_36_keysize_equals_elemsize` [x] |
| 37 | `strkey` (:939) | `n` at the `int` extremes: `INT_MIN`, `INT_MAX`, `-1`, `0` | `sprintf(buffer,"test_%d",n)` → `"test_-2147483648"` etc.; buffer is a 256-byte static, never overflowed | `err_37_strkey_extremes` [x] |
| 38 | `intput` (:953) | `num == 9` — `hmput(intmap,9,7)` then `hmput(intmap,9,num)` overwrites value 7 with 9, so `assert(hmget(intmap,num)==7)` **fails** | `SIGABRT` (assertion `hmget(intmap, num) == 7` at line 955) | `err_38_intput_9_aborts` [x] |
| 39 | `intput` (:954) | `num == 11` — `hmput(intmap,11,7)` then `hmput(intmap,11,3)`, so `assert(hmget(intmap,num)==7)` **fails** | `SIGABRT` (assertion `hmget(intmap, num) == 7` at line 955) | `err_39_intput_11_aborts` [x] |
| 40 | `intput` | every other `int` (`INT_MIN`, `-1`, `0`, `8`, `10`, `12`, `INT_MAX`, random) | all three asserts hold → returns normally (leaks the map, as the C does) | `err_40_intput_ok_values` [x] |
| 41 | `stbds_arrfreef` (:314) | `a == NULL` → `free((header*)NULL - 1)` = `free((void*)-32)` | **`SIGSEGV` (signal 11)** in glibc's `free`, identically in both; measured in a forked child | `err_41_arrfreef_null_both_abort` [x] |
| 42 | `stbds_hmget_key_ts` (:638) | `temp == NULL` out-parameter | unconditional `*temp = ...` ⇒ NULL deref, `SIGSEGV` (11) in both | `err_42_hmget_ts_null_temp_both_crash` [x] |
| 43 | `stbds_arrgrowf` (:286) | `a == NULL` **and** `addlen == 0` **and** `min_cap == 0` ⇒ `min_cap (0) <= arrcap (0)` | returns its **NULL** argument without allocating — the one input for which `arrgrowf` yields NULL | `err_43_arrgrowf_returns_null` [x] |
| 44 | `stbds_hmdel_key` (:842) | `mode >= 2` (a "string" mode that is not exactly `STBDS_HM_STRING`) on a string map, deleting an element that is **not** the last one | the `mode == STBDS_HM_STRING` test at :842 is false, so the moved element is re-looked-up by passing the **address of the key field** instead of the key pointer; `find_slot` then hashes 8 pointer bytes as a string, fails, and `assert(slot >= 0)` fires ⇒ `SIGABRT` with `lib.c:846: stbds_hmdel_key: Assertion \`slot >= 0' failed.` | `err_44_hmdel_mode_two_abort` [x] |
| 45 | `stbds_stralloc` (:906) | `block` large enough that `512 << (block>>1)` (shift count masked to 6 bits) asks for a multi-terabyte block ⇒ `realloc` returns NULL ⇒ `sb->next = ...` dereferences NULL | `SIGSEGV` (11) in both | `err_24b_stralloc_block_alloc_failure` [x] |

## Boundary checklist (generic, required even where not a table row)

* NULL map pointer → rows 5, 8, 12, 34.
* NULL `temp` out-param → row 42.
* zero length / zero size → rows 4 (`elemsize=0`), 27 (`len=0`), 35 (`keysize=0`), 25 (`""`).
* one step past a valid range → rows 3 (`min_cap=3` vs floor 4), 24 (`block=21/22`),
  31/32 (`mode = 2` and `mode = -1`, one step outside `{0,1}` and `{0..3}`).
* out-of-range enum across FFI → rows 31, 32 (`int` modes `-1, 2, 3, 4, 255, 256, INT_MIN, INT_MAX`).

## Result

Every row above has a passing differential test. Run them with:

```sh
cd translation && cargo test --offline --test phase_c -- --test-threads=1
# 42 passed; 0 failed
```

Fatal-signal rows (16/17, 38, 39, 41, 42, 44, 45) are executed in forked child
processes; the harness compares the **signal number, the exit code and the
glibc diagnostic text** (file, line, function and the stringified expression),
not merely "both failed". Observed, byte-identical:

```
lib.c:955: intput: Assertion `hmget(intmap, num) == 7' failed.      (SIGABRT, both)
lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.           (SIGABRT, both)
<no diagnostic>                                                     (SIGSEGV, both)
```
