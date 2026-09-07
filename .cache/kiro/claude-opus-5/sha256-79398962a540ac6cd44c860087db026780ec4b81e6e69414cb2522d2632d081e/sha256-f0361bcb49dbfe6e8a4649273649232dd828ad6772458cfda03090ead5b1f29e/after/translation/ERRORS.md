# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/lib.c`. Greps used:

```
grep -n "STBDS_ASSERT\|assert("            lib.c
grep -n "return 0;\|return NULL\|return -1" lib.c
grep -n "== NULL\|!= NULL\|== 0)\|a ?"      lib.c
grep -n "mode >=\|mode ==\|hash < 2\|< 0"   lib.c
```

`stb_ds` has **no error enum and no error-return macro**. Its entire rejection
surface consists of (a) NULL-pointer early-outs, (b) the `-1` /
`STBDS_INDEX_EMPTY` "not found" sentinel, (c) the `0` / `NULL` return from
`stbds_hmdel_key`, (d) reserved-hash-value fixups, (e) out-of-domain `mode`
ints, and (f) live `assert()`s (the CMake build defines no `NDEBUG`; the C `.so`
imports `__assert_fail`).

`aN` below = "array pointer as returned by a previous call"; `hN` = "hash-map
pointer (element pointer, i.e. `raw + elemsize`)".

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|--------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` | `a == NULL` (nothing to grow from) | fresh alloc; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap,addlen,4)` | `err_01_arrgrowf_null` |
| 2 | `stbds_arrgrowf` | `min_cap <= arrcap(a)` — request already satisfied | returns **the same pointer `a`**, unchanged (no realloc) | `err_02_arrgrowf_noop` |
| 3 | `stbds_arrgrowf` | `a == NULL` with `addlen == 0 && min_cap == 0` | `min_cap (0) <= arrcap(NULL) (0)` ⇒ **returns NULL without allocating** | `err_03_arrgrowf_zero_zero` |
| 4 | `stbds_arrgrowf` | `elemsize == 0` (degenerate element), `min_cap >= 1` | allocates `sizeof(header)` only; `capacity=4` | `err_04_arrgrowf_elemsize0` |
| 5 | `stbds_hmfree_func` | `a == NULL` | early `return;` — no crash, no free | `err_05_hmfree_null` |
| 6 | `stbds_hmfree_func` | `a != NULL` but `header(a)->hash_table == NULL` | skips key-free + `strreset`; still frees `hash_table` (NULL) and header | `err_06_hmfree_no_table` |
| 7 | `stbds_hm_find_slot` (via `stbds_hmget_key`) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` in the `i = pos&MASK .. 7` loop → key absent | `slot = -1` ⇒ `header(raw)->temp == -1` | `err_07_hmget_absent` |
| 8 | `stbds_hm_find_slot` (wrap-around loop) | probe hits `hash[i] == STBDS_HASH_EMPTY` in the `0 .. limit` loop → key absent | `slot = -1` ⇒ `temp == -1` | `err_08_hmget_absent_wrap` |
| 9 | `stbds_hmget_key_ts` | `a == NULL` | allocates a 1-element array, `length=1`, zeroed; `*temp = STBDS_INDEX_EMPTY (-1)`; returns `raw+elemsize` | `err_09_hmget_ts_null` |
| 10 | `stbds_hmget_key_ts` | `a != NULL` but `hash_table == NULL` (map made by `hmput_default` only) | `*temp = -1`, returns `a` unchanged | `err_10_hmget_ts_no_table` |
| 11 | `stbds_hmget_key_ts` | key not present (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err_11_hmget_ts_absent` |
| 12 | `stbds_hmget_key` | `a == NULL` | same as #9 plus `header(raw)->temp = -1` | `err_12_hmget_null` |
| 13 | `stbds_hmput_default` | `a == NULL` | allocates 1 zeroed element, `length=1` | `err_13_hmput_default_null` |
| 14 | `stbds_hmput_default` | `a != NULL` **and** `header(raw)->length == 0` | re-grows and bumps `length` to 1 again | `err_14_hmput_default_len0` |
| 15 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (no alloc) | `err_15_hmput_default_noop` |
| 16 | `stbds_hmput_key` | `a == NULL` | bootstraps a 1-element array before inserting | `err_16_hmput_null` |
| 17 | `stbds_hmput_key` | `hash_table == NULL` | builds an 8-slot index; `string.mode = (mode>=1 ? SH_DEFAULT : 0)` | `err_17_hmput_no_table` |
| 18 | `stbds_hmdel_key` | `a == NULL` | `return 0` (**NULL**) — the only pointer-NULL return in the library | `err_18_hmdel_null` |
| 19 | `stbds_hmdel_key` | `a != NULL`, `hash_table == NULL` | `temp` set to `0`, returns `a` unchanged, nothing deleted | `err_19_hmdel_no_table` |
| 20 | `stbds_hmdel_key` | key absent (`stbds_hm_find_slot < 0`) | `temp == 0`, returns `a`, `length` unchanged | `err_20_hmdel_absent` |
| 21 | `stbds_hmdel_key` | delete the *same* key twice | 2nd call: `temp == 0` (reject), length unchanged | `err_21_hmdel_twice` |
| 22 | `stbds_hm_find_slot` / `stbds_hmput_key` | computed `hash < 2` (0 and 1 are the reserved `HASH_EMPTY` / `HASH_DELETED` markers) | `hash += 2` — value silently remapped, never stored as-is | `err_22_reserved_hash` |
| 23 | `stbds_hmput_key` | `mode` out of the `{0,1}` enum domain, e.g. `2`, `7`, `INT_MAX` | `mode >= STBDS_HM_STRING` ⇒ treated as **string** mode | `err_23_mode_out_of_range_hi` |
| 24 | `stbds_hmput_key` | `mode` negative, e.g. `-1`, `INT_MIN` | `mode >= 1` false ⇒ treated as **binary** mode | `err_24_mode_negative` |
| 25 | `stbds_hmdel_key` | `mode == 2` (not exactly `STBDS_HM_STRING`) on a `SH_STRDUP` map, deleting the **last** element | `mode == STBDS_HM_STRING` false ⇒ key **not** freed; `mode >= 1` in `find_slot` ⇒ string compare, delete succeeds (`temp == 1`) | `err_25_hmdel_mode2_last_element` |
| 25b | `stbds_hmdel_key` | `mode == 2` deleting a **non-final** element on a string map: the re-find takes the `else` branch and string-hashes the *address* of the key field ⇒ `slot == -1` | live `assert(slot >= 0)` at lib.c:846 ⇒ `__assert_fail` ⇒ **SIGABRT** (exit 134). Both libraries must die identically | `err_25b_hmdel_mode2_middle_aborts` (forked) |
| 26 | `stbds_shmode_func` | `mode` outside `{0,1,2,3}` — truncated by `(unsigned char) mode` | `string.mode = mode & 0xFF`; e.g. `mode=256` ⇒ `0`, `mode=259` ⇒ `3` (arena!) | `err_26_shmode_out_of_range` |
| 27 | `stbds_hmput_key` | `table->string.mode` not in `{SH_STRDUP,SH_ARENA,SH_DEFAULT}` (e.g. 7 via #26) | `switch` `default:` ⇒ `memcpy(elem, key, keysize)` — key bytes copied, not pointer | `err_27_string_mode_default` |
| 28 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` **and** `a->storage == NULL` | oversized block: `sb->next = NULL`, `a->storage = sb`, `a->remaining = 0` | `err_28_stralloc_big_first` |
| 29 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` **and** `a->storage != NULL` | oversized block spliced in *after* head; `a->remaining` **left untouched** | `err_29_stralloc_big_after` |
| 30 | `stbds_stralloc` | `a->block` already at saturation (`512<<(block>>1) >= 1<<20`) | `a->block` no longer incremented | `err_30_stralloc_block_saturate` |
| 31 | `stbds_stralloc` | empty string `""` (`len == 1`) | still consumes 1 byte of arena; returns pointer to a `'\0'` | `err_31_stralloc_empty` |
| 32 | `stbds_strreset` | arena already zeroed / never used (`storage == NULL`) | loop body never runs; memset to 0 — no crash, no double free | `err_32_strreset_fresh` |
| 33 | `stbds_strreset` | called twice in a row | second call is a no-op (arena was zeroed) | `err_33_strreset_twice` |
| 34 | `stbds_hash_string` | empty string `""` | loop skipped; returns the pure-seed mix (deterministic constant per seed) | `err_34_hash_string_empty` |
| 35 | `stbds_hash_bytes` | `len == 0` | main loop skipped, `switch(0)` falls to `case 0: break` — `data = 0` | `err_35_hash_bytes_len0` |
| 36 | `stbds_hash_bytes` | `len` in `1..7` (short tail, `switch` fall-through, sign-extending `d[3]<<24`) | tail assembled with C's implementation-defined sign extension | `err_36_hash_bytes_tail` |
| 37 | `stbds_hash_bytes` | high bytes `>= 0x80` at `d[3]` / `d[7]` (int overflow / sign extension in the loader) | negative `int` sign-extends into `size_t` | `err_37_hash_bytes_signext` |
| 38 | `stbds_hash_string` / `stbds_hash_bytes` | `seed == 0`, `seed == SIZE_MAX` (boundary seeds) | no special case — plain arithmetic | `err_38_hash_seed_bounds` |
| 39 | `strkey` | `n < 0` (e.g. `-1`) — `sprintf("test_%d")` | `"test_-1"` | `err_39_strkey_negative` |
| 40 | `strkey` | `n == INT_MIN` (`-2147483648`, magnitude not representable in `int`) | `"test_-2147483648"` | `err_40_strkey_int_min` |
| 41 | `sh_puts` | `num <= 0` (`0`, `-1`, `INT_MIN`) | `for (i=0;i<num;++i)` never runs; still prints `a <num>` | `err_41_sh_puts_nonpositive` |
| 42 | `assert` @ lib.c:401 | `used_count_threshold + tombstone_count_threshold < slot_count` | not reachable through the public API: `stbds_make_hash_index` is only ever called with `slot_count >= 8` (`BUCKET_LENGTH`, `*2`, or `>>1` of something `> 8`) | `err_42_46_asserts_do_not_fire_on_valid_input` (forked, must exit 0) |
| 43 | `assert` @ lib.c:778 | `(size_t)i+1 <= arrcap(a)` after the grow | not reachable: `arrgrowf` guarantees it | same |
| 44 | `assert` @ lib.c:828/832/846/849 | slot / used_count / re-find invariants in `hmdel_key` | hold on well-formed maps; 846 IS reachable via out-of-domain `mode` (row 25b) | same + `err_25b` |
| 45 | `assert` @ lib.c:913 | `len <= a->remaining` in `stralloc` | not reachable: every path either returns early or sets `remaining = blocksize >= len` | same |
| 46 | `assert` @ lib.c:959–961 | `*strmap[0].key=='a'`, `key != s.key`, `value == s.value` | hold for every `num` | same + `cfg_54..57` |
| 47 | `stbds_arrfreef` | `a == NULL` → `free((header*)NULL - 1)` = `free((void*)-32)` | **undefined behaviour / abort in both** — no NULL guard exists in the C. Not tested (would abort the harness); recorded for completeness | n/a (UB, excluded) |

All ten `STBDS_ASSERT`s are **implemented in the Rust translation** (they were
previously only comments): `translation/src/lib.rs` declares glibc
`__assert_fail` and calls it with the same assertion text, C line number and
enclosing function name, so a failing assertion aborts with SIGABRT in both
libraries. Verified: `err_25b` shows both children terminating with
`Signalled(6)` and the identical message
``…lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.`` (only the
`__FILE__` path prefix differs, since that is the C build's absolute source
path).

## Generic FFI boundary cases also covered

| case | where |
|------|-------|
| NULL `a` into every pointer-taking entry point | #1, #5, #9, #12, #13, #16, #18 |
| zero length / zero size | #3, #4, #31, #35 |
| one past a valid enum value (`mode = 2`, `-1`, `256`, `INT_MIN`, `INT_MAX`) | #23, #24, #25, #26, #27 |
| reserved sentinel values (`hash` 0/1, index `-1`/`-2`) | #7, #8, #11, #19, #22 |
| `INT_MIN` / `INT_MAX` scalars | #40, #41 |
| `SIZE_MAX` seed | #38 |

## Status

All 48 rows (1–47 plus 25b) have a passing differential test in
`translation/tests/phase_c_errors.rs` (42 `#[test]` functions; several rows share
one function where they are two arms of the same check, e.g. rows 7+8 and rows
42–46). Row 47 is the single exception: `stbds_arrfreef(NULL)` computes
`free((void *) -32)`, which is undefined behaviour in the C itself, so it is
recorded rather than tested.

Verified by:

```
cd translation && cargo test --test phase_c_errors -- --test-threads=1
# 42 passed; 0 failed
```
