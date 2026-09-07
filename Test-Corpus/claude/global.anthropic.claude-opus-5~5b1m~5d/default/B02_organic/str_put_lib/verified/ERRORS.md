# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every early `return`, sentinel
return, `STBDS_ASSERT` (= `assert`, **enabled** — the C `.so` imports
`__assert_fail`), explicit range/null check and min/max constant gets a row.

Legend for the check column: `[x]` = differential test written and passing,
`[U]` = documented as unreachable through the public ABI with self-consistent
state (verified by reading the code; no test possible without deliberately
corrupting memory, which would be UB in both languages).

| #  | function | trigger (exact invalid input / condition) | expected C result | ✔ |
|----|----------|-------------------------------------------|-------------------|---|
| 1  | `stbds_arrgrowf` | `max(min_cap, arrlen(a)+addlen) <= arrcap(a)` | returns `a` byte-identical, no realloc (`c_src/src/lib.c:286`) | [x] |
| 2  | `stbds_arrgrowf` | `a == NULL` | fresh alloc; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=min_cap` (`:300`) | [x] |
| 3  | `stbds_arrgrowf` | resulting `min_cap < 4` (and `2*arrcap` not larger) | `min_cap` forced to 4 (`:291`) | [x] |
| 4  | `stbds_arrgrowf` | `min_cap < 2*arrcap(a)` | `min_cap = 2*arrcap(a)` (doubling floor, `:289`) | [x] |
| 5  | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0` on non-NULL `a` | returns `a` unchanged (0 <= cap) | [x] |
| 6  | `stbds_hmfree_func` | `a == NULL` | returns immediately, frees nothing (`:573`) | [x] |
| 7  | `stbds_hmfree_func` | `stbds_hash_table(a) == NULL` | skips strdup-key loop + `strreset`, still frees `hash_table` and header (`:574`) | [x] |
| 8  | `stbds_hm_find_slot` | probe reaches a bucket slot with `hash == STBDS_HASH_EMPTY(0)` | returns `-1` (`:610`, `:621`) | [x] |
| 9  | `stbds_hmget_key_ts` | `a == NULL` | allocates 1-elem array, `length=1`, zeroed, `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` (`:634`) | [x] |
| 10 | `stbds_hmget_key_ts` | `a != NULL` but `header->hash_table == NULL` | `*temp = -1`, returns `a` unchanged (`:644`) | [x] |
| 11 | `stbds_hmget_key_ts` | key absent from a populated table | `*temp = STBDS_INDEX_EMPTY (-1)` (`:648`) | [x] |
| 12 | `stbds_hmget_key` | any of rows 9–11 | header `temp` field set to `-1` (`:663`) | [x] |
| 13 | `stbds_hmput_default` | `a == NULL` | alloc, `length += 1`, zero elem 0 (`:669`) | [x] |
| 14 | `stbds_hmput_default` | `a != NULL` and `header(arr)->length == 0` | grow, `length += 1`, zero elem 0 (`:669`) | [x] |
| 15 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** — rejects re-init (`:675`) | [x] |
| 16 | `stbds_hmdel_key` | `a == NULL` | returns `0` / `NULL` (`:810`) | [x] |
| 17 | `stbds_hmdel_key` | `header(raw_a)->hash_table == NULL` | `temp = 0`, returns `a` unchanged, length unchanged (`:816`) | [x] |
| 18 | `stbds_hmdel_key` | key not present (`slot < 0`) | `temp = 0`, returns `a`, length unchanged (`:821`) | [x] |
| 19 | `stbds_hmdel_key` | key present | `temp = 1`, `hash[i]=STBDS_HASH_DELETED(1)`, `index[i]=STBDS_INDEX_DELETED(-2)`, `length -= 1` (`:831`) | [x] |
| 20 | `stbds_hm_find_slot` / `stbds_hmput_key` | computed `hash < 2` (collides with EMPTY=0 / DELETED=1) | `hash += 2` (`:596`, `:719`) | [x] |
| 21 | `stbds_is_key_equal` | `mode >= STBDS_HM_STRING` — **including out-of-range enum ints** `2, 3, 99, INT_MAX` | `strcmp` on `*(char**)` path (`:560`) | [x] |
| 22 | `stbds_is_key_equal` | `mode < STBDS_HM_STRING` — including **negative** `mode` (`-1`, `INT_MIN`) | `memcmp` of `keysize` bytes (`:563`) | [x] |
| 23 | `stbds_hmput_key` | `a == NULL`, `mode >= STBDS_HM_STRING` | new table gets `string.mode = STBDS_SH_DEFAULT(1)` (`:707`) | [x] |
| 24 | `stbds_hmput_key` | `a == NULL`, `mode < STBDS_HM_STRING` (0 or negative) | new table gets `string.mode = 0` (`STBDS_SH_NONE`) (`:707`) | [x] |
| 25 | `stbds_hmdel_key` | `mode != STBDS_HM_STRING` **exactly** (e.g. `mode == 2`) while `string.mode == STBDS_SH_STRDUP` | key pointer **not** freed and the fixup `find_slot` takes the non-string branch (`:836`, `:842`). For a TAIL delete this succeeds; for a NON-tail delete the fixup lookup hashes the raw pointer bytes, `find_slot` returns -1 and `STBDS_ASSERT(slot >= 0)` **aborts** (`SIGABRT`) | [x] |
| 26 | `stbds_hmput_key` | `table->string.mode` not in `{STRDUP,ARENA,DEFAULT}` (i.e. `NONE` or the truncated out-of-range value from `stbds_shmode_func`) | `default:` branch — raw `memcpy` of `keysize` bytes (`:789`) | [x] |
| 27 | `stbds_shmode_func` | `mode` outside `{0..3}`: `4`, `99`, `256`, `-1`, `INT_MAX` | stored as `(unsigned char) mode` (truncated: `256 -> 0`, `-1 -> 255`) (`:803`) | [x] |
| 28 | `stbds_hash_string` | empty string `""` | loop body never runs; returns mix of `seed` alone (`:480`) | [x] |
| 29 | `stbds_hash_bytes` | `len == 0` | main loop skipped, tail `switch` hits `case 0`, `data = 0 << 56` (`:532`) | [x] |
| 30 | `stbds_hash_bytes` | `len` in `1..7` (partial tail) and `len % 8 != 0` (tail after full words) | fall-through `switch` at `:532` accumulates bytes 6..0 | [x] |
| 31 | `stbds_hash_bytes` | tail byte `d[3] >= 0x80` with `len-i >= 4` | `data |= (d[3] << 24)` is **`int`** arithmetic ⇒ sign-extended to 64 bits (`:536`) | [x] |
| 32 | `stbds_hash_bytes` | any full 8-byte word whose byte 3 or byte 7 has the high bit set | `d[0]|(d[1]<<8)|(d[2]<<16)|(d[3]<<24)` is `int` ⇒ sign-extension (`:523`,`:524`) | [x] |
| 33 | `stbds_stralloc` | `len <= a->remaining` | pure bump, no allocation, `remaining -= len` (`:885`) | [x] |
| 34 | `stbds_stralloc` | `len > remaining` **and** `len > blocksize` **and** `a->storage != NULL` | dedicated oversized block spliced in *after* head; `remaining` untouched (`:896`) | [x] |
| 35 | `stbds_stralloc` | `len > remaining` **and** `len > blocksize` **and** `a->storage == NULL` | dedicated block becomes head, `remaining = 0` (`:900`) | [x] |
| 36 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`, i.e. `a->block >= 24` | `a->block` **not** incremented (saturates) (`:890`) | [x] |
| 37 | `stbds_stralloc` | `a->block >= 110` ⇒ `512 << (block>>1)` overflows `size_t`; `a->block >= 128` ⇒ shift count `>= 64` (C UB, x86 `shl` masks `&63`) | blocksize becomes 0 (110..127) or `512 << ((block>>1)&63)` (>=128); must match bit-for-bit (`:888`) | [x] |
| 38 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` after block setup | `abort()`; unreachable with self-consistent arena (`:913`) | [U] |
| 39 | `stbds_strreset` | all-zero arena (`storage == NULL`) | no frees; `memset(a,0,sizeof)` ⇒ idempotent (`:920`) | [x] |
| 40 | `stbds_make_hash_index` | `STBDS_ASSERT(uct + tct < slot_count)` fails for `slot_count in {0,1,2}` | `abort()`; static fn, min public `slot_count` is 8 (`:401`) | [U] |
| 41 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` | `abort()`; unreachable, `arrgrowf` guarantees it (`:778`) | [U] |
| 42 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | `abort()`; `find_slot` always returns `< slot_count` (`:828`) | [U] |
| 43 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t` ⇒ tautology, never fires (`:832`) | [U] |
| 44 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` after re-finding the relocated tail key | `abort()` (`SIGABRT`). **Reachable** through the public ABI: non-tail delete of a `SH_STRDUP` map with `mode == 2` (`:846`) | [x] |
| 45 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` | `abort()`; holds for consistent tables (`:849`) | [U] |
| 46 | `str_put` | `STBDS_ASSERT(*strmap[0].key == 'a')` / `key == s.key` / `value == s.value` | `abort()`; all three always hold (`:958`–`:960`) | [U] |
| 47 | `str_put` | `num <= 0` (0, -1, INT_MIN) | `stralloc` loop body skipped entirely; still runs `shputs` + print + `shfree` (`:951`) | [x] |
| 48 | `stbds_hmput_key` | key found during the **wrap-around** (second) scan loop | `temp` set, but `temp_key` is **NOT** updated — asymmetry vs. the first loop (`:747` vs `:732`) | [x] |
| 49 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` | table shrunk to `slot_count>>1`, old table freed (`:854`) | [x] |
| 50 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (and no shrink) | table rebuilt at same `slot_count` (`:858`) | [x] |
| 51 | `stbds_hmput_key` | `used_count >= used_count_threshold` (`slot_count - slot_count/4`) | table doubled and fully rehashed (`:698`) | [x] |
| 52 | `stbds_hmput_key` | probe hits a tombstone (`index == STBDS_INDEX_DELETED`) before an empty slot | reuses the tombstone slot, `--tombstone_count` (`:739`, `:766`) | [x] |
| 53 | `stbds_arrfreef` | `a == NULL` | `free((char*)NULL - 32)` — UB / heap corruption in **both** languages; deliberately not exercised (`:312`) | [U] |
| 54 | `stbds_hmget_key_ts` | `temp == NULL` out-param | unconditional `*temp = ...` store ⇒ segfault in both; not exercised (`:638`) | [U] |
| 55 | `strkey` | `n` = `INT_MIN`, `-1`, large | `sprintf(buffer,"test_%d",n)` into the 256-byte file-static buffer; returns that same buffer pointer every call | [x] |

## Reachability notes

`STBDS_ASSERT` is plain `assert` and the C is compiled **without** `NDEBUG` (the
C `.so` imports `__assert_fail`), so every one of these assertions is live and a
failure means `abort()` / `SIGABRT`. The Rust translation therefore carries the
same `assert!`s (`src/lib.rs`, each tagged with the original `STBDS_ASSERT(...)`
text) and `[profile.release] panic = "abort"` makes a failure terminate with
`SIGABRT` too.

* **Row 44 (and therefore row 25) is reachable** through the public ABI and is
  covered by a real differential test: `tests/phase_c_aborts.rs` runs the case
  (non-tail delete of a `STBDS_SH_STRDUP` map with `mode == 2`) in a child
  process for each library and asserts both die with the *same* signal
  (`SIGABRT`, status 134). Before the `assert!`s were added, the Rust silently
  corrupted memory where the C aborted -- this test is what proved the fix.
* Rows 38, 40, 41, 42, 43, 45 and 46 are provably dead:
  - row 38: entering the `len > a->remaining` branch either returns early
    (`len > blocksize`) or sets `remaining = blocksize >= len`;
  - row 40: `stbds_make_hash_index` is `static` and is only ever called with
    `8`, `slot_count*2`, or `slot_count>>1` where `slot_count > 8`, so
    `slot_count >= 8` always and `uct + tct < slot_count` always holds;
  - row 41: `stbds_arrgrowf` guarantees `arrcap(a) >= i+1` immediately before;
  - row 42: `stbds_hm_find_slot` masks `pos` with `slot_count-1`;
  - row 43: `used_count` is `size_t`, so `>= 0` is a tautology;
  - row 45: holds for any table the library itself built;
  - row 46: `shputs` stores `temp_key`, which for a fresh `SH_DEFAULT` table is
    exactly the caller's pointer, so all three `str_put` assertions hold -- and
    they are *executed* (not skipped) on every one of the ~1000 `str_put` calls
    in `tests/phase_b_strput.rs`.
  They are still present in the Rust so that a caller who *does* corrupt state
  gets the same abort.
* Rows 53-54 are unconditional null-pointer dereferences in the C
  (`free((char*)NULL - 32)`, and `*temp = ...` with `temp == NULL`). Both
  libraries fault identically; exercising them would kill the test harness, so
  the harness explicitly guards against them (see `free_both` in
  `tests/phase_b_arr.rs`).

## Verification status

55 rows total: **46 `[x]`** rows each have a passing differential test in
`tests/phase_c_errors.rs` (27 tests) and `tests/phase_c_aborts.rs` (1 test),
reinforced by the Phase B suite (67 more tests). The **9 `[U]`** rows are
38, 40, 41, 42, 43, 45, 46 (provably dead assertions) and 53, 54 (unconditional
NULL dereferences in the C), all justified above.

Additional generic boundaries covered in Phase C:

* NULL map pointer into `stbds_hmdel_key` / `stbds_hmfree_func` (rows 6, 16),
  for every `mode` including out-of-range ones;
* `elemsize == 0` into `stbds_arrgrowf` and `stbds_shmode_func`;
* `keysize == 0` into `stbds_hmput_key` (every key compares equal with a
  zero-length `memcmp`, so the map collapses to a single entry);
* `len == 0` with a NULL data pointer into `stbds_hash_bytes`;
* oversized lengths: 65537-byte hashes, 2 MB arena strings, 4096-slot tables;
* out-of-range **enum values across the FFI boundary**: `mode` =
  `-1, -2, -99, INT_MIN, 2, 3, 4, 5, 99, 1<<20, INT_MAX` for
  `hmput_key` / `hmget_key` / `hmget_key_ts` / `hmdel_key`, and `mode` =
  `4, 5, 6, 127, 128, 200, 254, 255, 256, 257, 258, 259, 260, 511, 512, -1, -2,
  -3, -4, -255, -256, INT_MAX, INT_MIN` for `stbds_shmode_func` (verifying the
  `(unsigned char)` truncation and which `switch` arm it selects);
* one step past every documented range: `min_cap` 3/4/5, `arrcap` and
  `arrcap+1`, element counts 6/7/8/9 around `used_count_threshold`, arena
  `block` 21/22/23/24/25 around the `1<<20` saturation, and 110..=127 /
  128..=160 around the shift-width boundary.
