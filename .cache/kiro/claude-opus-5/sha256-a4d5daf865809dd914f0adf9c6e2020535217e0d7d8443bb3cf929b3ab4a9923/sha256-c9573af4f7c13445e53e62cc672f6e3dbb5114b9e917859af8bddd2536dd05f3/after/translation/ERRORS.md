# ERRORS.md — Phase C error-surface table

## Mechanical grep result

```
$ grep -nE 'return|assert|NULL|ERROR|errno|-1|exit|abort' c_src/src/lib.c c_src/include/lib.h
(no matches)
```

`update_frame_header` is `void`, takes a single `tflac *`, contains **no**
`return` statement, **no** error enum, **no** `assert`, **no** null check, **no**
range check that rejects input, and **no** min/max constant. There is no error
code, sentinel, or out-parameter status. The C therefore has **zero explicit
rejection paths**: every input is "accepted" and produces a defined bit pattern.

Because of that, the table below is the exhaustive set of *implicit* rejections —
the branches where the C deliberately declines to set bits (the `default` arms
that fall through without OR-ing anything), plus the generic FFI boundaries the
task mandates (null pointer, zero/oversized lengths, one-past-range values, and
out-of-range enum values crossing the FFI boundary). "Expected C result" is the
exact observable, i.e. the resulting `t->frame_header`.

`BASE = 0xFFF80000` (`0xFFF8U << 16`), always written unconditionally first.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `update_frame_header` | `cur_blocksize` not in the 13 enumerated sizes **and** `<= 256` (e.g. 0, 1, 255, 256 is enumerated so 255) — implicit "unknown small block size" | block-size nibble = `0x06`, i.e. `BASE \| 0x6000` | `err_01_blocksize_default_le_256` |
| 2 | `update_frame_header` | `cur_blocksize` not enumerated **and** `> 256` (e.g. 257, 32769, `u32::MAX`) — implicit "unknown large block size" | block-size nibble = `0x07`, i.e. `BASE \| 0x7000` | `err_02_blocksize_default_gt_256` |
| 3 | `update_frame_header` | `cur_blocksize == 0` (zero "length" boundary; `0 <= 256`) | nibble `0x06` | `err_03_blocksize_zero` |
| 4 | `update_frame_header` | `cur_blocksize == u32::MAX` (oversized "length" boundary) | nibble `0x07` | `err_04_blocksize_u32max` |
| 5 | `update_frame_header` | `samplerate % 1000 == 0` **and** `samplerate / 1000 >= 256` (e.g. 256000, 1000000) — falls off the end of the `if`, **no** sample-rate bits set | sample-rate nibble = `0x00` | `err_05_samplerate_khz_overflow` |
| 6 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate >= 65536`, `samplerate % 10 == 0`, `samplerate / 10 >= 65536` (e.g. 655370) — no bits set | nibble `0x00` | `err_06_samplerate_dahz_overflow` |
| 7 | `update_frame_header` | `samplerate % 1000 != 0`, `samplerate >= 65536`, `samplerate % 10 != 0` (e.g. 65537) — no branch taken at all | nibble `0x00` | `err_07_samplerate_unrepresentable` |
| 8 | `update_frame_header` | `samplerate == 0` (zero boundary; `0 % 1000 == 0` and `0 / 1000 < 256`) | nibble `0x0C` (**not** 0) | `err_08_samplerate_zero` |
| 9 | `update_frame_header` | `samplerate == u32::MAX` (oversized boundary: `%1000!=0`, `>=65536`, `%10!=0`) | nibble `0x00` | `err_09_samplerate_u32max` |
| 10 | `update_frame_header` | `samplerate == 65535` / `65536` — one step either side of the `< 65536` range check | 65535 → `0x0D` (`%1000!=0`, `<65536`); 65536 → `0x00` (`%1000==536`, `>=65536`, `%10==6`) | `err_10_samplerate_65536_boundary` |
| 11 | `update_frame_header` | `samplerate == 255000` / `256000` — one step either side of the `/1000 < 256` check | 255000 → `0x0C`; 256000 → `0x00` | `err_11_samplerate_256khz_boundary` |
| 12 | `update_frame_header` | `samplerate == 655350` / `655360` — one step either side of the `/10 < 65536` check | 655350 → `0x0E` (`/10 == 65535`); 655360 → `0x00` (`/10 == 65536`, not `< 65536`) | `err_12_samplerate_655360_boundary` |
| 13 | `update_frame_header` | `channel_mode` **out-of-range enum value** (any `u8` with no valid `TFLAC_CHANNEL_MODE` variant: 4..=255, incl. `TFLAC_CHANNEL_MODE_COUNT == 4`). C reduces it with `% 4`, so the `default:` arm is unreachable and the value **aliases** onto 0..=3 | behaves exactly as `channel_mode % 4`; e.g. 4→independent, 255→mid-side | `err_13_channel_mode_out_of_range_enum` |
| 14 | `update_frame_header` | `channels == 0` with independent mode — unsigned underflow of `(t->channels - 1)` | `(0u32-1) << 4 == 0xFFFFFFF0` OR-ed in, saturating the whole header | `err_14_channels_zero_underflow` |
| 15 | `update_frame_header` | `channels > 8` (out of FLAC's valid 1..=8 range, no check in C) e.g. 9, 16, 17, `u32::MAX` — raw `(channels-1) << 4`, bits spill past the 4-bit field | `BASE \| ((channels-1)<<4) \| ...` with spill | `err_15_channels_out_of_range` |
| 16 | `update_frame_header` | `channels == u32::MAX` — `(u32::MAX - 1) << 4` shifts high bits **out** of the 32-bit word | `0xFFFFFFE0` truncated to 32 bits | `err_16_channels_u32max` |
| 17 | `update_frame_header` | `bitdepth` not in {8,12,16,20,24,32} — implicit reject, no bits set (e.g. 0, 1, 7, 9, 33, `u32::MAX`) | bit-depth field = `0`, i.e. no `<< 1` OR | `err_17_bitdepth_default` |
| 18 | `update_frame_header` | `bitdepth == 0` (zero boundary) | field `0` | `err_18_bitdepth_zero` |
| 19 | `update_frame_header` | `bitdepth == 31` / `33` — one step past the largest valid value (32) | field `0` for both | `err_19_bitdepth_past_range` |
| 20 | `update_frame_header` | `t == NULL` — the C dereferences unconditionally with no null check (undefined behaviour → SIGSEGV in practice) | process dies on `SIGSEGV` (signal 11); the Rust must die the same way, **not** unwind, abort, or return | `err_20_null_pointer` (subprocess, differential on exit status) |

## Result

All 20 rows pass in both the release and debug profiles. Row 20 found a real
divergence — see below.

### Divergence found and fixed: row 20 (null pointer)

The original Rust opened with `let t = &mut *t;`. Forming a `&mut` reference from
the incoming raw pointer trips rustc's **debug-only** null/misaligned dereference
check, which panics; because the panic crosses an `extern "C"` boundary it is
converted to an abort. So on `update_frame_header(NULL)`:

| profile | C | Rust (before) | Rust (after) |
|---------|---|---------------|--------------|
| release | `SIGSEGV` (11) | `SIGSEGV` (11) | `SIGSEGV` (11) |
| debug   | `SIGSEGV` (11) | **`SIGABRT` (6)** | `SIGSEGV` (11) |

Note this was invisible in the release profile, which is why the profile matrix
matters. The fix reads and writes fields through `addr_of!` / `addr_of_mut!`
raw-pointer accesses instead of a reference, which both reproduces the C's fault
behaviour and avoids asserting the validity/aliasing guarantees a `&mut` would
imply for a pointer supplied by an external caller.

The `mutation_check.sh` mutant *"reference instead of raw ptr"* re-introduces
this bug on demand; it is reported as `release=pass debug=fail`, confirming both
that the test catches it and that only the debug profile can see it.
