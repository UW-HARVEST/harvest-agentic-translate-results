# ERRORS.md — Phase C error / rejection surface table

Every distinct way `c_src/src/lib.c` rejects, sentinel-returns, asserts, or
otherwise refuses input. Derived mechanically by grepping the C source for
`STBDS_ASSERT`, `return -1`, `return 0`, `return a` (early-out), `== NULL`,
`== 0`, `< 0`, and every named limit constant. Line numbers refer to
`c_src/src/lib.c`.

The C library has **no error enum and no error-code return convention**: it
signals "not found / nothing to do" with sentinel values
(`STBDS_INDEX_EMPTY == -1`, `0`, `NULL`, the unchanged input pointer) and it
signals "programmer error" with `assert` ⇒ `SIGABRT`. Invalid pointers produce
`SIGSEGV`. Rows are therefore checked by comparing either the returned
sentinel/observable state, or — for the aborting/faulting rows — the exact
process termination signal of a subprocess that performs the same call against
the C `.so` and against the Rust `.so`.

Test file: `tests/errors.rs` (in-process rows) and
`tests/errors.rs::fatal_*` + the `scenario` subprocess runner (fatal rows).

| #  | function | trigger (exact invalid input/condition) | expected C result | test | ✓ |
|----|----------|------------------------------------------|-------------------|------|---|
| 1  | `stbds_arrgrowf` (L286) | `a == NULL`, `addlen == 0`, `min_cap == 0` ⇒ `min_cap (0) <= stbds_arrcap(NULL) (0)` | early `return a` ⇒ returns `NULL` (no allocation) | `err_arrgrowf_null_nogrow` | [x] |
| 2  | `stbds_arrgrowf` (L286) | non-NULL `a`, `min_cap <= capacity`, `addlen == 0` | early `return a` ⇒ same pointer, header untouched | `err_arrgrowf_no_grow_returns_same` | [x] |
| 3  | `stbds_arrgrowf` (L283/291) | `a == NULL`, `min_cap == 1..3`, `addlen == 0` ⇒ `min_cap < 4` clamp | allocates, `capacity == 4` (never 1..3) | `err_arrgrowf_min_cap_clamped_to_4` | [x] |
| 4  | `stbds_arrgrowf` (L297) | `elemsize * min_cap` overflows `size_t` (e.g. `elemsize = SIZE_MAX/2`, `min_cap = 4`) | wraps, `realloc` of the wrapped size; header still written; `capacity == min_cap` | `err_arrgrowf_size_overflow` | [x] |
| 5  | `stbds_arrfreef` (L314) | `a == NULL` ⇒ `free((stbds_array_header*)NULL - 1)` = `free((void*)-32)` | glibc dereferences the bogus chunk header ⇒ `SIGSEGV` (observed: rc 139 on both libraries, in both cargo profiles — this is NOT a Rust null check, so parity is exact) | `fatal_arrfreef_null` | [x] |
| 6  | `stbds_hash_string` (L480) | `str == NULL` ⇒ `*str` dereference | `SIGSEGV` | `fatal_hash_string_null` | [x] |
| 7  | `stbds_hash_bytes` (L522/532) | `p == NULL`, `len == 0` (degenerate but **legal**: no dereference) | returns the well-defined empty-input hash | `err_hash_bytes_null_len0` | [x] |
| 8  | `stbds_hash_bytes` (L522) | `p == NULL`, `len > 0` ⇒ dereference of `d[0]` | `SIGSEGV` | `fatal_hash_bytes_null_len1` | [x] |
| 9  | `stbds_hash_bytes` (L522) | `len == SIZE_MAX` (oversized length) ⇒ `i + 8 <= len` walks off the buffer | `SIGSEGV` | `fatal_hash_bytes_huge_len` | [x] |
| 10 | `stbds_hmfree_func` (L573) | `a == NULL` | `return;` — no-op, no free | `err_hmfree_null_is_noop` | [x] |
| 11 | `stbds_hmfree_func` (L574) | `a != NULL` but `stbds_header(a)->hash_table == NULL` (array made by `stbds_arrgrowf`, never hashed) | skips the strdup/arena teardown, still `free(NULL)` + `free(header)` — no crash | `err_hmfree_no_table` | [x] |
| 12 | `stbds_hmget_key_ts` (L634) | `a == NULL` | allocates a 1-element default array, writes `*temp = STBDS_INDEX_EMPTY (-1)`, returns the new **hash** pointer (non-NULL, `!= a`) | `err_hmget_ts_null_a` | [x] |
| 13 | `stbds_hmget_key_ts` (L644) | `a != NULL` but `hash_table == 0` (e.g. straight from `stbds_hmput_default`) | `*temp = -1`, returns `a` unchanged | `err_hmget_ts_no_table` | [x] |
| 14 | `stbds_hm_find_slot` (L609/620) → `stbds_hmget_key_ts` (L648) | key absent from a populated table (probe hits `STBDS_HASH_EMPTY`) | `stbds_hm_find_slot` returns `-1` ⇒ `*temp = STBDS_INDEX_EMPTY (-1)` | `err_hmget_ts_missing_key` | [x] |
| 15 | `stbds_hmget_key_ts` (L638/649) | `temp == NULL` (null out-param) | `SIGSEGV` on `*temp = ...` | `fatal_hmget_ts_null_temp` | [x] |
| 16 | `stbds_hmget_key` (L663) | `a == NULL` — the wrapper writes `stbds_temp(...)` of the *newly allocated* array | `stbds_header(ret - elemsize)->temp == -1` | `err_hmget_key_null_a` | [x] |
| 17 | `stbds_hmget_key` / `stbds_hmput_key` (L560, L590, L713) | `mode` out of the enum range: `mode = -1`, `-2147483648` (`< STBDS_HM_STRING`) | `mode >= STBDS_HM_STRING` is false ⇒ **binary** path (`memcmp`/`stbds_hash_bytes`), no crash | `err_mode_out_of_range_negative` | [x] |
| 18 | `stbds_hmget_key` / `stbds_hmput_key` (L560, L590, L713) | `mode` out of the enum range: `mode = 2, 3, 999, 2147483647` (`>= STBDS_HM_STRING`) | treated exactly like `STBDS_HM_STRING` (`strcmp`/`stbds_hash_string`) | `err_mode_out_of_range_positive` | [x] |
| 19 | `stbds_hmdel_key` (L836/842) | `mode >= 2` on a string map: `mode == STBDS_HM_STRING` is **false**, so the strdup-free and the string re-find are skipped while `stbds_hm_find_slot` still hashes as a string | binary re-find of a pointer-valued "key" ⇒ `stbds_hm_find_slot` returns `-1` ⇒ `STBDS_ASSERT(slot >= 0)` fires ⇒ `SIGABRT` | `fatal_hmdel_mode2_string_map` | [x] |
| 20 | `stbds_hmput_default` (L669) | `a == NULL` | allocates the 1-element default array and returns the hash pointer | `err_hmput_default_null_a` | [x] |
| 21 | `stbds_hmput_default` (L669) | `a != NULL` and `length != 0` | returns `a` unchanged (does **not** reallocate or re-zero) | `err_hmput_default_idempotent` | [x] |
| 22 | `stbds_hmput_key` (L686) | `a == NULL` | bootstraps a fresh map (default slot + `STBDS_BUCKET_LENGTH` table) rather than failing | `err_hmput_key_null_a` | [x] |
| 23 | `stbds_hmput_key` (L713) | `key == NULL`, `mode >= STBDS_HM_STRING` ⇒ `stbds_hash_string(NULL, seed)` | `SIGSEGV` | `fatal_hmput_key_null_string_key` | [x] |
| 24 | `stbds_hmput_key` (L713) | `key == NULL`, `mode == STBDS_HM_BINARY`, `keysize == 0` | `stbds_hash_bytes(NULL,0,seed)` — legal, inserts a zero-width key, `memcpy(dst,NULL,0)` | `err_hmput_key_null_key_keysize0` | [x] |
| 25 | `stbds_hmput_key` (L713) | `key == NULL`, `mode == STBDS_HM_BINARY`, `keysize > 0` | `SIGSEGV` in `stbds_hash_bytes` | `fatal_hmput_key_null_binary_key` | [x] |
| 26 | `stbds_hmput_key` (L789) | `keysize > elemsize` (e.g. `elemsize = 8`, `keysize = 64`) ⇒ `memcpy` past the element | the 64-byte `memcpy` runs past the 8-byte element into the `realloc` slack; observed identically in a child process on both libraries (same `stbds_temp`, exit 0, rc 0) | `err_keysize_gt_elemsize` | [x] |
| 27 | `stbds_hmput_key` (L778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | unreachable through the public API (`stbds_arrgrowf` on the line above guarantees it); asserted never to fire over the whole Phase B corpus | covered by all of Phase B | [x] |
| 28 | `stbds_shmode_func` (L803) | `mode` outside `{0,1,2,3}`: stored as `(unsigned char) mode`, so `mode = 256` ⇒ `0`, `mode = 259` ⇒ `3` (`SH_ARENA`), `mode = -1` ⇒ `255` | truncation, then `stbds_hmput_key`'s `switch` falls to `default:` ⇒ raw `memcpy` of `keysize` bytes instead of storing a `char*` | `err_shmode_out_of_range` | [x] |
| 29 | `stbds_shmode_func` (L798) | `elemsize == 0` | `stbds_arrgrowf(0,0,0,1)` allocates only the header, `memset(a,0,0)`, `length = 1`; returned hash pointer aliases the array base | `err_shmode_elemsize0` | [x] |
| 30 | `stbds_hmdel_key` (L809) | `a == NULL` | `return 0` ⇒ returns `NULL` (this is the sentinel the `shdel` macro tests) | `err_hmdel_null_a` | [x] |
| 31 | `stbds_hmdel_key` (L816) | `a != NULL`, `hash_table == 0` | sets `stbds_temp(raw_a) = 0` then `return a` unchanged | `err_hmdel_no_table` | [x] |
| 32 | `stbds_hmdel_key` (L821) | key absent ⇒ `stbds_hm_find_slot` returns `-1` | `stbds_temp(raw_a) == 0`, returns `a`, `used_count`/`length` unchanged | `err_hmdel_missing_key` | [x] |
| 33 | `stbds_hmdel_key` (L828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | unreachable (`stbds_hm_find_slot` masks with `slot_count-1`); asserted never to fire over Phase B | covered by all of Phase B | [x] |
| 34 | `stbds_hmdel_key` (L832) | `STBDS_ASSERT(table->used_count >= 0)` on a `size_t` | tautology — the C compiler folds it away; can never fire even after the `--table->used_count` wrap | `err_hmdel_used_count_tautology` | [x] |
| 35 | `stbds_hmdel_key` (L846) | `STBDS_ASSERT(slot >= 0)` — re-find of the moved-in tail element fails | fires when the map's keys were mutated behind the library's back, and via row 19 | `fatal_hmdel_mode2_string_map` | [x] |
| 36 | `stbds_hmdel_key` (L849) | `STBDS_ASSERT(b->index[i] == final_index)` | fires if the tail element's key duplicates another live key (`memmove` then re-find lands on the wrong slot) | `fatal_hmdel_corrupted_index` | [x] |
| 37 | `stbds_hmdel_key` (L826/827) | delete on a map with **zero** live entries but a live table (`length == 1`) and a key that hashes to a stale in-use slot | `final_index == -1`; only reachable with a forged table, asserted never to fire over Phase B | covered by all of Phase B | [x] |
| 38 | `stbds_hmdel_key` (L854) | `used_count < used_count_shrink_threshold && slot_count > STBDS_BUCKET_LENGTH` — boundary: `slot_count == STBDS_BUCKET_LENGTH (8)` never shrinks (`shrink_threshold` forced to 0 at L399) | table kept at 8 slots no matter how many deletes | `err_no_shrink_at_min_slots` | [x] |
| 39 | `stbds_make_hash_index` (L401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` — fails for `slot_count <= 2` (`2-0 + 0 = 2 !< 2`) | unreachable: `slot_count` is only ever `8`, `2*n`, or `n>>1` guarded by `n > 8`, so `>= 8` always; asserted never to fire over Phase B | covered by all of Phase B | [x] |
| 40 | `stbds_stralloc` (L885) | `a == NULL` ⇒ `a->remaining` | `SIGSEGV` | `fatal_stralloc_null_arena` | [x] |
| 41 | `stbds_stralloc` (L884) | `str == NULL` ⇒ `strlen(NULL)` | `SIGSEGV` | `fatal_stralloc_null_str` | [x] |
| 42 | `stbds_stralloc` (L913) | `STBDS_ASSERT(len <= a->remaining)` — forged arena with `remaining` huge but `storage == NULL` (`len <= remaining` so the alloc path is skipped) | assert passes, then `a->storage->storage + ...` faults ⇒ `SIGSEGV` | `fatal_stralloc_forged_remaining` | [x] |
| 43a | `stbds_stralloc` (L888) | `a->block` large enough that `512 << (block>>1)` shifts by `>= 64` (`block >= 128`) — C shift-count UB. Verified for `block` in `0..=25`, `110..=127`, `128..=153`, `238..=255`, i.e. every masked shift `0..13` plus every shift `>= 55` where `blocksize` itself wraps to 0 | x86-64 `shl` masks the count to 6 bits; Rust `wrapping_shl` masks identically ⇒ same `blocksize`, same `remaining`, same path (bump / new block / big block) | `err_stralloc_block_shift_overflow` | [x] |
| 43b | `stbds_stralloc` (L906) | `a->block` such that `blocksize == 2^63` (`block` 108, 109, 236, 237): the block `realloc` cannot succeed and returns NULL, then `sb->next = a->storage` writes through it | `SIGSEGV` (Rust release: identical; Rust debug: `SIGABRT` from the `debug_assertions` null-pointer UB check — see the note below) | `fatal_stralloc_huge_block` | [x] |
| 44 | `stbds_stralloc` (L890) | `a->block` at the `BLOCKSIZE_MAX` ceiling (`512 << (block>>1) >= 1<<20`, i.e. `block >= 22`) | `++a->block` is **skipped**; `block` saturates | `err_stralloc_block_saturates` | [x] |
| 44b | `stbds_is_key_equal` (L561) via `stbds_hmget_key` | `table->string.mode == STBDS_SH_NONE` (or any value outside `{1,2,3}`) combined with `mode >= STBDS_HM_STRING`: `stbds_hmput_key`'s `switch` falls to `default:` and `memcpy`s the key's **text** into the element, which the next lookup dereferences as a `char*` | `SIGSEGV` (insert-only is well defined — that half is CONFIGS.md row 44) | `fatal_sh_none_string_lookup` | [x] |
| 45 | `stbds_stralloc` (L893/896) | `len > blocksize` **and** `a->storage == NULL` | big-block path, `sb->next = 0`, `a->storage = sb`, `a->remaining = 0` | `err_stralloc_bigblock_paths` | [x] |
| 46 | `stbds_stralloc` (L893/896) | `len > blocksize` **and** `a->storage != NULL` | big block is spliced **after** the head (`sb->next = a->storage->next`), `a->remaining` untouched | `err_stralloc_bigblock_paths` | [x] |
| 47 | `stbds_stralloc` (L885) | `len == a->remaining` exactly (boundary, one step inside the range) | no new block; `remaining` becomes 0 | `err_stralloc_fit_boundary` | [x] |
| 48 | `stbds_stralloc` (L885) | `len == a->remaining + 1` (one step past) | new block allocated | `err_stralloc_fit_boundary` | [x] |
| 49 | `stbds_strreset` (L924) | `a == NULL` ⇒ `a->storage` | `SIGSEGV` | `fatal_strreset_null_arena` | [x] |
| 50 | `stbds_strreset` (L924) | `a->storage == NULL` (empty / already reset arena) | while-loop body never runs, `memset(a,0,sizeof *a)` — idempotent, no double free | `err_strreset_empty_idempotent` | [x] |
| 51 | `strkey` (L941) | `n == INT_MIN`, `INT_MAX`, `-1` — `sprintf("test_%d")` into a 256-byte static buffer | `"test_-2147483648"`, `"test_2147483647"`, `"test_-1"` — never overflows | `err_strkey_extremes` | [x] |
| 52 | `sh_geti` (L951) | `num <= 0` (`0`, `-1`, `INT_MIN`) — every `for` loop body is skipped | no output; the three `shgeti(strmap,"foo") == -1` asserts still run and pass; both `j` iterations `shfree` cleanly | `err_sh_geti_nonpositive` | [x] |
| 53 | `sh_geti` (L956/961/963) | `STBDS_ASSERT(shgeti(strmap,"foo") == -1)` | must hold on a NULL map, on a fresh `sh_new_*` map, and after `shdefault` | `err_sh_geti_nonpositive` + `cfg67_sh_geti_positive` | [x] |
| 54 | `sh_geti` (L971/976/981) | `STBDS_ASSERT(shget(...) == -2 / == i*3)` | the `shdefault(strmap,-2)` value must be returned for every missing/deleted key | `cfg67_sh_geti_positive` / `cfg69_sh_geti_seeded` / `cfg70_sh_geti_twice` | [x] |
| 55 | `sh_geti` (L965) | `num > INT_MAX/3` ⇒ `i*3` signed-overflow UB in `shput(strmap, strkey(i), i*3)` | not reachable: `num` that large would require ~2^31 inserts; the loop bound is the practical limit. Guarded instead at `num` up to 4096 | `cfg67_sh_geti_positive` / `cfg69_sh_geti_seeded` / `cfg70_sh_geti_twice` | [x] |

## Generic FFI boundary rows (required even though not in the C source)

| #  | surface | trigger | expected C result | test | ✓ |
|----|---------|---------|-------------------|------|---|
| G1 | every pointer parameter | `NULL` in each position | rows 5, 6, 8, 10, 12, 15, 23, 25, 30, 40, 41, 49 above | as listed | [x] |
| G2 | every `size_t` length | `0` (`elemsize`, `keysize`, `len`, `addlen`, `min_cap`) | rows 1, 7, 24, 29 above + `err_zero_lengths_matrix` | `err_zero_lengths_matrix` | [x] |
| G3 | every `size_t` length | oversized (`SIZE_MAX`, `SIZE_MAX/2`) | rows 4, 9 above | as listed | [x] |
| G4 | `int mode` (C enum across FFI) | `-2147483648`, `-1`, `2`, `3`, `4`, `255`, `256`, `259`, `2147483647` in `stbds_hmget_key`, `stbds_hmget_key_ts`, `stbds_hmput_key`, `stbds_hmdel_key`, `stbds_shmode_func` | rows 17, 18, 19, 28 above + `err_mode_enum_matrix` | `err_mode_enum_matrix` | [x] |
| G5 | `size_t keyoffset` | value one past the element (`keyoffset == elemsize`) in `stbds_hmdel_key` | reads the *next* element's bytes as the key; must diverge identically | `err_keyoffset_past_element` | [x] |
| G6 | `int n` / `int num` | `INT_MIN`, `-1`, `0`, `INT_MAX` | rows 51, 52 above | as listed | [x] |


## Note on Rust's `debug_assertions` UB checks

Since Rust 1.78 a build with `debug_assertions` inserts a
`"null pointer dereference occurred"` check in front of raw-pointer
dereferences. On the rows above whose C behaviour is a NULL dereference
(rows 6, 8, 9, 15, 23, 25, 40, 41, 42, 43b, 49) the **release** `cdylib` — the
shipping artifact, built with `[profile.release] panic = "abort"` — raises
`SIGSEGV` exactly like the C library, while a **debug** `cdylib` raises
`SIGABRT` instead because that check fires first.

`tests/common/mod.rs::assert_fatal_scenario_matches` therefore requires exact
signal parity, and additionally tolerates `SIGABRT`-for-`SIGSEGV` *only* when
the `.so` under test carries `debug_assertions`
(`common::rust_so_has_ub_checks`). Rows whose C behaviour is `SIGABRT` from a
live `STBDS_ASSERT` (19, 35, 36) and row 5 (`free()` of an invalid pointer)
require exact parity in both profiles. stdout is always required to be
byte-identical.

## Note on uninitialised memory

Three places in the C library leave memory uninitialised, so those bytes are
**not** comparable across the two libraries' independent heaps and are excluded
from the differential snapshot (with the exclusion documented at the point of
use):

1. `stbds_hash_index::temp_key` — `stbds_make_hash_index` never writes it.
   It is only meaningful right after a `mode >= STBDS_HM_STRING` put, and is
   compared there by `Pair::temp_key` (which also pins down the stb quirk that
   `stbds_hmput_key`'s *wrap-around* probe loop does **not** refresh it).
2. Bytes `[keysize..elemsize)` of a freshly appended element — `stbds_hmput_key`
   only `memcpy`s the key. The `hmput`/`shput` macros store a value there
   immediately, and every test does the same (`Map::write_value` /
   `init_value_region`) before snapshotting.
3. The capacity slack past `length` after `stbds_arrgrowf` — excluded from the
   snapshot, and explicitly zeroed on both sides in
   `err_keyoffset_past_element`, which deliberately reads past the last element.
