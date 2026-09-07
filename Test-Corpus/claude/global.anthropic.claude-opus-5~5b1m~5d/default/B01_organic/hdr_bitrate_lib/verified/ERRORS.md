# ERRORS.md — Phase A: error-surface table

Mechanically derived from the *entire* C source (`c_src/src/lib.c`, 14 lines) and
header (`c_src/include/lib.h`, 3 lines).

Grep audit of every rejection mechanism a C library can use:

```
grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|if *\(|switch|for|while|<|>|\?|goto|enum' c_src/src/lib.c c_src/include/lib.h
```

Result: **zero** matches for error returns, asserts, range checks, null checks,
error enums, or min/max constants. `hdr_bitrate` is total over its declared
argument type: it has no branches, returns `unsigned`, and has no sentinel or
failure value. There is no "rejection" path in the source.

Consequently the error surface consists *entirely* of unchecked conditions —
inputs the C accepts but which drive it outside the declared bounds of its
lookup table, plus the unchecked pointer. The C behaviour for each is the ground
truth and was captured empirically from the compiled `.so`; the Rust must
reproduce it bit-for-bit.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `hdr_bitrate` | `h == NULL` — pointer is dereferenced at `h[1]`/`h[2]` with no null check | no error value; process dies with `SIGSEGV` (11). Rust must die with the *same* signal, not return a value |
| 2 | `hdr_bitrate` | layer field `((h[1]>>1)&3) == 0` (MPEG "reserved" layer) → inner index `-1`, i.e. a read 15 bytes *before* the row, **with plane bit `h[1]&0x8 == 0`** (flat offset `-15 ..= -1`, before the whole `halfrate` object) | returns `0` for every one of the 16 bitrate nibbles (verified on the built `.so`) |
| 3 | `hdr_bitrate` | layer field `== 0` (index `-1`) **with plane bit `h[1]&0x8 != 0`** (flat offset `30 ..= 45`) | aliases plane 0 / layer 2: `2*halfrate[0][2][rate]` — i.e. `0,32,48,56,64,80,96,112,128,144,160,176,192,224,256` for `rate = 0..14` |
| 4 | `hdr_bitrate` | bitrate nibble `h[2]>>4 == 0xF` (MPEG "bad" bitrate) → row-relative index `15`, one past the 15-element row, for any `(plane, layer)` other than the last row | reads element `0` of the following row, which is `0` in this table → returns `0` |
| 5 | `hdr_bitrate` | bitrate nibble `== 0xF` **and** `plane == 1, layer == 2` (last row) → flat offset `90`, one byte past the end of the whole `halfrate` object | reads the zero padding after the object → returns `0` |
| 6 | `hdr_bitrate` | both out-of-range fields at once: layer field `== 0` **and** bitrate nibble `== 0xF` | `plane 0`: `0` (row 2 rule); `plane 1`: `0` (aliases `halfrate[0][2][15]` → element 0 of next row → `0`) |
| 7 | `hdr_bitrate` | bitrate nibble `== 0` (MPEG "free format") — a valid encoding of an *unusable* rate, no check | `2*halfrate[p][l][0] == 0` for every plane/layer |
| 8 | `hdr_bitrate` | ignored/undefined bits set: `h[1]` bits 0 and 4–7, and the low nibble of `h[2]`, and the whole of `h[0]` — none are masked or validated | must not affect the result; identical output to the same input with those bits cleared |
| 9 | `hdr_bitrate` | `h[3]` and beyond unmapped (buffer ends exactly after `h[2]`, next page `PROT_NONE`) — the C never reads past `h[2]`, so this must *not* fault | returns normally; Rust must also not fault (proves Rust reads no extra bytes) |
| 10 | `hdr_bitrate` | `h[0]` unmapped / poisoned — the C never reads `h[0]` | returns normally; Rust must also not read it |
| 11 | `hdr_bitrate` | misaligned pointer (`h` at an odd address) — `uint8_t*` has no alignment requirement | returns normally, same value |
| 12 | `hdr_bitrate` | "out-of-range enum value" analogue: the layer field is a 2-bit enum whose value `0` has no valid variant, and the bitrate field is a 4-bit enum whose value `0xF` has no valid variant. Every one of the `2 x 4 x 16 = 128` field combinations is passed across the FFI boundary, including all no-valid-variant ones | every combination must match the C `.so` exactly (this is rows 2–6 exhaustively) |

## Checklist

- [x] 1 — `err_null_pointer_same_signal`
- [x] 2 — `err_layer_reserved_plane0`
- [x] 3 — `err_layer_reserved_plane1`
- [x] 4 — `err_bitrate_bad_nibble_all_rows`
- [x] 5 — `err_bitrate_bad_nibble_last_row`
- [x] 6 — `err_both_out_of_range`
- [x] 7 — `err_free_format_zero_nibble`
- [x] 8 — `err_ignored_bits_do_not_matter`
- [x] 9 — `err_no_read_past_h2_guard_page`
- [x] 10 — `err_h0_never_read_guard_page`
- [x] 11 — `err_misaligned_pointer`
- [x] 12 — `exhaustive_all_65536_inputs`
