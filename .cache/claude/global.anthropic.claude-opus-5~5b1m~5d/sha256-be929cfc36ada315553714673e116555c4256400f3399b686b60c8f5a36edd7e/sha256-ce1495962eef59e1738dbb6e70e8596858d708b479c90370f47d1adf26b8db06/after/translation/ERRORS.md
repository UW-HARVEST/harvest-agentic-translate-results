# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping for every
`return`-with-sentinel, every `STBDS_ASSERT` (== `assert`), every explicit null
check, every range/threshold comparison, and every min/max constant.

`lib.c` has **no** error enum and **no** `RETURN_ERROR` macro. It rejects input
in exactly three ways:

* **S** — returns a *sentinel* (`NULL` / `-1` / `STBDS_INDEX_EMPTY` / unchanged
  pointer) and/or writes a sentinel into `header->temp` / `*temp`.
* **A** — `assert()` fails → glibc prints
  `<prog>: <file>:<line>: <func>: Assertion '<expr>' failed.` to stderr and
  raises `SIGABRT` (exit signal 6).
* **U** — undefined behaviour / memory fault (`SIGSEGV`) — the C code does *not*
  check, so the Rust must fault the same way. Tested only where the fault is
  deterministic.

Grep inventory (`lib.c` line numbers):

```
287  return a;                     (arrgrowf: no-grow early return)
401  STBDS_ASSERT(t->used_count_threshold + t->tombstone_count_threshold < t->slot_count)
573  if (a == NULL) return;        (hmfree_func null check)
596  if (hash < 2) hash += 2;      (reserved-hash clamp)
610  return -1;                    (hm_find_slot: empty slot in forward scan)
621  return -1;                    (hm_find_slot: empty slot in wrap scan)
638  *temp = STBDS_INDEX_EMPTY;    (hmget_key_ts: a == NULL)
645  *temp = -1;                   (hmget_key_ts: table == 0)
649  *temp = STBDS_INDEX_EMPTY;    (hmget_key_ts: slot < 0)
698  table == NULL || used_count >= used_count_threshold   (grow trigger)
719  if (hash < 2) hash += 2;
778  STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))
810  return 0;                     (hmdel_key: a == NULL -> NULL)
817  return a;                     (hmdel_key: table == 0, temp = 0)
822  return a;                     (hmdel_key: slot < 0, temp = 0)
828  STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)
832  STBDS_ASSERT(table->used_count >= 0)
846  STBDS_ASSERT(slot >= 0)
849  STBDS_ASSERT(b->index[i] == final_index)
854  used_count < used_count_shrink_threshold && slot_count > 8   (shrink)
858  tombstone_count > tombstone_count_threshold                  (rebuild)
878  STBDS_STRING_ARENA_BLOCKSIZE_MIN  512u
879  STBDS_STRING_ARENA_BLOCKSIZE_MAX  (1u<<20)
890  if (blocksize < MAX) ++a->block;   (block-index saturation)
913  STBDS_ASSERT(len <= a->remaining)
953  STBDS_ASSERT(hmget(intmap, 9) == num)
954  STBDS_ASSERT(hmget(intmap, 11) == 3)
955  STBDS_ASSERT(hmget(intmap, num) == 7)
```

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | kind | differential test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-------------------|-----|
| E1  | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap` (nothing to do) — e.g. `a` non-NULL cap 4, `addlen=0`, `min_cap=0` | returns `a` **unchanged** (same pointer, header untouched, no realloc) | S | `err_e1_arrgrowf_nogrow` | [x] |
| E2  | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` → `min_len = 0`, `0 > 0` false, then `min_cap(0) <= arrcap(NULL)(0)` **true** | returns `a` = **NULL**; nothing allocated (the floor-to-4 code is never reached) | S | `err_e2_arrgrowf_null_zero` | [x] |
| E3  | `stbds_arrgrowf` / `stbds_shmode_func` | `min_cap`/`elemsize` so large that `realloc` returns `NULL` (e.g. `arrgrowf(NULL, 1, 0, SIZE_MAX/2)`) | `b = NULL + 32`, so `stbds_header(b)` == address 0 and the `length`/`hash_table`/`temp`/`capacity` stores fault → `SIGSEGV` | U | `err_e3_arrgrowf_alloc_failure` (subprocess, signal compare) | [x] |
| E4  | `stbds_hmfree_func` | `a == NULL` | returns immediately, no free, no crash | S | `err_e4_hmfree_null` | [x] |
| E5  | `stbds_hmfree_func` | `a` non-NULL but `header->hash_table == NULL` (array made by `arrgrowf`, never `hmput`) | skips string-arena reset, `free(NULL)`, frees header; no crash | S | `err_e5_hmfree_no_table` | [x] |
| E6  | `stbds_hmget_key_ts` | `a == NULL` (map never created) | allocates 1-elem zeroed array, `*temp = -1` (`STBDS_INDEX_EMPTY`), returns hash-side ptr | S | `err_e6_hmget_ts_null` | [x] |
| E7  | `stbds_hmget_key_ts` | `a != NULL` but `header(raw_a)->hash_table == 0` (e.g. from `hmput_default` only) | `*temp = -1`, returns `a` unchanged | S | `err_e7_hmget_ts_no_table` | [x] |
| E8  | `stbds_hmget_key_ts` | key absent from a populated table (`hm_find_slot` hits `STBDS_HASH_EMPTY`, lines 610/621) | `*temp = -1`, returns `a` unchanged | S | `err_e8_hmget_ts_missing` | [x] |
| E9  | `stbds_hmget_key` | same three conditions as E6/E7/E8 | `header(raw)->temp` set to `-1` in every case | S | `err_e9_hmget_key_missing` | [x] |
| E10 | `stbds_hmget_key` / `stbds_hmput_key` | `keysize == 0` in binary mode → `memcmp(...,0)` always equal | first probed slot with matching hash compares equal | S | `err_e10_keysize_zero` | [x] |
| E11 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` (`0`) — caller's `t?stbds_temp(t-1):0` yields 0 | S | `err_e11_hmdel_null` | [x] |
| E12 | `stbds_hmdel_key` | `a != NULL`, `header(raw_a)->hash_table == 0` | sets `temp = 0`, returns `a` unchanged, length unchanged | S | `err_e12_hmdel_no_table` | [x] |
| E13 | `stbds_hmdel_key` | key absent (`hm_find_slot < 0`) | sets `temp = 0`, returns `a`, length/used_count unchanged | S | `err_e13_hmdel_missing` | [x] |
| E14 | `stbds_hmdel_key` | delete the *same* key twice | 1st: `temp=1`; 2nd: `temp=0` and no further length change | S | `err_e14_hmdel_twice` | [x] |
| E15 | `stbds_hmdel_key` | delete from a map of size 1 (`old_index == final_index`) | skips the memmove/re-find branch (so assert 846/849 unreachable), `length -= 1` | S | `err_e15_hmdel_last` | [x] |
| E16 | `stbds_hmdel_key` | `mode == 2` (`STBDS_HM_PTR_TO_STRING`, an out-of-range value for this TU which only `#define`s 0 and 1) | hashing uses the *string* path (`mode >= 1`) but the strdup-free and the re-find key deref use `mode == 1`, so the re-find hashes the **raw slot bytes as a C string** | S / U | `err_e16_mode2_hmdel` | [x] |
| E17 | `stbds_hmput_key` / `stbds_hmget_key` | negative `mode` (e.g. `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING` false → binary path, identical to `mode == 0` | S | `err_e17_negative_mode` | [x] |
| E18 | `stbds_hmput_key` / `stbds_hmget_key` | large `mode` (e.g. `7`, `1000`, `INT_MAX`) | `mode >= STBDS_HM_STRING` true → string path, identical to `mode == 1` for put/get | S | `err_e18_large_mode` | [x] |
| E19 | `stbds_shmode_func` | `mode` outside `{0,1,2,3}`: `4`, `256`, `-1`, `INT_MAX`, `INT_MIN` | `(unsigned char) mode` truncation: `256→0`, `-1→255`, `INT_MAX→255`, `INT_MIN→0`, `4→4`; later `switch(string.mode)` falls to `default:` → `memcpy(key, keysize)` instead of storing a `char*` | S | `err_e19_shmode_out_of_range` | [x] |
| E20 | `stbds_shmode_func` | `mode == 0` (`STBDS_SH_NONE`) with a *string* `hmput_key(mode=1)` | `string.mode == 0` → `default:` branch → key bytes memcpy'd, **not** strdup'd | S | `err_e20_shmode_none_string_put` | [x] |
| E21 | `stbds_hmput_default` | `a == NULL` | allocates 1 zeroed element, returns hash-side ptr, `length == 1`, `hash_table == NULL` | S | `err_e21_hmput_default_null` | [x] |
| E22 | `stbds_hmput_default` | `a != NULL` and `header(raw)->length == 0` (array grown by `arrgrowf` but empty) | grows/keeps buffer, `length += 1`, zeroes element 0 | S | `err_e22_hmput_default_len0` | [x] |
| E23 | `stbds_hmput_default` | called twice (`length != 0` second time) | second call is a no-op, returns the same pointer | S | `err_e23_hmput_default_twice` | [x] |
| E24 | `stbds_hash_bytes` | `len == 0` (with `p == NULL`) | no dereference of `p`; returns the finalised siphash of `data = 0` | S | `err_e24_hash_bytes_zero_len` | [x] |
| E25 | `stbds_hash_bytes` | `len` not a multiple of 8 → `switch (len - i)` fall-through cases 7…1 | tail bytes folded with the documented shift pattern (incl. the `(d[3]<<24)` signed-int-promotion sign extension for cases 4/3/2/1) | S | `err_e25_hash_bytes_tail` | [x] |
| E26 | `stbds_hash_bytes` / `stbds_hash_string` | hash result `< 2` (reserved values 0=EMPTY, 1=DELETED) | callers clamp with `if (hash < 2) hash += 2` (lines 596, 719) — key still findable | S | `err_e26_hash_lt_2_clamp` | [x] |
| E27 | `stbds_hash_string` | empty string `""` | loop body never runs; avalanche applied to `seed` alone | S | `err_e27_hash_string_empty` | [x] |
| E28 | `stbds_hash_string` | `str == NULL` | dereferences NULL → `SIGSEGV` | U | `err_e28_hash_string_null` (subprocess, signal compare) | [x] |
| E29 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` (huge string, e.g. 4096 bytes with `block == 0` → blocksize 512) | allocates an oversized dedicated block, returns `sb->storage`; if `a->storage == NULL` also sets `a->remaining = 0` | S | `err_e29_stralloc_huge` | [x] |
| E30 | `stbds_stralloc` | `a->block` saturation: repeated blocks until `blocksize >= 1<<20` | `++a->block` stops once `512 << (block>>1) >= 1<<20`, i.e. `block` saturates at **22** (`512<<11 == 1<<20`) | S | `err_e30_stralloc_block_cap` | [x] |
| E31 | `stbds_stralloc` | assert `len <= a->remaining` (lib.c:913). **Proved unreachable:** the only way to reach line 913 with `len > remaining` is via the `len <= blocksize` sub-branch, which sets `remaining = blocksize >= len`; the `len > blocksize` sub-branch `return`s first. The nearest reachable *out-of-range* input is instead an arena whose `block` field is out of the range the library itself produces (`block > 109`), making `512u << (block>>1)` shift by ≥ 64 → `blocksize` wraps to 0 → the `len > blocksize` (dedicated-block) branch is taken for every string | both libs must pick the same branch and return the same string/arena state | S | `err_e31_stralloc_block_shift_overflow` | [x] |
| E32 | `stbds_stralloc` | empty string `""` (`len == 1`) | 1 byte consumed from the current block | S | `err_e32_stralloc_empty_string` | [x] |
| E33 | `stbds_strreset` | `a->storage == NULL` (already-reset / zeroed arena) | while-loop skipped, whole struct memset to 0 | S | `err_e33_strreset_empty` | [x] |
| E34 | `stbds_strreset` | called twice | second call is a no-op-with-memset, no double free | S | `err_e34_strreset_twice` | [x] |
| E35 | `stbds_make_hash_index` (via `stbds_hmput_key`) | assert lib.c:401 — `slot_count ∈ {0,1,2}` makes `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` → stderr `…lib.c:401: stbds_make_hash_index: Assertion 't->used_count_threshold + t->tombstone_count_threshold < t->slot_count' failed.` + `SIGABRT`. Reached by handing `hmput_key` a header whose `hash_table` points at a hand-built `stbds_hash_index` with `slot_count == 1` (doubling → 2). | A | `err_e35_make_hash_index_assert` (subprocess) | [x] |
| E36 | `stbds_hmput_key` | assert lib.c:778 `(size_t) i+1 <= stbds_arrcap(a)` — unreachable after the preceding `arrgrowf`, verified by exercising the grow boundary (`i+1 == cap` and `i+1 == cap+1`) | never fires; both libs proceed | A/S | `err_e36_hmput_grow_boundary` | [x] |
| E37 | `stbds_hmdel_key` | asserts lib.c:828 / 832 / 846 / 849 — structurally unreachable from a consistent table; verified by exhaustive random delete sequences (any divergence would trip one of them) | never fires | A/S | covered by `cfg_*` delete rows + `err_e37_hmdel_assert_free` | [x] |
| E38 | `intput` | `num == 9` — `hmput(intmap,9,7)` then `hmput(intmap,9,num=9)` overwrites value to 9, so `hmget(intmap,num) == 7` is false | stderr `…lib.c:955: intput: Assertion 'hmget(intmap, num) == 7' failed.` + `SIGABRT` | A | `err_e38_intput_9` (subprocess) | [x] |
| E39 | `intput` | `num == 11` — `hmput(intmap,11,7)` then `hmput(intmap,11,3)`, so `hmget(intmap,num) == 7` is false | stderr `…lib.c:955: intput: Assertion 'hmget(intmap, num) == 7' failed.` + `SIGABRT` | A | `err_e39_intput_11` (subprocess) | [x] |
| E40 | `intput` | any `num ∉ {9,11}` incl. `0`, `INT_MIN`, `INT_MAX`, `-1` | all three asserts hold, returns normally (leaks the map, as C does) | S | `err_e40_intput_ok` | [x] |
| E41 | `strkey` | `n == INT_MIN` (`sprintf("test_%d")` on the most-negative int) | `"test_-2147483648"` in the shared `static char buffer[256]` | S | `err_e41_strkey_int_min` | [x] |
| E42 | `strkey` | called twice — returns the *same* static buffer, so the first result is clobbered | both pointers equal; contents = last call | S | `err_e42_strkey_static_reuse` | [x] |
| E43 | `stbds_arrfreef` | `a == NULL` → `free((char*)NULL - 32)` | glibc faults reading the chunk header → `SIGSEGV` | U | `err_e43_arrfreef_null` (subprocess, signal compare) | [x] |
| E44 | `stbds_hmput_key` | `mode >= 1` (string) but `string.mode == STBDS_SH_DEFAULT` and the *same* `char*` key pointer re-put | existing-key path taken, `temp_key` set to the stored pointer, value slot reused (no new element) | S | `err_e44_shput_same_key` | [x] |
| E45 | `stbds_hmdel_key` | `keyoffset != 0` (e.g. key is the 2nd field of the element struct) | `is_key_equal` / re-find use `elem + keyoffset`; delete still succeeds | S | `err_e45_keyoffset_nonzero` | [x] |
| E46 | `stbds_hmget_key_ts` | `temp` points to caller storage; `mode` string with `table == 0` | `*temp = -1` written, `header->temp` **not** touched (unlike `hmget_key`) | S | `err_e46_hmget_ts_leaves_header_temp` | [x] |
| E47 | `stbds_hash_bytes` | `p == NULL` with `len > 0` (`len ∈ {1,4,7,8,9,16,1000}` covers both the full-block loop and every tail case) | dereferences `p` → `SIGSEGV` | U | `err_e47_hash_bytes_null_nonzero_len` (subprocess) | [x] |
| E48 | `stbds_stralloc` | `a == NULL` | reads `a->remaining` → `SIGSEGV` | U | `err_e48_stralloc_null_arena` (subprocess) | [x] |
| E49 | `stbds_stralloc` | `str == NULL` | `strlen(NULL)` → `SIGSEGV` | U | `err_e49_stralloc_null_str` (subprocess) | [x] |
| E50 | `stbds_strreset` | `a == NULL` | reads `a->storage` → `SIGSEGV` | U | `err_e50_strreset_null` (subprocess) | [x] |
| E51 | `stbds_hmget_key_ts` | `temp == NULL` | `*temp = STBDS_INDEX_EMPTY` → `SIGSEGV` | U | `err_e51_hmget_ts_null_temp` (subprocess) | [x] |
| E52 | `stbds_hmput_key` | `key == NULL` on a populated map | `stbds_hash_bytes(NULL, keysize, seed)` → `SIGSEGV` | U | `err_e52_e54_null_key` (subprocess) | [x] |
| E53 | `stbds_hmdel_key` | `key == NULL` on a populated map | `stbds_hm_find_slot` → `stbds_hash_bytes(NULL, …)` → `SIGSEGV` | U | `err_e52_e54_null_key` (subprocess) | [x] |
| E54 | `stbds_hmget_key` | `key == NULL` on a populated map | `stbds_hash_bytes(NULL, …)` → `SIGSEGV` | U | `err_e52_e54_null_key` (subprocess) | [x] |

## Notes on faithful reproduction of the UB rows

`lib.c` never null-checks a pointer parameter, so rows E28 and E47-E54 are
undefined behaviour that faults with `SIGSEGV` in practice.  A plain `*p` in Rust
would instead trip rustc's `"null pointer dereference occurred"` UB check and
abort with `SIGABRT` whenever the crate is compiled with debug assertions, which
made the observable failure mode profile-dependent.  `src/lib.rs` therefore
performs those specific FFI-boundary loads/stores with
`ptr::read_volatile` / `ptr::write_volatile`, which emit the same machine access
without the inserted check.  The subprocess tests assert `signal == SIGSEGV (11)`
for **both** libraries, in **both** the dev and release profiles.
