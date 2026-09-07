# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from the `if` / `switch` / `? :` branches the C code takes
on its *runtime options* and on its *input shapes*.

## Axes the C actually branches on

**A. `mode` (an `int` parameter of `stbds_hmget_key`, `stbds_hmget_key_ts`,
`stbds_hmput_key`, `stbds_hmdel_key`).** Branch predicates found in the source:

* `mode >= STBDS_HM_STRING` (l.560, l.590, l.707, l.713, l.732, l.842… ) — hash
  and compare as a C string.
* `mode == STBDS_HM_STRING` (**exactly** 1) — only in `stbds_hmdel_key` (l.836,
  l.842) for the strdup-free and the re-find key form.

⇒ distinguished values: `mode < 0` (binary), `mode == 0` (binary),
`mode == 1` (`STBDS_HM_STRING`), `mode == 2` (`STBDS_HM_PTR_TO_STRING`),
`mode >= 3`.

**B. `table->string.mode` (the arena/strdup option, set by
`stbds_shmode_func(elemsize, mode)` or implicitly by `stbds_hmput_key`).**
`switch` at l.785: `STBDS_SH_STRDUP (2)`, `STBDS_SH_ARENA (3)`,
`STBDS_SH_DEFAULT (1)`, `default` (0 and 4..255 ⇒ raw `memcpy`).
Also read at l.575 (`== STBDS_SH_STRDUP` ⇒ free keys) and l.836.

**C. Global hash seed** — `stbds_rand_seed(size_t)` (l.355) sets
`stbds_hash_seed`; `stbds_make_hash_index(…, NULL)` *advances* it (l.412), while
`stbds_make_hash_index(…, ot)` copies `ot->seed` and leaves the global alone.
⇒ seed = default `0x31415926`, `0`, `1`, `SIZE_MAX`, random.

**D. `elemsize` / `keysize` / `keyoffset` shapes.** `elemsize` scales every
`elemsize*i` pointer step; `keysize` is the `memcmp`/`memcpy` width; `keyoffset`
is only non-zero for `stbds_hmdel_key`. Distinguished shapes: `elemsize` 4, 8,
12 (unaligned stride), 16, 24; `keysize` 0, 1, 2, 4, 8, `elemsize`;
`keyoffset` 0 and > 0.

**E. Table-size / occupancy shapes** — every threshold branch:
`slot_count == 8` initial (l.702), doubling `slot_count*2` (l.702),
`used_count >= used_count_threshold` grow (l.698),
`used_count < used_count_shrink_threshold && slot_count > 8` shrink (l.854),
`tombstone_count > tombstone_count_threshold` rebuild (l.858),
`slot_count <= STBDS_BUCKET_LENGTH ⇒ shrink_threshold = 0` (l.399).
⇒ counts: 0, 1, 5, 6 (first grow at `used_count_threshold`=6), 7, 8, 12, 13,
100, 1000; plus delete-down-to-empty to force shrink + rebuild.

**F. `stbds_hash_bytes` length shapes** — main loop steps 8 bytes (l.522) and
the tail `switch` has a distinct case per `len % 8` (l.532-541):
`len` = 0,1,2,3,4,5,6,7,8,9,15,16,17,23,24,31,32,64,100 + byte values with
bit 7 set at offsets 3 and 7 (the `int` sign-extension quirk).

**G. `stbds_hash_string` shapes** — empty, 1 char, 7, 8, 9, 63, 255 chars;
bytes ≥ 0x80; embedded high/low ASCII.

**H. `stbds_arrgrowf` shapes** — `a` NULL vs non-NULL crossed with the three
capacity branches (l.286 early return, l.289 `min_cap < 2*cap`, l.291
`min_cap < 4`), `addlen` 0/1/n, `min_cap` 0/1/3/4/5/exact-cap/huge.

**I. `stbds_stralloc` shapes** — `len <= remaining` fast path, fresh arena,
`block` 0..22 (blocksize `512 << (block>>1)` up to the `1<<20` cap, l.888-891),
`len > blocksize` oversize path with and without an existing `a->storage`
(l.893-904).

**J. `arr_push(int num)`** — `num` ≤ 0, 1, 49, 50, 51, 100, 101, 500, 2000
(the `i += 50` outer stride and the `j < i` inner fill).

## Rows (pruned cross-product of the axes the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | `len` = 0 (pointer never read) | [x] |
| C2 | `stbds_hash_bytes` | `len` = 1..7, random bytes, seed = default — exercises every tail `case` | [x] |
| C3 | `stbds_hash_bytes` | `len` = 8, 16, 24, 32, 64 (exact multiples ⇒ empty tail, `case 0`) | [x] |
| C4 | `stbds_hash_bytes` | `len` = 9..15, 17..23, 100 (main loop **and** tail) | [x] |
| C5 | `stbds_hash_bytes` | bytes with bit 7 set at word offsets 3 and 7 ⇒ `int` sign-extension (E32); all `len` 1..40 | [x] |
| C6 | `stbds_hash_bytes` | seed = `0`, `1`, `SIZE_MAX`, `0x31415926`, random × `len` 0..40 | [x] |
| C7 | `stbds_hash_string` | empty string, seed sweep | [x] |
| C8 | `stbds_hash_string` | ASCII strings length 1..64, seed sweep | [x] |
| C9 | `stbds_hash_string` | strings containing bytes 0x80..0xFF (zero-extended, cf. E34) | [x] |
| C10 | `stbds_hash_string` | long string (255 chars) — many `ROTATE_LEFT(hash,9)` rounds | [x] |
| C11 | `stbds_rand_seed` + `stbds_hmput_key` | seed set explicitly, then verify the global seed advance (`seed*a+b`) is identical by observing the `seed` stored in successive fresh tables | [x] |
| C12 | `stbds_arrgrowf` | `a = NULL`, `addlen` 0, `min_cap` 0 ⇒ E1 early return of `NULL` | [x] |
| C13 | `stbds_arrgrowf` | `a = NULL`, `addlen` 1, `min_cap` 0 ⇒ `min_cap` bumped to 4 | [x] |
| C14 | `stbds_arrgrowf` | `a = NULL`, `min_cap` 1/2/3 ⇒ bumped to 4; `min_cap` 5/17/1000 ⇒ used as-is | [x] |
| C15 | `stbds_arrgrowf` | non-NULL `a`, `min_cap <= cap` ⇒ pointer returned unchanged, header untouched | [x] |
| C16 | `stbds_arrgrowf` | non-NULL `a`, `min_cap` in `(cap, 2*cap)` ⇒ doubled to `2*cap` | [x] |
| C17 | `stbds_arrgrowf` | non-NULL `a`, `min_cap > 2*cap` ⇒ `min_cap` used verbatim | [x] |
| C18 | `stbds_arrgrowf` | repeated `addlen=1` growth chain (the `arrpush` pattern) for `elemsize` 1,4,8,12,16,24 — capacity/length trajectory compared at every step | [x] |
| C19 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free, `elemsize` 4 and 24 | [x] |
| C20 | `arr_push` | `num` = 0, negative, 1, 49, 50, 51, 100, 101, 500, 2000 (no output; must not crash, must not leak differently) | [x] |
| C21 | `strkey` | `n` = 0, 1, 9, 10, 99, 100, −1, −99, `INT_MIN`, `INT_MAX`, random — byte-compare the returned `static` buffer | [x] |
| C22 | `stbds_hmput_default` | `a = NULL`, `elemsize` 4/8/16/24 | [x] |
| C23 | `stbds_hmput_default` | called twice (2nd call is a no-op because `length != 0`) | [x] |
| C24 | `stbds_hmput_default` | after `stbds_hmput_key` has already built a table | [x] |
| C25 | `stbds_hmget_key_ts` | `a = NULL` ⇒ fresh 1-element array, `*temp = -1` | [x] |
| C26 | `stbds_hmget_key_ts` | array from `hmput_default` (no hash table) ⇒ `*temp = -1` | [x] |
| C27 | `stbds_hmget_key_ts` | mode 0 (binary), key present / key absent, `keysize` 4 and 8 | [x] |
| C28 | `stbds_hmget_key_ts` | mode 1 (string), key present / absent | [x] |
| C29 | `stbds_hmget_key` | same as C27/C28 but through the non-`_ts` wrapper, checking `header->temp` | [x] |
| C30 | `stbds_hmput_key` | mode 0, `string.mode` implicit 0, `elemsize` 8 / `keysize` 4, 1 insert | [x] |
| C31 | `stbds_hmput_key` | mode 0, N = 1,5,6,7,8,12,13 inserts ⇒ crosses the first (`8→16`) and second (`16→32`) grow | [x] |
| C32 | `stbds_hmput_key` | mode 0, N = 100 and 1000 inserts ⇒ many rehashes; compare every bucket | [x] |
| C33 | `stbds_hmput_key` | mode 0, re-put of an existing key ⇒ update path (l.729-735), `temp` = existing index | [x] |
| C34 | `stbds_hmput_key` | mode 0, `keysize` 1 / 2 / 4 / 8 / 16 crossed with `elemsize` 8 / 12 / 16 / 24 | [x] |
| C35 | `stbds_hmput_key` | mode 0, `keysize == 0` ⇒ all keys equal (E46) | [x] |
| C36 | `stbds_hmput_key` | mode 1, no prior `shmode` ⇒ `string.mode = STBDS_SH_DEFAULT` (key pointer stored verbatim) | [x] |
| C37 | `stbds_hmput_key` | mode 2 (`PTR_TO_STRING`) on a fresh table ⇒ also `SH_DEFAULT` (`mode >= 1`) | [x] |
| C38 | `stbds_hmput_key` | mode −1 ⇒ binary path (`string.mode = 0`) (E45) | [x] |
| C39 | `stbds_shmode_func(elemsize, STBDS_SH_STRDUP)` + `stbds_hmput_key(mode=1)` | strdup arena: keys `strdup`'d; N = 1,8,20 keys; `temp_key` checked | [x] |
| C40 | `stbds_shmode_func(elemsize, STBDS_SH_ARENA)` + `stbds_hmput_key(mode=1)` | arena mode: `stbds_stralloc` used; N = 1,8,40,200 keys ⇒ crosses arena block boundaries | [x] |
| C41 | `stbds_shmode_func(elemsize, STBDS_SH_DEFAULT)` + `stbds_hmput_key(mode=1)` | keys stored as raw pointers | [x] |
| C42 | `stbds_shmode_func(elemsize, STBDS_SH_NONE=0)` + `stbds_hmput_key(mode=1)` | `default:` `memcpy` branch even though `mode >= 1` (E37) | [x] |
| C43 | `stbds_shmode_func(elemsize, 4)` and `(elemsize, 255)` | out-of-enum `string.mode` ⇒ `default:` `memcpy` branch | [x] |
| C44 | `stbds_shmode_func` | `mode` = 256, −1, 1000 ⇒ `(unsigned char)` truncation (E38) | [x] |
| C45 | `stbds_hmdel_key` | mode 0, delete an existing key that **is** the final element (`old_index == final_index`) | [x] |
| C46 | `stbds_hmdel_key` | mode 0, delete a key in the middle (`old_index != final_index` ⇒ `memmove` + re-find + index patch) | [x] |
| C47 | `stbds_hmdel_key` | mode 0, `keyoffset` = 0 and `keyoffset` > 0 (key not at the start of the element) | [x] |
| C48 | `stbds_hmdel_key` | mode 0, delete enough keys to trip the **shrink** branch (l.854) | [x] |
| C49 | `stbds_hmdel_key` | mode 0, delete/insert churn to trip the **tombstone rebuild** branch (l.858) without shrinking | [x] |
| C50 | `stbds_hmdel_key` | mode 1 + `SH_STRDUP` ⇒ the strdup'd key **is** freed (l.836) | [x] |
| C51 | `stbds_hmdel_key` | mode 1 + `SH_ARENA` / `SH_DEFAULT` ⇒ key not freed, string re-find | [x] |
| C52 | `stbds_hmdel_key` | mode 2 ⇒ string find + **binary** re-find asymmetry (E15) | [x] |
| C53 | `stbds_hmdel_key` | delete every key one by one down to empty, comparing the whole table each step | [x] |
| C54 | full pipeline | mode 0 binary map: randomized interleaved put/get/del over 500 ops, `elemsize` 8/16, `keysize` 4/8, whole-table snapshot compared after **every** op | [x] |
| C55 | full pipeline | mode 1 string map × `string.mode` ∈ {DEFAULT, STRDUP, ARENA}: randomized interleaved put/get/del over 300 ops, snapshot compared after every op | [x] |
| C56 | `stbds_hmfree_func` | `elemsize` 8/16 × `string.mode` ∈ {0, DEFAULT, STRDUP, ARENA}, non-empty and empty maps | [x] |
| C57 | `stbds_stralloc` | fresh arena, short string ⇒ 512-byte block, `block` 0→1, `remaining` compared | [x] |
| C58 | `stbds_stralloc` | many short strings ⇒ fast path (`len <= remaining`) then block refill chain; `block`/`remaining` compared at every step | [x] |
| C59 | `stbds_stralloc` | string longer than the next blocksize, arena **empty** ⇒ oversize path with `remaining = 0` (E27a) | [x] |
| C60 | `stbds_stralloc` | string longer than the next blocksize, arena **non-empty** ⇒ oversize splice path, `remaining` preserved (E27b) | [x] |
| C61 | `stbds_stralloc` | empty string `""` (`len == 1`) on fresh and warm arenas (E28) | [x] |
| C62 | `stbds_stralloc` | `block` driven to 22+ ⇒ blocksize saturates at `1<<20`, `block` stops incrementing (E26) | [x] |
| C63 | `stbds_strreset` | fresh/zeroed arena (no-op), single-block arena, multi-block arena, arena with a spliced oversize block | [x] |
| C64 | seed interaction | `stbds_rand_seed(s)` for s ∈ {0, 1, 0x31415926, SIZE_MAX, random} × the C54 pipeline ⇒ different probe orders and bucket layouts | [x] |
| C65 | `elemsize`/`keysize` mismatch | `keysize < elemsize` (the normal case) **and** `keysize == elemsize` for mode 0 | [x] |

## Row -> test mapping

Valid-path differential tests live in `tests/phase_b_low.rs` (C1-C21, C57-C63)
and `tests/phase_b_map.rs` (C22-C56, C64, C65). Every row is checked off only
after it passed across its randomized inputs (fixed seeds, so reruns are
reproducible).

| rows | covering test |
|------|---------------|
| C1 | `c1_hash_bytes_len_zero` |
| C2, C3, C4 | `c2_c3_c4_hash_bytes_all_lengths_random` (len 0..=200 x 24 buffers x 6 seeds) |
| C5 | `c5_hash_bytes_high_bit_sign_extension` |
| C6 | `c6_hash_bytes_seed_sweep` (400 buffers x all 64 single-bit seeds and their complements) |
| C7 | `c7_hash_string_empty` |
| C8, C10 | `c8_c10_hash_string_lengths` (len 0..=255) |
| C9 | `c9_hash_string_high_bytes` |
| C11 | `c11_rand_seed_and_global_advance` |
| C12 | `c12_arrgrowf_null_noop` |
| C13, C14 | `c13_c14_arrgrowf_fresh_min_cap_matrix` (6 elemsize x 8 addlen x 10 min_cap) |
| C15, C16, C17 | `c15_c16_c17_arrgrowf_existing` |
| C18, C19 | `c18_c19_arrgrowf_push_chain` (600 randomized grows per elemsize, header+payload compared at every step) |
| C20 | `c20_arr_push` |
| C21 | `c21_strkey` (-300..=300, INT_MIN/INT_MAX, 2000 random) |
| C22 | `c22_hmput_default_from_null` |
| C23 | `c23_hmput_default_twice_is_noop` |
| C24 | `c24_hmput_default_after_puts` |
| C25 | `c25_hmget_key_ts_from_null` |
| C26 | `c26_hmget_on_table_less_array` |
| C27, C29 | `c27_c29_binary_get_present_and_absent` |
| C28 | `c28_string_get_present_and_absent` |
| C30, C31 | `c30_c31_binary_put_growth_boundaries` |
| C32 | `c32_binary_put_many` (100 and 1000 inserts, every bucket compared after each) |
| C33 | `c33_binary_reput_existing_key` |
| C34, C65 | `c34_c65_elemsize_keysize_matrix` (5 elemsize x 5 keysize) |
| C35 | `c35_keysize_zero_all_keys_equal` |
| C36 | `c36_string_mode_fresh_table_gets_sh_default` |
| C37 | `c37_ptr_to_string_mode_fresh_table` |
| C38 | `c38_negative_mode_is_binary` |
| C39 | `c39_sh_strdup` (also checks `stbds_temp_key` content) |
| C40 | `c40_sh_arena` |
| C41 | `c41_sh_default_explicit` |
| C42, C43 | `c42_c43_out_of_enum_string_mode_uses_memcpy` |
| C44 | `c44_shmode_func_truncates_mode` |
| C45 | `c45_delete_final_element` |
| C46 | `c46_delete_middle_element` |
| C47 | `c47_delete_with_nonzero_keyoffset` |
| C48 | `c48_delete_triggers_shrink` (asserts the shrink branch really fired) |
| C49 | `c49_delete_insert_churn_triggers_rebuild` (asserts the rebuild branch really fired) |
| C50 | `c50_delete_strdup_frees_key` |
| C51 | `c51_delete_arena_and_default` |
| C52 | `c52_mode2_delete_last_element_only` |
| C53 | `c53_delete_everything_stepwise` (3 sizes x 3 deletion orders, incl. re-deleting an absent key) |
| C54 | `c54_pipeline_binary_randomized` (5 shapes x 500 ops) |
| C55 | `c55_pipeline_string_randomized` (4 string.modes x 400 ops) |
| C56 | `c56_hmfree_shapes` |
| C57..C63 | `c57_..c63_` in `tests/phase_b_low.rs` |
| C64 | `c64_seed_sweep_over_pipeline` (7 seeds x 3 pipelines) |
| extra | `strkey_driven_string_map` — keys produced by the library's own `strkey` |

## Notes

* Because the two `.so`s return heap pointers from different heaps, "outputs
  match byte-for-byte" is asserted on the **observable state**, not the pointer
  value: the full `stbds_array_header` (`length`, `capacity`, `temp`), all
  `length*elemsize` element bytes, and the full `stbds_hash_index`
  (`slot_count`, `used_count`, all four thresholds, `tombstone_count`, `seed`,
  `slot_count_log2`, `string.{remaining,block,mode}`) plus every
  `stbds_hash_bucket`'s `hash[8]` and `index[8]`. Pointer-valued fields
  (`temp_key`, `storage`, `string.storage`) are compared as
  *NULL / non-NULL / same-string-content / same-in-block-offset*, since raw
  addresses cannot match across heaps.
* Three properties of the C original constrain what a snapshot may compare, and
  each cost a false failure before being handled:
  1. `stbds_hmput_key` writes only the first `keysize` bytes of a new element;
     bytes `keysize..elemsize` are **caller-owned scratch** left as whatever
     `realloc` returned. The tests therefore define those bytes themselves
     (`MapPair::fill_value`) or normalise them (`zero_scratch`) before
     comparing.
  2. `stbds_make_hash_index` leaves `stbds_hash_index::temp_key`
     **uninitialised**, and no function ever clears it, so it is only defined
     immediately after a string-mode put. It is excluded from the generic
     state check and verified separately (`MapPair::temp_key_strs`).
  3. `SH_DEFAULT` stores the **caller's** key pointer verbatim, so key buffers
     must outlive the map; the randomized string pipeline leaks its key
     universe rather than handing over temporaries.
* `stbds_hash_seed` is a **process-global** mutable and `dlopen` of the same
  path returns the same loaded object, so `cargo test`'s default parallelism
  lets one test advance another's seed. `tests/common/both()` holds a
  process-wide mutex for the lifetime of the returned `Lib`s.
* There is **no `[features]` table** in `Cargo.toml`, so the only feature
  combination is the default = empty set; `run_all.sh` derives this
  mechanically from `Cargo.toml` (and would enumerate the full power set if
  features were added) and runs both `--release` and
  `--release --no-default-features`.
* The project builds **no binary**: `Cargo.toml` is `crate-type = ["cdylib"]`
  with no `src/main.rs` or `[[bin]]`, and `c_src/CMakeLists.txt` uses
  `add_library(... SHARED ...)` with no `add_executable`. There is therefore no
  stdout to compare; `run_all.sh` asserts this premise rather than assuming it.
* `cargo test` does **not** rebuild a `cdylib`, so `tests/common/mod.rs`
  refuses to run against a `.so` older than `src/` — otherwise the whole suite
  can report green against a binary that does not match the sources.
