# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (145 lines) and
`c_src/include/lib.h`.

## Mechanical grep results

```
$ grep -nE 'return|RETURN_ERROR|assert|NULL|errno|goto|exit|abort' c_src/src/lib.c
(no matches)

$ grep -nE 'return|assert|NULL|errno' c_src/include/lib.h
(no matches)
```

The library has **exactly one** public function, `void update_frame_header(tflac *t)`:

* return type is `void` — there is **no** error code, no sentinel, no `-1`, no
  `NULL` return;
* there is **no** `assert`, no `RETURN_ERROR` macro, no error enum, no `errno`
  use, no `goto`-to-cleanup, no `exit`/`abort`;
* there is **no** explicit range check, null check, or min/max constant that
  rejects an input.

So the C code **never rejects any input**. Every reachable input is
"accepted" and produces a defined `frame_header`. The rejection surface is
therefore the set of *degenerate / out-of-domain inputs* that the C handles
implicitly (by fallthrough, wrapping arithmetic, or truncation) rather than by
returning an error. Each such implicit-handling branch is one row: the Rust must
reproduce the same non-error behaviour bit for bit.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `update_frame_header` | `t == NULL` | Undefined behaviour: C dereferences `t` unconditionally at `t->frame_header = ...` with no null check → SIGSEGV. Rust does the same (`&mut *t`). Documented, NOT differentially executed (a crash cannot be compared). | `err_01_null_pointer_documented_not_executed` |
| 2 | `update_frame_header` | `cur_blocksize` not any of the 13 listed cases **and** `<= 256` (e.g. `0`, `1`, `191`, `255`) | no error; `default:` arm ORs `0x06 << 12`. Note `cur_blocksize == 0` is accepted. | `err_02_blocksize_default_low` |
| 3 | `update_frame_header` | `cur_blocksize` not listed and `> 256` (e.g. `257`, `4294967295`) | no error; `default:` arm ORs `0x07 << 12` | `err_03_blocksize_default_high` |
| 4 | `update_frame_header` | `cur_blocksize` one step past each listed case (`191/193`, `575/577`, `32767/32769`, …) | no error; falls to `default:` and takes the `<=256 ? 0x06 : 0x07` branch, NOT the neighbouring case's code | `err_04_blocksize_off_by_one` |
| 5 | `update_frame_header` | `samplerate == 0` | no error; `0 % 1000 == 0` and `0/1000 == 0 < 256` → ORs `0x0C << 8`. Division by zero is impossible (0 is the dividend). | `err_05_samplerate_zero` |
| 6 | `update_frame_header` | `samplerate % 1000 == 0` but `samplerate / 1000 >= 256` (e.g. `256000`, `4294000000`) | no error; **inner `if` fails and NOTHING is ORed** — sample-rate nibble stays `0x0` (the "get from STREAMINFO" code). This is the C's silent-reject path. | `err_06_samplerate_khz_overflow` |
| 7 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate < 65536` (e.g. `1`, `65535`, `22051`) | no error; ORs `0x0D << 8` | `err_07_samplerate_hz_small` |
| 8 | `update_frame_header` | `samplerate % 1000 != 0`, `>= 65536`, `% 10 == 0`, `samplerate / 10 < 65536` (e.g. `65540`, `655350`) | no error; ORs `0x0E << 8` | `err_08_samplerate_dahz` |
| 9 | `update_frame_header` | `samplerate % 1000 != 0`, `>= 65536`, `% 10 == 0`, `samplerate / 10 >= 65536` (e.g. `655360`, `4294967290`) | no error; **nothing ORed**, nibble stays `0x0` — second silent-reject path | `err_09_samplerate_dahz_overflow` |
| 10 | `update_frame_header` | `samplerate % 1000 != 0`, `>= 65536`, `% 10 != 0` (e.g. `65537`, `4294967295`) | no error; all three branches fail, **nothing ORed**, nibble stays `0x0` — third silent-reject path | `err_10_samplerate_no_branch` |
| 11 | `update_frame_header` | `samplerate` one step past each boundary: `65535/65536/65537`, `255000/256000`, `655350/655360` | no error; the exact branch selected per rows 6–10 | `err_11_samplerate_boundaries` |
| 12 | `update_frame_header` | `channel_mode` out-of-range for `enum TFLAC_CHANNEL_MODE`, i.e. `>= 4` (`4 == TFLAC_CHANNEL_MODE_COUNT`, `5`, `255`) | no error; C computes `mode = channel_mode % 4` FIRST, so `4→0`, `5→1`, `255→3`. The `default:` arm of that switch is therefore **unreachable**. An out-of-range enum value is silently folded, not rejected. | `err_12_channel_mode_out_of_range` |
| 13 | `update_frame_header` | `channels == 0` with `channel_mode % 4 == 0` | no error; `(t->channels - 1)` is `uint32_t` arithmetic → wraps to `0xFFFFFFFF`, `<< 4` → `0xFFFFFFF0`, ORed in. Sets many high bits of `frame_header`. Rust must use `wrapping_sub`, not `-`. | `err_13_channels_zero_wraps` |
| 14 | `update_frame_header` | `channels` huge / out of FLAC range, mode `0` (e.g. `9`, `16`, `17`, `0xFFFFFFFF`) | no error; `(channels-1) << 4` is ORed with high bits intact — no masking to 4 bits, no range check. `channels == 0x10000000+` shifts bits out the top (truncation, not UB). | `err_14_channels_out_of_range` |
| 15 | `update_frame_header` | `channels` non-zero but `channel_mode % 4 != 0` | no error; `channels` is **completely ignored** (not validated against the 2-channel requirement of side/mid modes) | `err_15_channels_ignored_in_stereo_modes` |
| 16 | `update_frame_header` | `bitdepth` not in `{8,12,16,20,24,32}` (e.g. `0`, `1`, `7`, `9`, `33`, `0xFFFFFFFF`) | no error; `default:` arm is **empty** → nothing ORed, sample-size nibble stays `0` ("get from STREAMINFO"). Silent accept. | `err_16_bitdepth_unlisted` |
| 17 | `update_frame_header` | `bitdepth` one step past each listed case (`7/9`, `11/13`, `31/33`, …) | no error; empty `default:`, bits `1..3` stay `0` | `err_17_bitdepth_off_by_one` |
| 18 | `update_frame_header` | `frame_header` pre-populated with garbage (e.g. `0xFFFFFFFF`) before the call | no error; first statement is an unconditional **assignment** (`=`, not `|=`), so the prior value is fully discarded | `err_18_frame_header_overwritten` |
| 19 | `update_frame_header` | all-max input: every `u32` field `0xFFFFFFFF`, `channel_mode = 0xFF` | no error; must not panic. In Rust, debug-mode `-` and `<<` overflow checks would panic where C wraps/truncates — this row guards that. | `err_19_all_fields_max` |
| 20 | `update_frame_header` | bit 0 and bits 4..7 of the reserved/blocking-strategy area | no error; bit 0 of `frame_header` is **never set** by any path (always 0), and the padding bytes of the struct are never written | `err_20_reserved_bit_and_padding` |

**All 20 rows are checked off — see `tests/phase_c_errors.rs`.**
