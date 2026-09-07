# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return` that is not the
normal success return, every range/limit comparison, every magic constant that
gates behaviour, and every unchecked-input class. There are **no** `assert`s,
**no** `NULL` checks, **no** `errno` use, **no** error enums and **no**
`RETURN_ERROR`-style macros in this library (`grep -E 'assert|NULL|errno|enum|abort|exit'`
over `src/lib.c` + `include/lib.h` matches nothing), so the surface is: one
sentinel return in `get_bits`, three `return -1` sites in `read_side_info`, and
the unchecked pointer/index classes below.

`grep -n return c_src/src/lib.c`:

```
8:          return 0;                         <- get_bits limit sentinel   (E1)
14:         return cache | (next >> -shl);    <- get_bits success
106:            return -1;                    <- big_values > 288          (E2)
117:                return -1;                <- block_type == 0           (E3)
160:        return -1;                        <- main-data bounds check    (E4)
162:    return main_data_begin;               <- success
```

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `get_bits` (L8) | `(bs->pos += n) > bs->limit` — the read would cross `limit`. `pos` is **still advanced by `n`** and no byte is dereferenced. | returns `0` (sentinel, indistinguishable from a real zero field); `bs->pos == pos_before + n` | `err_e1_get_bits_limit_sentinel`, `err_e1_limit_sweep_every_field` | [x] |
| E2 | `read_side_info` (L105-107) | `gr->big_values > 288`, i.e. the 9-bit `big_values` field of **any** granule reads 289..511. | returns `-1` immediately. Struct is **partially written**: `part_23_length` and `big_values` of that granule are set, `global_gain`/`scalefac_compress`/`sfbtab`/`n_*_sfb` and everything after are NOT; earlier granules are fully written. `bs->pos` left where the `big_values` read finished. | `err_e2_big_values_over_288`, `err_e2_boundary_288_vs_289`, `err_e2_second_granule` | [x] |
| E3 | `read_side_info` (L116-118) | window-switching bit == 1 **and** the following 2-bit `block_type` == 0. | returns `-1` immediately. `gr->block_type` is set to 0, `mixed_block_flag` and `region_count[]` are NOT written; `sfbtab`/`n_long_sfb=22`/`n_short_sfb=0` already were. | `err_e3_block_type_zero`, `err_e3_all_block_types` | [x] |
| E4 | `read_side_info` (L159-161) | `part_23_sum + bs->pos > bs->limit + main_data_begin * 8` after all granules are parsed (sum of the 12-bit `part_23_length` fields overruns the available main data). | returns `-1`. All granules are **fully written** (this check is last). | `err_e4_main_data_bounds`, `err_e4_bounds_boundary` | [x] |
| E5 | `read_side_info` (L98-99) | `sr_idx` reaches **8**, one past the last valid row of the `[8][23]` / `[8][40]` tables (reachable: `hdr[1] & 0x18 == 0x18` and `(hdr[2] >> 2) & 3 == 3`). **The C does not check this** — it forms an out-of-range table address. | **No rejection.** Returns normally; `sfbtab` points one row past the table end. Rust must also form the address without panicking or clamping. | `err_e5_sr_idx_out_of_range` | [x] |
| E6 | `read_side_info` / `get_bits` | `bs->limit < 0`, or `bs->limit == 0`, i.e. every single `get_bits` immediately trips E1. | **No rejection at entry.** All fields become 0 → `big_values==0` (no E2), window-switch bit 0, `main_data_begin==0`, `part_23_sum==0`; final check `0 + pos > limit + 0` is then true, so returns `-1`. | `err_e6_zero_and_negative_limit` | [x] |
| E7 | `read_side_info` / `get_bits` | `bs->pos > bs->limit` already on entry (oversized start offset), including `pos` far past the buffer. | **No rejection.** Same as E6: E1 fires on every read, no byte is dereferenced, returns `-1` via E4. | `err_e7_pos_past_limit_on_entry` | [x] |
| E8 | `read_side_info` / `get_bits` | `bs->pos < 0` (negative start offset). `bs->buf + (pos >> 3)` is an out-of-bounds pointer and `pos & 7` is still 0..7 (arithmetic shift / two's-complement AND). | **No rejection and no clamp** — C dereferences before the buffer. Both libraries are handed the *same* `buf` pointer, so the bytes read are identical and the results must match. | `err_e8_negative_pos` | [x] |
| E9 | `read_side_info` | `hdr` byte values with no "valid" meaning — this API takes raw `uint8_t`, so **every** one of the 2^24 relevant `hdr[1..3]` combinations is an accepted input; there is no validity check and no enum. Includes reserved sample-rate index 3, reserved MPEG version bits, and reserved channel-mode bits. | **No rejection.** Every value selects some branch; must match bit-for-bit. This is the "out-of-range enum across FFI" class for this library. | `err_e9_exhaustive_hdr_bytes` (all 2^24 `hdr[1..3]`, in-range `sr_idx` and out) | [x] |
| E10 | `read_side_info` | `gr` array shorter than the header's granule count (`gr_count` is 1, 2 or 4 depending on `hdr[1]&0x8` and `(hdr[3]&0xC0)==0xC0`). | **No rejection** — C writes past the caller's array. Not differentially testable without invoking UB in the test harness itself; instead every test allocates the maximum 4 granules and asserts the *number of granules written* (i.e. the trailing untouched granules keep their pre-fill) matches between C and Rust. | `cfg_*` (granule-count assertion in `cmp_all`) | [x] |
| E11 | `read_side_info` | `bs == NULL`, `gr == NULL` or `hdr == NULL`. **Unchecked** — the very first statements dereference `hdr` and `bs`. | Both libraries fault identically (SIGSEGV). Verified out-of-process so the harness survives. | `err_e11_null_pointers` (subprocess, compares termination signal) | [x] |

## Constants / thresholds the C branches on (all exercised above)

| constant | site | meaning |
|----------|------|---------|
| `288` | L105 | max legal `big_values` (E2) |
| `500` | L152 | `preflag = scalefac_compress >= 500` (MPEG2 path only) |
| `0x8` on `hdr[1]` | L92, L102, L133, L152 | MPEG1 vs MPEG2/2.5 |
| `0xC0` on `hdr[3]` | L91, L97 | mono vs multi-channel |
| `0x0F0F` | L121 | scfsi mask, `block_type == 2` only |
| `255` | L119, L120, L147 | `region_count` sentinel |
| `22 / 39 / 30 / 8 / 6 / 0` | L110-131 | `n_long_sfb` / `n_short_sfb` per table |
| `7 / 8` | L119, L124 | `region_count[0]` for window-switched frames |
| `9 / 8+gr_count / 7+gr_count / 12 / 9 / 8 / 4 / 1 / 2 / 10 / 15 / 3` | `get_bits` widths | every distinct bit width requested |

## Harness sensitivity (why these checks are not vacuous)

Passing tests only mean something if the tests can fail. `scripts/mutation_check.sh`
injects 47 single-edit mutations into `src/lib.rs`, rebuilds the cdylib and runs
the full suite for each. **47/47 are killed**, including every error-surface
constant in the table above:

- `big_values > 288` -> `>= 288` (E2 threshold)
- `block_type == 0` -> `== 4` (E3 trigger removed)
- final bounds `>` -> `>=`, `*8` -> `*4`, and check deleted entirely (E4)
- `get_bits` limit `>` -> `>=`, and `pos` clamped instead of advanced (E1 sentinel
  semantics, including the "pos is still advanced" detail)
- `sr_idx -= (sr_idx != 0)` removed, `sr_idx` table strides 23->24 / 40->41 (E5)

## Build-profile note on E11

The verified artifact is the **release** cdylib (`crate-type = ["cdylib"]`,
`[profile.release] panic = "abort"`), which faults with SIGSEGV on a null
pointer exactly as the C does. A *debug* cdylib is compiled with
`-C debug-assertions`, so rustc inserts an explicit "null pointer dereference
occurred" check that aborts (SIGABRT) instead. `err_e11_null_pointers` therefore
requires *both* libraries to fault in all cases and compares the exact signal
only for the release build. Every other test in the suite passes unchanged
against the debug cdylib too, which additionally proves no arithmetic in the
translation can overflow-panic (debug builds have overflow checks enabled).
