# ERRORS.md — error-surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c` / `c_src/include/lib.h`:

```
$ grep -n 'return' c_src/src/lib.c        # -> lines 91, 93, 122, 130 in ima_parse
$ grep -n -i 'assert\|NULL\|ERROR\|errno\|abort\|exit(' c_src/src/lib.c
$ grep -n 'if\s*(\|else\|for\s*(\|switch' c_src/src/lib.c
```

Findings: there are **no** `assert`s, no error enums, no `RETURN_ERROR`-style
macros, no `errno` use, no `return NULL`, and no `#if`/`#ifdef` branches. The
whole library has exactly **three** explicit rejection branches, all inside
`ima_parse`, plus a set of unchecked/UB conditions (recorded separately below,
since C performs no check there — they are still real inputs and are covered by
the crash-parity harness).

There is no length/size parameter in the API (`ima_parse(struct ima_info *,
const void *)`), so there is no length range check to enumerate — the C trusts
the buffer completely.

## A. Explicit rejections (one row per distinct rejection branch)

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `ima_parse` | `lib.c:87` — `ima_btoh32(header->type) != 'caff'`: first 4 bytes of `data` are not the ASCII bytes `c`,`a`,`f`,`f` (big-endian FourCC `'f' \| 'f'<<8 \| 'a'<<16 \| 'c'<<24` compared after byte swap) | returns `-1`; `*info` left **untouched** | `err_e1_bad_magic` |
| E2 | `ima_parse` | `lib.c:92` — `ima_btoh16(header->version) != 1`: bytes 4..6 of `data` (big-endian `u16`) are any value other than `0x0001` (i.e. bytes `00 01`) | returns `-2`; `*info` left **untouched** | `err_e2_bad_version` |
| E3 | `ima_parse` | `lib.c:118` — `ima_btoh32(desc->format_id) != 'ima4'`: the `desc` chunk's `format_id` (big-endian FourCC, offset +8 in the `desc` payload) is not the ASCII bytes `i`,`m`,`a`,`4` | returns `-3`; `*info` left **untouched** | `err_e3_bad_format_id` |

Success return: `0` (`lib.c:130`), all five `info` fields written.

Notes on ordering that the tests must pin down (they are part of "same error"):

* E1 is checked before E2, so a buffer that is wrong in *both* ways returns
  `-1`, not `-2`.
* E1/E2 are checked **before** the chunk-walking loop, so a malformed magic is
  rejected even when the chunk list is total garbage / would loop forever.
* E3 is checked **after** the loop terminates on the `data` chunk, so reaching
  E3 requires a well-formed-enough chunk list containing `data`.
* On every non-zero return the C writes **nothing** to `*info` — the tests
  assert the whole 40-byte `ima_info` is bit-identical (and unmodified) for
  both libraries.

## B. Boundary / generic-C-API cases (no check in C — behaviour is the contract)

These are the "one step past the valid range" and "hostile int across FFI"
cases required by Phase C. The C performs no validation, so *its* observed
behaviour is the expected result and the Rust must reproduce it exactly.

| # | trigger | expected C result | test |
|---|---------|-------------------|------|
| E4 | `header->type` == `'caff'` but off by one bit / one ASCII char (`caff`→`cafg`, `caff`→`baff`, and the little-endian spelling `ffac`) | `-1` | `err_e4_magic_one_step_off` |
| E5 | `header->version` == `0`, `2`, `0xFFFF`, and the byte-swapped spelling `0x0100` | `-2` | `err_e5_version_one_step_off` |
| E6 | `desc->format_id` == every one-ASCII-char neighbour of `ima4` and the byte-swapped spelling `4ami` | `-3` | `err_e6_format_one_step_off` |
| E7 | *chunk-type* field holding an arbitrary out-of-range 32-bit value (the C compares a `unsigned` against 4 FourCC constants — there is no enum validation, so any of the other 2^32−4 values is a valid "unknown chunk" that is skipped using its `size`). Exhaustively probed with random `u32`s incl. values that collide in only 3 of 4 bytes, `0`, `0xFFFFFFFF` | chunk skipped; parse continues; `0` on reaching `data` | `err_e7_out_of_range_chunk_type` |
| E8 | `data`-chunk `size` field one step past every interesting boundary: `0`, `-1`, `i64::MIN`, `i64::MAX`, `2^32`, `-2^32` | stored verbatim (reinterpreted as `u64`) into `info->size`, return `0` | `err_e8_data_size_extremes` |
| E9 | unknown-chunk `size` = `0` (chunk list advances by exactly 16 and does **not** hang) | parse continues | `err_e9_zero_size_skip` |
| E10 | unknown-chunk `size` negative — pointer walks *backwards* | parse continues from the moved-back position (identical pointer arithmetic in both) | `err_e10_negative_size_skip` |
| E11 | `channels_per_frame` = `0`, `1`, `0xFFFFFFFF`; `pakt->frame_count` = `0`, `-1`, `i64::MIN`, `i64::MAX` (no range check in C) | stored verbatim after byte swap, return `0` | `err_e11_field_extremes` |
| E12 | `desc->sample_rate` raw bytes forming NaN / ±Inf / ±0 / subnormal / ≥2^63 / < −2^63 — i.e. every input for which the C's `double`→`unsigned long long` conversion at `lib.c:127` is *undefined* by the standard | the exact x86-64 `cvttsd2si`-based result, byte-swapped and re-read as `double`; return `0` | `err_e12_sample_rate_ub_domain` |
| E13 | `info == NULL` **with** a buffer that fails E1/E2/E3 (C returns before touching `info`, so NULL is harmless) | `-1` / `-2` / `-3`, no fault | `err_e13_null_info_on_error_paths` |
| E14 | `info == NULL` on the success path (C writes through NULL) | fatal signal (`SIGSEGV`) | `crash_parity` harness |
| E15 | `data == NULL` (C dereferences it immediately at `lib.c:87`) | fatal signal (`SIGSEGV`) | `crash_parity` harness |
| E16 | `data` chunk appears **before** any `desc` chunk ⇒ `desc` stays `NULL` and `lib.c:118` dereferences `NULL+8` | fatal signal (`SIGSEGV`) | `crash_parity` harness |
| E17 | `desc` present and valid, but no `pakt` chunk before `data` ⇒ `pakt` stays `NULL` and `lib.c:125` dereferences `NULL+8` | fatal signal (`SIGSEGV`) | `crash_parity` harness |
| E18 | unaligned `data` pointer (offsets 1..7) — every field load becomes unaligned | identical results to the aligned case (x86-64 tolerates it; C emits plain loads) | `cfg_unaligned_data_pointer` |

Rows E14–E17 are compared by running the *same* call in a forked child process
for each library and asserting that the C child and the Rust child die with the
**same** signal (not merely "both failed"): this is what makes "same
rejection" checkable for the unchecked-NULL paths.

## Checklist

- [x] E1  — `-1` bad magic
- [x] E2  — `-2` bad version
- [x] E3  — `-3` bad `format_id`
- [x] E4  — magic one step off
- [x] E5  — version one step off
- [x] E6  — `format_id` one step off
- [x] E7  — out-of-range chunk-type value across FFI
- [x] E8  — `data` size extremes
- [x] E9  — zero-size chunk skip
- [x] E10 — negative-size chunk skip
- [x] E11 — `channel_count` / `frame_count` extremes
- [x] E12 — `sample_rate` UB domain (NaN/Inf/±0/subnormal/≥2^63/<−2^63)
- [x] E13 — NULL `info` on error paths
- [x] E14 — NULL `info` on success path (signal parity)
- [x] E15 — NULL `data` (signal parity)
- [x] E16 — missing `desc` (signal parity)
- [x] E17 — missing `pakt` (signal parity)
- [x] E18 — unaligned `data` pointer
