# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical inventory of rejection constructs

```console
$ grep -c 'return'          c_src/src/lib.c   -> 0
$ grep -c 'assert'          c_src/src/lib.c   -> 0
$ grep -c 'NULL\|nullptr'   c_src/src/lib.c   -> 0
$ grep -c 'errno\|RETURN_ERROR\|_ERROR\|-1'   c_src/src/lib.c   -> 0
$ grep -n  'if ('           c_src/src/lib.c   -> lines 93, 94, 97, 99, 100
```

`update_frame_header` returns `void`, has **no** `return` statement, **no**
`assert`, **no** error enum, **no** sentinel value and **no** null check. Its
entire rejection surface is therefore made of:

* the **5 explicit range/predicate checks** on lines 93/94/97/99/100,
* the **4 `switch` `default:` arms** (lines 53, 92, 120, 142),
* the paths through those arms that OR in **no bits at all**, silently leaving a
  header field at `0x0` (in FLAC: "invalid / get it from STREAMINFO") — this is
  how this function "rejects" an input, and
* the **unchecked unsigned underflow / overflow** of `(t->channels - 1) << 4`,
* the **unchecked null pointer dereference** of `t`.

`expected C result` below is always expressed as the value written to
`t->frame_header` (base value `0xFFF80000` = `0xFFF8U << 16`).

## Error-surface table

| #   | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|-----|----------|---------------------------------------------|-------------------|------|---|
| E1  | `update_frame_header` | `t == NULL` — no null check anywhere; line 12 writes `t->frame_header` | dereference of address 0 ⇒ process killed by `SIGSEGV` (signal 11); no value returned | `err_e1_null_pointer_both_segv` | [x] |
| E2  | `update_frame_header` | `cur_blocksize` not in {192,576,1152,2304,4608,256,512,1024,2048,4096,8192,16384,32768} **and** `<= 256` (line 53 `default:`, line 55 true arm) — e.g. 0, 1, 191, 255 | blocksize nibble := `0x6` ⇒ `|= 0x6000` | `err_e2_blocksize_default_le256` | [x] |
| E3  | `update_frame_header` | same `default:` but `> 256` (line 55 false arm) — e.g. 257, 65535, 0xFFFFFFFF | blocksize nibble := `0x7` ⇒ `|= 0x7000` | `err_e3_blocksize_default_gt256` | [x] |
| E4  | `update_frame_header` | line 94 range check FAILS: `samplerate % 1000 == 0 && samplerate / 1000 >= 256` — e.g. 256000, 300000, 1000000, 4294000000 | **no bits OR-ed**; samplerate nibble stays `0x0` | `err_e4_samplerate_khz_out_of_range` | [x] |
| E5  | `update_frame_header` | line 100 range check FAILS: `%1000 != 0 && >= 65536 && %10 == 0 && samplerate / 10 >= 65536` — e.g. 655360, 4294967290 | **no bits OR-ed**; nibble stays `0x0` | `err_e5_samplerate_dahz_out_of_range` | [x] |
| E6  | `update_frame_header` | line 99 predicate FAILS with **no `else`**: `%1000 != 0 && >= 65536 && %10 != 0` — e.g. 65537, 65539, 4294967295 | **no bits OR-ed**; nibble stays `0x0` | `err_e6_samplerate_no_branch_taken` | [x] |
| E7  | `update_frame_header` | out-of-range `enum TFLAC_CHANNEL_MODE` value across the FFI boundary: `channel_mode >= 4`, i.e. `TFLAC_CHANNEL_MODE_COUNT` (4) or any of 4..=255 | line 106 folds it: `mode = channel_mode % 4`, so the switch `default:` (line 120) is **unreachable** and the value behaves exactly like `channel_mode % 4` | `err_e7_channel_mode_out_of_range_enum` | [x] |
| E8  | `update_frame_header` | `channels == 0` with `channel_mode % 4 == 0` — unchecked unsigned underflow on line 109 | `(0u32 - 1) << 4` = `0xFFFFFFF0` OR-ed in ⇒ `frame_header == 0xFFFFFFF*` (whole header corrupted, every field forced to 1-bits) | `err_e8_channels_zero_underflow` | [x] |
| E9  | `update_frame_header` | `channels > 9` with `channel_mode % 4 == 0` — value overflows the 4-bit channel field on line 109 (e.g. 17, 4096, 0x10000001, 0xFFFFFFFF) | `(channels-1) << 4` mod 2^32 OR-ed in, bleeding into the samplerate/blocksize/sync fields and shifting bits off the top | `err_e9_channels_overflow_nibble` | [x] |
| E10 | `update_frame_header` | `bitdepth` not in {8,12,16,20,24,32} (line 142 `default:`) — e.g. 0, 1, 7, 9, 11, 33, 64 | **no bits OR-ed**; sample-size field stays `0x0` | `err_e10_bitdepth_default` | [x] |
| E11 | `update_frame_header` | `bitdepth` oversized / one step past the largest valid value — 33, 0x80000000, 0xFFFFFFFF (`switch` is on `tflac_u32`, so no truncation to 8 happens) | `default:` ⇒ no bits; in particular `bitdepth == 0x100000008` is impossible, and `0x108` does **not** alias `8` | `err_e11_bitdepth_oversized_no_truncation` | [x] |
| E12 | `update_frame_header` | pre-existing garbage in `t->frame_header` (e.g. `0xDEADBEEF`) — line 12 is an assignment `=`, not `|=` | prior contents fully discarded; result identical to calling on a zeroed `frame_header` | `err_e12_frame_header_garbage_overwritten` | [x] |
| E13 | `update_frame_header` | every field simultaneously at its extreme: all-`0x00` struct, and all-`0xFF` struct (`samplerate=channels=bitdepth=cur_blocksize=0xFFFFFFFF`, `channel_mode=0xFF`) | all-zero ⇒ `0xFFF8` base + blocksize `0x6` + samplerate `0xC`; all-`0xFF` ⇒ blocksize `0x7`, samplerate none, `mode=3`, bitdepth none | `err_e13_all_min_all_max` | [x] |
| E14 | `update_frame_header` | *aliasing / no other field written*: only `frame_header` may be modified; `samplerate`, `channels`, `bitdepth`, `channel_mode`, `cur_blocksize` and the 3 padding bytes at offset 13..15 must be byte-identical after the call | all 24 struct bytes except offsets 16..19 unchanged | `err_e14_only_frame_header_written` | [x] |
| E15 | `update_frame_header` | one step past each boundary of the explicit range checks: `samplerate/1000` at 255 vs 256 (255000 / 256000), `samplerate/10` at 65535 vs 65536 (655350 / 655360), `samplerate` at 65535 vs 65536, `cur_blocksize` at 256 vs 257 | 255000⇒`0x0C00`; 256000⇒none. 655350⇒`0x0E00`; 655360⇒none. 65535 (`%1000`=535≠0, `<65536`)⇒`0x0D00`; 65536 (`%1000`=536≠0, not `<65536`, `%10`=6≠0)⇒none. 256⇒blocksize `0x8`; 257⇒`default:`⇒`0x7` | `err_e15_off_by_one_boundaries` | [x] |
| E16 | `update_frame_header` | unaligned `tflac*` (pointer at offset+1 of a buffer). UB in both C and Rust, but x86-64 permits unaligned 32-bit access; included because it is a real input an FFI caller can construct | identical result to the aligned call | `err_e16_unaligned_pointer` | [x] |

## Results

All 16 rows have a passing differential test:

```
$ cargo test --no-default-features --test phase_c_errors
running 17 tests
test err_e1_null_pointer_both_segv ................. ok   (both die with signal 11)
test err_e2_blocksize_default_le256 ................ ok
test err_e3_blocksize_default_gt256 ................ ok
test err_e4_samplerate_khz_out_of_range ............ ok
test err_e5_samplerate_dahz_out_of_range ........... ok
test err_e6_samplerate_no_branch_taken ............. ok
test err_e7_channel_mode_out_of_range_enum ......... ok
test err_e8_channels_zero_underflow ................ ok
test err_e9_channels_overflow_nibble ............... ok
test err_e10_bitdepth_default ...................... ok
test err_e11_bitdepth_oversized_no_truncation ...... ok
test err_e12_frame_header_garbage_overwritten ...... ok
test err_e13_all_min_all_max ....................... ok
test err_e14_only_frame_header_written ............. ok
test err_e15_off_by_one_boundaries ................. ok
test err_e16_unaligned_pointer ..................... ok
test null_deref_probe .............................. ignored (subprocess probe for E1)

test result: ok. 16 passed; 0 failed; 1 ignored
```

Every row asserts the *specific* C outcome (the exact `frame_header` value or
nibble the C produces, or the exact fatal signal), not merely "both failed".

**E16 initially FAILED** and exposed the one real divergence in the
translation — the Rust reference-creation alignment check aborted where the C
performed the unaligned access. See `VERIFICATION.md` ("Divergence found and
fixed") for the fix in `src/lib.rs`.
