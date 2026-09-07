# ERRORS.md — error-surface table

Mechanically derived from every rejection / early-return / bound in
`c_src/src/lib.c`. Greps used:

```
grep -n 'return' c_src/src/lib.c      # 5 return statements total
grep -n 'assert\|NULL\|abort\|errno' c_src/src/lib.c   # none
grep -n '> 288\|>= 500\|0x0F0F\|255 >>\|& 7' c_src/src/lib.c
```

`c_src` contains **no** `assert`, **no** null checks, **no** error enum and no
`errno` use. All rejection is by return value: `get_bits` returns `0`,
`read_side_info` returns `-1`.

| #  | function | trigger (exact invalid input/condition) | expected C result |
|----|----------|------------------------------------------|-------------------|
| E1 | `get_bits` (`lib.c:7`) | `(bs->pos += n) > bs->limit` — request runs past the limit. Note `bs->pos` is advanced *before* the test, so it stays advanced on failure and every subsequent call also fails. Buffer is **not** read. | returns `0`; `bs->pos == old_pos + n` |
| E2 | `get_bits` (`lib.c:7`) | boundary: `bs->pos + n == bs->limit` exactly — NOT an error (test is `>`, not `>=`) | normal value returned |
| E3 | `read_side_info` (`lib.c:105`) | `gr->big_values > 288` after reading 9 bits, i.e. any of the 223 encodings 289..511 in the `big_values` field of ANY granule | returns `-1` immediately; `gr[i]` partially written (`sfbtab`/`n_long_sfb`/`n_short_sfb` and everything after `big_values` untouched for that granule), `bs->pos` left where the `big_values` read ended |
| E4 | `read_side_info` (`lib.c:116`) | `!gr->block_type` — window-switching flag bit is 1 but the following 2 `block_type` bits are `00` | returns `-1` immediately; `gr[i].block_type == 0`, fields from `mixed_block_flag` on untouched, `bs->pos` left after the 2-bit read |
| E5 | `read_side_info` (`lib.c:159`) | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` — sum of all granules' `part_23_length` overflows the available main-data | returns `-1` **after** all granules have been fully written |
| E6 | `read_side_info` (`lib.c:159`) | boundary: `part_23_sum + bs->pos == bs->limit + main_data_begin * 8` exactly — NOT an error | returns `main_data_begin` |
| E7 | `read_side_info` (`lib.c:162`) | success | returns `main_data_begin` (0..511 for MPEG1, 0..511 for MPEG2 after the `>> gr_count`), never negative |
| E8 | `read_side_info` (`lib.c:111,126,130`) | `sr_idx == 8` (sample-rate bits `11` + MPEG1 bit + extension bit) indexes one row PAST `g_scf_long[8]` / `g_scf_short[8]` / `g_scf_mixed[8]`. No check exists — C reads out of bounds. | no error returned; `sfbtab` aliases whatever the linker placed next in `.rodata`. Two of the three rows land on the next table inside the same section and are compared byte-for-byte; the third runs off the section end (see N5). The array order is optimisation dependent (`-O0`: long, pad, short, mixed; `-O2`: mixed, short, long), which is why `src/lib.rs` carries both layouts behind the `c_layout_o2` feature. |
| E9 | `read_side_info` | `bs->limit < 0`, or `bs->pos > bs->limit` on entry — every `get_bits` returns 0 from the first call | all fields derive from all-zero bits: `block_type=0`, `region_count={0,0,255}`, and E5's check then decides the return |
| E10 | `read_side_info` | `bs->limit` large but `bs->pos` negative → `bs->buf + (pos>>3)` reads *before* the buffer (arithmetic shift keeps the sign) | no check; C reads OOB. Reproduced by Rust identically (same pointer arithmetic) |
| E11 | `get_bits` (`lib.c:4,9`) | `s = bs->pos & 7` masks the first byte with `255 >> s`; `bs->pos` not byte-aligned on entry | first partial byte's high `s` bits dropped |
| E12 | `read_side_info` (`lib.c:152`) | `scalefac_compress >= 500` (only reachable for non-MPEG1, where the field is 9 bits, so 500..511) sets `preflag = 1` without reading a bit | `preflag == 1` |
| E13 | `read_side_info` (`lib.c:123`) | `block_type == 2` masks `scfsi &= 0x0F0F`, discarding scfsi bits for later granules | later granules' `gr->scfsi` change |

## Not testable / UB in both implementations (documented, deliberately not exercised)

| # | trigger | why not tested |
|---|---------|----------------|
| N1 | `bs == NULL`, `gr == NULL`, or `hdr == NULL` | C has no null check; it dereferences immediately (`hdr[2]`, `bs->pos`). Both C and Rust segfault. A differential test would abort the test process, so this is asserted only by inspection. |
| N2 | `hdr` shorter than 4 bytes | C reads `hdr[1..3]` unconditionally; OOB read in both. Tests always pass a 4-byte header. |
| N3 | `bs->buf` shorter than `(limit+7)/8` bytes | C reads up to `(pos+n+7)/8` bytes; OOB read in both. Tests always over-allocate the buffer. |
| N4 | `gr` array shorter than `gr_count` (1/2/4) entries | C writes `gr[0..gr_count]`; OOB write in both. Tests always pass a 4-element array. |
| N5 | `sr_idx == 8` combined with whichever table the C compiler emitted LAST in `.rodata` (`g_scf_mixed` at `-O0`, `g_scf_long` at `-O2`) | that row 8 runs off the end of `.rodata` into `.eh_frame_hdr`; no implementation can reproduce those bytes portably. The return value, `bs->pos` and all 32 struct bytes *are* still compared; only the dereferenced `sfbtab` contents are skipped, for that one sub-case only. The harness picks the right sub-case from `UNREPRODUCIBLE_ROW8` (`run_all.sh` sets it per configuration). |

## Checklist (Phase C)

- [x] E1  — `err_e1_get_bits_past_limit_returns_zero_and_advances`
- [x] E2  — `err_e2_get_bits_exact_limit_is_not_an_error`
- [x] E3  — `err_e3_big_values_gt_288`
- [x] E4  — `err_e4_block_type_zero`
- [x] E5  — `err_e5_part23_overflow`
- [x] E6  — `err_e6_part23_exact_boundary`
- [x] E7  — `err_e7_success_returns_main_data_begin`
- [x] E8  — `err_e8_sr_idx_8_out_of_range_row` (verified under both `.rodata` layouts)
- [x] E9  — `err_e9_zero_and_negative_limit`
- [x] E10 — `err_e10_negative_pos`
- [x] E11 — `err_e11_unaligned_start_pos`
- [x] E12 — `err_e12_scalefac_compress_500_sets_preflag`
- [x] E13 — `err_e13_block_type_2_masks_scfsi`
- [x] generic: zero length / oversized length (`err_generic_extreme_limits`)
- [x] generic: one step past valid range —
      `err_generic_one_past_big_values` (287/288 ok vs 289/290 error),
      `err_generic_one_past_scalefac_compress` (499 vs 500), and
      `err_e2_get_bits_exact_limit_is_not_an_error` (`pos+n` == vs > `limit`)
- [x] generic: out-of-range "enum" values across FFI — `block_type` (0..3, all
      4 raw 2-bit values incl. the rejected 0), `sr_idx` (0..8 incl. the
      reserved sample-rate 3 and the reserved MPEG version bits), channel-mode
      `hdr[3]>>6` (all 4), and all 256 values of `hdr[1]`/`hdr[2]`/`hdr[3]`
      (`err_generic_exhaustive_header_bytes`, plus
      `c39_all_hdr1_x_hdr3_and_all_hdr2` for all 65 536 `hdr[1]`x`hdr[3]` pairs)

## Harness sensitivity (negative control)

The error/config suites were validated by mutation testing: 27 single-token
mutations were injected into `src/lib.rs` one at a time (wrong comparison
operator on every bound, wrong bit width on every `get_bits` call, wrong shift
amount, wrong constant, swapped field writes, `.rodata` offsets swapped,
`get_bits` advancing `pos` after instead of before the limit test, granule
pointer not advancing, `part_23_sum` not accumulating, ...). **All 27 were
detected**; no mutant survived. See the "MUTANT [...]" runs in the session log.
