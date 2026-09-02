# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `STBDS_ASSERT`,
every early `return` that signals "nothing done / not found", every `NULL`
check, every sentinel constant, and every min/max constant. `STBDS_ASSERT` is
`assert` and `NDEBUG` is **not** defined by `c_src/CMakeLists.txt` (no
`CMAKE_BUILD_TYPE`, no `-DNDEBUG`), so all asserts are live and a violation
calls `__assert_fail` → `abort()` (SIGABRT).

Sentinels / constants: `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`, `STBDS_HM_BINARY = 0`,
`STBDS_HM_STRING = 1`, `STBDS_SH_NONE/DEFAULT/STRDUP/ARENA = 0/1/2/3`,
`STBDS_BUCKET_LENGTH = 8`, `STBDS_STRING_ARENA_BLOCKSIZE_MIN = 512`,
`STBDS_STRING_ARENA_BLOCKSIZE_MAX = 1<<20`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| 1 | `stbds_arrgrowf` (lib.c:286) | `min_cap <= stbds_arrcap(a)` (incl. `a==NULL, addlen==0, min_cap==0`) — nothing to grow | returns `a` unchanged (may be `NULL`); no realloc, capacity untouched | `err_01_arrgrowf_nogrow` | [x] |
| 2 | `stbds_arrgrowf` (lib.c:300) | `a == NULL` | fresh header with `length=0, hash_table=NULL, temp=0`, `capacity=max(min_cap,addlen,4)` | `err_02_arrgrowf_null_a` | [x] |
| 3 | `stbds_make_hash_index` (lib.c:401) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count ∈ {0,1,2}` | `assert` fires → `abort()` (SIGABRT) | `err_03_make_hash_index_assert` (invariant checked; unreachable) | [x] |
| 4 | `stbds_hmfree_func` (lib.c:573) | `a == NULL` | returns immediately, no free | `err_04_hmfree_null` | [x] |
| 5 | `stbds_hmfree_func` (lib.c:574) | `stbds_hash_table(a) == NULL` (array built by `arrgrowf`, never by `hmput`) | skips strdup-key sweep and `strreset`; still frees `hash_table` (NULL) and header | `err_05_hmfree_no_table` | [x] |
| 6 | `stbds_hm_find_slot` (lib.c:610) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` in the `i = pos&MASK .. 7` scan → key absent | returns `-1` | `err_06_07_find_slot_absent` | [x] |
| 7 | `stbds_hm_find_slot` (lib.c:621) | probe hits `STBDS_HASH_EMPTY` in the wrap-around `0 .. pos&MASK` scan → key absent | returns `-1` | `err_06_07_find_slot_absent` | [x] |
| 8 | `stbds_hmget_key_ts` (lib.c:634) | `a == NULL` | allocates 1-elem zeroed array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash ptr | `err_08_hmget_ts_null_a` | [x] |
| 9 | `stbds_hmget_key_ts` (lib.c:644) | `a != NULL` but `hash_table == 0` | `*temp = -1`, returns `a` unchanged (key never hashed → no deref of `key`) | `err_09_20_no_table_shortcircuit` | [x] |
| 10 | `stbds_hmget_key_ts` (lib.c:648) | `stbds_hm_find_slot() < 0` (key not present) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `err_10_11_missing_key_temp` | [x] |
| 11 | `stbds_hmget_key` (lib.c:660) | same three conditions as 8–10 | writes `temp` into `stbds_header(arr)->temp`; `-1` for "absent" | `err_10_11_missing_key_temp` | [x] |
| 12 | `stbds_hmput_default` (lib.c:669) | `a == NULL` | allocates 1-elem zeroed array, returns hash ptr with `length==1` | `err_12_13_14_hmput_default` | [x] |
| 13 | `stbds_hmput_default` (lib.c:669) | `a != NULL` and `stbds_header(arr)->length == 0` | re-grows and bumps length to 1 (arrgrowf on the *raw* pointer) | `err_12_13_14_hmput_default` | [x] |
| 14 | `stbds_hmput_default` (lib.c:675) | `a != NULL` and `length != 0` | returns `a` completely unchanged | `err_12_13_14_hmput_default` | [x] |
| 15 | `stbds_hmput_key` (lib.c:686) | `a == NULL` | bootstraps 1-elem zeroed array before proceeding | `err_15_16_hmput_bootstrap` | [x] |
| 16 | `stbds_hmput_key` (lib.c:698) | `table == NULL` | builds `slot_count = STBDS_BUCKET_LENGTH (8)` index; `string.mode = SH_DEFAULT` iff `mode >= STBDS_HM_STRING` else `0` | `err_15_16_hmput_bootstrap` | [x] |
| 17 | `stbds_hmput_key` (lib.c:778) | `(size_t)i+1 > stbds_arrcap(a)` after the grow | `assert` fires → `abort()` (unreachable: `arrgrowf` guarantees capacity) | `err_17_hmput_cap_assert` (invariant checked; unreachable) | [x] |
| 18 | `stbds_hmput_key` (lib.c:791) | `table->string.mode` is not `SH_STRDUP/SH_ARENA/SH_DEFAULT` (i.e. `SH_NONE` or an out-of-range `unsigned char`) | falls into `default:` → raw `memcpy` of `keysize` bytes, `temp_key` NOT written | `err_18_hmput_mode_default_branch` | [x] |
| 19 | `stbds_hmdel_key` (lib.c:809) | `a == NULL` | returns `0` (NULL) — the only NULL return in the library | `err_19_hmdel_null_a` | [x] |
| 20 | `stbds_hmdel_key` (lib.c:816) | `hash_table == 0` | sets `stbds_temp(raw_a) = 0`, returns `a`; **no** key deref | `err_09_20_no_table_shortcircuit` | [x] |
| 21 | `stbds_hmdel_key` (lib.c:822) | `stbds_hm_find_slot() < 0` (key absent) | `stbds_temp(raw_a) == 0`, returns `a`, length/used_count unchanged | `err_21_hmdel_absent_key` | [x] |
| 22 | `stbds_hmdel_key` (lib.c:828) | `slot >= (ptrdiff_t) table->slot_count` | `assert` → `abort()` (unreachable: `find_slot` masks with `slot_count-1`) | `err_22_hmdel_slot_assert` (invariant checked; unreachable) | [x] |
| 23 | `stbds_hmdel_key` (lib.c:832) | `table->used_count >= 0` — `used_count` is `size_t`, so always true even after wrap | assert never fires, even when `used_count` underflows to `SIZE_MAX` | `err_23_hmdel_used_count_wrap` | [x] |
| 24 | `stbds_hmdel_key` (lib.c:846) | re-lookup of the moved-in tail element returns `slot < 0` | `assert` → `abort()` | `err_24_25_hmdel_relookup_invariants` (invariant checked; unreachable) | [x] |
| 25 | `stbds_hmdel_key` (lib.c:849) | `b->index[i] != final_index` on the re-lookup | `assert` → `abort()` | `err_24_25_hmdel_relookup_invariants` (invariant checked; unreachable) | [x] |
| 26 | `stbds_stralloc` (lib.c:913) | `len > a->remaining` after the block allocation | `assert` → `abort()`. Reachable: an arena whose `storage` is a *big-string* block (`remaining == 0`) is fine, but a hand-crafted arena with `remaining` large and `storage == NULL` dereferences NULL instead | `crash_equivalence_all_scenarios` / `stralloc_null_storage` | [x] |
| 27 | `stbds_stralloc` (lib.c:895) | `len > blocksize` (string longer than the current arena block size) | dedicated over-size block; if `a->storage == NULL` also sets `remaining = 0`; returns `sb->storage` | `err_27_stralloc_oversize` | [x] |
| 28 | `stbds_stralloc` (lib.c:891) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` stops incrementing (saturates at 22/23) | `err_28_stralloc_block_saturate` | [x] |
| 29 | `stbds_strreset` (lib.c:923) | `a->storage == NULL` (already-empty / zeroed arena) | frees nothing, memsets the arena to 0 | `err_29_strreset_empty` | [x] |
| 30 | `str_dups` (lib.c:960) | `*strmap[0].key != 'a'` | `assert` → `abort()` (unreachable: key is `"a"`) | `crash_equivalence_all_scenarios` / `str_dups_ok` (must exit 0) | [x] |
| 31 | `str_dups` (lib.c:961) | `strmap[0].key == s.key` — i.e. `SH_STRDUP` failed to duplicate | `assert` → `abort()` (unreachable) | `crash_equivalence_all_scenarios` / `str_dups_ok` (must exit 0) | [x] |
| 32 | `str_dups` (lib.c:962) | `strmap[0].value != s.value` | `assert` → `abort()` (unreachable) | `crash_equivalence_all_scenarios` / `str_dups_ok` (must exit 0) | [x] |
| 33 | `str_dups` (lib.c:947) | `num <= 0` (0 or negative) — loop body never runs | no `stralloc` calls; still runs the strdup-map block and prints `a <num>` | `err_33_strdups_nonpositive` | [x] |
| 34 | `stbds_hash_bytes` / `stbds_siphash_bytes` (lib.c:504) | `len == 0` (and `p == NULL`) | no byte is read; `data = 0 << 56`, tail switch takes `case 0: break` | `err_34_hash_bytes_len0` | [x] |
| 35 | `stbds_hash_bytes` tail (lib.c:530 `case 4`) | tail byte `d[3] >= 0x80` → `d[3] << 24` is a negative `int`, **sign-extended** into `size_t` | upper 32 bits of `data` become all ones (an stb bug, must be reproduced) | `err_35_hash_bytes_sign_extend` | [x] |
| 36 | `stbds_hash_string` (lib.c:465) | empty string `""` | loop never runs; hash is a pure function of `seed` | `err_36_hash_string_empty` | [x] |
| 37 | `stbds_hm_find_slot` / `stbds_hmput_key` | computed `hash < 2` (collides with `HASH_EMPTY`/`HASH_DELETED`) | `hash += 2` fix-up before probing | `err_37_hash_lt_2_fixup` | [x] |
| 38 | all `mode` params | out-of-range `int` enum values across the FFI boundary: `mode = 2` (`STBDS_HM_PTR_TO_STRING`), `3`, `255`, `-1`, `INT_MIN`, `INT_MAX` | `mode >= STBDS_HM_STRING` → string path for `>=1`; binary for `<=0`. In `hmdel_key` the strdup-free and key-reload branches test `mode == STBDS_HM_STRING` **exactly**, so `mode==2` takes the *binary* reload branch while `find_slot` still uses *string* hashing | `err_38_mode_out_of_range` | [x] |
| 39 | `stbds_shmode_func` | out-of-range `mode`: `-1`, `4`, `255`, `256`, `INT_MAX` | `h->string.mode = (unsigned char) mode` — truncated mod 256 (`-1 → 255`, `256 → 0`) | `err_39_shmode_out_of_range` | [x] |
| 40 | `stbds_arrfreef` | `a == NULL` | `free(stbds_header(NULL))` = `free((char*)0 - 32)` → invalid free / undefined; **not** a guarded path | `crash_equivalence_all_scenarios` / `arrfreef_null` (both SIGSEGV) | [x] |
| 41 | `stbds_hmget_key_ts` | `temp == NULL` with `a == NULL` | unconditional `*temp = STBDS_INDEX_EMPTY` → NULL deref, SIGSEGV in both | `crash_equivalence_all_scenarios` / `hmget_ts_null_temp` (both SIGSEGV) | [x] |
| 42 | `stbds_hmput_key` | `keysize == 0` with `mode == STBDS_HM_BINARY` | `memcmp(...,0) == 0` always true → the first probed slot with a matching hash "equals" any key; `memcpy(...,0)` copies nothing | `err_42_keysize_zero` | [x] |
| 43 | `stbds_hmput_key` | `elemsize == 0` | `arrgrowf(0, 0, 0, 1)` allocates only the header; all `elemsize*i` offsets collapse to 0 → every entry aliases | `err_43_elemsize_zero` | [x] |
| 44 | `stbds_arrgrowf` | oversized request, e.g. `elemsize = SIZE_MAX/2, min_cap = 4` → `realloc` returns `NULL` | C writes through `NULL + sizeof(header)` → SIGSEGV; identical in Rust | `crash_equivalence_all_scenarios` / `arrgrowf_oversize` + `arrgrowf_realloc_fail` | [x] |
| 45 | `strkey` | negative / extreme `n` (`INT_MIN`, `INT_MAX`) | `sprintf(buffer, "test_%d", n)` — fits in the 256-byte static buffer | `err_45_strkey_extremes` | [x] |

## Notes on the rows that cannot be triggered directly

* **Rows 3, 17, 22, 24, 25** — asserts that are unreachable through the exported
  API. Rather than skipping them, each has a test that continuously checks the
  *condition the assert guards* over a long randomized churn on both
  implementations (`slot_count` is always a power of two ≥ 8;
  `length <= capacity` after every put; every bucket index is `-1`, `-2`, or a
  live element index; every remaining key stays reachable after each delete).
  If the Rust translation ever diverged so as to make one of these asserts fire,
  the invariant check would catch it first.
* **Row 26** — the `len <= a->remaining` assert at lib.c:913 is itself
  unreachable (the `else` branch always sets `remaining = blocksize >= len`, and
  the `len > blocksize` branch returns early). The *reachable* failure for a
  malformed arena is the `a->storage->storage` dereference with
  `storage == NULL`, which is what scenario `stralloc_null_storage` compares.
* **Row 37** — a 64-bit hash below 2 cannot be found by search (probability
  ≈ 2⁻⁶³ per input). The test instead pins that both implementations produce
  bit-identical hashes over 40 000 randomized inputs, so they would take the
  `hash += 2` branch on exactly the same inputs.
* **Row 43** — `elemsize == 0` combined with `keysize > 0` makes the C `memcpy`
  write `keysize` bytes into a zero-byte element region, i.e. heap corruption
  whose observable consequences are allocator-state dependent. The test uses
  `keysize == 0`, the only combination with well-defined, comparable behaviour.
* **Rows 38 (mode == 2 + middle delete)** — `hmdel_key` with `mode != 1` on a
  *string* map hashes the element's raw pointer **bytes** when re-locating the
  moved-in tail element. Those bytes are heap addresses, which legitimately
  differ between two independently allocated libraries, so full state equality
  is not a meaningful assertion there. The test covers every deterministic part
  of that configuration (put, get, delete-absent, delete-last).

## Result

All 45 rows have a passing differential test. See `FEATURE_MATRIX.md`.
