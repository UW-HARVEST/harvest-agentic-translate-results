# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`,
`STBDS_ASSERT` (`= assert`, **live** because CMake's default build type passes
no `-DNDEBUG`), explicit range/null check, and min/max constant. Line numbers
refer to `c_src/src/lib.c`.

Sentinel constants the C uses as its "error" values:

| constant | value | meaning |
|---|---|---|
| `STBDS_INDEX_EMPTY` | `-1` | slot/index not found (written to `*temp` / `header->temp`) |
| `STBDS_INDEX_DELETED` | `-2` | tombstone marker in `bucket->index[]` |
| `STBDS_HASH_EMPTY` | `0` | never-used slot in `bucket->hash[]` |
| `STBDS_HASH_DELETED` | `1` | tombstoned slot in `bucket->hash[]` |
| `stbds_hm_find_slot` failure | `-1` | key absent |
| `stbds_hmdel_key(NULL,…)` | `NULL` | nothing to delete |
| `header->temp` after `hmdel_key` | `0` / `1` | 0 = not deleted, 1 = deleted |

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|---------------------------------------------|-------------------|---|
| 1 | `stbds_arrgrowf` | `a != NULL`, `max(min_cap, arrlen+addlen) <= capacity` (line 287) | returns `a` **unchanged** (same pointer, capacity untouched) | [x] |
| 2 | `stbds_arrgrowf` | `a == NULL, addlen == 0, min_cap == 0` → `min_len 0 <= arrcap(NULL) 0` | returns **`NULL`**, no allocation | [x] |
| 3 | `stbds_arrgrowf` | `a == NULL`, `min_cap` in 1..3 → `min_cap < 2*0` false, `min_cap < 4` true | capacity forced up to **4** | [x] |
| 4 | `stbds_arrgrowf` | `addlen == SIZE_MAX` (wrapping `min_len`) with `a == NULL` | `min_len` wraps to `SIZE_MAX`; `min_cap = SIZE_MAX`; `realloc` of a wrapped byte count fails → C writes through `NULL+32` (crash). Not exercised (OOM/UB). | n/a |
| 5 | `stbds_arrfreef` | `a == NULL` | no null check: `free((char*)NULL - 32)` → invalid free / abort. Not exercised (UB, would kill the test process). | n/a |
| 6 | `stbds_hash_bytes` | `p == NULL, len == 0` | **no dereference** (`switch(len-i)` takes `case 0`); returns the hash of the empty message | [x] |
| 7 | `stbds_hash_bytes` | `len` remainder `len % 8` in 1..7 with a tail byte `>= 0x80` at offset 3 | `d[3] << 24` is an `int` → **sign-extends** through `data |= …`, setting the top 32 bits | [x] |
| 8 | `stbds_hash_bytes` | `len >= 8` with `d[3] >= 0x80` in any full block | same `int` sign-extension inside the main loop | [x] |
| 9 | `stbds_hash_string` | empty string `""` | loop body never runs; finalizer still applied to `hash = seed` | [x] |
| 10 | `stbds_hash_string` | bytes `>= 0x80` | added as `(unsigned char)`, i.e. **not** sign-extended (unlike `hash_bytes`) | [x] |
| 11 | `stbds_hash_string` | `str == NULL` | dereferences immediately → segfault. Not exercised (UB). | n/a |
| 12 | `stbds_hmfree_func` | `a == NULL` (line 573) | returns immediately, **no free** | [x] |
| 13 | `stbds_hmfree_func` | `a != NULL`, `header->hash_table == NULL` (line 574) | skips `strreset`; `free(NULL)`; frees header | [x] |
| 14 | `stbds_hmfree_func` | `hash_table->string.mode == STBDS_SH_STRDUP` (line 575) | additionally frees `*(char**)(a + elemsize*i)` for `i` in `1..length` | [x] |
| 15 | `stbds_hm_find_slot` | probe reaches a slot with `hash[i] == STBDS_HASH_EMPTY` (lines 610, 621) | returns `-1` | [x] |
| 16 | `stbds_hmget_key_ts` | `a == NULL` (line 634) | bootstraps a 1-element zeroed array, `length = 1`, `*temp = -1`, returns `arr + elemsize`; `key`/`keysize`/`mode` **ignored** (`key` may be `NULL`) | [x] |
| 17 | `stbds_hmget_key_ts` | `a != NULL`, `header->hash_table == 0` (line 644) | `*temp = -1`, returns `a` **unchanged** | [x] |
| 18 | `stbds_hmget_key_ts` | key absent, `slot < 0` (line 648) | `*temp = STBDS_INDEX_EMPTY (-1)` | [x] |
| 19 | `stbds_hmget_key` | `a == NULL` | as row 16, and `header(result-elemsize)->temp = -1` | [x] |
| 20 | `stbds_hmget_key` | `hash_table == 0` | `header(a-elemsize)->temp = -1`, returns `a` | [x] |
| 21 | `stbds_hmget_key` | key absent | `header(a-elemsize)->temp = -1` | [x] |
| 22 | `stbds_hmput_default` | `a == NULL` (line 669) | fresh 1-element zeroed array, `length = 1` | [x] |
| 23 | `stbds_hmput_default` | `a != NULL` and `header(a-elemsize)->length == 0` | regrows and sets `length = 1` (zeroing element 0) | [x] |
| 24 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** — the request is rejected | [x] |
| 25 | `stbds_hmput_key` | `a == NULL` (line 686) | bootstraps a 1-element zeroed array before inserting | [x] |
| 26 | `stbds_hmput_key` | key already present, found in the **upper** sub-loop (line 730) | early return, **no insert**, `header->temp = existing index`; if `mode >= 1` also `temp_key = stored key ptr` | [x] |
| 27 | `stbds_hmput_key` | key already present, found in the **wrap-around** sub-loop (line 746) | early return, `header->temp` set, but `temp_key` is **NOT** updated (asymmetry in the C) | [x] |
| 28 | `stbds_hmput_key` | `hash < 2` after hashing (line 720) | `hash += 2` so it can never collide with `HASH_EMPTY`/`HASH_DELETED` | [x] |
| 29 | `stbds_hmput_key` | `table == NULL` (first insert) | `slot_count = 8`; on a **fresh** table `string.mode = (mode >= 1) ? SH_DEFAULT : 0` | [x] |
| 30 | `stbds_hmput_key` | `used_count >= used_count_threshold` (line 698) | table doubled (`slot_count*2`), old table freed, entries rehashed | [x] |
| 31 | `stbds_hmput_key` | a tombstone was seen before the empty slot (line 766) | insert reuses the tombstone slot, `--tombstone_count` | [x] |
| 32 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= arrcap(a))` (line 778) | `abort()` via `__assert_fail`. Unreachable through the public API (preceding grow guarantees it). | n/a |
| 33 | `stbds_hmput_key` | `mode` out of the `{0,1}` enum range: `2, 3, 99, INT_MAX` | `mode >= STBDS_HM_STRING` ⇒ treated as **string** mode (`strcmp` compares, `stbds_hash_string` hashes) | [x] |
| 34 | `stbds_hmput_key` | `mode` negative: `-1, INT_MIN` | `mode >= 1` false ⇒ treated as **binary** mode, `string.mode = 0` on a fresh table | [x] |
| 35 | `stbds_hmput_key` | `table->string.mode` not in `{1,2,3}` (default arm, line 789) | `memcpy(a + elemsize*i, key, keysize)` — the key **bytes** are copied, not the pointer | [x] |
| 36 | `stbds_hmput_key` | `keysize == 0` in binary mode | `memcmp(...,0) == 0` always ⇒ every key with an equal hash matches; `memcpy(...,0)` stores nothing | [x] |
| 37 | `stbds_shmode_func` | `mode` out of the `{0,1,2,3}` enum range | no validation; `string.mode = (unsigned char) mode`, i.e. **truncated mod 256** (`256 → 0`, `-1 → 255`, `259 → 3 = SH_ARENA`) | [x] |
| 38 | `stbds_hmdel_key` | `a == NULL` (line 809) | returns **`NULL`** (`0`) | [x] |
| 39 | `stbds_hmdel_key` | `header(a-elemsize)->hash_table == 0` (line 816) | `header->temp = 0`, returns `a` unchanged | [x] |
| 40 | `stbds_hmdel_key` | key absent, `slot < 0` (line 821) | `header->temp = 0` (not-deleted flag), returns `a` unchanged, `length` untouched | [x] |
| 41 | `stbds_hmdel_key` | key present | `header->temp = 1`, `--used_count`, `++tombstone_count`, slot ⇒ `hash = 1`/`index = -2`, `--length` | [x] |
| 42 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` (line 828) | `abort()`; unreachable — `find_slot` only returns in-range slots | n/a |
| 43 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` (line 832) | `used_count` is `size_t`, so the condition is **always true** — dead assert, never fires | [x] |
| 44 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` (line 846) after re-finding the moved last element | `abort()`; unreachable for consistent tables | n/a |
| 45 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` (line 849) | `abort()`; unreachable for consistent tables | n/a |
| 46 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` is an **exact** `==` (lines 835, 841) while hashing uses `>= 1` | with `mode == 2` the key is *hashed/compared* as a string but the moved element is re-found as **binary** — a distinct, reproducible behaviour | [x] |
| 47 | `stbds_hmdel_key` | deleting the element that is already `final_index` | the `memmove` + re-find branch is skipped entirely | [x] |
| 48 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` (line 854) | table rebuilt at `slot_count >> 1` | [x] |
| 49 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (line 858) | table rebuilt at the **same** `slot_count` (tombstones purged) | [x] |
| 50 | `stbds_hmdel_key` | `slot_count == 8` (`shrink_threshold` forced to 0, line 404) | never shrinks below 8 slots | [x] |
| 51 | `stbds_stralloc` | `len <= a->remaining` | no allocation; carves from the current block, `remaining -= len` | [x] |
| 52 | `stbds_stralloc` | `len > remaining` and `len <= blocksize` (line 906) | new block prepended, `remaining = blocksize`, then carve | [x] |
| 53 | `stbds_stralloc` | `len > remaining` and `len > blocksize`, `a->storage != NULL` (line 894) | oversized block spliced in **after** the head (`sb->next = storage->next; storage->next = sb`), `remaining` untouched, returns `sb->storage` | [x] |
| 54 | `stbds_stralloc` | `len > remaining` and `len > blocksize`, `a->storage == NULL` | `sb->next = 0; a->storage = sb; a->remaining = 0`, returns `sb->storage` | [x] |
| 55 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` (line 890) | `a->block` is **not** incremented — the block counter saturates | [x] |
| 56 | `stbds_stralloc` | caller pre-sets `a->block` large (e.g. `200`) → `512 << (block>>1)` shifts by ≥ 64 | `size_t` shift ≥ width; both sides must agree on the resulting `blocksize` and the branch taken | [x] |
| 57 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` (line 913) | `abort()`; reachable only with a hand-corrupted arena | n/a |
| 58 | `stbds_stralloc` | empty string `""` (`len == 1`) | 1 byte carved | [x] |
| 59 | `stbds_strreset` | `a->storage == NULL` | loop never runs; arena fully `memset` to 0 (`block` and `mode` cleared too) | [x] |
| 60 | `stbds_strreset` | `a == NULL` | dereferences `a->storage` → segfault. Not exercised (UB). | n/a |
| 61 | `strkey` | `n == INT_MIN` | `sprintf("test_%d")` → `"test_-2147483648"` (16 chars + NUL ≤ 256, no overflow) | [x] |
| 62 | `strkey` | `n == INT_MAX`, `0`, negatives | formatted into the **shared** `static char buffer[256]`; the previous return value is clobbered | [x] |
| 63 | `helxo` | `letter == 0` | stored as the `"jen"` value; `printf("%c")` emits a NUL byte into stdout | [x] |
| 64 | `helxo` | `letter` with the high bit set (`0x80..0xFF` / negative `char`) | `%c` converts the `int` to `unsigned char` → the raw byte is emitted | [x] |
| 65 | `helxo` | any `letter` | `printf("%s %c\n", hash[z], hash[z].value)` passes the 16-byte element **by value** to a varargs `%s`; SysV classifies it as `INTEGER,INTEGER`, so `%s` reads `.key` and `%c` reads the second eightbyte (low byte `.value`) | [x] |

## Not-testable rows

Rows 4, 5, 11, 32, 42, 44, 45, 57, 60 are `abort()`/segfault/OOM paths. Six of
them (32, 42, 44, 45, 57 and the row-4 OOM) are unreachable through the public
API on a consistent data structure; rows 5, 11 and 60 are missing null checks in
the C that would terminate the test process in **both** libraries identically.
They are recorded here for completeness but are deliberately not turned into
differential tests, per the instruction not to get stuck.

All 56 remaining rows have a differential test in
`translation/tests/errors.rs` (rows 1–65 minus the n/a set) and are checked off
above only after that test passed against both `.so`s.

---

## Row → test mapping (Phase C)

Every checked row above is covered by a named test in
`translation/tests/errors.rs` (plus the Phase B files where noted). All 30 tests
in `errors.rs` pass against both `.so`s in the `release` and `dev` profiles.

| rows | test |
|------|------|
| 1 | `errors::err01_arrgrowf_returns_same_pointer` |
| 2 | `errors::err02_arrgrowf_null_when_nothing_requested` |
| 3 | `errors::err03_arrgrowf_minimum_capacity_four` |
| 6 | `errors::err06_hash_bytes_null_zero_length` |
| 7, 8 | `errors::err07_08_hash_bytes_sign_extension` |
| 9, 10 | `errors::err09_10_hash_string_edges` |
| 12 | `errors::err12_hmfree_null` |
| 13 | `errors::err13_hmfree_without_hash_table` |
| 14 | `errors::err14_hmfree_strdup_frees_keys` |
| 15, 18, 21 | `errors::err15_18_21_absent_key_is_minus_one` |
| 16, 19 | `errors::err16_19_bootstrap_from_null_ignores_key` |
| 17, 20 | `errors::err17_20_no_hash_table_yields_minus_one` |
| 22, 23, 24 | `errors::err22_23_24_hmput_default` |
| 25, 29 | `errors::err25_29_hmput_key_bootstrap` |
| 26, 27 | `errors::err26_27_duplicate_hit_temp_key_asymmetry` |
| 28 | `errors::err28_reserved_hash_values_never_stored` |
| 30, 31 | `errors::err30_31_growth_and_tombstone_reuse` |
| 33, 34 | `errors::err33_34_out_of_range_mode_enum` |
| 35 | `errors::err35_string_mode_default_arm_memcpys` |
| 36 | `errors::err36_keysize_zero_always_matches` |
| 37 | `errors::err37_shmode_func_truncation` |
| 38 | `errors::err38_hmdel_null_returns_null` |
| 39 | `errors::err39_hmdel_without_hash_table` |
| 40, 41, 43, 47 | `errors::err40_41_43_47_delete_flag_and_paths` |
| 46 | `errors::err46_mode_two_skips_strdup_free` |
| 48, 49, 50 | `errors::err48_49_50_shrink_and_rebuild` |
| 51, 52, 53, 54, 55, 58 | `errors::err51_58_stralloc_branches` |
| 56 | `errors::err56_stralloc_shift_overflow` |
| 59 | `errors::err59_strreset_zeroes_everything` |
| 61, 62 | `errors::err61_62_strkey_extremes` |
| 63, 64, 65 | `helxo::helxo_rows_73_and_74` (all 256 `letter` values, stdout captured) |
| 42, 44, 45 | dead/unreachable asserts; the reachable structure is covered by `maps_binary::row45..row54` and `errors::err48_49_50_…` |

### Row 27 evidence

`err26_27_duplicate_hit_temp_key_asymmetry` inserts 4000 string keys, then for
each key poisons `table->temp_key` with a sentinel and re-puts the key. If
`temp_key` still holds the sentinel afterwards, the duplicate was found by the
wrap-around sub-loop (which does **not** write `temp_key`); otherwise by the
upper sub-loop (which does). The test asserts both libraries classify every one
of the 4000 keys identically, and that both classes are non-empty — so the C's
asymmetry is genuinely exercised, not merely assumed.

### Row 56 refinement

`512 << (block >> 1)` on x86-64 at `-O0` compiles to a variable `shl`, whose
count is taken mod 64. `block == 110/111` ⇒ shift 55 ⇒ `2^64` wraps to
`blocksize == 0`, so the oversized-block branch is taken for any length —
testable, and tested. `block == 128` ⇒ shift `64 & 63 == 0` ⇒ `blocksize == 512`
again. Values such as `block == 200` (shift `100 & 63 == 36` ⇒ `blocksize == 2^45`)
make the C request 32 TiB from `realloc`, get `NULL`, and write through it: that
is row 4's OOM crash and is excluded from the test, identically for both
libraries.
