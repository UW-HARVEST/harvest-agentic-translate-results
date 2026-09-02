# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Every `return` that is not the
normal success return, every implicit rejection, every explicit range check and
every magic constant in the source was enumerated. There are **no** `assert`s,
**no** `NULL` checks, **no** error enums and **no** `#ifdef`s in this library —
the whole rejection surface is four sites plus the unchecked-input consequences
listed at the end.

Source sites (line numbers from `c_src/src/lib.c`):

* `L7–L8`   `if ((bs->pos += n) > bs->limit) return 0;`   (in `get_bits`)
* `L105–L106` `if (gr->big_values > 288) { return -1; }`
* `L116–L117` `if (!gr->block_type) { return -1; }`
* `L159–L160` `if (part_23_sum + bs->pos > bs->limit + main_data_begin * 8) { return -1; }`
* `L162`    `return main_data_begin;` (success)

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|--------------------------------------------|-------------------|------|
| E1 | `get_bits` | `bs->pos + n > bs->limit` — bit request runs past the end of the bit reservoir | returns `0`; **`bs->pos` is still advanced by `n`** (the `+=` happens inside the condition, before the early return). No byte is read from `bs->buf`. | `e1_get_bits_past_limit` |
| E2 | `get_bits` | `bs->limit < 0` (or `bs->pos` already `> limit` on entry) — every call rejects | every `get_bits` returns `0` ⇒ all granule fields become the all-zero decode; `block_type` is `0` on the `window_switching==0` path so E4 does not fire; final check `0 + pos > limit + 0` ⇒ `-1` | `e2_limit_negative_all_reads_rejected` |
| E3 | `read_side_info` | `big_values` field (9 bits, granule *g*) decodes to `> 288`, i.e. any of `289..=511` | returns `-1` **immediately**, after `part_23_length` and `big_values` of granule *g* were already stored (partial write is observable) | `e3_big_values_over_288` |
| E4 | `read_side_info` | `window_switching` bit is `1` **and** the following 2-bit `block_type` field is `0` | returns `-1`, after `part_23_length`, `big_values`, `global_gain`, `scalefac_compress`, `sfbtab`, `n_long_sfb`, `n_short_sfb`, `block_type=0` were stored for granule *g* | `e4_block_type_zero` |
| E5 | `read_side_info` | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` — the granules claim more main-data bits than the reservoir plus `main_data_begin` can supply | returns `-1` after **all** granules have been fully written | `e5_part23_sum_overruns` |
| E6 | `read_side_info` | boundary of E3: `big_values == 288` exactly | **accepted** (`>` not `>=`) — must not return `-1` for this reason | `e3_big_values_over_288` (boundary half) |
| E7 | `read_side_info` | boundary of E5: `part_23_sum + bs->pos == bs->limit + main_data_begin * 8` exactly | **accepted** (`>` not `>=`) — returns `main_data_begin` | `e5_part23_sum_overruns` (boundary half) |
| E8 | `read_side_info` | `block_type` decodes to `1`, `2` or `3` | **accepted** — only `0` is rejected; there is no upper check, and a 2-bit field cannot exceed `3` | `phase_b_*` rows |

## Unchecked inputs (C performs NO validation — the Rust must reproduce, not fix)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| U1 | `read_side_info` | `sr_idx == 8` (reachable: `hdr[1]` bits 3 **and** 4 set and `(hdr[2]>>2)&3 == 3` ⇒ `9-1 = 8`) while the tables are only `[8][...]` | **out-of-range table index, no check.** `gr->sfbtab` is set to `&g_scf_*[0][0] + 8*rowsize`, i.e. one row past the end of the table. The pointer is computed but never dereferenced by this function. | `u1_sr_idx_out_of_range` |
| U2 | `read_side_info` | `gr` array shorter than `gr_count` granules (`gr_count` is 1, 2 or 4 and is derived from `hdr`, never from a caller-supplied capacity) | **no check** — writes `gr[0..gr_count)` unconditionally | covered by sizing the buffer to 8 granules and asserting the untouched tail is identical |
| U3 | `read_side_info` / `get_bits` | `bs->buf` shorter than the bits `get_bits` touches (`get_bits` reads `ceil((n + (pos&7))/8)` bytes, which can extend past `bs->limit`) | **no check** — reads past `limit` as long as `pos+n <= limit` | exercised by every row (buffers are oversized so the read is defined) |
| U4 | `read_side_info` | `hdr` is only ever indexed `[1]`, `[2]`, `[3]` — `hdr[0]` is never read | `hdr[0]` must not affect the result | `u4_hdr0_ignored` |
| U5 | `read_side_info` | `bs`, `gr` or `hdr` is `NULL` | **no null check** — the C dereferences immediately and faults. Not a defined rejection; deliberately NOT executed as a differential test (it would abort the harness). Documented for completeness; the Rust translation is likewise unchecked (`(*bs)`, `(*gr)`, `*hdr.add(1)`), so it matches. | n/a (documented; would SIGSEGV in both) |
| U6 | `read_side_info` | `bs->pos` large enough that `pos += n` overflows `int` | Signed overflow — UB in C. **Not reachable as a differential test:** after the wrap `pos` is negative, so the `pos > limit` guard stops rejecting and `get_bits` dereferences `bs->buf + (pos >> 3)` at a hugely negative byte index. Both the C and the Rust (`wrapping_add` + the same raw pointer arithmetic) fault there, so the harness would die instead of comparing. Documented, and the reachable non-overflowing extremes (`pos` up to `i32::MAX - 512`, `limit` down to `i32::MIN`) are asserted. | `u6_signed_overflow_documented_not_executed`, `ffi_extreme_pos_limit` |

## Magic constants inventory (all reproduced in the Rust)

| constant | site | meaning |
|----------|------|---------|
| `288` | `L105` | max `big_values` |
| `500` | `L152` | `scalefac_compress >= 500` ⇒ `preflag` on the MPEG-1 (`hdr[1]&8 == 0`) path |
| `255` | `L9`, `L120`, `L146` | first-byte mask `255 >> s`; `region_count[1]`/`region_count[2]` sentinel |
| `0x0F0F` | `L123` | `scfsi &= 0x0F0F` when `block_type == 2` |
| `0xC0` | `L90`, `L99` | `(hdr[3] & 0xC0) == 0xC0` ⇒ single-channel ⇒ `gr_count = 1` and extra `scfsi <<= 4` |
| `0x8` | `L91`, `L110`, `L128`, `L152` | `hdr[1] & 8`: granule-count doubling, 4-vs-9-bit `scalefac_compress`, `n_long_sfb` 8-vs-6, `preflag` read-vs-derived |
| `22 / 0 / 39 / 30` | `L112-113`, `L127-131` | `n_long_sfb` / `n_short_sfb` per table selection |
| `7 / 8 / 255` | `L119-120`, `L126` | `region_count` defaults for the window-switching path |

## Phase C status

All rows above are checked off; `cargo test --release --test phase_c_errors`
reports **16 passed, 0 failed**. Test-to-row mapping:

| test | rows covered |
|------|--------------|
| `e1_get_bits_past_limit` | E1 |
| `e1_one_bit_short_advances_pos` | E1 (pos-advance semantics) |
| `e2_limit_negative_all_reads_rejected` | E2 |
| `e2b_pos_already_past_limit` | E2 |
| `e3_big_values_over_288` | E3, E6 |
| `e3b_big_values_over_288_on_later_granule` | E3 (per-granule) |
| `e4_block_type_zero` | E4 |
| `e4b_block_type_zero_later_granule` | E4 (per-granule) |
| `e8_block_type_1_2_3_accepted` | E8 |
| `e5_part23_sum_overruns` | E5, E7 |
| `e5b_reservoir_slack_sweep` | E5, E7 (±8-bit window around the boundary) |
| `u1_sr_idx_out_of_range` | U1 |
| `u4_hdr0_ignored` | U4 |
| `u6_signed_overflow_documented_not_executed` | U6 |
| `ffi_all_header_bitfield_combinations` | every combination of the header bit-fields the C reads (the "out-of-range enum" analogue for this API — all 64 combinations of `hdr[1]` bits 3–4 × `hdr[2]` bits 2–3 × `hdr[3]` bits 6–7, ×64 random bitstreams each) |
| `ffi_extreme_pos_limit` | generic boundaries: `limit` `i32::MIN`, `-1`, `0`, `pos-1`, `pos`, `pos+1`, `i32::MAX`; `pos` from `0` to `i32::MAX - 512` |
