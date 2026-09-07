# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`.  Every early-return sentinel,
`STBDS_ASSERT` (which is `<assert.h>`'s `assert`, live because `NDEBUG` is not
defined ⇒ `abort()`), explicit range/null check, and min/max constant gets one
row.

The C library has **no error enum and no error return codes**.  Its entire
rejection surface consists of:
* sentinel returns (`NULL` / `-1` / unchanged pointer / `temp = -1`), and
* fatal `assert()` aborts.

`assert`-abort rows are verified by comparing that *both* libraries abort
(SIGABRT via a forked child) — or, where the condition is unreachable from the
public API, they are marked *unreachable* with the reason.

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|----|----------|-------------------------------------------|-------------------|------|---|
| 1  | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after `min_len` fold (e.g. `a` non-NULL cap 4, `addlen=0`, `min_cap=0`) | returns `a` **unchanged**, no realloc, header untouched | `err_01_arrgrowf_noop` | [x] |
| 2a | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` → `min_len=0`; `min_cap <= stbds_arrcap(NULL) == 0` | returns the **NULL** input unchanged — no allocation at all | `err_02_arrgrowf_null_noalloc` | [x] |
| 2b | `stbds_arrgrowf` | `a == NULL`, `min_cap ∈ {1,2,3}` → `min_cap < 4` branch | fresh array, `length=0`, `capacity=4`, `hash_table=NULL`, `temp=0` | `err_02_arrgrowf_min4` | [x] |
| 3  | `stbds_arrgrowf` | `a == NULL`, huge `min_cap` so that `elemsize*min_cap + sizeof(header)` overflows `size_t` (`elemsize=16, min_cap=2^60+254` ⇒ 4096 bytes actually allocated; `elemsize=8, min_cap=2^61+100` ⇒ 832) | `realloc` of the wrapped-around size; both must compute the identical wrapped size and identical `capacity` field | `err_03_arrgrowf_overflow` | [x] |
| 4  | `stbds_hmget_key_ts` | `a == NULL` | allocates 1-elem array, `length=1`, elem zeroed, `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` | `err_04_hmget_ts_null` | [x] |
| 5  | `stbds_hmget_key_ts` | `a != NULL` but `stbds_header(raw_a)->hash_table == 0` (map made only by `hmput_default`) | `*temp = -1`, returns `a` unchanged | `err_05_hmget_ts_no_table` | [x] |
| 6  | `stbds_hmget_key_ts` | key absent from a populated table (`stbds_hm_find_slot` → `-1`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `err_06_hmget_ts_miss` | [x] |
| 7  | `stbds_hm_find_slot` (via 6) | probe reaches a slot with `hash == STBDS_HASH_EMPTY (0)` | returns `-1` (propagated as `temp = -1`) | `err_06_hmget_ts_miss` | [x] |
| 8  | `stbds_hmget_key` | `a == NULL` (delegates to row 4) | `stbds_temp(arr) = -1`; returns `arr+elemsize` | `err_07_hmget_key_null` | [x] |
| 9  | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (no alloc, no zeroing) | `err_08_hmput_default_noop` | [x] |
| 10 | `stbds_hmput_default` | `a == NULL` | allocates, `length=1`, elem zeroed | `err_09_hmput_default_null` | [x] |
| 11 | `stbds_hmdel_key` | `a == NULL` | returns `0` (**NULL**) — the only NULL-returning path in the library | `err_10_hmdel_null` | [x] |
| 12 | `stbds_hmdel_key` | `a != NULL`, `hash_table == 0` | `stbds_temp(raw_a) = 0`, returns `a` unchanged, length unchanged | `err_11_hmdel_no_table` | [x] |
| 13 | `stbds_hmdel_key` | key not present (`slot < 0`) | `stbds_temp(raw_a) = 0`, returns `a`, length unchanged, `used_count`/`tombstone_count` unchanged | `err_12_hmdel_miss` | [x] |
| 14 | `stbds_hmfree_func` | `a == NULL` | returns immediately, no free | `err_13_hmfree_null` | [x] |
| 15 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` — fails only for `slot_count == 0` (`0 + 0 < 0` false) | `abort()` | *unreachable from the public API*: `slot_count` is `STBDS_BUCKET_LENGTH (8)` or a previous `slot_count*2`/`>>1` bounded below by 8 (`slot_count > 8` guard on shrink). Documented; no test. | [x] |
| 16 | `stbds_hmput_key` | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` after the `arrgrowf` on insert | `abort()` | *unreachable*: `arrgrowf(a,elemsize,1,0)` guarantees `cap >= len+1`. Documented; no test. | [x] |
| 17 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | `abort()` | *unreachable*: `hm_find_slot` masks `pos` with `slot_count-1`. Documented; no test. | [x] |
| 18 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` | never fires — `used_count` is `size_t`, so `>= 0` is tautologically true (this is a real C quirk: deleting from an empty-but-tabled map wraps `used_count` to `SIZE_MAX` **without** aborting) | `err_14_hmdel_used_count_wrap` — checks both libs wrap `used_count` identically instead of aborting | [x] |
| 19 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` on the re-find of the moved-in last element | `abort()` | *unreachable* for well-formed maps. Documented; no test. | [x] |
| 20 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` | `abort()` | *unreachable* for well-formed maps. Documented; no test. | [x] |
| 21 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` | `abort()` | *unreachable*: the preceding `if (len > a->remaining)` always installs a block with `remaining >= len`, or returns early. Documented; no test. | [x] |
| 22 | `stbds_stralloc` | `len > blocksize` (string longer than the current block size, incl. `>` `STBDS_STRING_ARENA_BLOCKSIZE_MIN` = 512 on a fresh arena) | dedicated oversized block, spliced *after* `a->storage` (or installed as `storage` with `remaining=0` when arena empty); returns `sb->storage`; `a->remaining` **not** decremented | `err_15_stralloc_oversize` | [x] |
| 23 | `stbds_stralloc` | `a->block` at/over the `STBDS_STRING_ARENA_BLOCKSIZE_MAX` (`1<<20`) ceiling: `blocksize = 512 << (block>>1)` stops incrementing `a->block` once `blocksize >= 1<<20` (i.e. `block >= 22`) | `a->block` freezes at **22**; `remaining = blocksize - len`.  Note the *ceiling only gates the increment*, not the shift: larger `block` values still compute ever-larger `blocksize` | `err_16_stralloc_blocksize_max` (blocks 0,1,2,19..24,40) | [x] |
| 24 | `stbds_stralloc` | every `a->block` in `0..=255`: shift counts `>= 55` wrap the product to `0` (defined C), shift counts `>= 64` (`block >= 128`) are **UB in C** and get masked to 6 bits by the hardware `shl`; `blocksize == 0` then forces the oversize path and `0 < MAX` still does `++block`, wrapping `255 -> 0` | identical `blocksize`, `remaining`, `block` and copied bytes in both libraries | `err_17_stralloc_block_255` (skips only the `block` values whose `blocksize` is a multi-GiB request that `realloc` fails and the C then NULL-derefs — an identical crash in both, not observable from a live harness) | [x] |
| 25 | `stbds_stralloc` | empty string (`len == 1`) on fresh arena (`remaining == 0`) | `1 > 0` ⇒ new 512-byte block, `remaining = 511`, returns block end-1 | `err_18_stralloc_empty_str` | [x] |
| 26 | `stbds_strreset` | already-empty arena (`storage == NULL`) | no frees, arena memset to all-zero | `err_19_strreset_empty` | [x] |
| 27 | `stbds_hash_bytes` | `len == 0` (pointer never dereferenced; `p` may even be NULL) | `switch(0)` hits `case 0: break;` ⇒ `data = 0 << 56`; deterministic hash of "nothing" | `err_20_hash_bytes_zero_len` | [x] |
| 28 | `stbds_hash_bytes` | `p == NULL, len == 0` | same as row 27, no crash | `err_21_hash_bytes_null_zero` | [x] |
| 29 | `stbds_hash_string` | empty string `""` | `while(*str)` never runs; avalanche applied to `seed` alone | `err_22_hash_string_empty` | [x] |
| 30 | `stbds_hash_string` | bytes ≥ 0x80 (`(unsigned char)` cast — must **not** sign-extend) | value-dependent hash; must match exactly | `err_23_hash_string_high_bytes` | [x] |
| 31 | `stbds_hash_bytes` | tail byte `d[3] >= 0x80` ⇒ `case 4: data |= (d[3] << 24)` is a **negative `int`** ⇒ sign-extends to `size_t` | value-dependent hash; must match | `err_24_hash_bytes_sign_extend` | [x] |
| 32 | `stbds_is_key_equal` / all map fns | out-of-range `mode` **above** the enum: `mode = 2, 7, 1000, INT_MAX` | `mode >= STBDS_HM_STRING` ⇒ treated as **string** mode (`strcmp`) | `err_25_mode_out_of_range_high` | [x] |
| 33 | `stbds_is_key_equal` / all map fns | out-of-range `mode` **below** the enum: `mode = -1, INT_MIN` | `mode >= STBDS_HM_STRING` false ⇒ treated as **binary** mode (`memcmp`) | `err_26_mode_out_of_range_low` | [x] |
| 34 | `stbds_hmput_key` | first insert with `mode >= STBDS_HM_STRING` ⇒ `nt->string.mode = STBDS_SH_DEFAULT`; with `mode < 1` ⇒ `nt->string.mode = 0` (`STBDS_SH_NONE`) | `string.mode` differs by branch; drives the later `switch` | `err_27_mode_selects_string_mode` | [x] |
| 35 | `stbds_shmode_func` | out-of-range `mode` enum value: `mode = 4, 99, 255, 256, -1, INT_MAX` | `h->string.mode = (unsigned char) mode` (truncating!), no validation; `switch(table->string.mode)` then falls to `default:` ⇒ raw `memcpy` of `keysize` bytes | `err_28_shmode_out_of_range` | [x] |
| 36 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` is tested with `==`, not `>=` (L836, L842).  With `mode = 2 / 5 / 1000` (still "string" for hashing) the re-find of the moved-in last element takes the **binary** branch and hashes the *address* of the key pointer, so the slot is not found and `STBDS_ASSERT(slot >= 0)` fires — whenever the deleted element is not the last one | `abort()` (SIGABRT) | `err_29_hmdel_mode_eq_vs_ge` — asserts **both** libraries die with SIGABRT, that `mode == 1` succeeds on the same input, and that deleting the *last* element (which skips the re-find) behaves identically for `mode = 2/5/1000` | [x] |
| 37 | `stbds_hmput_key` | duplicate key insert (found in the `i < BUCKET_LENGTH` first loop) | `temp = existing index`, `temp_key` updated, length unchanged | `err_30_hmput_duplicate` | [x] |
| 38 | `stbds_hmput_key` | duplicate key found in the **wrap-around** (`i < limit`) loop — note the C does **not** set `temp_key` there | `temp` set, `temp_key` **not** updated (asymmetry between the two loops) | `err_31_hmput_dup_wrap_loop` | [x] |
| 39 | `sh_geti` | `num <= 0` (`0`, `-1`, `INT_MIN`) | all `for` loops bodies skipped; the `j` loop still runs twice; prints nothing; no crash | `err_32_sh_geti_nonpositive` | [x] |
| 40 | `strkey` | negative / extreme `n` (`-1`, `INT_MIN`, `INT_MAX`) | `sprintf(buffer,"test_%d",n)` into the shared 256-byte static; returns that buffer | `err_33_strkey_extremes` | [x] |
| 41 | `stbds_arrfreef` | `a == NULL` ⇒ `free((stbds_array_header*)NULL - 1)` = `free((void*)-32)` | **undefined behaviour / crash in both**; not a defined rejection | *not tested* (both libs perform the identical invalid `free`; testing it aborts the harness). Documented. | [x] |
| 42 | `stbds_hmfree_func` | `a != NULL`, `hash_table == NULL` | skips the strdup-free loop and `strreset`, still `free(NULL)` + `free(header)` | `err_34_hmfree_no_table` | [x] |
| 43 | `stbds_hmfree_func` | `string.mode == STBDS_SH_STRDUP` and `length == 1` (only the default element) | strdup-free loop `for(i=1; i<1)` runs 0 times | `err_35_hmfree_strdup_empty` | [x] |
| 44 | `stbds_arrgrowf` | `elemsize == 0` (size becomes just `sizeof(header)`) | valid header-only array; `capacity` = requested count | `err_36_arrgrowf_elemsize_zero` | [x] |
| 45 | `stbds_hmput_key` / `hmget_key` / `hmdel_key` | `keysize == 0`: `memcmp(a,b,0) == 0` for **every** pair and `stbds_hash_bytes(k,0,seed)` is one fixed value ⇒ every key is "equal" | the map collapses to a single entry regardless of the keys supplied | `err_37_keysize_zero_binary` | [x] |
| 46 | `stbds_shmode_func` / `stbds_hmfree_func` | `elemsize == 0` (`STBDS_ARR_TO_HASH(a,0) == a`, the "default element" occupies 0 bytes) | table created, `length = 1`, `string.mode = mode`; `hmfree_func` frees cleanly | `err_38_shmode_elemsize_zero` | [x] |
| 47 | `stbds_hmget_key_ts` | `mode` out of range (`-1`, `7`) with `a == NULL` | mode is irrelevant on this path; `*temp = -1` | `err_04_hmget_ts_null` | [x] |
| 48 | `stbds_hmdel_key` | `a == NULL` with every out-of-range `mode` (`-1`, `5`, `INT_MAX`, `INT_MIN`) and a non-zero `keyoffset` | `NULL` in all combinations | `err_10_hmdel_null` | [x] |

## Known, deliberately-unreproduced difference

When an `assert()` fails, glibc writes
`<prog>: <path>/lib.c:<line>: <func>: Assertion \`<expr>' failed.` to **stderr**
before `abort()`ing.  The Rust translation calls `abort()` directly, so it dies
with the same signal (SIGABRT, verified in `err_29_hmdel_mode_eq_vs_ge`) but
prints no message.  The message embeds the absolute path of the C source at
*compile* time, so it is not a reproducible property of the library's behaviour;
only the fatal outcome is.  No other observable output differs.

## Result

39 tests in `tests/phase_c_errors.rs`, all passing against both the C `.so` and
the Rust `.so`; every row above is checked off.
