# Phase A.2 — ERROR-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c`. The library has **no error enum and no
`RETURN_ERROR` macro**: it rejects input by (a) returning early with a sentinel
(`NULL` / unchanged pointer / `temp = -1` / `temp = 0`), (b) failing a live
`assert()` (the C `.so` imports `__assert_fail`, `NDEBUG` is *not* defined — verified
with `nm -D`), or (c) dereferencing the bad pointer (crash). Every such site in the
file is one row below.

Grep basis:

```sh
grep -n 'return\|assert\|STBDS_ASSERT\|== *NULL\|!= *NULL\|== *0\|< *0\|>= *STBDS_HM\|switch' c_src/src/lib.c
```

Legend for "expected C result": `temp` = `stbds_header(...)->temp`, i.e. what the
`hmgeti`/`hmdel`/`hmput` macros yield.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` (lib.c:286) | `min_cap <= stbds_arrcap(a)` after clamping to `arrlen(a)+addlen` — growth request that is already satisfied | returns `a` **unchanged** (same pointer, header untouched); with `a == NULL` and `min_cap == 0` returns `NULL` | `err_01_arrgrowf_noop` |
| 2 | `stbds_arrgrowf` (lib.c:297) | `realloc` returns `NULL` because the request is unsatisfiable (`elemsize = 1<<50`, `min_cap` clamped to 4 ⇒ 4 PiB) | `b = NULL+32`, then `stbds_header(b)->length = 0` writes to address 0 ⇒ **SIGSEGV** | `crash…::arrgrowf_oom` |
| 3 | `stbds_arrgrowf` (lib.c:280) | `elemsize == 0` (degenerate element size) | no error: allocates `min_cap = 4`, header only | `err_03_arrgrowf_zero_elemsize` |
| 4 | `stbds_arrfreef` (lib.c:314) | `a == NULL` (no NULL check in the C) | `free((char*)NULL - 32)` ⇒ glibc walks the wild chunk header ⇒ **SIGSEGV** (observed: signal 11 for both) | `crash_parity_all_scenarios::arrfreef_null` |
| 5 | `stbds_hash_string` (lib.c:480) | `str == NULL` | dereferences `*str` ⇒ **SIGSEGV** | `crash…::hash_string_null` |
| 6 | `stbds_hash_string` | `str` = `""` (empty string, zero-length input) | no loop iteration; returns the seed-mixing of `hash = seed` | `err_06_hash_string_empty` |
| 7 | `stbds_hash_bytes` (lib.c:522/532) | `len == 0` **with `p == NULL`** | `p` is never dereferenced ⇒ well-defined result (`data = 0`) | `err_07_hash_bytes_null_len0` |
| 8 | `stbds_hash_bytes` | `len == 1..7` (short/odd length, `switch (len-i)` fall-through) | partial-word mix, no read past `len` | `err_08_hash_bytes_short` |
| 9 | `stbds_is_key_equal` (lib.c:560) | `mode` outside `{0,1}`: any `mode >= 1` selects `strcmp`, any `mode < 0` selects `memcmp` | out-of-range enum ⇒ string path for `mode = 2,3,7,INT_MAX`; binary path for `mode = -1,INT_MIN` | `err_09_mode_out_of_range` |
| 10 | `stbds_is_key_equal` (lib.c:563) | `keysize == 0` in binary mode | `memcmp(...,0) == 0` ⇒ *every* key compares equal | `err_10_keysize_zero` |
| 11 | `stbds_hmfree_func` (lib.c:573) | `a == NULL` | returns immediately, no free | `err_11_hmfree_null` |
| 12 | `stbds_hm_find_slot` (lib.c:609/620) | probe reaches a slot with `hash == STBDS_HASH_EMPTY (0)` — key absent | returns `-1` | `err_12_15_16_find_slot_miss` |
| 13 | `stbds_hmget_key_ts` (lib.c:634) | `a == NULL` | allocates a 1-element array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL map | `err_13_14_get_ts_sentinels` |
| 14 | `stbds_hmget_key_ts` (lib.c:644) | `a != NULL` but `hash_table == NULL` (map created by `stbds_hmput_default`, never `put`) | `*temp = -1`, returns `a` unchanged | `err_13_14_get_ts_sentinels` |
| 15 | `stbds_hmget_key_ts` (lib.c:648) | key not present (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err_12_15_16_find_slot_miss` |
| 16 | `stbds_hmget_key` (lib.c:663) | same three cases, through the non-`_ts` wrapper | `stbds_header(t-1)->temp = -1` (what `hmgeti`/`shgeti` return; `hmgetp_null` ⇒ `NULL`) | `err_12_15_16_find_slot_miss` |
| 17 | `stbds_hmput_default` (lib.c:669) | `a == NULL` **or** `length == 0` | allocates + zeroes element 0, returns hash-side pointer | `err_17_18_put_default` |
| 18 | `stbds_hmput_default` (lib.c:669) | already-initialised map (`length != 0`) | returns `a` **unchanged**, no allocation | `err_17_18_put_default` |
| 19 | `stbds_hmput_key` (lib.c:789) | `table->string.mode` not in `{1,2,3}` (e.g. `0`, or `4..255` produced by `stbds_shmode_func` truncation) | `switch` `default:` ⇒ `memcpy(elem, key, keysize)` — key bytes copied even though `mode >= STBDS_HM_STRING` hashed it as a string | `err_19_smode_default_branch` |
| 20 | `stbds_make_hash_index` (lib.c:401) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` — `slot_count < 8` or `0` | **SIGABRT** (unreachable through the public API: `slot_count` is always `8<<k`) | `err_20_to_25_and_31_documented_unreachable` |
| 21 | `stbds_hmput_key` (lib.c:778) | `assert((size_t)i+1 <= stbds_arrcap(a))` | **SIGABRT** (unreachable: the preceding `if` grows the array) | `err_20_to_25_and_31_documented_unreachable` |
| 22 | `stbds_hmdel_key` (lib.c:828) | `assert(slot < (ptrdiff_t)table->slot_count)` | **SIGABRT** (unreachable: `find_slot` masks with `slot_count-1`) | `err_20_to_25_and_31_documented_unreachable` |
| 23 | `stbds_hmdel_key` (lib.c:832) | `assert(table->used_count >= 0)` on a `size_t` | folded to `true` by the compiler (no string in the `.so`), never fires | `err_20_to_25_and_31_documented_unreachable` |
| 24 | `stbds_hmdel_key` (lib.c:846) | `assert(slot >= 0)` — the *moved* last element cannot be found again. Reachable: `mode >= 2` (`STBDS_HM_PTR_TO_STRING`) on a `string.mode == DEFAULT/STRDUP/ARENA` map deletes a non-last entry: the re-lookup at lib.c:845 passes the *address of* the key pointer (because the guard is `mode == 1`, not `mode >= 1`) while `find_slot` string-hashes it | **SIGABRT** | `crash…::hmdel_ptr_to_string` |
| 25 | `stbds_hmdel_key` (lib.c:849) | `assert(b->index[i] == final_index)` | **SIGABRT** (only reachable together with row 24) | `err_20_to_25_and_31_documented_unreachable` |
| 26 | `stbds_hmdel_key` (lib.c:809) | `a == NULL` | returns `NULL` (`0`), the `hmdel` macro then yields `0` | `err_26_del_null_map` |
| 27 | `stbds_hmdel_key` (lib.c:816) | `hash_table == NULL` | `temp = 0`, returns `a` unchanged | `err_27_28_del_no_table_and_miss` |
| 28 | `stbds_hmdel_key` (lib.c:821) | key not present | `temp = 0`, returns `a` unchanged (`hmdel` ⇒ `0` = "nothing deleted") | `err_27_28_del_no_table_and_miss` |
| 29 | `stbds_hmdel_key` (lib.c:836) | `mode == 2` (not `== STBDS_HM_STRING`) on a `STBDS_SH_STRDUP` map | the strdup'ed key is **not** freed (leak); still `temp = 1` | `err_29_del_mode2_no_free` |
| 30 | `stbds_hmdel_key` (lib.c:807) | `keyoffset` out of range (e.g. `keyoffset = elemsize`, reading the neighbouring element) | no check: hashes the wrong bytes ⇒ `find_slot` misses ⇒ `temp = 0`, map unchanged | `err_30_del_bad_keyoffset` |
| 31 | `stbds_stralloc` (lib.c:913) | `assert(len <= a->remaining)` | **SIGABRT** (unreachable: the `len > blocksize` branch returns early) | `err_20_to_25_and_31_documented_unreachable` |
| 32 | `stbds_stralloc` (lib.c:893) | oversized string: `strlen+1 > blocksize` | dedicated block, spliced *after* the head block; `remaining` untouched (or set to 0 when the arena was empty) | `err_32_stralloc_oversized` |
| 33 | `stbds_stralloc` (lib.c:885) | `a` all-zero arena but `a->remaining` forged `>= len` with `storage == NULL` | `a->storage->storage` dereferences NULL ⇒ **SIGSEGV** | `crash…::stralloc_forged_arena` |
| 34 | `stbds_stralloc` (lib.c:888) | `a->block` out of range (`block >= 128` ⇒ shift count `>= 64`, C UB / x86 masks it) | `blocksize = 512 << (block>>1 & 63)`; both libraries must agree | `err_34_41_out_of_range_bytes`, `cfg_46b…` |
| 35 | `stbds_stralloc` (lib.c:884) | `str == NULL` | `strlen(NULL)` ⇒ **SIGSEGV** | `crash…::stralloc_null_str` |
| 36 | `stbds_strreset` (lib.c:924) | all-zero arena (`storage == NULL`) — the empty case | loop body never runs, arena zeroed, returns | `err_36_strreset_empty` |
| 37 | `stbds_strreset` | `a == NULL` | `a->storage` read from address 0 ⇒ **SIGSEGV** | `crash…::strreset_null` |
| 38 | `strkey` (lib.c:941) | `INT_MIN` / `INT_MAX` (widest `%d` expansion, 17 bytes incl. NUL) | fits the 256-byte static buffer, no overflow | `err_38_strkey_extremes` |
| 39 | `helxo` (lib.h) | `letter` = `0`, `0x80..0xFF` (negative `char`), `'\n'` | printed verbatim by `%c`; byte-identical stdout required | `cfg_49_50_helxo` |
| 40 | `stbds_hmput_key` / `stbds_hmget_key` (lib.c:596/719) | key whose `stbds_hash_*` result is `0` or `1` (the reserved EMPTY/DELETED hashes) | `if (hash < 2) hash += 2;` — remapped, no error | `err_40_reserved_hash_values` |
| 41 | `stbds_shmode_func` (lib.c:803) | `mode` out of the `STBDS_SH_*` enum range, e.g. `-1`, `256`, `INT_MAX` | `(unsigned char)mode` truncation ⇒ `string.mode = 255/0/255`; subsequent `put` takes the binary `default:` branch | `err_34_41_out_of_range_bytes` |
| 42 | `stbds_hmput_key` (lib.c:686) | `a == NULL` (implicit map creation) + `elemsize == 0` (degenerate) | `ARR_TO_HASH == HASH_TO_ARR == a`; insert still succeeds with `temp = 0`, `memcpy(...,0)` | `err_42_put_zero_elemsize` |

### Generic FFI-boundary rows (every C API has them, checked even though the C has no explicit test for them)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 43 | `stbds_arrgrowf` | `elemsize = 1<<62`, `min_cap = 1` ⇒ `elemsize*4 + 32` **wraps** to 32 | *no* crash: a 32-byte allocation, `capacity = 4`, `length = 0` | `crash…::arrgrowf_size_overflow` |
| 44 | `stbds_hash_bytes` | `p == NULL` with `len = 1` (non-zero length) | reads `d[0]` ⇒ **SIGSEGV** | `crash…::hash_bytes_null_len1` |
| 45 | `stbds_hash_bytes` | oversized `len` (`usize::MAX/2`) over a 64-byte buffer | walks off the buffer ⇒ **SIGSEGV** | `crash…::hash_bytes_huge_len` |
| 46 | `stbds_hmput_key` | `key == NULL`, `mode = 0` ⇒ `stbds_hash_bytes(NULL, 8, …)` | **SIGSEGV** | `crash…::hmput_key_null_key_bin` |
| 47 | `stbds_hmput_key` | `key == NULL`, `mode = 1` ⇒ `stbds_hash_string(NULL, …)` | **SIGSEGV** | `crash…::hmput_key_null_key_str` |
| 48 | `stbds_hmget_key_ts` | `temp == NULL` (NULL out-parameter) | `*temp = -1` writes to address 0 ⇒ **SIGSEGV** | `crash…::hmget_key_ts_null_temp` |
| 49 | `stbds_hmput_key` | oversized `keysize` (`usize::MAX/2`) | `stbds_hash_bytes` walks off the key ⇒ **SIGSEGV** | `crash…::hmput_key_huge_keysize` |
| 50 | `stbds_stralloc` | forged `a->block = 100` ⇒ `blocksize = 512<<50`, `realloc` fails | `sb->next = …` writes through NULL ⇒ **SIGSEGV** | `crash…::stralloc_huge_block` |

## Notes

* Rows 20–23, 25, 31 are asserts that cannot be triggered through the exported API.
  They are nevertheless **replicated verbatim in the Rust translation** (same
  expression text, function name and line number, raised through the same libc
  `__assert_fail` entry point) so that a caller that does reach them observes the
  same `SIGABRT` instead of Rust UB. The `function` argument is the bare
  identifier, exactly what C's `__PRETTY_FUNCTION__` expands to (verified against
  the `.rodata` of the C `.so`: `stbds_make_hash_index`, `stbds_hmput_key`,
  `stbds_hmdel_key`, `stbds_stralloc`). Row 24 proves this works end to end -
  both libraries die with `SIGABRT` and the byte-identical message tail

  ```
  lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.
  ```

  Only the directory part of `__FILE__` differs (the C build records the absolute
  path of the CMake source tree, which is a property of the build tree and not of
  the translation), so the comparison starts at the basename.
* Crash rows are compared in a **child process** (`tests/phase_c_crash.rs`): the
  child loads exactly one of the two libraries, performs the operation, and the
  parent compares the wait-status (exit code / fatal signal) and the assertion
  text. A surviving child exits with code 7, so "survived" can never be confused
  with a signal death or a libtest failure (101).
* Observed results (both profiles):

  | scenario | C | Rust |
  |----------|---|------|
  | `control_survive`, `arrgrowf_size_overflow`, `hash_bytes_null_len0`, `hmfree_func_null`, `hmdel_key_null_map` | exit 7 | exit 7 |
  | `arrgrowf_oom`, `arrfreef_null`, `hash_string_null`, `hash_bytes_null_len1`, `hash_bytes_huge_len`, `stralloc_forged_arena`, `stralloc_null_str`, `stralloc_huge_block`, `strreset_null`, `hmput_key_null_key_bin`, `hmput_key_null_key_str`, `hmget_key_ts_null_temp`, `hmput_key_huge_keysize` | SIGSEGV | SIGSEGV |
  | `hmdel_ptr_to_string` | SIGABRT + `slot >= 0` | SIGABRT + `slot >= 0` |
* `[profile.dev] debug-assertions = false` is required for this parity: libstd's
  debug assertions would turn the C's *deliberate* UB paths (a write through
  `NULL + sizeof(header)`) into a Rust panic, so the dev profile would report
  `SIGABRT` where the C reports `SIGSEGV`. With it, the dev and release cdylibs
  behave identically and both match the C.
