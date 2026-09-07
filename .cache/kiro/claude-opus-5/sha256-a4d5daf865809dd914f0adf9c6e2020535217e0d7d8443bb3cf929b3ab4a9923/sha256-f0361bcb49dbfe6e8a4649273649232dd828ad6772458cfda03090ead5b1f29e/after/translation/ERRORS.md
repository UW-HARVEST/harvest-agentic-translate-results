# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

## Mechanical derivation

```
$ grep -nE 'return|assert|NULL|ERROR|-1|errno|exit|abort' c_src/src/lib.c c_src/include/lib.h
NO explicit error/return/assert constructs found
```

`update_frame_header` is `void`, takes one pointer, performs **no** null check,
**no** range check, **no** `assert`, and has **no** error return, error enum, or
sentinel value. Therefore the error surface is *not* a set of return codes. It
is the set of paths on which the C **silently declines to contribute bits** to
`frame_header` (the `default:`/nested-`if` fall-through paths), plus the generic
FFI boundaries every C API has.

Each row below is one distinct rejection/fall-through *decision point* found in
the source. "Expected C result" is stated as the contribution that the rejecting
field makes to `t->frame_header`; the observable contract is the final 32-bit
`frame_header` (and that no other struct field is modified).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `update_frame_header` | `cur_blocksize` matches no `case` **and** `cur_blocksize <= 256` (i.e. `0,1,…,191,193,…,255` — every value ≤256 except 192 and 256) | `default:` taken; ORs `0x06 << 12`. Never rejects. |
| 2 | `update_frame_header` | `cur_blocksize` matches no `case` **and** `cur_blocksize > 256` (e.g. `257`, `1000`, `65535`, `0xFFFFFFFF`) | `default:` taken; ORs `0x07 << 12`. Never rejects. |
| 3 | `update_frame_header` | `samplerate` matches no `case`, `samplerate % 1000 == 0`, `samplerate / 1000 < 256` (e.g. `0`, `1000`, `255000`) | inner `if` true; ORs `0x0C << 8`. |
| 4 | `update_frame_header` | `samplerate % 1000 == 0` **and** `samplerate / 1000 >= 256` (e.g. `256000`, `1000000`, `4294967000`) | **silent rejection**: inner `if` false → *no* sample-rate bits ORed (bits 8–11 stay 0) |
| 5 | `update_frame_header` | `samplerate % 1000 != 0` **and** `samplerate < 65536` (e.g. `1`, `12345`, `65535`) | `else if` true; ORs `0x0D << 8`. |
| 6 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate >= 65536`, `samplerate % 10 == 0`, `samplerate / 10 < 65536` (e.g. `65540`, `655350`) | ORs `0x0E << 8`. |
| 7 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate >= 65536`, `samplerate % 10 == 0`, `samplerate / 10 >= 65536` (e.g. `655370`, `4294967290`) | **silent rejection**: innermost `if` false → *no* sample-rate bits ORed |
| 8 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate >= 65536`, `samplerate % 10 != 0` (e.g. `65537`, `4294967295`) | **silent rejection**: all `else if` chains fail → *no* sample-rate bits ORed |
| 9 | `update_frame_header` | `channel_mode` outside the named enum range, i.e. `channel_mode >= 4` including `TFLAC_CHANNEL_MODE_COUNT == 4` and `255` | **not** rejected: `mode = channel_mode % 4` folds it into `0..3`, so e.g. `4→0`, `5→1`, `255→3`. Behaves exactly like the folded value. |
| 10 | `update_frame_header` | `channel_mode % 4` reaching the `switch` `default:` label | **dead code** — unreachable because `% 4 ∈ {0,1,2,3}` and all four have a `case`. No rejection path exists. |
| 11 | `update_frame_header` | `channel_mode % 4 == 0` (INDEPENDENT) **and** `channels == 0` | **no check**: `(0u - 1) << 4` = unsigned wraparound `0xFFFFFFFF << 4` = `0xFFFFFFF0` ORed in → `frame_header` becomes `0xFFFFFFF0 | other bits`. Must be reproduced, not "fixed". |
| 12 | `update_frame_header` | `channel_mode % 4 == 0` **and** `channels - 1 > 0x0F` (e.g. `channels = 17`, `4096`, `0xFFFFFFFF`) | **no check**: `(channels-1) << 4` bleeds out of the 4-bit channel field and corrupts the sample-rate / block-size / sync nibbles. Reproduce exactly. |
| 13 | `update_frame_header` | `channel_mode % 4 == 0` **and** `channels - 1 >= 0x10000000` so that `<< 4` discards high bits (e.g. `channels = 0xF0000001`) | `<< 4` truncates modulo 2^32 (C: shift of `unsigned int`). Rust must use wrapping shift semantics, not panic. |
| 14 | `update_frame_header` | `bitdepth` matches no `case` (`0`, `1`, `7`, `9`, `11`, `13`, `15`, `17`, `33`, `0xFFFFFFFF`, …) | **silent rejection**: `default: break` → *no* bit-depth bits ORed (bits 1–3 stay 0) |
| 15 | `update_frame_header` | `t == NULL` | **no null check**: unconditional `t->frame_header = …` store → SIGSEGV / UB. Rust also dereferences unconditionally; both must fault. Verified out-of-process. Release: both SIGSEGV(11) exactly. Debug: rustc inserts its own null-deref UB check ahead of every raw dereference, so the Rust `.so` traps with SIGABRT(6) — compiler-inserted sanitization (the analogue of building the C with `-fsanitize=null`), not a translation difference; the test asserts exact signal equality on the release artifact and accepts SIGSEGV-or-SIGABRT under `debug_assertions`. |
| 16 | `update_frame_header` | any input: fields other than `frame_header` | C never writes `samplerate`, `channels`, `bitdepth`, `channel_mode`, `cur_blocksize`, nor the struct tail padding at offsets 13–15. Rust must leave the same 20 bytes + padding untouched. |

## Row checklist (Phase C)

- [x] 1 — `err_row_01_blocksize_default_le_256`
- [x] 2 — `err_row_02_blocksize_default_gt_256`
- [x] 3 — `err_row_03_samplerate_mod1000_kilo_ok`
- [x] 4 — `err_row_04_samplerate_mod1000_kilo_overflow_silent`
- [x] 5 — `err_row_05_samplerate_below_65536`
- [x] 6 — `err_row_06_samplerate_deca_ok`
- [x] 7 — `err_row_07_samplerate_deca_overflow_silent`
- [x] 8 — `err_row_08_samplerate_no_representation_silent`
- [x] 9 — `err_row_09_channel_mode_out_of_enum_range`
- [x] 10 — `err_row_10_channel_mode_switch_default_dead`
- [x] 11 — `err_row_11_channels_zero_underflow`
- [x] 12 — `err_row_12_channels_overflow_bleeds`
- [x] 13 — `err_row_13_channels_shift_truncates`
- [x] 14 — `err_row_14_bitdepth_unsupported_silent`
- [x] 15 — `err_row_15_null_pointer` (out-of-process fault comparison)
- [x] 16 — `err_row_16_only_frame_header_written`
