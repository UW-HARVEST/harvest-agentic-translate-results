# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return` that is not the
normal success return, every explicit comparison/range check, and every place
where the C performs **no** check at all (and therefore has a definite,
observable behaviour that the Rust must reproduce) gets one row.

Grep evidence — the complete set of `return` statements and `if`s in the C:

```
lib.c:7    if ((bs->pos += n) > bs->limit)
lib.c:8        return 0;                      <- get_bits truncation
lib.c:14   return cache | (next >> -shl);     <- get_bits success
lib.c:105  if (gr->big_values > 288) {
lib.c:106      return -1;                     <- error 1
lib.c:116  if (!gr->block_type) {
lib.c:117      return -1;                     <- error 2
lib.c:159  if (part_23_sum + bs->pos > bs->limit + main_data_begin * 8) {
lib.c:160      return -1;                     <- error 3
lib.c:162  return main_data_begin;            <- success
```

There are **no** `assert`s, no `return NULL`, no error enums, no null-pointer
checks and no `errno` use anywhere in the library
(`grep -cE 'assert|NULL|errno|enum' c_src/src/lib.c` ⇒ 0).

`main_data_begin` (the success value) is in `0..=511`, so `-1` is an
unambiguous sentinel.

Legend for "expected C result": `ret` = value returned by `read_side_info`.

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `get_bits` (via `read_side_info`) | `bs->pos + n > bs->limit` for a single read (`limit` cut so that the *first* read of the side-info already overruns) | returns 0 **without reading memory**, but `bs->pos` is *still* advanced by `n`; every later read also returns 0; `read_side_info` then falls through and hits row 4 ⇒ `ret == -1` | `err_01_get_bits_truncates_first_read` |
| 2  | `get_bits` | `bs->pos + n > bs->limit` for a read *in the middle* of a granule (limit set to any bit position inside the side info) | all fields from that read onward decode as 0 (`block_type` becomes 0 on the window-switch path ⇒ row 3, or `ret == -1` via row 4); `bs->pos` keeps advancing past `limit` | `err_02_get_bits_truncates_midstream` (all cut points 0..=total_bits) |
| 3  | `get_bits` | `bs->limit < 0` (e.g. `-1`, `INT_MIN`) with `bs->pos == 0` | first read overruns immediately ⇒ every field 0, no memory read at all ⇒ `ret == -1` | `err_03_negative_limit` |
| 4  | `get_bits` | `bs->pos == bs->limit` on entry (zero readable bits) | all reads truncate ⇒ `ret == -1` | `err_04_zero_readable_bits` |
| 5  | `get_bits` | `bs->pos + n == bs->limit` exactly (boundary, **not** an error: the check is `>` not `>=`) | read succeeds and returns the real bits | `err_05_limit_boundary_exact` |
| 6  | `get_bits` | `bs->pos` overflow: `pos = INT_MAX`, `limit = INT_MIN` ⇒ `pos += n` wraps to a negative value which is still `> limit` … then wraps below `limit` | 2's-complement wrap of `bs->pos`, truncating reads, no memory access; `ret == -1`; final `bs->pos` must match bit-for-bit | `err_06_pos_int_overflow` |
| 7  | `read_side_info` | `gr[0].big_values > 288` (i.e. the 9-bit field decodes to 289..511) | `ret == -1`; `gr[0]` has only `part_23_length` and `big_values` written, everything else (incl. `sfbtab`, `region_count[2]`) left at the caller's previous value; `gr[1..]` untouched; `bs->pos` left at the position after the `big_values` read | `err_07_big_values_gt_288_granule0` (all 289..=511) |
| 8  | `read_side_info` | `gr[k].big_values > 288` for a **later** granule `k` (k = 1, 2, 3 for the gr_count = 2 / 4 configurations) | `ret == -1` after granules `0..k` were fully written and `gr[k]` partially written; later granules untouched | `err_08_big_values_gt_288_late_granule` |
| 9  | `read_side_info` | `big_values == 288` (boundary — **not** an error) | no error from this check | `err_09_big_values_288_ok` |
| 10 | `read_side_info` | window-switching flag = 1 **and** the 2-bit `block_type` field = 0 | `ret == -1`; `gr[k]` has `part_23_length`, `big_values`, `global_gain`, `scalefac_compress`, `sfbtab = &g_scf_long[sr_idx]`, `n_long_sfb = 22`, `n_short_sfb = 0`, `block_type = 0` written; `mixed_block_flag`, `region_count`, `table_select`, `subblock_gain`, `preflag`, `scalefac_scale`, `count1_table`, `scfsi` untouched | `err_10_block_type_zero` (every granule index) |
| 11 | `read_side_info` | window-switching flag = 1 and `block_type` = 0 caused *indirectly* by truncation (`limit` cut exactly before the 2-bit block_type read) | same as row 10, `ret == -1` | `err_11_block_type_zero_by_truncation` |
| 12 | `read_side_info` | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` (all granules decoded fine, but the sum of `part_23_length`s does not fit) | `ret == -1`, **after** all `gr_count` granules were fully written | `err_12_part23_sum_overrun` |
| 13 | `read_side_info` | `part_23_sum + bs->pos == bs->limit + main_data_begin * 8` exactly (boundary — check is `>`) | `ret == main_data_begin` (success) | `err_13_part23_sum_exact_boundary` |
| 14 | `read_side_info` | signed overflow of the right-hand side: `bs->limit` near `INT_MAX` and `main_data_begin * 8 > 0` ⇒ `limit + mdb*8` wraps negative, so a perfectly valid stream is rejected | `ret == -1` (2's-complement wrap) | `err_14_limit_plus_mdb_overflow` |
| 15 | `read_side_info` | `bs->pos < 0` (negative bit position; `pos >> 3` is an arithmetic shift so `bs->buf` is indexed *before* the start) — the C has no check | reads `buf[pos>>3]`, i.e. behind the pointer, and returns those bits; both libraries must agree bit-for-bit (tested with a pointer into the middle of a large buffer so the access is defined) | `err_15_negative_pos` |
| 16 | `read_side_info` | `sr_idx == 8` (`hdr[2]` bits 2-3 = 3, `hdr[1]` bits 3 **and** 4 set) — one row **past** the end of the `[8][…]` tables; the C has no bounds check | out-of-bounds `.rodata` read: `&g_scf_long[8]` = 8 zero pad bytes + start of `g_scf_short[0]`; `&g_scf_short[8]` = `g_scf_mixed[0]`; `&g_scf_mixed[8]` runs off the end of `.rodata` (build-specific unwind data — see note below) | `err_16_sr_idx_8_oob_rows` |
| 17 | `read_side_info` | `bs == NULL` | no check ⇒ immediate deref of `bs->pos` ⇒ `SIGSEGV`; the Rust must fault identically (not panic, not return) | `err_17_null_bs` (subprocess, compares signal) |
| 18 | `read_side_info` | `gr == NULL` (with a valid, non-truncating `bs`) | no check ⇒ deref of `gr->part_23_length` ⇒ `SIGSEGV` | `err_18_null_gr` (subprocess) |
| 19 | `read_side_info` | `hdr == NULL` | no check ⇒ deref of `hdr[2]` ⇒ `SIGSEGV` | `err_19_null_hdr` (subprocess) |
| 20 | `read_side_info` | `bs->buf == NULL` **and** `limit` large enough that `get_bits` actually reads | no check ⇒ deref of `NULL + (pos>>3)` ⇒ `SIGSEGV` | `err_20_null_bs_buf` (subprocess) |
| 21 | `read_side_info` | `bs->buf == NULL` but `bs->limit < bs->pos + n` (every read truncates before dereferencing `p`) | **no** fault — the C computes `p` but never loads from it ⇒ `ret == -1` | `err_21_null_buf_but_truncated` |
| 22 | `read_side_info` | out-of-range "enum" values across the FFI boundary | *N/A by construction*: the API takes no `enum` and no flag word. The only quasi-enumerated inputs are the 2-bit `block_type` (all four values 0..3 are reachable and handled: 0 ⇒ row 10, 1/3 ⇒ long tables, 2 ⇒ short/mixed) and the four `hdr` bytes, for which **all** 2^32 bit patterns are legal inputs that the C accepts without validation. Covered exhaustively over `block_type` and by the randomized `hdr` sweep. | `err_22_block_type_all_values`, `cfg_38_random_hdr_sweep` |
| 23 | `read_side_info` | **unaligned** `bs_t *` and `L3_gr_info_t *` (pointers skewed by 1..7 bytes) — the C has no alignment requirement enforcement and x86-64 loads/stores tolerate it | identical outputs to the aligned case; no fault | `err_23_unaligned_pointers` |

## Result

All 23 rows have a passing differential test:

```
$ cargo build && cargo test --test phase_c_errors
running 23 tests
...
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Note on row 16 (`sr_idx == 8`, mixed table)

`&g_scf_mixed[8]` points 832 bytes into the C object's `.rodata`, which is
exactly its end (`objdump -h` ⇒ `.rodata 00000340 @ 0x2000`). The next bytes
belong to `.eh_frame_hdr`, i.e. link-time unwind data containing PC-relative
offsets — they differ between any two builds of the *same* C source and are not
reproducible in principle. `src/lib.rs` therefore lays the three tables out
exactly like the reference C build (`long` +0, 8 pad bytes, `short` +192,
`mixed` +512) so that rows 8 of the **long** and **short** tables match the C
byte-for-byte, and supplies deterministic zeros beyond +832 so the Rust can
never fault. The differential test asserts equality for the long and short
row-8 reads and only skips the byte comparison for the `mixed` row-8 read,
while still asserting that every other output (return value, all 24
non-pointer struct bytes of every granule, `bs->pos`) matches.

Additionally, gcc only emits the three tables in declaration order at `-O0`
(the reference build). At `-O1` and above the order is reversed
(`g_scf_mixed`, `g_scf_short`, `g_scf_long`), so which bytes rows 8 alias onto
is a property of the *particular* C build, not of the source:

```
$ for o in -O0 -O2; do gcc $o -fPIC -shared lib.c -o t.so; nm t.so | grep g_scf; done
-O0: 2000 g_scf_long   20c0 g_scf_short  2200 g_scf_mixed
-O2: 2000 g_scf_mixed  2140 g_scf_short  2280 g_scf_long
```

`src/lib.rs` matches the `-O0` layout, i.e. the build produced by
`c_src/CMakeLists.txt` (which sets no `CMAKE_BUILD_TYPE`).
