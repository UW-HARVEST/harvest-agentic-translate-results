# CONFIGS.md — Phase A: the configuration-surface table (valid inputs)

Derived mechanically from the `if` / `switch` / loop-bound branches in
`c_src/src/lib.c`.  There are no compile-time (`#ifdef`) options and no runtime
"option struct"; the library's *configuration* is entirely (a) the values the
caller passes and (b) the fields inside the bit-stream that select code paths.

## Axes the C code actually branches on

**`unfilter(w, h, bpp, raw)`** — `lib.c:417`
* `A1` filter byte of a row: `switch (*raw++)` → `0 | 1 | 2 | 3 | 4 | default`
  (`lib.c:422` for row 0, `lib.c:446` for rows `y >= 1`).
* `A2` row position: **row 0** has its own `switch` in which `case 2` is a
  *no-op* and `case 4` degenerates to `paeth(a,0,0) == a`, and cases `1/3/4`
  start at `x = bpp` (no prologue).  Rows `y >= 1` use `prev[]` and have an
  `x < bpp` prologue loop.
* `A3` `h`: `< 0`, `0`, `1` (row 0 only), `2`, `>= 3` (`prev` chaining).
* `A4` `len = w * bpp` vs `bpp`: `len > bpp` (normal), `len == bpp`
  (`w == 1`, inner loops empty), `len == 0` (`w == 0` or `bpp == 0`),
  `len < bpp` , `len < 0` (negative `w`/`bpp`).
* `A5` `bpp`: `1, 2, 3, 4` (the PNG-real values) and `> len`.
* `A6` data values: `cp_paeth` has three outcomes (`a`, `b`, `c`) selected by
  `pa <= pb && pa <= pc` / `pb <= pc` (`lib.c:383`), and `+=` on `uint8_t`
  wraps — so byte values must be randomised, not fixed.
* `A7` per-row filter mixing: the filter byte is read per row, so a multi-row
  image can use a different `case` on every row.

**`cp_inflate(in, in_bytes, out, out_bytes)`** — `lib.c:314`
* `B1` `BTYPE`: `0` stored / `1` fixed Huffman / `2` dynamic Huffman / `3`
  reserved (`lib.c:339`).
* `B2` `BFINAL`: one block (`bfinal = 1`) vs a `do..while (!bfinal)` chain of
  several blocks, possibly of *different* `BTYPE`s (`lib.c:336..371`).
* `B3` input pointer alignment: `first_bytes = align4(in) - in` ∈ `{0,1,2,3}`
  seeds `s->bits` byte-by-byte and sets `count = first_bytes * 8`
  (`lib.c:320..330`).
* `B4` input tail: `last_bytes = (in_bytes - first_bytes) & 3` ∈ `{0,1,2,3}`
  selects the `final_word` / `final_word_available` path in `cp_peak_bits`
  (`lib.c:105`, `lib.c:323..329`).
* `B5` `word_count == 0` (whole stream fits in `first_bytes + final_word`) vs
  `> 0` (the `s->words[s->word_index++]` refill path, `lib.c:100`).
* `B6` `cp_block` symbol class: `< 256` literal / `== 256` end-of-block /
  `> 256` length-distance pair (`lib.c:258..307`).
* `B7` `backwards_distance == 1` → `memset` fast path, else the byte-at-a-time
  overlapping copy (`lib.c:299..306`) — the two differ for overlapping runs.
* `B8` length symbol class: `cp_len_extra_bits[]` has runs of `0,1,2,3,4,5,0`
  extra bits (`lib.c:58`); symbol `285` (`len_base 258`, 0 extra) is its own
  case, and `len_base[29] == len_base[30] == 0`.
* `B9` distance symbol class: `cp_dist_extra_bits[]` runs `0..13`
  (`lib.c:64`), distances `1 .. 32768`.
* `B10` `cp_build` lookup table: only `len <= 9` fills `s->lookup`
  (`lib.c:158`); codes of length `10..15` exist only in the sorted `tree[]`
  binary search.  `cp_build(0, ...)` (dist/lenlen trees) skips the lookup
  entirely.
* `B11` dynamic header shape (`lib.c:222..252`): `HLIT ∈ 257..288`,
  `HDIST ∈ 1..32`, `HCLEN ∈ 4..19` — and the code-length alphabet's own
  `switch (sym)` with `16` (copy-previous, 3..6), `17` (zero run 3..10),
  `18` (zero run 11..138) vs a direct literal length.
* `B12` stored-block shape (`lib.c:170..195`): `LEN == 0`, `LEN` small,
  several stored blocks in a row; plus the byte-alignment discard
  `cp_read_bits(s, s->count & 7)`.
* `B13` `out_bytes` slack: exactly the decompressed size vs larger (affects
  nothing on the happy path, but pins the `out_end` comparisons).

## Rows — one per combination the C treats differently

Each row is exercised with **many randomised inputs** (`SEED = 0x5EED_1234`,
`rand::rngs::StdRng::seed_from_u64`) unless marked *(exhaustive)*, and asserts
that the C `.so` and Rust `.so` agree on the return value, the **whole** output
buffer byte-for-byte, and `cp_error_reason`.

### `unfilter`

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `unfilter` | `h == 1`, filter `0` (None), `bpp ∈ {1,2,3,4}`, random `w ∈ 1..64`, random bytes *(exhaustive over filter×bpp)* | `row01_h1_filter_none` | [x] |
| 2 | `unfilter` | `h == 1`, filter `1` (Sub) — row-0 variant starting at `x = bpp`, all `bpp` | `row02_h1_filter_sub` | [x] |
| 3 | `unfilter` | `h == 1`, filter `2` (Up) — row-0 no-op, all `bpp` | `row03_h1_filter_up` | [x] |
| 4 | `unfilter` | `h == 1`, filter `3` (Average) — row-0 variant `raw[x] += raw[x-bpp]/2`, all `bpp` | `row04_h1_filter_average` | [x] |
| 5 | `unfilter` | `h == 1`, filter `4` (Paeth) — row-0 variant `paeth(a,0,0)`, all `bpp` | `row05_h1_filter_paeth` | [x] |
| 6 | `unfilter` | `h == 2`, row 0 filter `0`, row 1 filter `f ∈ 0..4` — first use of the `prev[]` path *(exhaustive over f×bpp)* | `row06_h2_first_prev_row` | [x] |
| 7 | `unfilter` | `h ∈ 3..8`, **random filter byte `0..4` per row**, `bpp ∈ {1,2,3,4}`, `w ∈ 1..48`, random bytes — `prev` chaining + `A7` mixing | `row07_multirow_mixed_filters` | [x] |
| 8 | `unfilter` | `h ∈ 2..6`, `bpp == 3`, byte values drawn from `{0,1,127,128,254,255}` to force `uint8_t` wrap-around and all three `cp_paeth` outcomes (`A6`) | `row08_extreme_byte_values_paeth_branches` | [x] |
| 9 | `unfilter` | `w == 1` (`len == bpp`): inner `x < len` loops empty, row-`y` prologue exactly fills the row, `bpp ∈ {1,2,3,4}`, all filters | `row09_w1_len_equals_bpp` | [x] |
| 10 | `unfilter` | `w == 0` → `len == 0`: only filter bytes consumed, `h ∈ 1..5`, all filters | `row10_w0_len_zero` | [x] |
| 11 | `unfilter` | `bpp == 0` → `len == 0`, `w ∈ 1..8`, `h ∈ 1..4`, all filters | `row11_bpp0_len_zero` | [x] |
| 12 | `unfilter` | `bpp > len` (`bpp ∈ 5..9`, `w == 1`, `h ∈ 2..4`): the row-`y` `x < bpp` prologue deliberately runs past `len` into the next row's bytes | `row12_bpp_greater_than_len` | [x] |
| 13 | `unfilter` | `h == 0` and `h < 0` with a non-zero filter byte present in the buffer (must be ignored) | `row13_h_zero_and_negative` | [x] |
| 14 | `unfilter` | negative `w` and/or negative `bpp` (`len <= 0` or sign-flipped), `h ∈ 1..3` | `row14_negative_w_bpp` | [x] |
| 15 | `unfilter` | large `w` (`256..1024`), `bpp == 4`, `h ∈ 4..16`, filter `4` on every row — long Paeth runs | `row15_large_paeth` | [x] |

### `cp_inflate`

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 16 | `cp_inflate` | `BTYPE 0` stored, `BFINAL 1`, `LEN ∈ 1..200` random payload, `out_bytes` exact (`B12`) | `row16_stored_block` | [x] |
| 17 | `cp_inflate` | `BTYPE 0` stored with `LEN == 0` | `row17_stored_block_len_zero` | [x] |
| 18 | `cp_inflate` | `BTYPE 1` fixed Huffman, literals only (`B6` `< 256` + `== 256`), random payload `1..300`, all 256 byte values represented | `row18_fixed_literals_only` | [x] |
| 19 | `cp_inflate` | `BTYPE 1` fixed, single length/distance pair, `distance == 1` → `memset` path (`B7`), `length` sweeping `3..258` *(exhaustive over length)* | `row19_distance_one_memset_path` | [x] |
| 20 | `cp_inflate` | `BTYPE 1` fixed, `distance ∈ 2..64` (byte-copy path, overlapping when `length > distance`), random `length` | `row20_distance_byte_copy_path` | [x] |
| 21 | `cp_inflate` | `BTYPE 1` fixed, length symbols chosen to cover **every** `cp_len_extra_bits` class `0..5` incl. symbol 285 (`B8`) *(exhaustive over the 29 length symbols)* | `row21_all_length_symbols` | [x] |
| 22 | `cp_inflate` | `BTYPE 1` fixed, distance symbols covering **every** `cp_dist_extra_bits` class `0..13` (`B9`) *(exhaustive over the 30 distance symbols, with enough preceding output)* | `row22_all_distance_symbols` | [x] |
| 23 | `cp_inflate` | `BTYPE 1` fixed, literals `>= 144` only — forces the 9-bit fixed codes (`cp_fixed_table` second run) and the `len <= 9` lookup fill (`B10`) | `row23_fixed_nine_bit_literals` | [x] |
| 24 | `cp_inflate` | `BTYPE 2` dynamic, real `flate2`/miniz-produced streams over random payloads `1..4096` (covers `B10` codes of length `10..15`, `B11` `16/17/18` run-length symbols) | `row24_dynamic_real_streams` | [x] |
| 25 | `cp_inflate` | `BTYPE 2` dynamic, hand-built header at the extremes: `HCLEN == 4`, `HDIST == 1`, `HLIT == 257` | `row25_dynamic_header_minimum` | [x] |
| 26 | `cp_inflate` | `BTYPE 2` dynamic, hand-built header with `HCLEN == 19`, `HLIT == 288`, `HDIST == 32` and code-length symbols `16`, `17`, `18` all used (`B11`) | `row26_dynamic_header_maximum_deep_tree` | [x] |
| 27 | `cp_inflate` | `BFINAL` chain: 2..5 blocks, `BTYPE` mixed `{0,1,2}` in every order (`B2`) | `row27_multi_block_chains` | [x] |
| 28 | `cp_inflate` | input pointer offset `0,1,2,3` from a 4-aligned base × input length `≡ 0,1,2,3 (mod 4)` → all 16 `(first_bytes, last_bytes)` combinations (`B3`,`B4`) *(exhaustive)* | `row28_input_alignment_and_tail` | [x] |
| 29 | `cp_inflate` | total input `< 4` bytes after alignment so `word_count == 0` and everything comes from `bits`+`final_word` (`B5`) | `row29_word_count_zero` | [x] |
| 30 | `cp_inflate` | `out_bytes` exactly the decompressed size vs `+1`, `+7`, `×2` slack (`B13`), over rows 18/20/24 streams | `row30_out_buffer_slack` | [x] |
| 31 | `cp_inflate` | large payloads (`8 KiB..64 KiB`) via `flate2` at compression levels `0,1,6,9` — level 0 emits stored blocks, level 9 emits deep dynamic trees; multi-block `B2` at scale | `row31_large_payloads_all_levels` | [x] |
| 32 | `cp_inflate` | payload of one repeated byte (`64 KiB`) — maximal `distance == 1` `memset` runs and `length == 258` chains | `row32_repeated_byte_runs` | [x] |
| 33 | `cp_inflate` | payload with a 32 KiB-distance back-reference (distance symbols 28/29, `dist_base 16385/24577`) | `row33_far_back_references` | [x] |
| 34 | `cp_inflate` | `unfilter` ∘ `cp_inflate`: inflate a real zlib-compressed PNG-style filtered scanline set, then `unfilter` the result — the composed pipeline both `.so`s must agree on end-to-end | `row34_inflate_then_unfilter_pipeline` | [x] |

## Notes discovered while running the rows

* **Row 16 / 27 / 31 (`BTYPE 0`)** — `cp_stored`'s copy source, `cp_ptr() =
  (char*)(words + word_index) - count/8`, is only correct while the bit buffer
  was last refilled from `s->words`.  Once `cp_peak_bits` falls back to
  `s->final_word` (which does not advance `word_index`), the pointer is short by
  `last_bytes` and the C code copies the `LEN`/`NLEN` bytes instead of the
  payload.  A lone stored block is `LEN + 5` input bytes, so the payload only
  lands correctly when `LEN % 4 == 3`.  Row 16 asserts the true plaintext for
  those and compares the rest purely differentially; see `ERRORS.md` §E.
* **Row 27** — a stored block can only be the *last* block in a chain, because
  `cp_stored` rejects anything with more than `LEN` input bytes still to come
  (`ERRORS.md` row 4).
* **Row 31** — miniz level 0 emits a *chain* of stored blocks, so payloads over
  64 KiB are rejected by the C library with
  `"Stored block extends beyond end of input stream."`; the test asserts exactly
  that, and that the Rust library rejects identically.
* **Row 21** — length `258` sits in both symbol 27's extra-bit range
  (`227 + 31`) and symbol 28's base, so the canonical encoder's choice (28) is
  not the only legal one; the test only asserts symbol identity for the
  unambiguous lengths.
* **Row 26** — the "deep" tree (`lit_lens_deep` / `dst_lens_deep`) deliberately
  places the 256 literals at depth 9 (so they land in `s->lookup`) and the
  length/distance symbols at depths 10..15, which `cp_build` leaves *out* of
  `s->lookup` on purpose — those are only reachable through `cp_decode`'s
  binary search over the sorted `tree[]`.  Its code-length stream is asserted to
  use symbols 16, 17 **and** 18.

## Result

All 34 rows pass, under all three build configurations
(`default`, `--no-default-features`, `--all-features`), against **both** C
builds (`cmake ..` with live `assert()`s and `-DCMAKE_BUILD_TYPE=Release`).
There is **no binary/driver target** in this crate (`Cargo.toml` declares only
`crate-type = ["cdylib"]`, there is no `src/main.rs` and no `[[bin]]`), and the
C project builds only a shared library (`add_library(... SHARED)`), so the
"compare the binaries' stdout" gate does not apply.
