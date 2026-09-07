# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from the C source: every `return` of a sentinel, every
early-out, every `STBDS_ASSERT`, every range/null check and every min/max
constant.  Line numbers refer to `c_src/src/lib.c`.

The library has **no error-code enum**; it signals "no such thing" with the
sentinels `-1` (`STBDS_INDEX_EMPTY`), `-2` (`STBDS_INDEX_DELETED`), `NULL`, or
by returning the input pointer unchanged, and it signals "impossible state"
with `assert()`.  All rows below are therefore expressed in terms of those
observable results (returned pointer, `*temp` out-parameter, and the
`stbds_array_header.temp` field that the C macros read back).

## Rejection / sentinel rows

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| 1  | `stbds_arrgrowf` (L286) | `min_cap` (after `max(min_cap, arrlen+addlen)`) `<= arrcap(a)` — growth request that is already satisfied | returns `a` **unchanged** (same pointer, header untouched); no realloc |
| 2  | `stbds_arrgrowf` (L280) | `a == NULL` (nothing to grow from) | fresh block, `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(addlen,min_cap,4)` |
| 3  | `stbds_arrgrowf` (L291) | `min_cap < 4` and `min_cap >= 2*arrcap` — under-minimum capacity | capacity forced up to `4` |
| 4  | `stbds_hmfree_func` (L573) | `a == NULL` | returns immediately, no free, no crash |
| 5  | `stbds_hmfree_func` (L574) | `stbds_hash_table(a) == NULL` (array that never got a hash index) | skips string-arena teardown, still frees `hash_table` (NULL) and the header |
| 6  | `stbds_hm_find_slot` (L609) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` in the *tail* scan → key absent | returns `-1` |
| 7  | `stbds_hm_find_slot` (L620) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` in the *wrap-around* scan → key absent | returns `-1` |
| 8  | `stbds_hmget_key_ts` (L634) | `a == NULL` (get on an empty/never-created map) | allocates the 1-element "default" slot, sets `*temp = -1`, returns hash-side pointer |
| 9  | `stbds_hmget_key_ts` (L644) | `a != NULL` but `hash_table == 0` (only the default element exists, e.g. after `hmdefault` alone) | `*temp = -1`, returns `a` unchanged |
| 10 | `stbds_hmget_key_ts` (L648) | key not present (`stbds_hm_find_slot < 0`) | `*temp = -1` (`STBDS_INDEX_EMPTY`), returns `a` unchanged |
| 11 | `stbds_hmget_key` (L663) | same three cases as 8/9/10 | additionally writes `-1` into `stbds_header(p-elemsize)->temp` |
| 12 | `stbds_hmput_default` (L669) | `a == NULL` | creates 1-element block, `length=1`, zeroed |
| 13 | `stbds_hmput_default` (L669) | `a != NULL` but `length == 0` (degenerate/empty array) | grows again and bumps `length` to 1 |
| 14 | `stbds_hmdel_key` (L809) | `a == NULL` | returns `NULL` (`0`) — no `temp` written |
| 15 | `stbds_hmdel_key` (L816) | `hash_table == 0` | sets `temp = 0` (i.e. `hmdel` yields 0 = "not deleted"), returns `a` |
| 16 | `stbds_hmdel_key` (L821) | key absent (`slot < 0`) | `temp` stays `0`, returns `a` unchanged, `used_count`/`tombstone_count` untouched |
| 17 | `stbds_hmdel_key` (L831) | key present | `temp = 1`, slot marked `hash=1 (DELETED)` / `index=-2 (INDEX_DELETED)`, `length -= 1` |
| 18 | `stbds_hmdel_key` (L836) | `mode == STBDS_HM_STRING` **exactly** (not `>=`) and `string.mode == STBDS_SH_STRDUP` | frees the duplicated key; for `mode == 2` (`PTR_TO_STRING`) it does **not** free — an asymmetry that must be reproduced |
| 19 | `stbds_hm_find_slot` / `stbds_hmput_key` (L596, L719) | computed hash `< 2` (would collide with the `EMPTY=0` / `DELETED=1` markers) | `hash += 2` before probing |
| 20 | `stbds_hmput_key` (L686) | `a == NULL` | creates the default element first, then proceeds |
| 21 | `stbds_hmput_key` (L698) | `table == NULL` **or** `used_count >= used_count_threshold` (load factor 3/4 exceeded) | allocates/derives a new index of `8` or `2*slot_count` slots |
| 22 | `stbds_hmput_key` (L707) | first index creation with `mode >= STBDS_HM_STRING` | `string.mode = STBDS_SH_DEFAULT (1)`; otherwise `0` |
| 23 | `stbds_hmput_key` (L729) | key already present | overwrite path: `temp = existing index`, returns without growing the array |
| 24 | `stbds_hmput_key` (L766) | a tombstone was seen before the empty slot | reuses tombstone, `--tombstone_count` |
| 25 | `stbds_hmput_key` (L789) | `table->string.mode` not one of `STRDUP/ARENA/DEFAULT` (`default:` label, e.g. `SH_NONE`) | raw `memcpy` of `keysize` bytes instead of a pointer store |
| 26 | `stbds_hmdel_key` (L854) | `used_count < used_count_shrink_threshold` **and** `slot_count > 8` | index rebuilt at half size |
| 27 | `stbds_hmdel_key` (L858) | `tombstone_count > tombstone_count_threshold` | index rebuilt at same size |
| 28 | `stbds_make_hash_index` (L399) | `slot_count <= STBDS_BUCKET_LENGTH (8)` | `used_count_shrink_threshold` forced to `0` so an 8-slot table never shrinks |
| 29 | `stbds_stralloc` (L885) | `len > a->remaining` (current block exhausted / fresh arena with `remaining == 0`) | new block allocated, `a->block` bumped while `blocksize < 1<<20` |
| 30 | `stbds_stralloc` (L893) | `len > blocksize` (string longer than a whole block) | over-sized dedicated block, spliced *after* the head block; when `a->storage == NULL` also sets `remaining = 0` and returns that block's storage directly |
| 31 | `stbds_stralloc` (L890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` is **not** incremented any more (saturates) |
| 32 | `stbds_strreset` (L924) | `a->storage == NULL` (empty arena) | loop body never runs, arena zeroed, no free |
| 33 | `stbds_shmode_func` (L803) | `mode` outside the `STBDS_SH_*` enum (e.g. `4`, `259`, `-1`) — C enums accept any `int` | stored as `(unsigned char) mode`, i.e. truncated mod 256; later `switch` in `hmput_key` falls to `default:` (raw memcpy) for any value that is not 1/2/3 |
| 34 | `stbds_is_key_equal` (L560) | `mode >= STBDS_HM_STRING` (so also `2`, `3`, `1000`) | `strcmp` on the stored `char*`; for `mode <= 0` (incl. negative) `memcmp` of `keysize` bytes |
| 35 | `stbds_hash_bytes` / `stbds_siphash_bytes` (L522) | `len == 0` | main loop skipped, `switch (len - i)` takes `case 0`, `data` is just `len << 56` |
| 36 | `stbds_hash_string` (L480) | empty string `""` | loop body never runs, hash is the pure avalanche of `seed ^ seed == 0` |
| 37 | `strkey` (L941) | `n` negative / `INT_MIN` | `sprintf(buffer,"test_%d",n)` → `"test_-2147483648"`; shared 256-byte static buffer is overwritten each call |
| 38 | `stbds_arrfreef` (L314) | `a == NULL` | `free((header*)NULL - 1)` = `free((void*)-32)` → crashes; verified in a subprocess to crash **identically** in both (`SIGSEGV`, exit 139) |

## `STBDS_ASSERT` rows (asserts are *enabled* in the C build — no `NDEBUG`)

| #  | function | assert condition that must hold | reachability |
|----|----------|---------------------------------|--------------|
| A1 | `stbds_make_hash_index` (L401) | `used_count_threshold + tombstone_count_threshold < slot_count` | unreachable: for every power-of-two `n >= 8` the values are `(3/4)n` and `(3/16)n`, sum `(15/16)n < n` |
| A2 | `stbds_hmput_key` (L778) | `i+1 <= arrcap(a)` after the conditional `arrgrowf` | unreachable: `arrgrowf` guarantees it |
| A3 | `stbds_hmdel_key` (L828) | `slot < table->slot_count` | unreachable: `hm_find_slot` masks `pos` with `slot_count-1` |
| A4 | `stbds_hmdel_key` (L832) | `table->used_count >= 0` | vacuously true — `used_count` is `size_t` |
| A5 | `stbds_hmdel_key` (L846) | `slot >= 0` (re-find of the moved-in key) | REACHABLE with `mode == 2` (`PTR_TO_STRING`) when deleting a non-last element: the re-find takes the binary branch on a `char*` field, so the key is never found → `assert` fires → `abort()`.  Verified identical (`SIGABRT`, exit 134) in a subprocess |
| A6 | `stbds_hmdel_key` (L849) | `b->index[i] == final_index` | same trigger as A5 (A5 fires first) |
| A7 | `stbds_stralloc` (L913) | `len <= a->remaining` after block allocation | unreachable via the public path (only with a hand-forged arena, which segfaults in both implementations on the next line) |
| A8 | `arr_ins` (L953) | `arr[i] == num` right after `arrins` | unreachable: `arrins` writes `num` at `i` |
| A9 | `arr_ins` (L955) | `arr[4] == 4` for `i < 4` | unreachable: inserting at `i<4` into `[1,2,3,4]` always leaves `4` at index 4 |
| A10 | file scope (L495) | `sizeof(size_t) == 8` (compile-time `typedef` trick) | mirrored by `const _: () = assert!(size_of::<usize>() == 8)` in Rust |

A1–A4 and A7–A9 are unreachable through the exported API with a self-consistent
data structure (verified by `err_asserts_a1_a4_invariants_hold_in_both`), A5/A6
ARE reachable (see above), and A10 is a hard compile-time assertion in both.
The Rust translation uses plain `assert!` for all of them so that a failure
aborts exactly like the C `assert()`.

---

## Test coverage (Phase C)

Every row above has a differential test in `tests/phase_c_errors.rs` that builds
the exact condition, calls BOTH `.so`s, and asserts the same sentinel/state.
Run with `cargo test --offline --release -- --test-threads=1`.

| ERRORS.md row(s) | test |
|------------------|------|
| 1 | `err01_arrgrowf_already_satisfied_returns_input_pointer` |
| 2 | `err02_arrgrowf_from_null_initialises_header` |
| 3 | `err03_arrgrowf_minimum_capacity_is_four` |
| 4 | `err04_hmfree_func_null_is_noop` |
| 5 | `err05_hmfree_func_without_hash_table` |
| 6, 7, 10, 11 | `err06_07_10_11_missing_key_returns_minus_one` |
| 8, 9 | `err08_09_hmget_key_ts_null_and_tableless` |
| 12, 13 | `err12_13_hmput_default_null_and_zero_length` |
| 14 | `err14_hmdel_key_null_returns_null` |
| 15 | `err15_hmdel_key_without_hash_table` |
| 16 | `err16_hmdel_key_missing_key_leaves_everything_untouched` |
| 17 | `err17_hmdel_key_present_marks_tombstone` |
| 18 | `err18_strdup_free_only_for_mode_exactly_one` |
| 19 | `err19_hash_below_two_is_clamped_identically` |
| 20 | `err20_hmput_key_from_null` |
| 21, 22 | `err21_22_growth_threshold_and_initial_string_mode` |
| 23 | `err23_overwrite_existing_key` |
| 24 | `err24_tombstone_is_reused` |
| 25 | `err25_default_switch_branch_memcpys_the_key` |
| 26 | `err26_shrink_threshold` |
| 27 | `err27_tombstone_rebuild_same_size` |
| 28 | `err28_eight_slot_table_never_shrinks` |
| 29, 30, 31 | `err29_30_31_stralloc_block_paths` |
| 32 | `err32_strreset_on_empty_arena` |
| 33 | `err33_shmode_func_enum_truncation` |
| 34 | `err34_mode_classes_binary_vs_string` |
| 35 | `err35_hash_bytes_zero_length` |
| 36 | `err36_hash_string_empty` |
| 37 | `err37_strkey_extremes` |
| 38 | `err38_arrfreef_null_behaves_identically` (subprocess: both SIGSEGV / 139) |
| A1–A4 | `err_asserts_a1_a4_invariants_hold_in_both` (1500-op randomised workload) |
| A5, A6 | `err_assert_a5_a6_ptr_to_string_nonlast_delete` (subprocess: both SIGABRT / 134) |
| A7 | unreachable via the public API — see the note below |
| A8, A9 | `err_asserts_a8_a9_arr_ins_never_aborts` |
| A10 | compile-time `const _: () = assert!(size_of::<usize>() == 8)` |

Generic FFI boundaries additionally covered:

| boundary | test |
|----------|------|
| NULL pointers (`arrgrowf`, `hmfree_func`, `hmdel_key`, `hash_bytes`) | `err01`, `err04`, `err14`, `err35` |
| zero lengths (`keysize == 0`, `len == 0`, empty C string) | `err_generic_zero_keysize`, `err35`, `err36`, `err_generic_null_and_empty_string_keys` |
| oversized length (`min_cap = 1<<62` -> failed `realloc`) | `err_generic_oversized_allocation_behaves_identically` (subprocess: both SIGSEGV / 139) |
| out-of-range enum values over FFI (`mode` = -1/2/3/99/`INT_MIN`/`INT_MAX`; `shmode` = 4/255/256/259/`INT_MIN`) | `err33`, `err34`, `row30_*`, `row31_shmode_func_out_of_enum` |

### Assertions are ENABLED in the Rust `.so`

The C library is compiled without `NDEBUG`, so `assert()` is live.  The Rust
translation therefore uses plain `assert!` (not `debug_assert!`) for every
mirrored `STBDS_ASSERT`, and `[profile.release] panic = "abort"` turns a failed
assertion into `abort()` — the same `SIGABRT` the C `assert()` raises.  This is
what makes `err_assert_a5_a6_ptr_to_string_nonlast_delete` agree (134 vs 134);
with `debug_assert!` the Rust `.so` silently corrupted memory instead.

### Artifact under test: the release `cdylib`

`rust_so_path()` loads `target/release/libarr_ins_lib.so` — the artifact the
crate's `[lib] crate-type = ["cdylib"]` + `[profile.release]` produces, i.e. the
counterpart of the C `.so`.  A *debug*-profile `.so` diverges on exactly one
row — the oversized `realloc` failure (`err_generic_oversized_allocation…`) —
because rustc inserts a debug-only null-pointer-dereference check that turns the
C code's undefined behaviour into an `abort()` instead of the `SIGSEGV` the C
`.so` takes.  The C source is undefined behaviour there (it writes the array
header through `NULL + 32` after a failed `realloc`), so there is no defined
behaviour to match; the shipped release artifact matches the C byte for byte.
