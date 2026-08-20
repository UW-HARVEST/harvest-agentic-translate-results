# ERRORS.md — error-surface table (Phase A → gates Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping **every** `return`,
`STBDS_ASSERT`/`assert`, `== NULL` / `== 0` / `< 0` / `>= ` / `<= ` guard, and
every min/max constant. One row per *distinct* rejection / early-out / abort.

Notes that apply to the whole table:

* `CMAKE_BUILD_TYPE` is unset ⇒ **`NDEBUG` is not defined ⇒ `assert()` is
  live** in the C `.so` (`__assert_fail@GLIBC_2.2.5` is an undefined symbol of
  the C `.so`). An `assert` failure = `SIGABRT`.
* The library has **no error codes and no `errno`**. Every "error" is one of:
  an early `return` of the input pointer / `NULL` / `-1`, a sentinel written
  into `*temp` (`STBDS_INDEX_EMPTY == -1`), a live `assert`, or an unchecked
  NULL dereference (`SIGSEGV`).
* `mode` is a plain C `int`; **no function validates it**. Any out-of-range
  "enum" value is legal input across the FFI boundary (rows 9, 10, 27, 35, 45).

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
|  1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after `min_len` fixup — e.g. `a=NULL, elemsize=8, addlen=0, min_cap=0` (`lib.c:286`) | returns `a` **unchanged** (i.e. `NULL`); **no allocation at all** | `err_01_arrgrowf_no_growth` |
|  2 | `stbds_arrgrowf` | `a=NULL` with `0 < min_cap < 4` and `addlen < 4` (`lib.c:291`) | allocates; `capacity` is bumped to the `4` floor, `length=0`, `hash_table=NULL`, `temp=0` | `err_02_arrgrowf_cap_floor` |
|  3 | `stbds_arrgrowf` | `elemsize * min_cap + 32` **wraps** `size_t` (e.g. `elemsize=1<<62, min_cap=8`) — no `realloc` NULL check (`lib.c:297`) | wrapped size ⇒ `realloc` *succeeds* with a 32-byte block; the function returns a live pointer whose `capacity` is the bogus `min_cap` (no error reported) | `err_03_arrgrowf_size_wrap` |
|  4 | `stbds_arrfreef` | `a == NULL` — no NULL guard (`lib.c:312`) | `free((stbds_array_header*)NULL - 1)` = `free((void*)-32)` ⇒ `SIGSEGV` (verified: both `.so`s die with signal 11) | `err_04_arrfreef_null` (subprocess, signal compared) |
|  5 | `stbds_make_hash_index` *(static)* | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` (`lib.c:401`); only false for `slot_count == 0` | `SIGABRT`. **Unreachable from the public API** — `slot_count` is only ever `8`, `2*n`, or `n>>1` guarded by `slot_count > 8` | documented, unreachable (see note A) |
|  6 | `stbds_hash_string` | `str == NULL` (`lib.c:480`) | dereferences NULL ⇒ `SIGSEGV` | `err_06_hash_string_null` (subprocess) |
|  7 | `stbds_hash_bytes` | `p == NULL, len == 0` (`lib.c:522` loop never runs, `switch(0)` derefs nothing) | **no crash** — returns the siphash-2-4 of the empty message for that seed | `err_07_hash_bytes_null_len0` |
|  8 | `stbds_hash_bytes` | `p == NULL, len > 0` | dereferences NULL ⇒ `SIGSEGV` | `err_08_hash_bytes_null_nonzero_len` (subprocess, `len` = 1 and 8) |
|  9 | `stbds_is_key_equal` *(static)* | `mode >= STBDS_HM_STRING` where `mode` is an **out-of-range enum value** (`2`, `7`, `1000`, `INT_MAX`) (`lib.c:560`) | silently treated as STRING: `strcmp(key, *(char**)elem)` | `err_09_10_45_mode_enum_sweep`, `cfg_18_mode_out_of_range_string` |
| 10 | `stbds_is_key_equal` *(static)* | `mode < STBDS_HM_STRING` including **negative** `mode` (`-1`, `INT_MIN`) | silently treated as BINARY: `memcmp(key, elem, keysize)` | `err_09_10_45_mode_enum_sweep`, `cfg_19_mode_out_of_range_binary` |
| 11 | `stbds_hmfree_func` | `a == NULL` (`lib.c:573`) | returns immediately; nothing freed (the only explicit NULL guard that *doesn't* crash) | `err_11_hmfree_null` |
| 12 | `stbds_hm_find_slot` *(static)* | probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` ⇒ key absent (`lib.c:610`, `lib.c:621`) | returns `-1` | `err_12_16_18_absent_key_reports_minus_one` |
| 13 | `stbds_hm_find_slot` *(static)* | `stbds_header(raw_a)->hash_table == NULL` (`lib.c:589`, then `table->seed`) | dereferences NULL ⇒ `SIGSEGV`. Every public caller guards it first (rows 15, 29) | documented, guarded (see note A) |
| 14 | `stbds_hmget_key_ts` | `a == NULL` (`lib.c:634`) | allocates a 1-element zeroed array, `length=1`, `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` | `err_14_hmget_ts_null_map` |
| 15 | `stbds_hmget_key_ts` | `a != NULL` but `hash_table == NULL` (reachable via `stbds_hmput_default(NULL,…)`) (`lib.c:644`) | `*temp = -1`; returns `a` **unchanged**; no table created | `err_15_hmget_ts_no_table` |
| 16 | `stbds_hmget_key_ts` | key absent, table present (`lib.c:648`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err_12_16_18_absent_key_reports_minus_one` |
| 17 | `stbds_hmget_key_ts` | `temp == NULL` | stores through NULL ⇒ `SIGSEGV` | `err_17_hmget_ts_null_temp` (subprocess) |
| 18 | `stbds_hmget_key` | all of rows 14/15/16, via `stbds_hmget_key_ts`, then `stbds_temp(p-elemsize) = temp` (`lib.c:663`) | `header(p-elemsize)->temp == -1` in every "not found" case | `err_12_16_18_absent_key_reports_minus_one` |
| 19 | `stbds_hmput_default` | `a == NULL` (`lib.c:669`) | allocates 1-element zeroed array, `length=1`, returns `arr+elemsize`; `hash_table` stays `NULL` | `err_19_20_21_hmput_default_paths` |
| 20 | `stbds_hmput_default` | `a != NULL` and `header(a-elemsize)->length == 0` | re-grows (`arrgrowf(a-elemsize,…,0,1)`), `length` 0→1, element re-zeroed | `err_19_20_21_hmput_default_paths` |
| 21 | `stbds_hmput_default` | `a != NULL` and `length != 0` | **rejects**: returns `a` byte-identical, element **not** re-zeroed | `err_19_20_21_hmput_default_paths`, `cfg_22_hmput_default` |
| 22 | `stbds_hmput_key` | `a == NULL` (`lib.c:686`) | allocates, then proceeds; **never returns NULL** | `err_22_hmput_key_null_map` |
| 23 | `stbds_hmput_key` | key already present, found in the **first** (`i = pos&MASK … 8`) loop (`lib.c:730`) | returns early *without* inserting; `temp` = existing index; for `mode >= 1` `temp_key` is set to the **already-stored** key pointer, not the caller's | `err_23_24_duplicate_key_both_probe_loops`, `cfg_13_string_default_autotable` |
| 24 | `stbds_hmput_key` | key already present, found in the **second** (`0 … limit`) loop (`lib.c:748`) | returns early without inserting; `temp` = existing index; **`temp_key` is NOT written** — C asymmetry, must be reproduced | `err_23_24_duplicate_key_both_probe_loops` |
| 25 | `stbds_hmput_key` | `assert((size_t)i+1 <= stbds_arrcap(a))` (`lib.c:778`) | `SIGABRT`. Unreachable — the preceding `arrgrowf` guarantees it | documented, unreachable (see note A) |
| 26 | `stbds_hmput_key` | `table->string.mode` is **not** STRDUP/ARENA/DEFAULT (i.e. `STBDS_SH_NONE==0`, or any other `unsigned char` from row 27) while `mode >= 1` (`lib.c:789` `default:`) | falls into `default:` ⇒ `memcpy(elem, key, keysize)` copies the *string bytes*, not the pointer, even though the hash was computed with `stbds_hash_string` | `err_26_27_shmode_truncation_and_default_branch`, `cfg_16_string_mode_on_none_table` |
| 27 | `stbds_shmode_func` | `mode` outside `0..3`: `4`, `255`, `256`, `259`, `-1`, `INT_MIN` (`lib.c:803`) | **no validation**: `h->string.mode = (unsigned char) mode` truncates — `256→0` (NONE), `259→3` (ARENA), `-1→255`, `INT_MIN→0` | `err_26_27_shmode_truncation_and_default_branch` |
| 28 | `stbds_hmdel_key` | `a == NULL` (`lib.c:809`) | returns `0` (NULL) | `err_28_29_30_hmdel_rejections` |
| 29 | `stbds_hmdel_key` | `hash_table == NULL` (`lib.c:816`) | sets `stbds_temp(a-elemsize) = 0`, returns `a` unchanged | `err_28_29_30_hmdel_rejections` |
| 30 | `stbds_hmdel_key` | key absent (`find_slot < 0`) (`lib.c:821`) | sets `temp = 0` — **the one state change a failed delete does make** — then returns `a` unchanged; `length`/`used_count`/`tombstone_count` untouched | `err_28_29_30_hmdel_rejections` |
| 31 | `stbds_hmdel_key` | `assert(slot < (ptrdiff_t) table->slot_count)` (`lib.c:828`) | `SIGABRT`. Unreachable — `find_slot` masks `pos` with `slot_count-1` | documented, unreachable (see note A) |
| 32 | `stbds_hmdel_key` | `assert(table->used_count >= 0)` (`lib.c:832`) | `used_count` is `size_t` ⇒ the comparison is **always true**; a **dead assert** that does not fire even when `--used_count` wraps to `SIZE_MAX` | `err_32_used_count_wraps_without_assert` |
| 33 | `stbds_hmdel_key` | `assert(slot >= 0)` for the swap-with-last re-find (`lib.c:846`). **REACHABLE**: a non-zero `keyoffset` makes the delete match on an element's *value* half, and the re-find then looks up the moved-in element's value, which is not a key. Build `(1,1),(2,2),(3,99)` then `hmdel_key(t,8,&1,4,keyoffset=4,mode=0)` | `SIGABRT`. The Rust translation reproduces the assert with `abort()`; without it Rust would write through `storage[-1]` | `err_33_hmdel_assert_slot_ge_zero` (subprocess, signal compared) |
| 34 | `stbds_hmdel_key` | `assert(b->index[i] == final_index)` (`lib.c:849`). **REACHABLE** the same way: build `(1,1),(2,2),(3,2)` then `hmdel_key(t,8,&1,4,keyoffset=4,mode=0)` — the re-find lands on key `2`, whose index is 1, not `final_index == 2` | `SIGABRT`, reproduced in Rust via `stbds_assert` | `err_34_hmdel_assert_index_eq_final` (subprocess, signal compared) |
| 35 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **exact** equality (`lib.c:836`, `lib.c:842`) but `find_slot`/hash use `mode >= 1`. Pass `mode = 2` (or 7) on a string table | hash/compare use the STRING path, yet the strdup'd key is **not freed** and the re-find uses the **binary** key expression `(char*)a+elemsize*old_index+keyoffset` (address of the pointer slot) rather than `*(char**)…` — a genuine C quirk. That re-find hashes the *bytes of a heap pointer*, so its outcome depends on the actual address and is **not** comparable between two `.so`s; the test therefore deletes LIFO so `old_index == final_index` and the re-find never runs | `err_35_hmdel_mode2_on_string_table` |
| 36 | `stbds_stralloc` | `str == NULL` (`lib.c:884` `strlen`) | `SIGSEGV` | `err_36_stralloc_null_str` (subprocess) |
| 37 | `stbds_stralloc` | `assert(len <= a->remaining)` (`lib.c:913`) | `SIGABRT`. Unreachable — the `else` branch always sets `remaining = blocksize >= len` | documented, unreachable (see note A) |
| 38 | `stbds_stralloc` | `a->block` large: `blocksize = (size_t)512 << (block>>1)`; `block` is `unsigned char` so `block>>1` reaches `127` ⇒ **shift ≥ 64 is C UB**; x86-64 `shlq` masks to 6 bits (`lib.c:888`) | `512 << ((block>>1) & 63)`; for `(block>>1)&63 >= 55` the value wraps to `0`, so `blocksize == 0 < MAX` ⇒ `++block` (wrapping `255→0`) and **every** string takes the "own block" path of rows 39/40. At `block == 128` the masked count is 0 again, so the blocksize jumps back to `512`. For `(block>>1)&63` in ~19..54 the blocksize is huge but non-zero, `realloc` fails and the unchecked result is dereferenced ⇒ `SIGSEGV` in both | `err_38_39_40_stralloc_block_branches`, `cfg_40_arena_block_counter`, `err_38b_stralloc_huge_block_alloc_failure` (subprocess) |
| 39 | `stbds_stralloc` | `len > blocksize` **and** `a->storage == NULL` (`lib.c:894`) | dedicated `8+len` block; `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`; returns `sb->storage` (bypasses the `assert`) | `err_38_39_40_stralloc_block_branches`, `cfg_38_arena_oversize_first` |
| 40 | `stbds_stralloc` | `len > blocksize` **and** `a->storage != NULL` (`lib.c:896`) | dedicated block spliced in as `storage->next` (`sb->next = storage->next; storage->next = sb`); `a->remaining` left **untouched** | `err_38_39_40_stralloc_block_branches`, `cfg_39_arena_oversize_later` |
| 41 | `stbds_strreset` | `a == NULL` (`lib.c:923`) | dereferences NULL ⇒ `SIGSEGV` | `err_41_strreset_null` (subprocess) |
| 42 | `strkey` | any `int`, including `INT_MIN` / `INT_MAX` | `sprintf(buffer,"test_%d",n)` into a shared 256-byte static; longest output is 16 chars + NUL ⇒ **never overflows**; returns the *same* pointer on every call, so the previous result is clobbered | `err_42_43_44_strkey_and_str_put_edges`, `cfg_42_strkey` |
| 43 | `str_put` | `num <= 0` (`lib.c:951` loop) | the `stralloc`/`strkey` loop body never runs; `strreset` on an untouched arena is a no-op; the map part runs unchanged and prints `a <num>` | `err_42_43_44_strkey_and_str_put_edges`, `cfg_43_str_put_stdout` |
| 44 | `str_put` | live asserts `*strmap[0].key=='a'`, `strmap[0].key==s.key`, `strmap[0].value==s.value` (`lib.c:958-960`) | hold for every `num`; `strmap[0]` is raw element **1** because `strmap` is `arr+elemsize` | `err_42_43_44_strkey_and_str_put_edges`, `cfg_43_str_put_stdout` |
| 45 | *every* `hm*`/`sh*` entry point | `mode` is a raw C `int` with **zero** validation anywhere in the file: `INT_MIN`, `-1`, `0`, `1`, `2`, `7`, `1000`, `INT_MAX` are all accepted | `mode >= 1` ⇒ string path, `mode <= 0` ⇒ binary path; `stbds_hmdel_key` additionally special-cases `mode == 1` exactly (row 35) | `err_09_10_45_mode_enum_sweep` |

## Note A — the five rows that are unreachable through the public API

Rows 5, 13, 25, 31 and 37 are live `assert`s / NULL dereferences whose guard
condition cannot be produced by any sequence of calls to the 16 exported
functions:

| row | why unreachable |
|-----|-----------------|
|  5 | `slot_count` is only ever `STBDS_BUCKET_LENGTH` (8), `table->slot_count*2`, or `table->slot_count>>1` behind `slot_count > STBDS_BUCKET_LENGTH`, so it is always a power of two ≥ 8 and `used_thr + tomb_thr < slot_count` always holds (8→7<8, 16→15<16, 32→30<32, 64→60<64, …). |
| 13 | both public callers (`stbds_hmget_key_ts` at `lib.c:644` and `stbds_hmdel_key` at `lib.c:816`) test `table == 0` *before* calling `stbds_hm_find_slot`. |
| 25 | the immediately preceding `if ((size_t) i+1 > stbds_arrcap(a)) arrgrowf(a, elemsize, 1, 0)` guarantees `arrcap(a) >= i+1`. |
| 31 | `stbds_hm_find_slot` masks every `pos` with `table->slot_count-1`, so the returned slot is always `< slot_count`. |
| 37 | the `else` branch that runs when `len > a->remaining` sets `a->remaining = blocksize` and is only taken when `len <= blocksize`; the `len > blocksize` branch returns early. |

They are nevertheless **reproduced in the Rust translation** (`stbds_assert`,
which calls libc `abort()`), so that if a future change ever makes one
reachable the two libraries still terminate identically. Rows 33 and 34 are the
proof that this matters: they *are* reachable (via a non-zero `keyoffset`) and
without the ported `assert`s the Rust would silently write through
`table->storage[-1]` while the C aborts.

## Crash-parity methodology

`tests/phase_c_errors.rs` compares crashes by re-executing the test binary as a
child process (`--exact err_child_runner` + `DIFF_CRASH_CASE`/`DIFF_CRASH_LIB`)
once per library and asserting the *same* `(signal, exit code)` pair. The Rust
child always loads **`target/release/libstr_put_lib.so`** via `DIFF_RUST_SO`:
that is the shipped artifact, and unlike a debug build it carries no
`debug_assertions` instrumentation, which would otherwise convert a NULL or
misaligned raw dereference into a Rust panic + `abort()` (SIGABRT) instead of
the fault (SIGSEGV) the C produces.

## Divergences found and fixed in `src/lib.rs` during Phase C

1. **Missing `assert()`s.** The C `.so` has `NDEBUG` undefined, so all eight
   `STBDS_ASSERT`s (plus the three in `str_put`) are live; the translation had
   dropped them. Rows 33/34 are reachable, so this was a real behavioural
   difference (`SIGABRT` vs. out-of-bounds write). Added `stbds_assert()`.
2. **Aligned `char**` dereferences.** The translation used `*(p as *mut *mut
   c_char)`, but `elemsize == 1` puts element *i* at `a + i`, so the C does
   *unaligned* pointer loads/stores. Replaced with `read_unaligned` /
   `write_unaligned` (`read_key_ptr` / `write_key_ptr`).
3. **Overflow-checked arithmetic.** `used_count -= 1`, `length -= 1`,
   `tombstone_count -= 1`, `slot_count - 1`, `len << 56`, `512 << (block>>1)`,
   … all wrap silently in C. They are now explicit `wrapping_*` /
   `wrapping_shl` operations so the debug and release builds behave alike
   (see row 32: `used_count` must wrap to `SIZE_MAX`, not panic).
4. **`core::ptr::copy`/`write_bytes` → libc `memcpy`/`memmove`/`memset`**, and
   `<*mut T>::add/sub` → wrapping address arithmetic, so degenerate arguments
   (`n == 0` with NULL, `stbds_header(NULL)`) behave like the C build instead
   of hitting Rust's stricter pointer preconditions.

## Generic FFI boundary cases also covered

| case | covered by |
|------|-----------|
| NULL map pointer to every `hm*` entry point | rows 11, 14, 19, 22, 28 |
| NULL data/string pointer | rows 4, 6, 8, 17, 36, 41 |
| zero length / zero size (`len=0`, `elemsize=0`, `keysize=0`, `min_cap=0`, `addlen=0`) | rows 1, 7 + `err_46_zero_sizes` |
| oversized length (`size_t` wrap in the allocation size) | row 3 |
| allocation size so large that `realloc` returns NULL and the unchecked result is dereferenced | row 38 (`err_38b_stralloc_huge_block_alloc_failure`) |
| one step past a valid range (`mode = 4` for `shmode_func`, `mode = 2` for HM) | rows 9, 27, 35, 45 |
| out-of-range C enum values across FFI (`INT_MIN`, `-1000`, `-2`, `-1`, `2`, `3`, `4`, `7`, `255`, `256`, `1000`, `INT_MAX`) | rows 9, 10, 27, 45 |
| `elemsize` / `keysize` / `min_cap` / `addlen` / `len` == 0 | rows 1, 7, 46 |
| `elemsize` too small for the data the library stores (a string-mode table writes an 8-byte `char *`, so `elemsize < 8` overflows the heap block in **both** libraries and glibc aborts on the next `free`) | documented in `err_22_hmput_key_null_map` |

## Row status

**45 / 45 rows have a passing differential test or a documented
unreachability proof (note A).** 40 rows are directly tested; the 5 rows in
note A are unreachable through the public API *and* their `assert` is ported
into the Rust anyway.
