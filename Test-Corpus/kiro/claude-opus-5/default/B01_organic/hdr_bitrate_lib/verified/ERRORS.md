# ERRORS.md — error / rejection surface table (Phase C gate)

## Mechanical derivation

```
$ grep -rnE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|ERROR|_ERR|if *\(|switch|\?|#ifdef|#if |goto|exit\(|abort\(' c_src/src c_src/include
(no matches)
```

`c_src/src/lib.c` contains **no** conditional, no `assert`, no error enum, no
sentinel return, no range check, no null check, and no `#ifdef`. `hdr_bitrate`
is a single unconditional `return` expression. There is therefore **no explicit
error-return path to differential-test**, and no error code or sentinel value
exists in this API — every input produces a normal `unsigned` return.

That makes the real rejection surface *implicit*: inputs the C accepts without
complaint but that push the array subscript outside the declared bounds of
`static const uint8_t halfrate[2][3][15]`. The compiled code
(`objdump -d`) performs a flat, unchecked byte load:

```
lea rax,[rip+0xe8b]   # halfrate.0  (.rodata, addr 0x2000)
add rax,rdx           # rdx = 45*i + 15*j + k   (sign-extended, may be negative)
movzx eax,BYTE PTR [rax]
add eax,eax           # result = 2 * byte
```

with `i = !!(h[1] & 0x8)` ∈ {0,1}, `j = ((h[1] >> 1) & 3) - 1` ∈ {**-1**,0,1,2},
`k = h[2] >> 4` ∈ 0..15. Flat offset range is therefore **-15 .. +90**, while
the table is only bytes 0..89. Rows below are every distinct way that happens.

Verified byte layout of the C `.so` (this is the ground truth the Rust must
reproduce):

* Table `halfrate.0` is at vaddr `0x2000`, the **first byte of a LOAD segment**.
* vaddr `0x1ff0..0x1fff` (the 16 bytes read by negative offsets) are `00` — file
  padding between `.fini` and `.rodata`, mapped as part of the R+E segment page.
* vaddr `0x205a..0x205b` (the bytes read by offset 90) are `00` — `.rodata`
  alignment padding before `.eh_frame_hdr`.

So **every out-of-bounds read yields byte 0, hence return value 0.**

## Rejection / edge-condition rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `hdr_bitrate` | reserved layer bits `(h[1]>>1)&3 == 0` ⇒ `j = -1`, with `h[1]&0x8 == 0` (`i=0`) and `h[2]>>4` = 0..14 ⇒ flat offset **-15..-1**, i.e. read **before** the table | reads zero padding ⇒ returns **0** |
| 2 | `hdr_bitrate` | reserved layer bits (`j = -1`), `i=0`, `h[2]>>4 == 15` ⇒ offset **0** — negative row index cancels, lands back **inside** the table at `halfrate[0][0][0]` | returns `2*0` = **0** |
| 3 | `hdr_bitrate` | reserved layer bits (`j = -1`) with `h[1]&0x8 != 0` (`i=1`) ⇒ offsets **30..45**: silently **aliases a different row**, `halfrate[0][2][k]` for k=0..14, and `halfrate[1][0][0]` for k=15 | returns `2*halfrate_flat[30+k]` — i.e. **0,32,48,56,64,80,96,112,128,144,160,176,192,224,256,0** |
| 4 | `hdr_bitrate` | "bad"/reserved bitrate nibble `h[2]>>4 == 15` with a *valid* layer, any `i`, where `(i,j) != (1,2)` ⇒ offset is the **first byte of the next row** (15, 30, 45, 60, 75) | returns `2*halfrate_flat[45i+15j+15]`, which is `2*0` = **0** in every such case (every row starts with 0) |
| 5 | `hdr_bitrate` | `h[2]>>4 == 15` **and** `i=1, j=2` (`h[1]&0x8` set, layer bits `0b11`) ⇒ flat offset **90**, one byte **past** the 90-byte table | reads `.rodata` alignment padding ⇒ returns **0** |
| 6 | `hdr_bitrate` | `h == NULL` | unconditional `h[1]` deref ⇒ **SIGSEGV**. No null check exists to differential-test a return value; the Rust must fault identically rather than returning a value. Covered by a *dedicated crash-parity subprocess test*, not by a return-value comparison. |
| 7 | `hdr_bitrate` | buffer shorter than 3 bytes (length 0/1/2) | there is no length parameter, so nothing is checked: the C reads `h[1]`/`h[2]` unconditionally, past the end of the caller's buffer. Not a rejection; the Rust must read exactly the same two offsets and no others. Tested by a guard-page/offset-read test. |
| 8 | `hdr_bitrate` | `h[0]` — never read by the C at all | must be **ignored**: result must be invariant under any value of `h[0]`. A Rust that read `h[0]` would be a divergence. |
| 9 | `hdr_bitrate` | out-of-range "enum-like" ints across FFI: `h[1]`/`h[2]` are `uint8_t`, so **all 256 values of each are in-range for the type** and there is no invalid encoding to reject. The layer field's reserved value `0b00` (row 1/2/3) *is* this API's out-of-range-enum case. | no rejection path exists; result is whatever the flat load produces (rows 1–5) |

## Notes on completeness

Rows 1–5 partition every out-of-declared-bounds subscript the function can
produce; rows 6–8 are the generic C-API boundaries (null pointer, undersized
buffer, unread input byte); row 9 records that the FFI enum-range class of bug
collapses into the reserved-layer rows because both inputs are `uint8_t`.

Since the total input space that affects the result is only
`h[1]` × `h[2]` = 256 × 256 = **65 536** cases, Phase B/C do not sample — they
**exhaustively** compare all 65 536, which covers every row above by
construction, in addition to each row's own targeted test.

## Phase C status — every row has a passing differential test

Tests live in `tests/phase_c_errors.rs` and reach both implementations only via
`dlopen` + `hdr_bitrate` (`tests/common/mod.rs`). All 11 pass.

| row | test | status |
|-----|------|--------|
| 1 | `errors_row1_reserved_layer_negative_offset` | [x] pass |
| 2 | `errors_row2_reserved_layer_wraps_to_offset_zero` | [x] pass |
| 3 | `errors_row3_reserved_layer_aliases_other_row` | [x] pass |
| 4 | `errors_row4_bad_nibble_reads_next_row_first_byte` | [x] pass |
| 5 | `errors_row5_offset_90_past_end_of_table` | [x] pass |
| 6 | `errors_row6_null_pointer_crash_parity` (forked child, compares death signal) | [x] pass — both die with SIGSEGV(11) |
| 7 | `errors_row7_reads_no_further_than_h2` (guard page after `h[2]`, all 65 536 pairs) | [x] pass |
| 7 | `errors_row7b_undersized_buffer_crash_parity` (0/1/2 readable bytes) | [x] pass — identical fault behaviour |
| 8 | `errors_row8_h0_never_read` | [x] pass |
| 9 | `errors_row9_no_invalid_encoding_is_rejected` | [x] pass |
| generic | `errors_boundary_one_past_each_field_range` (one step past each bit-field range) | [x] pass |

## Robustness of the "OOB reads yield 0" ground truth

The zero result for rows 1 and 5 comes from the C `.so`'s memory layout, so it
was checked rather than assumed. `halfrate.0` is the only object in `.rodata`
and sits at the first byte of a page-aligned LOAD segment, which makes the
preceding bytes segment padding and the following bytes alignment padding:

| C build | table vaddr | 15 bytes before | byte at +90 |
|---------|-------------|-----------------|-------------|
| `-O0` | `0x2000` | all `00` | `00` |
| `-O1` | `0x2000` | all `00` | `00` |
| `-O2` | `0x2000` | all `00` | `00` |
| `-O3` | `0x2000` | all `00` | `00` |
| `-Os` | `0x2000` | all `00` | `00` |

The full differential suite (16 tests) was re-run against the C library built at
each of those five optimization levels; 16/16 passed every time. Note this is a
property of the current single-translation-unit library: were more `.rodata`
ever linked ahead of the table, the C's own result for rows 1 and 5 would change
(the reads are UB), and the Rust would need to change with it.
