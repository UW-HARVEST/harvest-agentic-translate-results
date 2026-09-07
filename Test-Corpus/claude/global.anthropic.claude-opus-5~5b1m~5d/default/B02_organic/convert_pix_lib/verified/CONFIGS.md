# CONFIGS.md — configuration-surface table (Phase B)

Mechanically derived from the branches `c_src/src/lib.c` actually takes on
valid input. Axes found in the source:

**`cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`**

* `A1` input-pointer misalignment `first_bytes = (align4(in) - in)` ∈ {0,1,2,3}
  (L314) — decides how many bytes are pre-loaded into `s->bits` and where
  `s->words` starts.
* `A2` tail length `last_bytes = (in_bytes - first_bytes) & 3` ∈ {0,1,2,3}
  (L317) — decides `final_word_available` and the `else if` arm of
  `cp_peak_bits` (L99).
* `A3` `BTYPE` ∈ {0 = stored (`cp_stored`), 1 = fixed (`cp_fixed`),
  2 = dynamic (`cp_dynamic`)} (L333). (`BTYPE == 3` is an error → ERRORS.md #6.)
* `A4` `BFINAL`: single final block vs. multiple chained blocks (L330/L365).
* `A5` `cp_block` copy path: `backwards_distance == 1` (`memset`, L295) vs.
  `> 1` (byte-wise, possibly overlapping, copy loop L298).
* `A6` symbol classes in `cp_block`: `< 256` literal, `== 256` end-of-block,
  `> 256` length/distance pair (L252/L264/L301).
* `A7` length symbol extra-bit classes from `cp_len_extra_bits` (0,1,2,3,4,5
  extra bits) and the special symbol 285 (`len_base = 258`, 0 extra bits).
* `A8` distance symbol extra-bit classes from `cp_dist_extra_bits`
  (0..13 extra bits) i.e. distances 1..32768.
* `A9` `cp_build` lookup-table path: `s != NULL && len <= 9` (fills
  `s->lookup`) vs. `s == NULL` / `len > 9` (L143/L152). Hit by the literal tree
  (`s` passed) vs. the distance tree (`s == 0`).
* `A10` `cp_dynamic` code-length RLE symbols: `16` (copy previous 3-6), `17`
  (zero-run 3-10), `18` (zero-run 11-138), and direct lengths (L227-243).
* `A11` `cp_dynamic` header sizes `HLIT` (`nlit` 257..288), `HDIST`
  (`ndst` 1..32), `HCLEN` (`nlen` 4..19) (L218-220).
* `A12` `cp_stored` bit-alignment discard `cp_read_bits(s, s->count & 7)`
  (L166) and the `bits_left/8 <= LEN` acceptance window (L179) — a stored block
  is only accepted when it is the *last* thing in the input.
* `A13` `out_bytes` exactly equal to the decompressed size vs. larger.

**`convert_pix(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst)`**

* `B1` `bpp` ∈ {1, 2, 3, 4} (the four `switch` arms, L477-490) — `1`/`3` take
  `cp_make_pixel` (alpha forced to `0xFF`), `2`/`4` take `cp_make_pixel_a`.
* `B2` `w` ∈ {0, 1, many}; `B3` `h` ∈ {0, 1, many} (the two loop nests, and the
  per-row `src++` filter-byte skip at L475).

## Rows

Each row is exercised with many randomized inputs (fixed seed `0x5EED_1234`,
`tests/common/mod.rs::Rng`), not one hand-picked value. "match" means the C and
Rust `.so` agree byte-for-byte on: the return value, the full `out` buffer, and
the `cp_error_reason` pointer's string contents.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `cp_inflate` | `A3=1` fixed-Huffman, literals only, random payload 1..64 B, `A1=0`, `A13`=exact | [x] |
| 2 | `cp_inflate` | `A3=1` fixed, literals only, `A1=1` (input pointer at addr%4==1) | [x] |
| 3 | `cp_inflate` | `A3=1` fixed, literals only, `A1=2` | [x] |
| 4 | `cp_inflate` | `A3=1` fixed, literals only, `A1=3` | [x] |
| 5 | `cp_inflate` | `A3=1` fixed, `A2=0` (input length ≡ first_bytes mod 4, no final partial word) | [x] |
| 6 | `cp_inflate` | `A3=1` fixed, `A2=1` | [x] |
| 7 | `cp_inflate` | `A3=1` fixed, `A2=2` | [x] |
| 8 | `cp_inflate` | `A3=1` fixed, `A2=3` | [x] |
| 9 | `cp_inflate` | `A3=1` fixed, `A5=1`/`A6=">256"` — run-length payload (`b"a"*N`), forcing the `memset` copy path with `distance == 1` | [x] |
| 10 | `cp_inflate` | `A3=1` fixed, `A5>1` — overlapping copy, `distance` ∈ {2,3,5,17}, lengths spanning `A7` classes 0..5 | [x] |
| 11 | `cp_inflate` | `A3=1` fixed, `A7`: match lengths 3,4,10,11,12,17,18,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258 (one per `len_base`/extra-bits bucket incl. sym 285) | [x] |
| 12 | `cp_inflate` | `A3=1` fixed, `A8`: distances 1,2,3,4,5,7,9,13,17,25,33,49,...,24577 and 32768 (one per `dist_base` bucket) | [x] |
| 13 | `cp_inflate` | `A3=1` fixed, `A9`: `len > 9` codes present in the literal tree (fixed table has 9-bit codes → lookup partially filled) | [x] |
| 14 | `cp_inflate` | `A3=2` dynamic, `A10=17`/direct, small alphabet (few distinct literals ⇒ long zero runs); own encoder + `flate2` levels 6/9 | [x] |
| 15 | `cp_inflate` | `A3=2` dynamic, `A10=18` — ≥ 11-symbol zero runs (single-literal input) | [x] |
| 16 | `cp_inflate` | `A3=2` dynamic, `A10=16` — long runs of *equal* code lengths, from a wide (64..256 symbol) literal alphabet. NOTE: `flate2` emits a STORED block for uniform random data, so this row must use the hand-written encoder; `tests/coverage.rs` asserts that code 16 really is transmitted | [x] |
| 17 | `cp_inflate` | `A3=2` dynamic, `A11`: `HDIST=1` (`ndst == 1`, no matches emitted) | [x] |
| 18 | `cp_inflate` | `A3=2` dynamic, `A11`: both extremes of the header sizes — `HLIT=288` + `HDIST=32` (with trailing zero code lengths) and minimal `HLIT=257`; plus `flate2` output over a full-alphabet payload | [x] |
| 19 | `cp_inflate` | `A3=2` dynamic with matches — `A5`, `A6`, `A7`, `A8` all exercised through the dynamic trees | [x] |
| 20 | `cp_inflate` | `A3=0` stored, `A12`: single final stored block, `LEN` = exact remaining byte count, `LEN` ∈ {0,1,2,3,4,5,17,64,255,256,1000} | [x] |
| 21 | `cp_inflate` | `A3=0` stored, `A12`: stored block preceded by the byte-alignment discard being non-zero (`s->count & 7 != 0`) | [x] |
| 22 | `cp_inflate` | `A3=0` stored, `A1` ∈ {0,1,2,3} × `A2` ∈ {0,1,2,3} — `cp_ptr` byte-address computation across every alignment/tail combination | [x] |
| 23 | `cp_inflate` | `A4`: multi-block stream — `BFINAL=0` fixed block followed by a `BFINAL=1` fixed block | [x] |
| 24 | `cp_inflate` | `A4`+`A3` mixed: dynamic block then fixed block then final dynamic block | [x] |
| 25 | `cp_inflate` | `A13`: `out_bytes` strictly larger than the decompressed size (trailing bytes of `out` must be untouched identically) | [x] |
| 26 | `cp_inflate` | real zlib/flate2-produced raw-deflate streams at every compression level 0..9 over randomized payloads (mixes `A3`, `A5`-`A11` as the encoder chooses) | [x] |
| 27 | `cp_inflate` | large payload (64 KiB) with 32 KiB-scale distances, level 9 | [x] |
| 28 | `cp_inflate` | idempotence/state: `cp_inflate` called repeatedly in a loop on different streams (no leaked state between calls) | [x] |
| 29 | `convert_pix` | `B1=1`, `B2`/`B3` ∈ {0,1,7,64} × {0,1,5,33}, randomized `src` | [x] |
| 30 | `convert_pix` | `B1=2`, same shape cross-product | [x] |
| 31 | `convert_pix` | `B1=3`, same shape cross-product | [x] |
| 32 | `convert_pix` | `B1=4`, same shape cross-product | [x] |
| 33 | `convert_pix` | `B1` ∈ {1,2,3,4} with `w*h` large (256×64) randomized, checking full `dst` and that `dst` beyond `w*h` is untouched | [x] |
| 34 | exported data | `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base` byte-compared between the two `.so`s (these are mutable exported globals the algorithm reads through) | [x] |
| 35 | `cp_inflate` | after tampering the exported `cp_fixed_table` identically in both libraries (a legitimate use of a mutable exported global): a fixed block decoded with a permuted-but-valid table | [x] |
| 36 | `cp_inflate` | `A3=2` dynamic with a maximally SKEWED literal code (code lengths 1..15), so `cp_build` takes both its `len <= 9` lookup-fill path and its `len > 9` skip path inside one tree, and `cp_decode` binary-searches deep entries; swept over 2..16 symbols | [x] |
| 37 | `cp_inflate` | `A3=2` dynamic, `A11`: HCLEN swept from the minimum that fits up to the maximum 19 (unused trailing code-length slots transmitted as zeros) | [x] |
| 38 | `cp_inflate` | `A3=2` dynamic, `A10`: code lengths transmitted LITERALLY (no 16/17/18 at all) — 320 iterations of `cp_dynamic`'s `default:` arm | [x] |
| 39 | `cp_inflate` | `A3=2` dynamic after tampering the exported `cp_permutation_order` identically in both libraries (reversed transmission order), with the stream written to match | [x] |
| 40 | `cp_inflate` | `A3=0` stored with `LEN` GREATER than the bytes remaining in the input (the L179 check only bounds `LEN` from below): the `memcpy` over-reads past `in + in_bytes` and over-writes past `out + out_bytes` (there is no `out_end` check in `cp_stored` at all). Driven with a deterministic over-read region and a large real `out` allocation. `align` x `m` x `k` cross-product | [x] |
| 41 | `cp_inflate` | `in_bytes` SMALLER than `first_bytes` (a misaligned input shorter than its alignment padding), so `word_count` and `last_bytes` are computed from a negative numerator and the pre-load / final-word loops read outside `[in, in+in_bytes)`. `align` in 1..3 x `in_bytes` in 1..6. Some combinations return, others `assert`, so it is driven in a child process comparing stdout + stderr + exit status | [x] |

## Notes on deliberately-untested (undefined-behaviour) configurations

The C code has paths whose behaviour is not a function of its inputs, so
"byte-identical" is not definable for them. They are listed here so their
absence is explicit rather than an oversight:

* `cp_dynamic`'s `lens[288+32]` is an **uninitialised** stack array, and code
  `16` at `n == 0` reads `lens[-1]`; likewise the RLE codes can push `n` past
  320. Streams that do this are rejected by any real encoder and produce
  indeterminate results in C.
* `cp_block` indexes `cp_len_extra_bits[symbol-257]` /
  `cp_dist_extra_bits[distance_symbol]` without bounds checks; a corrupt tree
  yielding a symbol > 285 (resp. > 29) reads past the end of a global.
* `cp_decode` reads `tree[lo - 1]`, i.e. `tree[-1]`, when the search lands at
  `lo == 0`.
* `cp_stored` `memcpy`s `LEN` bytes even when fewer than `LEN` bytes of input
  remain (the L179 check only bounds it from the other side). This IS covered, by
  row 40, by making the over-read region part of the buffer the harness hands to
  both libraries — but only for over-reads that stay inside that allocation.

Every row above is built from streams a conforming DEFLATE encoder can emit, so
none of them enter these paths; the abort-based rows in `ERRORS.md` cover the
ones that fail deterministically via `assert`.

## Row -> test mapping

| rows | test |
|------|------|
| 1-4   | `tests/valid.rs::row01..row04_fixed_literals_align{0,1,2,3}` |
| 5-8   | `tests/valid.rs::row05..row08_tail{0,1,2,3}` |
| 9     | `tests/valid.rs::row09_distance_one_memset_path` |
| 10    | `tests/valid.rs::row10_overlapping_copies` |
| 11    | `tests/valid.rs::row11_every_length_bucket` |
| 12    | `tests/valid.rs::row12_every_distance_bucket` |
| 13    | `tests/valid.rs::row13_nine_bit_literals` |
| 14    | `tests/valid.rs::row14_dynamic_small_alphabet` |
| 15    | `tests/valid.rs::row15_dynamic_long_zero_runs_code18` |
| 16    | `tests/valid.rs::row16_dynamic_repeat_code16` |
| 17    | `tests/valid.rs::row17_dynamic_hdist_one` |
| 18    | `tests/valid.rs::row18_dynamic_full_header_sizes` |
| 19    | `tests/valid.rs::row19_dynamic_matches_all_classes` |
| 20    | `tests/valid.rs::row20_stored_various_lengths` |
| 21    | `tests/valid.rs::row21_stored_after_alignment_discard` |
| 22    | `tests/valid.rs::row22_stored_alignment_times_tail_cross_product` |
| 23    | `tests/valid.rs::row23_two_fixed_blocks` |
| 24    | `tests/valid.rs::row24_many_chained_blocks` |
| 25    | `tests/valid.rs::row25_out_buffer_with_slack` |
| 26    | `tests/valid.rs::row26_all_compression_levels` |
| 27    | `tests/valid.rs::row27_large_payload` |
| 28    | `tests/valid.rs::row28_no_state_leak_between_calls` |
| 29-32 | `tests/valid.rs::row29..row32_convert_pix_bpp{1,2,3,4}` |
| 33    | `tests/valid.rs::row33_convert_pix_large` |
| 34    | `tests/valid.rs::row34_exported_tables_identical` |
| 35    | `tests/tamper.rs::row35_permuted_fixed_table` |
| 36    | `tests/valid.rs::row36_dynamic_codes_longer_than_nine_bits` |
| 37    | `tests/valid.rs::row37_dynamic_hclen_sweep` |
| 38    | `tests/valid.rs::row38_dynamic_code_lengths_without_rle` |
| 39    | `tests/tamper.rs::row39_permuted_permutation_order` |
| 40    | `tests/valid.rs::row40_stored_len_beyond_remaining_input` |
| 41    | `tests/risky.rs::row41_in_bytes_smaller_than_first_bytes` |

The five `static` C helpers (`cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find`,
`cp_unfilter`) have no configuration axis reachable from `cp_inflate` /
`convert_pix`, so they are not rows here; they are covered directly by
`tests/private.rs` (see `SYMBOLS.md` for how their addresses are resolved).

`tests/coverage.rs` independently re-decodes the streams the rows generate with a
textbook DEFLATE reader and asserts that the claimed block types and code-length
RLE symbols really are present, so no row can pass while exercising nothing.

## Binary executable

`c_src/CMakeLists.txt` builds a shared library only — there is no driver
executable and the crate declares no `[[bin]]`, so there is no stdout to compare.
