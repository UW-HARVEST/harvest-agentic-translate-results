# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every early `return`, sentinel
value, `STBDS_ASSERT`, null check, range check and min/max constant in the file
gets one row.

Notes on how the C behaves:

* `STBDS_ASSERT` is `assert` and `CMakeLists.txt` sets **no** `NDEBUG`
  (`CMAKE_BUILD_TYPE` is empty), so every assert is **live** in the C `.so`
  (it links `__assert_fail`); a violated assert `abort()`s with SIGABRT.
  **The translation originally dropped all 7 asserts — a real divergence that
  this table's rows 23-26/17/18/31 exposed.** Row 25 (`lib.c:846`) is reachable
  through the public API: the C aborted while the Rust silently continued into a
  wild pointer write. `src/lib.rs` now reproduces every assert with `assert!`
  (a panic under an `extern "C"` fn aborts too), and
  `err_asserts_abort_parity` compares the two libraries' termination status in a
  subprocess. The other six asserts are unreachable for any input a caller can
  supply, but are present so that a future divergence aborts identically rather
  than corrupting memory.
* "sentinel" values used by this file: `-1` (`STBDS_INDEX_EMPTY`),
  `-2` (`STBDS_INDEX_DELETED`), `0`/`NULL`, `0` (`STBDS_HASH_EMPTY`),
  `1` (`STBDS_HASH_DELETED`).
* `mode` is a plain `int` across the ABI; the C only ever tests
  `mode >= STBDS_HM_STRING` (i.e. `>= 1`) or `mode == STBDS_HM_STRING`
  (i.e. `== 1`), so *any* other int is a legal input with defined behaviour
  that the Rust must reproduce.

| # | function | trigger (the exact invalid input/condition) | expected C result | test (`tests/phase_c_errors.rs`) | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` → `min_len = 0`, `min_cap = 0 <= arrcap(NULL) = 0` (line 286) | returns `a` i.e. **`NULL`**, allocates nothing | `err_01_arrgrowf_null_zero_returns_null` | [x] |
| 2 | `stbds_arrgrowf` | existing `a`, `min_len <= min_cap <= arrcap(a)` (line 286) | returns the **same pointer** `a`, capacity/length untouched, no realloc | `err_02_arrgrowf_no_growth_returns_same` | [x] |
| 3 | `stbds_arrgrowf` | `elemsize == 0` (degenerate element size) | still allocates `0*min_cap + sizeof(header)`, `capacity = min_cap`, `length = 0` | `err_03_arrgrowf_zero_elemsize` | [x] |
| 4 | `stbds_arrgrowf` | huge `min_cap`/`addlen` so `elemsize*min_cap + sizeof(hdr)` overflows / `realloc` fails → **no null check** at line 297 | UB in C (write through `NULL+32`). *Not exercised* — documented as out of scope; both libs would fault identically. | `err_04_05_documented_ub` | [n/a] |
| 5 | `stbds_arrfreef` | `a == NULL` → `free(stbds_header(NULL))` = `free((char*)0 - 32)` | UB/`free(): invalid pointer` abort. *Not exercised.* | `err_04_05_documented_ub` | [n/a] |
| 6 | `stbds_hmfree_func` | `a == NULL` (line 573) | returns immediately, no free, no crash | `err_06_hmfree_null` | [x] |
| 7 | `stbds_hmfree_func` | valid `a` whose `hash_table == NULL` (line 574) | skips the strdup sweep and `strreset`, still frees `hash_table` (NULL) and the header | `err_07_hmfree_no_table` | [x] |
| 8 | `stbds_hm_find_slot` | probe reaches a bucket slot with `hash == STBDS_HASH_EMPTY (0)` (lines 609-611 / 620-622) | returns **`-1`** | `err_11_hmget_ts_missing_key` + `err_22_hmdel_missing_key` | [x] |
| 9 | `stbds_hmget_key_ts` | `a == NULL` (line 634) | `*temp = STBDS_INDEX_EMPTY (-1)`; returns a **newly allocated** hash pointer (len 1, zeroed elem 0) | `err_09_hmget_ts_null_a` | [x] |
| 10 | `stbds_hmget_key_ts` | valid `a` but `hash_table == 0` (line 644) — e.g. array built by `stbds_hmput_default` only | `*temp = -1`; returns `a` unchanged | `err_10_hmget_ts_no_table` | [x] |
| 11 | `stbds_hmget_key_ts` | key not present in a populated table (`slot < 0`, line 648) | `*temp = STBDS_INDEX_EMPTY (-1)`; returns `a` unchanged | `err_11_hmget_ts_missing_key` | [x] |
| 12 | `stbds_hmget_key` | any of rows 9-11 | same as `..._ts` plus `stbds_header(p-elemsize)->temp == -1` | `err_12_hmget_key_missing_writes_temp` | [x] |
| 13 | `stbds_hmput_default` | `a == NULL` (line 669) | allocates, `length = 1`, elem 0 zeroed, returns `arr+elemsize` | `err_13_14_15_hmput_default` | [x] |
| 14 | `stbds_hmput_default` | `a != NULL` but `stbds_header(a-elemsize)->length == 0` (line 669) | re-grows and bumps `length` to 1 (re-zeroes elem 0) | `err_13_14_15_hmput_default` | [x] |
| 15 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged**, no allocation | `err_13_14_15_hmput_default` | [x] |
| 16 | `stbds_hmput_key` | `a == NULL` (line 686) | bootstraps a 1-element array before inserting; never returns NULL | `err_16_hmput_key_null_a` | [x] |
| 17 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` (line 778) | unreachable — `arrgrowf` above guarantees it; C returns normally | `err_17_hmput_key_assert_unreachable` + `err_asserts_abort_parity` | [x] |
| 18 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` (line 401) | unreachable for every `slot_count` the API can produce (8, 16, 32, …): `6+1<8`, `12+3<16`, `24+6<32`; C returns normally | `err_18_make_hash_index_assert_unreachable` + `err_asserts_abort_parity` | [x] |
| 19 | `stbds_shmode_func` | `mode` out of the `STBDS_SH_*` enum range (e.g. `7`, `-1`, `256`, `INT_MAX`) — stored as `(unsigned char) mode` (line 803) | `string.mode = mode & 0xff`; a later `stbds_hmput_key` then takes the `switch` **`default`** (memcpy) branch unless the truncation lands on 1/2/3 | `err_19_shmode_out_of_range` | [x] |
| 20 | `stbds_hmdel_key` | `a == NULL` (line 809) | returns **`0` (NULL)** | `err_20_hmdel_null_a` | [x] |
| 21 | `stbds_hmdel_key` | valid `a`, `hash_table == 0` (line 816) | sets `stbds_temp(raw_a) = 0`, returns `a` | `err_21_hmdel_no_table` | [x] |
| 22 | `stbds_hmdel_key` | key absent, `slot < 0` (line 821) | `stbds_temp(raw_a) = 0` (the "deleted 0 items" sentinel), returns `a`, length unchanged | `err_22_hmdel_missing_key` | [x] |
| 23 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` (line 828) | unreachable: `find_slot` returns `(pos & ~7) + i` with `pos < slot_count`, `i < 8`, so always `< slot_count`. Now also asserted in the Rust. | `err_23_25_26_hmdel_asserts` + `err_asserts_abort_parity` | [x] |
| 24 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` (line 832) | `used_count` is `size_t`, so always true — but note `--table->used_count` on a table with `used_count == 0` **wraps** instead of tripping the assert | `err_24_hmdel_used_count_wrap` | [x] |
| 25 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` after the re-find of the moved last element (line 846) | **REACHABLE**: with `mode > 1` and an interior delete, line 842 takes the `else` branch and hands `find_slot` the raw bytes of the stored `char *`, which it hashes as a string and does not find → `slot == -1` → the C `abort()`s with `lib.c:846: stbds_hmdel_key: Assertion 'slot >= 0' failed.` (SIGABRT / 134). **This is divergence #1 that verification found**: the Rust had dropped the assert and continued into `storage + (-1 >> 3)`. Fixed. | `err_23_25_26_hmdel_asserts` + `err_asserts_abort_parity` (5 aborting scenarios x both libs, in subprocesses) | [x] |
| 26 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` (line 849) | unreachable: a successful re-find always lands on the moved element's own slot. Now also asserted in the Rust. | `err_23_25_26_hmdel_asserts` + `err_asserts_abort_parity` | [x] |
| 27 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`, line 839) | skips the memmove + re-find entirely, just shrinks `length` | `err_27_hmdel_last_element` | [x] |
| 28 | `stbds_hmdel_key` | `mode == 2` (or any `mode > 1`) with a *string* table: line 836 `mode == STBDS_HM_STRING` is **false** so the strdup'd key is **not freed**, and line 842 takes the `else` branch, passing the raw bytes (not `*(char**)`) to `find_slot`, while `find_slot` itself uses `mode >= 1` → hashes those raw bytes as a C string | no rejection: C proceeds and produces a specific (surprising) result the Rust must match bit-for-bit | `err_28_hmdel_mode_two_string_table` | [x] |
| 29 | `stbds_hmdel_key` | shrink path: `used_count < used_count_shrink_threshold && slot_count > 8` (line 854) | rebuilds a half-size index and frees the old one | `err_29_30_hmdel_shrink_and_rebuild` | [x] |
| 30 | `stbds_hmdel_key` | rebuild path: `tombstone_count > tombstone_count_threshold` (line 858) | rebuilds a same-size index and frees the old one | `err_29_30_hmdel_shrink_and_rebuild` | [x] |
| 31 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` (line 913) | unreachable — the `len > blocksize` branch returns early and the else branch sets `remaining = blocksize >= len`. Now also asserted in the Rust. | `err_31_stralloc_assert` + `err_asserts_abort_parity` | [x] |
| 32 | `stbds_stralloc` | `len > blocksize` (huge string, line 893) | allocates a dedicated oversize block, splices it in **after** `a->storage` (or becomes `storage` with `remaining = 0`), returns `sb->storage`; `a->remaining` is left alone in the spliced case | `err_32_stralloc_oversize` | [x] |
| 33 | `stbds_stralloc` | empty string `""` → `len == 1`, on a fresh arena (`remaining == 0`) | takes the grow path, `blocksize = 512 << 0 = 512`, `block` → 1, returns `storage->storage + 512 - 1` | `err_33_stralloc_empty_string` | [x] |
| 34 | `stbds_stralloc` | `a->block` saturating: `blocksize = 512u << (block>>1)`; `++a->block` only while `blocksize < 1<<20` (min/max constants `STBDS_STRING_ARENA_BLOCKSIZE_MIN=512`, `MAX=1<<20`) | `block` stops incrementing once `512<<(block>>1) >= 1MiB` (i.e. `block>>1 >= 11`, `block == 22`); block size saturates at 1 MiB | `err_34_stralloc_block_saturation` + `err_34b_stralloc_block_full_range` (all 29 boundary values of the `unsigned char`, each in a subprocess) | [x] |
| 35 | `stbds_strreset` | arena with `storage == NULL` | walks nothing, memsets the arena to 0 | `err_35_strreset_empty` | [x] |
| 36 | `stbds_hash_string` / `stbds_hash_bytes` | `hash < 2` after hashing (callers add 2: lines 596, 719) — reserves 0/1 for EMPTY/DELETED | the raw hash functions themselves never reject; the +2 fixup is caller-side | `err_36_hash_low_values` | [x] |
| 37 | `stbds_hash_string` | empty string `""` (loop body never runs) | hashes `seed` alone; still returns a value | `err_37_hash_string_empty` | [x] |
| 38 | `stbds_hash_bytes` | `len == 0` | skips the block loop, `switch(0)` → `break`, `data = 0 << 56 = 0` | `err_38_hash_bytes_zero_len` | [x] |
| 39 | `stbds_hash_bytes` | `len` in 1..=7 (each `switch` fall-through case, incl. the sign-extending `case 4: data |= (d[3] << 24)`) | one distinct result per `len`; `d[3] >= 0x80` sign-extends to the top 32 bits | `err_39_hash_bytes_tail_lengths` |
| 40 | `stbds_hmget_key` / `stbds_hmput_key` / `stbds_hmdel_key` / `stbds_hm_find_slot` | out-of-range enum `mode`: negative (`-1`, `INT_MIN`) → `mode >= 1` false → **binary** path; `>= 2` (`2`, `INT_MAX`) → **string** path | no rejection; the C silently reinterprets the key. Rust must pick the same branch for every int. | `err_40_mode_out_of_range` | [x] |
| 41 | `stbds_hmput_key` | `keysize == 0` with binary mode | `memcmp(...,0) == 0` always true → first probed slot with a matching hash "matches"; `memcpy(...,0)` copies nothing | `err_41_zero_keysize` | [x] |
| 42 | `strkey` | `n == INT_MIN` (`-2147483648`) — `%d` of the most negative int | `"test_-2147483648"` (17 bytes + NUL); a naive negate would overflow | `err_42_strkey_int_min` | [x] |
| 43 | `strkey` | `n == 0`, `n < 0`, and large `n` | `"test_0"`, `"test_-N"`, `"test_2147483647"` | `err_43_strkey_boundaries` | [x] |
| 44 | `arr_del` | any `int num`, incl. `INT_MIN`/`INT_MAX` — the `i == 3` iteration makes `arrdeln`'s memmove count `length-n-i = 4-1-3 = 0` and `arrdelswap` self-assign `a[3] = a[3]` | returns `void`, no observable output; must not crash or leak differently | `err_44_arr_del_boundaries` | [x] |

## The two divergences verification found

### 2. `stbds_stralloc` shift count (`lib.c:888`)

`a->block` is an `unsigned char` and `stbds_stralloc` is **exported**, so a caller
can pass any value 0..=255 and the C's shift count `a->block >> 1` reaches 127.
gcc emits `shlq %cl`, which masks the count to 6 bits. The Rust used a plain
`<<`, which is fine in the release profile (`overflow-checks = false`) but
**panics and aborts** in the dev profile for any count >= 64:

```
a->block = 128   C            -> "hello", block=129, remaining=506  (exit 0)
                 Rust release -> identical
                 Rust debug   -> SIGABRT ("attempt to shift left with overflow")
a->block = 255   C            -> "hello", block=0,   remaining=0    (exit 0)
                 Rust debug   -> SIGABRT
```

Fixed with `wrapping_shl`, which masks the count the same way. Nine other plain
`+`/`-` sites reachable from caller-poked state were made `wrapping_*` for the
same reason. Verified with a negative control (restoring the plain `<<` makes
`err_34b_stralloc_block_full_range` fail at `block = 128`).

For `a->block` in {63, 64, 100, 200} the C's *unchecked* `realloc` (row 4) returns
NULL and the C stores through it: C and release-Rust both SIGSEGV; the dev
profile's null-pointer check traps the same UB one step earlier and aborts. The
test requires an exact match whenever the C returns normally, and "both died" on
the row-4 UB inputs.

### 1. The dropped `STBDS_ASSERT`s

`src/lib.rs` had translated all 7 `STBDS_ASSERT`s away (four silently, three as
comments). Row 25 is reachable, so the two libraries genuinely disagreed:

```
$ # interior stbds_hmdel_key with mode == 2 on a STBDS_SH_DEFAULT string map
C (reference)          -> SIGABRT   (lib.c:846: Assertion `slot >= 0' failed.)
Rust (as translated)   -> exit 0    (continued into a wild pointer write)
Rust (after the fix)   -> SIGABRT   (src/lib.rs: stbds_assert: lib.c:846: ...)
```

The fix adds all 7 asserts back with `assert!` (not `debug_assert!`, so they are
live in every profile, matching the C build). Re-checked with a negative control:
a Rust `.so` built with the assert macro neutered makes
`err_asserts_abort_parity` fail, so the test really does guard the behaviour.

## Verification status

* 42 of 44 rows have a passing differential test in
  `translation/tests/phase_c_errors.rs` (37 test functions; several rows share a
  test where the C shares a code path).
* Rows 4 and 5 are genuine undefined behaviour in the C (an unchecked `realloc`
  result and `free((char*)0 - 32)`). They are recorded, reasoned about in
  `err_04_05_documented_ub`, and deliberately not executed: both libraries fault
  identically, so running them proves nothing and destroys the harness. The same
  applies to the *size-overflow* variant of row 4 (`elemsize * min_cap +
  sizeof(header)` wrapping to a small value, e.g. `elemsize = 4`,
  `min_cap = SIZE_MAX`, which makes the C write a 32-byte header into a 28-byte
  allocation).
* One further C-level abort is documented rather than executed: with
  `mode != STBDS_HM_STRING` but `mode >= 1`, an **interior** delete makes
  `stbds_hmdel_key` hand the raw bytes of the stored `char *` to
  `stbds_hm_find_slot`, which hashes them as a string, finds nothing, and trips
  the live `STBDS_ASSERT(slot >= 0)` at line 846. `err_28_hmdel_mode_two_string_table` and `err_40_mode_out_of_range`
  therefore restrict `mode >= 2` deletes to the last element, where that block
  is skipped.
