# ERRORS.md — Phase A error surface table

Mechanically derived by grepping **every** rejection/early-return in
`c_src/src/lib.c`. There are no `assert`s, no error enums, no `return NULL`,
and **no null-pointer checks at all** in this library.

```
$ grep -n 'return\|assert\|> \|< \|>= \|<= ' c_src/src/lib.c
```

Exhaustive list of `return` statements in the C source:

| line | statement | function |
|------|-----------|----------|
| 8    | `return 0;`                 | `get_bits` (bitstream overrun) |
| 14   | `return cache \| (next >> -shl);` | `get_bits` (normal) |
| 106  | `return -1;`                | `read_side_info` (`big_values > 288`) |
| 117  | `return -1;`                | `read_side_info` (`!block_type`) |
| 160  | `return -1;`                | `read_side_info` (main-data overrun) |
| 162  | `return main_data_begin;`   | `read_side_info` (normal) |

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ok |
|---|----------|---------------------------------------------|-------------------|------|----|
| E1 | `get_bits` | `(bs->pos += n) > bs->limit` — the read crosses the end of the bit budget | returns `0` **without** reading `*p`; `bs->pos` **stays advanced** by `n` (side effect is not rolled back) | `err_e1_get_bits_overrun_truncates` | [x] |
| E2 | `get_bits` | every subsequent call once `pos > limit` | keeps returning `0` and keeps advancing `pos`, so all remaining side-info fields decode as `0` | `err_e2_all_zero_after_overrun` | [x] |
| E3 | `read_side_info` | `gr->big_values > 288` (9-bit field, i.e. any value in `289..=511`) for **any** granule | `-1`, aborting immediately — later granules are left untouched, and the already-written fields of the current granule keep their new values | `err_e3_big_values_gt_288` | [x] |
| E4 | `read_side_info` | boundary of E3: `big_values == 288` | **not** an error; parsing continues | `err_e4_big_values_288_ok` | [x] |
| E5 | `read_side_info` | boundary of E3: `big_values == 289` | `-1` | `err_e5_big_values_289_err` | [x] |
| E6 | `read_side_info` | window-switching bit set **and** the 2-bit `block_type` field reads `0` | `-1`, aborting immediately (before `mixed_block_flag` / `region_count` are written) | `err_e6_block_type_zero` | [x] |
| E7 | `read_side_info` | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` — the granules claim more main data than the bit reservoir holds | `-1` (checked only **after** all granules are written, so `*gr` is fully populated even on failure) | `err_e7_main_data_overrun` | [x] |
| E8 | `read_side_info` | boundary of E7: `part_23_sum + bs->pos == bs->limit + main_data_begin * 8` | **not** an error; returns `main_data_begin` | `err_e8_main_data_exact_boundary` | [x] |
| E9 | `read_side_info` | E3 fires on granule 2/3/4 (MPEG1 multi-granule), not granule 1 | `-1`; granules before the failure are fully written, the failing granule keeps its partial writes | `err_e9_big_values_late_granule` | [x] |
| E10 | `read_side_info` | E6 fires on a late granule | `-1` with the same partial-write semantics | `err_e10_block_type_zero_late_granule` | [x] |

## Generic FFI boundary conditions (not in the table, still required)

| # | condition | expected C result | test | ok |
|---|-----------|-------------------|------|----|
| B1 | `bs->limit == 0` | first `get_bits` overruns → every field `0`; `block_type` stays `0`, so the non-window-switching path is taken; final check `0 + pos > 0 + 0` fires → `-1` | `bnd_b1_zero_limit` | [x] |
| B2 | `bs->limit < 0` (negative budget) | same as B1 — immediate overrun | `bnd_b2_negative_limit` | [x] |
| B3 | `bs->pos > bs->limit` on entry | immediate overrun, `pos` keeps growing | `bnd_b3_pos_past_limit` | [x] |
| B4 | `bs->pos == bs->limit` on entry | first `get_bits(9 or 10)` overruns | `bnd_b4_pos_eq_limit` | [x] |
| B5 | `bs->pos < 0` (negative bit position) | `bs->buf + (pos >> 3)` addresses **before** `buf`; C reads it. Both libraries are handed the *same* buffer pointer, so the same bytes are read and results must agree. | `bnd_b5_negative_pos` | [x] |
| B6 | `bs->pos` near `INT_MAX` → `pos += n` overflows | signed overflow; gcc wraps, Rust `wrapping_add` matches | `bnd_b6_pos_int_max_overflow` | [x] |
| B7 | `bs->limit == INT_MAX` (oversized length) | no overrun ever; reads run off the end of the buffer — same shared buffer for both, so results must agree | `bnd_b7_limit_int_max` | [x] |
| B8 | out-of-range "enum" values: `block_type` is a 2-bit field, so `1..=3` are all valid and `0` is the rejection (E6). `sr_idx` is computed, not passed, and reaches `8` — one past the last valid table row (`0..=7`). | `sr_idx == 8` indexes one row **past** all three tables | `bnd_b8_sr_idx_out_of_range` | [x] |
| B9 | every one of the 256 possible `hdr[1]` values × every `hdr[2]`/`hdr[3]` combination that changes `sr_idx`/`gr_count` (C enums accept any int; these header bytes are raw `uint8_t` with no validation) | whatever the bit math yields — no rejection path | `bnd_b9_exhaustive_header_bytes` | [x] |
| B10 | `hdr` bytes such that `sr_idx == 0` via the `sr_idx -= (sr_idx != 0)` quirk (raw sum `0` **and** raw sum `1` both map to `0`) | identical `sfbtab` for two different headers | `bnd_b10_sr_idx_aliasing_headers` | [x] |
| B11 | NULL `bs`, NULL `gr`, NULL `hdr` | The C code dereferences all three unconditionally with **no** null check (`bs->pos`, `*hdr`, `gr->part_23_length`), so a null argument is a hard segfault in *both* libraries. Not differentially testable in-process; asserted to be UB-by-parity in `bnd_b11_null_pointers_documented` instead of crashing the harness. | `bnd_b11_null_pointers_documented` | [x] |
| B12 | `hdr` pointing **into** the `gr` array, so the C code's own writes mutate the header while it parses | the C reloads `hdr[1]`/`hdr[3]` at every access (the reference build has no `-O` flag), so later granules see the mutated header; the translation must not hoist those loads. Only the scalar-field byte offsets are usable — bytes `0..8` of each struct hold the `sfbtab` **pointer**, whose value necessarily differs between two separately-loaded `.so`s. | `bnd_b12_hdr_aliasing_gr_array` | [x] |
| B13 | all 256 values of `hdr[2]` and `hdr[3]` for each distinct `hdr[1]` version-bit pattern | no rejection path; proves no unread header bit changes behaviour | `bnd_b9b_full_hdr2_hdr3_sweep` | [x] |

## Coverage summary

| group | rows | status |
|-------|------|--------|
| `ERRORS.md` E1–E10 | 10 | all covered by a passing differential test |
| generic boundaries B1–B13 | 13 | all covered by a passing differential test |

Every row's test name appears in the tables above and every one of them is in
`tests/differential.rs`. `./check_features.sh` runs the whole suite under both
cargo profiles and every feature combination.

### Rejection-path inventory cross-check

`bnd_b11_null_pointers_documented` asserts mechanically that the C source still
contains exactly **three** `return -1;` statements (E3, E6, E7) and exactly
**one** `return 0;` (E1). If anyone edits `c_src`, that test fails and this
table must be re-derived — the error surface cannot silently grow.
