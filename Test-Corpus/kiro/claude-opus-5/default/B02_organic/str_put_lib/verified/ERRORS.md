# ERRORS.md — error / rejection surface table

Derived mechanically from every `return`-early, `STBDS_ASSERT`, sentinel value
and range/null check in `c_src/src/lib.c`.  `stb_ds` has **no error enum and no
`errno`**: it signals "rejection" through (a) sentinel `ptrdiff_t` values
(`STBDS_INDEX_EMPTY == -1`, `STBDS_INDEX_DELETED == -2`), (b) a `NULL` return,
(c) an unchanged return value plus a `temp` flag, or (d) `assert()` abort.

Legend for "expected C result": the value the C function returns **and** any
out-parameter / `stbds_array_header::temp` side effect, because that is what the
`stbds_*` macros actually consume.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap` (growth not needed) — lib.c:287 | returns `a` **unchanged**, no realloc, capacity untouched |
| 2 | `stbds_arrgrowf` | `a == NULL` (no existing buffer) | fresh alloc; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_len,min_cap,4)` |
| 3 | `stbds_arrgrowf` | `addlen == 0 && min_cap == 0` on `a == NULL` → `min_len = 0`, `min_cap = 0`; `0 <= arrcap(NULL)==0` | returns `NULL` (early-out at lib.c:287, **no allocation**) |
| 4 | `stbds_arrgrowf` | `min_cap` in `1..=3` with `a == NULL` | `min_cap` bumped to `4` (`else if (min_cap < 4)`) |
| 5 | `stbds_arrgrowf` | huge `elemsize * min_cap` (e.g. `elemsize=SIZE_MAX/2`) | `realloc` returns `NULL`; C dereferences `NULL + 32` → SIGSEGV (not a checked error) |
| 6 | `stbds_arrfreef` | `a == NULL` | `free((stbds_array_header*)NULL - 1)` = `free((void*)0xffff…ffe0)` → glibc abort/SIGSEGV. **Not** a checked error; both impls must be equally hazardous (untestable, documented only) |
| 7 | `stbds_make_hash_index` (static) | `used_count_threshold + tombstone_count_threshold >= slot_count` — lib.c:401 | `assert` abort. Unreachable for power-of-two `slot_count >= 8` (ratio 0.9375 < 1) |
| 8 | `stbds_hm_find_slot` (static) | probe reaches a bucket entry with `hash == STBDS_HASH_EMPTY` (0) before a match — lib.c:610 / lib.c:621 | returns `-1` (key not present) |
| 9 | `stbds_hmget_key_ts` | `a == NULL` | allocates a 1-element array (the "default" element), `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` (non-NULL) |
| 10 | `stbds_hmget_key_ts` | `a != NULL` but `stbds_header(a-elemsize)->hash_table == NULL` (array exists, no index yet) | `*temp = -1`, returns `a` unchanged |
| 11 | `stbds_hmget_key_ts` | key absent from a populated table (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` unchanged |
| 12 | `stbds_hmget_key` | same three cases as #9/#10/#11 | additionally writes the sentinel into `stbds_header(ret-elemsize)->temp` |
| 13 | `stbds_hmget_key` / `stbds_hmget_key_ts` | `mode` out of enum range (`2`, `7`, `INT_MAX`, `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING(1)` ⇒ **string** path (`stbds_hash_string` + `strcmp`); any `mode <= 0` ⇒ **binary** path (`stbds_hash_bytes` + `memcmp`). No validation, no rejection |
| 14 | `stbds_hmput_default` | `a == NULL` | allocates, `length = 1`, element 0 zeroed, returns `arr+elemsize` |
| 15 | `stbds_hmput_default` | `a != NULL` **and** `stbds_header(a-elemsize)->length == 0` | re-grows/zeroes and bumps `length` to 1 |
| 16 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (no allocation, default element preserved) |
| 17 | `stbds_hmput_key` | `a == NULL` | bootstraps 1-element array before inserting |
| 18 | `stbds_hmput_key` | `table == NULL` (first put) | new index with `slot_count = STBDS_BUCKET_LENGTH (8)`; `string.mode = (mode >= 1) ? STBDS_SH_DEFAULT : 0` |
| 19 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` (6 of 8 slots used) | rehash into `slot_count*2`; old index freed; `string`/`seed` inherited |
| 20 | `stbds_hmput_key` | `i+1 > stbds_arrcap(a)` was not satisfiable, i.e. `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` — lib.c:778 | `assert` abort (only if `arrgrowf` failed to grow) |
| 21 | `stbds_hmput_key` | duplicate key found in the **first** in-bucket scan | `temp = existing index`, and for `mode >= 1` **`temp_key` is also updated**; array length unchanged |
| 22 | `stbds_hmput_key` | duplicate key found in the **wrap-around** scan (`i < limit`) | `temp = existing index`, **`temp_key` is NOT updated** (asymmetry in the C, must be reproduced) |
| 23 | `stbds_hmput_key` | `table->string.mode` not one of `STRDUP/ARENA/DEFAULT` (i.e. `STBDS_SH_NONE` or any other `unsigned char`) | `default:` branch → `memcpy(elem, key, keysize)`; `temp_key` NOT written |
| 24 | `stbds_hmput_key` | `keysize == 0` with `string.mode == 0` | `memcpy(dst, key, 0)` — no bytes written, element stays zero-initialised garbage from `realloc` |
| 25 | `stbds_shmode_func` | `mode` outside `{0,1,2,3}` (e.g. `4`, `259`, `-1`, `255`) | `h->string.mode = (unsigned char) mode` — value truncated mod 256, **no validation**. `259 → 3 (ARENA)`, `-1 → 255` (falls to `default:` memcpy in `hmput_key`) |
| 26 | `stbds_hmdel_key` | `a == NULL` | returns `0` (NULL) — the only NULL-return sentinel in the library |
| 27 | `stbds_hmdel_key` | `hash_table == NULL` | sets `stbds_header(raw_a)->temp = 0`, returns `a` unchanged |
| 28 | `stbds_hmdel_key` | key not found (`slot < 0`) | `temp = 0`, returns `a` unchanged, `length` unchanged |
| 29 | `stbds_hmdel_key` | key found | `temp = 1`, `used_count--`, `tombstone_count++`, slot hash←`STBDS_HASH_DELETED(1)`, index←`STBDS_INDEX_DELETED(-2)`, `length--` |
| 30 | `stbds_hmdel_key` | `slot >= table->slot_count` — `STBDS_ASSERT` lib.c:828 | `assert` abort (unreachable: `find_slot` masks with `slot_count-1`) |
| 31 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` lib.c:832 | tautology for `size_t`; never fires |
| 32 | `stbds_hmdel_key` | re-find of the moved last element fails — `STBDS_ASSERT(slot >= 0)` lib.c:846 | `assert` abort |
| 33 | `stbds_hmdel_key` | re-found slot's index ≠ `final_index` — `STBDS_ASSERT(b->index[i] == final_index)` lib.c:849 | `assert` abort |
| 34 | `stbds_hmdel_key` | after delete `used_count < used_count_shrink_threshold && slot_count > 8` | index rebuilt at `slot_count>>1` |
| 35 | `stbds_hmdel_key` | after delete `tombstone_count > tombstone_count_threshold` | index rebuilt at the same `slot_count` |
| 36 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **exactly** (not `>= 1`) and `string.mode == STRDUP` | stored key `free()`d. `mode == 2` (also a "string" mode for hashing) does **not** free — asymmetry to reproduce |
| 37 | `stbds_hmfree_func` | `a == NULL` | returns immediately, no free |
| 38 | `stbds_hmfree_func` | `hash_table == NULL` | skips arena/strdup teardown, still `free`s `hash_table` (NULL) and the header |
| 39 | `stbds_hmfree_func` | `string.mode == STBDS_SH_STRDUP` | frees `*(char**)(a + elemsize*i)` for `i` in `1..length` (element 0 skipped) |
| 40 | `stbds_stralloc` | `len <= a->remaining` | no allocation; carve from the tail of the current block |
| 41 | `stbds_stralloc` | `len > a->remaining` and `len <= blocksize` | new block of `blocksize`, pushed at head, `remaining = blocksize` |
| 42 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` and `a->storage != NULL` | oversized block spliced **after** head; `a->remaining` left **unchanged**; returns `sb->storage` directly |
| 43 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` and `a->storage == NULL` | oversized block becomes head, `a->remaining = 0`; returns `sb->storage` |
| 44 | `stbds_stralloc` | `a->block` such that `512 << (block>>1) >= 1<<20` | `a->block` stops incrementing (saturates at 22 when driven from 0) |
| 45 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` lib.c:913 | `assert` abort (unreachable through the normal paths above) |
| 46 | `stbds_stralloc` | empty string `""` (`len == 1`) | allocates 1 byte from the arena; distinct pointer each call |
| 47 | `stbds_strreset` | `a->storage == NULL` (already empty / fresh arena) | no frees; whole 24-byte struct memset to 0 (idempotent) |
| 48 | `stbds_hash_bytes` | `len == 0` | no full-word rounds, no tail bytes: `data = 0`, still runs C+D rounds → deterministic non-zero digest |
| 49 | `stbds_hash_bytes` | `len - i == 4` (and 3, 2, 1) tail | `data |= (d[3] << 24)` is an `int` expression → **sign extends** into `size_t` when `d[3] >= 0x80`. Same for the full-word loads. Must be reproduced bit-for-bit |
| 50 | `stbds_hash_string` | empty string `""` | while-loop never runs; digest is a pure function of `seed` |
| 51 | `stbds_hash_string` | bytes with the high bit set (`0x80..0xFF`) | added as `(unsigned char)`, i.e. zero-extended (unlike `hash_bytes`) |
| 52 | `stbds_hash_string` / `stbds_hash_bytes` | `p == NULL` | unconditional deref → SIGSEGV; no null check in C (documented, not tested) |
| 53 | `strkey` | `n < 0` / `n == INT_MIN` | `sprintf(buffer, "test_%d", n)` → `"test_-2147483648"`; fits in the 256-byte buffer, no overflow |
| 54 | `str_put` | `num <= 0` | the `stralloc` loop body never executes; the `shputs`/asserts/`shfree` block still runs |
| 55 | `str_put` | `STBDS_ASSERT(*strmap[0].key == 'a')` lib.c:958 | `assert` abort if the string-mode key round-trip broke |
| 56 | `str_put` | `STBDS_ASSERT(strmap[0].key == s.key)` lib.c:959 | `assert` abort — requires `string.mode == STBDS_SH_DEFAULT` (pointer stored verbatim) |
| 57 | `str_put` | `STBDS_ASSERT(strmap[0].value == s.value)` lib.c:960 | `assert` abort |

## Coverage note

Every row above has a differential test in `tests/errors.rs` (test names are
prefixed `eNN_` after the row number; several closely-related rows share one
test, e.g. `e09_e10_e11_e12_get_sentinels`).

The rows whose trigger *terminates the process* are verified by running the call
in a **forked child** and comparing `(exit code, terminating signal)` between the
two libraries:

| row | test | observed outcome (both libraries) |
|-----|------|-----------------------------------|
| 5 | `e05_arrgrowf_allocation_failure_crashes_identically` | identical signal |
| 6 | `e06_arrfreef_null_crashes_identically` | identical signal |
| 52 | `e52_hash_null_pointer_crashes_identically` | identical signal |
| — | `e_boundary_oversized_hash_len_crashes_identically` (oversized `len`) | identical signal |
| — | `e_boundary_hmget_null_key_crashes_identically` (NULL key) | identical signal |

These five are asserted against the **release** `cdylib` — the crate's shipped
configuration (`crate-type = ["cdylib"]`, `[profile.release] panic = "abort"`).
Against a `dev`-profile `.so` they self-skip with a message, because a debug
build converts some of those raw-pointer faults into a Rust panic.  Everything
else passes identically under both profiles, which also confirms the translation
has no arithmetic-overflow-check panics.

Rows 7, 20, 30, 31, 32, 33, 45, 55, 56 and 57 are `STBDS_ASSERT`s whose trigger
is provably unreachable through the public ABI; `e_documented_only_rows_are_unreachable`
records the proofs (e.g. for row 7, `(sc - sc/4) + (sc/8 + sc/16) = 0.9375·sc < sc`
for every power-of-two `sc ≥ 8`, checked for `log2 = 3..40`).

### One non-obvious hazard found while testing

`stbds_make_hash_index` copies only `string` and `seed` from the old table, so
`stbds_hash_index::temp_key` is **uninitialised after every rehash**.  Because
`hmput_key` rehashes *before* checking for a duplicate, a duplicate `shput` that
crosses `used_count_threshold` returns with `stbds_temp_key` pointing at
uninitialised heap bytes — which `stbds_shputs` then stores into the element.
This is C behaviour, faithfully reproduced; the tests prime the field so the
comparison does not depend on garbage (see `CONFIGS.md`).
