# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep result (why the table looks the way it does)

```sh
grep -nE "return|assert|NULL|ERROR|errno|goto|exit\(|abort\(|MIN|MAX|#if" \
     c_src/src/lib.c c_src/include/lib.h
```

**0 matches.** The library has:

* no `return` statement (the single entry point is `void`),
* no error enum, no error code, no sentinel, no out-parameter status,
* no `assert`, no `NULL` check, no `errno` use, no `goto`, no `abort`/`exit`,
* no `#if`/`#ifdef` compile-time gate, no MIN/MAX constant.

Therefore `update_frame_header` **never rejects an input**. Its entire
"error surface" is made of (a) one UB path (null pointer) and (b) *silent*
rejections: `default:` arms and guarded `if`s that deliberately contribute
**no bits** to `frame_header`, plus one unsigned-underflow path. Those silent
paths are the observable "error result" and are exactly what the rows below
pin down. Every row asserts the *same* observable outcome (the full 24-byte
record, i.e. the resulting `frame_header`), not merely "both failed somehow".

Baseline for every row: `frame_header` is unconditionally *assigned*
`0xFFF8U << 16 == 0xFFF8_0000` at `lib.c:12`, so the incoming value of
`frame_header` never leaks into the result, and a "no bits contributed"
outcome means the corresponding nibble stays `0`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| 1 | `update_frame_header` | `t == NULL` (no null check; `lib.c:12` writes through `t` immediately) | UB → store to address `0x10` → `SIGSEGV` (measured: wait status 139). The **release** Rust `.so` matches exactly: `SIGSEGV`. See the note below about debug builds. | `err_01_null_pointer_both_segv` | [x] |
| 2 | `update_frame_header` | `cur_blocksize` matches no `case` **and** `<= 256` (`lib.c:53-55` `default`, true branch). Includes `cur_blocksize == 0`. | blocksize nibble `= 0x6`; `frame_header = 0xFFF8_6000 \| …` | `err_02_blocksize_default_le_256` | [x] |
| 3 | `update_frame_header` | `cur_blocksize` matches no `case` **and** `> 256` (`lib.c:53-55` `default`, false branch). Includes `u32::MAX`. | blocksize nibble `= 0x7` | `err_03_blocksize_default_gt_256` | [x] |
| 4 | `update_frame_header` | `cur_blocksize == 257` / `255` — one step past the `256` boundary of the `<=` test at `lib.c:55` | `257 -> 0x7`, `255 -> 0x6` (and exact `256` takes the `case 256` arm `-> 0x8`, not the default) | `err_04_blocksize_boundary_255_256_257` | [x] |
| 5 | `update_frame_header` | `samplerate` in `default`, `% 1000 == 0`, **`/ 1000 >= 256`** — inner `if` at `lib.c:94` FAILS | samplerate nibble stays `0x0` (no bits ORed) | `err_05_samplerate_khz_out_of_range` | [x] |
| 6 | `update_frame_header` | `samplerate` in `default`, `% 1000 == 0`, `/ 1000 == 256` exactly — one step past `< 256` (`samplerate == 256000`) | samplerate nibble `0x0`; while `255000 -> 0xC` | `err_06_samplerate_khz_boundary` | [x] |
| 7 | `update_frame_header` | `samplerate` in `default`, `% 1000 != 0`, `>= 65536`, `% 10 == 0`, **`/ 10 >= 65536`** — inner `if` at `lib.c:100` FAILS (e.g. `655360`) | samplerate nibble stays `0x0` | `err_07_samplerate_dahz_out_of_range` | [x] |
| 8 | `update_frame_header` | `samplerate` in `default`, `% 1000 != 0`, `>= 65536`, `% 10 == 0`, `/ 10 == 65536` exactly — one step past `< 65536` at `lib.c:100` | nibble `0x0`; while `655350 -> 0xE` | `err_08_samplerate_dahz_boundary` | [x] |
| 9 | `update_frame_header` | `samplerate` in `default`, `% 1000 != 0`, `>= 65536`, **`% 10 != 0`** — every `else if` at `lib.c:97,99` fails, no `else` (e.g. `65537`, `u32::MAX`) | samplerate nibble stays `0x0` | `err_09_samplerate_unrepresentable` | [x] |
| 10 | `update_frame_header` | `samplerate == 65536` / `65535` — one step past the `< 65536` test at `lib.c:97` | `65535 -> 0xD` (`%1000!=0`, `<65536`); `65536` falls through `lib.c:99` too (`65536 % 10 == 6 != 0`) `-> 0x0`, i.e. the boundary step silently loses the rate | `err_10_samplerate_65536_boundary` | [x] |
| 11 | `update_frame_header` | `samplerate == 0` (invalid rate; `0 % 1000 == 0` and `0 / 1000 == 0 < 256`) | nibble `0xC` — silently accepted, NOT rejected | `err_11_samplerate_zero` | [x] |
| 12 | `update_frame_header` | `channels == 0` with `channel_mode % 4 == 0`: unsigned underflow `(t->channels - 1)` at `lib.c:109` | `0u32.wrapping_sub(1) << 4 == 0xFFFF_FFF0` ORed in → `frame_header == 0xFFFF_FFF0 \| …` (all high bits set) | `err_12_channels_zero_underflow` | [x] |
| 13 | `update_frame_header` | `channels > 8` (beyond FLAC's legal 1..8) with mode 0, e.g. `17..=0x1000_0000` — no range check; `(channels-1) << 4` **overflows the 4-bit field** and corrupts the samplerate/blocksize nibbles | `(channels-1) << 4` ORed in with wrapping shift-out of the top 4 bits | `err_13_channels_out_of_range` | [x] |
| 14 | `update_frame_header` | `channels` such that `(channels-1) << 4` discards high bits, e.g. `channels == 0x1000_0001` → `0x1000_0000 << 4 == 0` | only the shifted-in low bits appear; no wrap-around panic | `err_14_channels_shift_truncation` | [x] |
| 15 | `update_frame_header` | `channel_mode` **out-of-range enum value** `== TFLAC_CHANNEL_MODE_COUNT (4)` — a `TFLAC_CHANNEL_MODE` with no meaningful variant, folded by `% 4` at `lib.c:106` | behaves exactly as mode `0` (INDEPENDENT), i.e. `(channels-1) << 4` | `err_15_channel_mode_count_alias` | [x] |
| 16 | `update_frame_header` | `channel_mode` out-of-range enum value `5..=255` (incl. `255`) crossing the FFI boundary as a plain int | folded by `% 4`: `mode = channel_mode % 4`, same result as `channel_mode & 3` | `err_16_channel_mode_all_256_values` | [x] |
| 17 | `update_frame_header` | the `default:` arm of the channel-mode `switch` (`lib.c:120-121`) — **unreachable**, since `mode = x % 4 ∈ {0,1,2,3}` covers all four enumerators | never taken; no `channel_mode` value may leave the channel nibble unset (except via mode 0 + `channels == 1`) | `err_17_channel_mode_default_unreachable` | [x] |
| 18 | `update_frame_header` | `bitdepth` matches no `case` → `default: break` (`lib.c:142-143`). Includes `0`, `1`, `7`, `9`, `11`, `13`, `33`, `u32::MAX`. | sample-size field stays `0x0` (bits 1..3 clear) — silently accepted | `err_18_bitdepth_default` | [x] |
| 19 | `update_frame_header` | `bitdepth` one step past each valid value (`7/9`, `11/13`, `15/17`, `19/21`, `23/25`, `31/33`) | all take `default` → no bits; only the exact 6 values set bits | `err_19_bitdepth_off_by_one` | [x] |
| 20 | `update_frame_header` | every field simultaneously at an extreme (`0` everywhere; `u32::MAX`/`0xFF` everywhere) — worst-case combination of the above silent rejections | deterministic single value each; must match bit-for-bit | `err_20_all_fields_extremes` | [x] |
| 21 | `update_frame_header` | bit 0 of `frame_header` and the padding bytes of `struct tflac` are never written by any path | bit 0 stays `0`; the 3 padding bytes at offsets 13..15 keep their pre-call value; nothing outside the 24-byte record is written | `err_21_no_stray_writes` | [x] |

Rows 1-21 plus three extra generic-boundary tests (`err_22_*`
out-of-range-enum cross product, `err_23_*` fully random record bytes, and the
`err_01_*` helper) are implemented in `tests/phase_c_errors.rs` — 23 tests, all
passing against both libraries.

## Note on row 1 (NULL) — debug vs release Rust

Measured wait statuses from a `dlopen`/`dlsym` harness calling
`update_frame_header(NULL)`:

| library | result |
|---------|--------|
| C `.so` (gcc 11.5) | `SIGSEGV` (139) |
| Rust `.so`, `--release` | `SIGSEGV` (139) — **identical to C** |
| Rust `.so`, debug | `SIGABRT` (134) |

The debug build differs because rustc, when `debug_assertions` is on, injects its
own null-pointer UB check that `panic!`s with `null pointer dereference
occurred`; inside an `extern "C"` function that panic cannot unwind, so it
aborts *before* the faulting store is executed. That instrumentation is a
compiler diagnostic, not part of the translation, and it is absent from the
release cdylib — the artifact an external caller actually loads.

`err_01_null_pointer_both_segv` therefore asserts:

1. the C `.so` dies of `SIGSEGV`;
2. the **release** Rust `.so` produces the byte-identical `(exit code, signal)`
   pair as the C;
3. the `.so` under test never *returns normally* from a NULL pointer, and if its
   signal differs from C's it must provably be a debug-assertions build (the test
   greps the `.so` for rustc's `null pointer dereference occurred` string) and
   the signal must be exactly `SIGABRT`.

So the divergence is detected, explained, and bounded — it cannot silently hide a
real translation defect.
