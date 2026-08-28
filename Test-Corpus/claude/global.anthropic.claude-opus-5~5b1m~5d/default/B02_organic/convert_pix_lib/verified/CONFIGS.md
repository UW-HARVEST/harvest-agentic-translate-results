# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the `if` / `switch` / loop branches that
`c_src/src/lib.c` actually takes, and from the two public entry points plus the
seven public **writable data** symbols (which are themselves runtime
configuration, since `cp_fixed`, `cp_dynamic` and `cp_block` read them on every
call).

## Axes the C code branches on

**Public entry points** (from `nm -D`, see `SYMBOLS.md`)

* `convert_pix(bpp, w, h, src, dst)` — the only header-declared function
* `cp_inflate(in, in_bytes, out, out_bytes)` — exported, drives the whole
  DEFLATE pipeline: `cp_read_bits` → `cp_peak_bits` / `cp_consume_bits` →
  `cp_stored` / `cp_fixed` / `cp_dynamic` → `cp_build` → `cp_decode` →
  `cp_block`.  These low-level helpers are `static`, so `cp_inflate` is the
  lowest-level reachable entry point for them and each must be driven through
  it.
* data symbols `cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`,
  `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base`, `cp_error_reason`

**`convert_pix` axes**

| axis | values the C distinguishes |
|------|----------------------------|
| `bpp` | `1` (`cp_make_pixel(g,g,g)`), `2` (`cp_make_pixel_a(g,g,g,a)`), `3` (`cp_make_pixel(r,g,b)`), `4` (`cp_make_pixel_a(r,g,b,a)`) — 4 distinct `switch` arms; anything else falls through the `switch` with **no** `default` |
| `w` | `0` (inner loop skipped), `1`, many |
| `h` | `0` (outer loop skipped), `1`, many — note `src++` once **per row** (the filter byte) |
| `src` bytes | value-dependent only through the copy; boundary bytes `0x00`/`0xFF` and random |

**`cp_inflate` axes**

| axis | values the C distinguishes |
|------|----------------------------|
| `in` alignment | `first_bytes = align4(in) - in ∈ {0,1,2,3}`: chooses how many bytes are pre-loaded into `s->bits` and where `s->words` starts |
| `in_bytes` mod 4 | `last_bytes ∈ {0,1,2,3}`: `final_word_available = 0` vs `1`, i.e. whether `cp_peak_bits` ever takes the `final_word` branch |
| `BTYPE` | `0` stored (`cp_stored`), `1` fixed (`cp_fixed`), `2` dynamic (`cp_dynamic`), `3` error |
| `BFINAL` | single block vs. a chain of blocks (`do…while(!bfinal)`), incl. mixed BTYPEs |
| fixed-block symbols | `< 256` literal, `== 256` end-of-block, `> 256` length/distance; literal `0…143` (8-bit code) vs `144…255` (9-bit code); length symbol `257…284` covering every `cp_len_extra_bits` bucket `0,1,2,3,4,5` and `285` (`base 258`, 0 extra) |
| distance | `cp_dist_base` buckets `0…29` covering every `cp_dist_extra_bits` value `0…13` |
| copy kind | `backwards_distance == 1` → `memset` arm; `!= 1` → byte-at-a-time arm; overlapping (`distance < length`) vs non-overlapping |
| dynamic header | `HLIT` → `nlit = 257 + 5 bits` (min 257, max 288); `HDIST` → `ndst = 1 + 5 bits` (min 1, max 32); `HCLEN` → `nlen = 4 + 4 bits` (min 4, max 19) |
| code-length alphabet | symbols `0…15` literal, `16` (copy previous, `3 + 2 bits`), `17` (zero run, `3 + 3 bits`), `18` (zero run, `11 + 7 bits`) — 4 `switch` arms in `cp_dynamic` |
| `cp_build` lookup fill | `len <= 9` fills `s->lookup` (only when `s != NULL`, i.e. the literal tree) vs `len > 9`; `s == NULL` for the distance and code-length trees |
| `out_bytes` | exactly the decompressed size vs. larger |
| global tables | untampered vs. tampered with alternative *valid* values (`cp_fixed_table` lengths `< 16`, `cp_len_base`, `cp_dist_base`, `cp_len_extra_bits`, `cp_dist_extra_bits`, `cp_permutation_order`) |

## Table

All rows are exercised with many pseudo-random inputs (`SplitMix64`, fixed seed
`0x243F6A8885A308D3`) unless the row is inherently a single shape.
Tests live in `tests/convert_pix.rs`, `tests/inflate.rs`, `tests/globals.rs`.

| #  | entry point(s) | configuration (options set + input shape) | [x] | test |
|----|----------------|-------------------------------------------|-----|------|
| 1  | data symbols | read all 320 bytes of `cp_fixed_table` from both `.so`s | [x] | `globals::g01_fixed_table` |
| 2  | data symbols | read all 19 bytes of `cp_permutation_order` | [x] | `globals::g02_permutation_order` |
| 3  | data symbols | read all 31 bytes of `cp_len_extra_bits` | [x] | `globals::g03_len_extra_bits` |
| 4  | data symbols | read all 31 `u32`s of `cp_len_base` | [x] | `globals::g04_len_base` |
| 5  | data symbols | read all 32 bytes of `cp_dist_extra_bits` | [x] | `globals::g05_dist_extra_bits` |
| 6  | data symbols | read all 32 `u32`s of `cp_dist_base` | [x] | `globals::g06_dist_base` |
| 7  | data symbols | `cp_error_reason` is `NULL` before the first call, and holds the identical *string* after each failing call | [x] | `globals::g07_error_reason_initially_null`, every `errors.rs` test |
| 8  | `convert_pix` | `bpp = 1`, `w × h` over the full cross product `w ∈ {0,1,2,3,7,16}` × `h ∈ {0,1,2,3,5}`, random `src` | [x] | `convert_pix::c01_bpp1_grid` |
| 9  | `convert_pix` | `bpp = 2`, same grid, random `src` | [x] | `convert_pix::c02_bpp2_grid` |
| 10 | `convert_pix` | `bpp = 3`, same grid, random `src` | [x] | `convert_pix::c03_bpp3_grid` |
| 11 | `convert_pix` | `bpp = 4`, same grid, random `src` | [x] | `convert_pix::c04_bpp4_grid` |
| 12 | `convert_pix` | `bpp ∈ {1,2,3,4}`, `src` all `0x00` and all `0xFF` (boundary byte values) | [x] | `convert_pix::c05_boundary_bytes` |
| 13 | `convert_pix` | `bpp ∈ {1,2,3,4}`, 1 000 randomized `(w,h,src)` cases, `w·h ≤ 4096` | [x] | `convert_pix::c06_random_property` |
| 14 | `cp_inflate` | BTYPE 0 (stored), BFINAL 1, `LEN ∈ {0,1,2,3,4,5,17,64,255,256,1024}`, random payload, `in` offset 0 | [x] | `inflate::i01_stored_sizes` |
| 15 | `cp_inflate` | BTYPE 0 (stored), BFINAL 1, `in` pointer offsets `0,1,2,3` from a 4-aligned base (`first_bytes` axis) × random `LEN` | [x] | `inflate::i02_stored_alignments` |
| 16 | `cp_inflate` | BTYPE 1 (fixed), BFINAL 1, literals only, all from `0…143` (8-bit codes), lengths `0…64` | [x] | `inflate::i03_fixed_literals_low` |
| 17 | `cp_inflate` | BTYPE 1 (fixed), BFINAL 1, literals only, all from `144…255` (9-bit codes) | [x] | `inflate::i04_fixed_literals_high` |
| 18 | `cp_inflate` | BTYPE 1 (fixed), BFINAL 1, random literals over the full `0…255` range, 200 random streams | [x] | `inflate::i05_fixed_literals_random` |
| 19 | `cp_inflate` | BTYPE 1 (fixed), match with `backwards_distance == 1` → `memset` arm, every length bucket | [x] | `inflate::i06_fixed_distance_one_memset` |
| 20 | `cp_inflate` | BTYPE 1 (fixed), match with `distance > 1`, non-overlapping (`distance >= length`) | [x] | `inflate::i07_fixed_nonoverlapping` |
| 21 | `cp_inflate` | BTYPE 1 (fixed), match with `distance > 1`, **overlapping** (`distance < length`) → byte-at-a-time arm | [x] | `inflate::i08_fixed_overlapping` |
| 22 | `cp_inflate` | BTYPE 1 (fixed), one match per `cp_len_extra_bits` bucket: length symbols `257…285` (extra bits `0,1,2,3,4,5,0`) | [x] | `inflate::i09_fixed_all_length_symbols` |
| 23 | `cp_inflate` | BTYPE 1 (fixed), one match per `cp_dist_base` bucket: distance symbols `0…29` (extra bits `0…13`) | [x] | `inflate::i10_fixed_all_distance_symbols` |
| 24 | `cp_inflate` | BTYPE 1 (fixed), random mix of literals + matches, 300 random streams, `out_bytes` exact fit | [x] | `inflate::i11_fixed_random_lz` |
| 25 | `cp_inflate` | BTYPE 1 (fixed), same as row 24 but `out_bytes` larger than needed | [x] | `inflate::i12_fixed_out_bigger` |
| 26 | `cp_inflate` | BTYPE 1 (fixed), `in` pointer offsets `0,1,2,3` × `in_bytes mod 4 ∈ {0,1,2,3}` (`first_bytes` × `last_bytes` cross product, i.e. word-load vs `final_word` paths) | [x] | `inflate::i13_fixed_alignment_matrix` |
| 27 | `cp_inflate` | BTYPE 2 (dynamic), minimal header: `nlit = 257`, `ndst = 1`, and the **smallest `nlen` that can still transmit the code-length symbols in use**.  (`nlen = 4` transmits only symbols `16,17,18,0`, so it can describe nothing but all-zero tables — that case is an empty tree, ERRORS.md row 20.) | [x] | `inflate::i14_dynamic_minimal` |
| 28 | `cp_inflate` | BTYPE 2 (dynamic), maximal header: `nlit = 288`, `ndst = 32`, `nlen = 19` | [x] | `inflate::i15_dynamic_maximal` |
| 29 | `cp_inflate` | BTYPE 2 (dynamic), code-length symbol `16` (copy previous, `3 + 2` extra) used | [x] | `inflate::i16_dynamic_clen_rep16` |
| 30 | `cp_inflate` | BTYPE 2 (dynamic), code-length symbol `17` (zero run, `3 + 3` extra) used | [x] | `inflate::i17_dynamic_clen_rep17` |
| 31 | `cp_inflate` | BTYPE 2 (dynamic), code-length symbol `18` (zero run, `11 + 7` extra) used | [x] | `inflate::i18_dynamic_clen_rep18` |
| 32 | `cp_inflate` | BTYPE 2 (dynamic), code lengths spanning `1…15` so `cp_build`'s `len <= 9` lookup-fill branch and the `len > 9` skip are both hit | [x] | `inflate::i19_dynamic_deep_codes` |
| 33 | `cp_inflate` | BTYPE 2 (dynamic), 300 random streams: random code-length distribution, random literals + matches | [x] | `inflate::i20_dynamic_random` |
| 34 | `cp_inflate` | BTYPE 2 (dynamic), `in` offsets `0,1,2,3` × `in_bytes mod 4` | [x] | `inflate::i21_dynamic_alignment_matrix` |
| 35 | `cp_inflate` | multi-block: `fixed, fixed` (BFINAL `0,1`) | [x] | `inflate::i22_multi_fixed_fixed` |
| 36 | `cp_inflate` | multi-block: `fixed, dynamic` | [x] | `inflate::i23_multi_fixed_dynamic` |
| 37 | `cp_inflate` | multi-block: `dynamic, fixed` | [x] | `inflate::i24_multi_dynamic_fixed` |
| 38 | `cp_inflate` | multi-block: `dynamic, dynamic` | [x] | `inflate::i25_multi_dynamic_dynamic` |
| 39 | `cp_inflate` | multi-block: 2…5 blocks with a randomly chosen BTYPE (1 or 2) each, 200 random streams | [x] | `inflate::i26_multi_random` |
| 40 | `cp_inflate` | BTYPE 0 stored as the **non-final** block (exercises the C's quirk that `cp_stored` does not advance the bit reader past the payload, so the next block is decoded out of the stored bytes) | [x] | `inflate::i27_stored_not_final` |
| 41 | `cp_inflate` + globals | `cp_fixed_table` tampered with a different *valid* (`< 16`) code-length assignment, then a fixed block decoded; table restored afterwards | [x] | `globals::g08_tamper_fixed_table` |
| 42 | `cp_inflate` + globals | `cp_len_base` / `cp_len_extra_bits` tampered with alternative valid values, then a fixed block with a match decoded | [x] | `globals::g09_tamper_len_tables` |
| 43 | `cp_inflate` + globals | `cp_dist_base` / `cp_dist_extra_bits` tampered with alternative valid values | [x] | `globals::g10_tamper_dist_tables` |
| 44 | `cp_inflate` + globals | `cp_permutation_order` permuted (still a permutation of `0…18`), then a dynamic block encoded with the same permutation | [x] | `globals::g11_tamper_permutation_order` |
| 45 | `cp_inflate` | `out_bytes` exactly `0` with an **empty** final stored block (`LEN = 0`) — nothing written, returns `1` | [x] | `inflate::i28_empty_output` |
| 46 | `cp_inflate` | fixed block that emits exactly `out_bytes` bytes and then end-of-block (boundary: `out == out_end` is legal) | [x] | `inflate::i29_exact_fill` |
| 47 | `cp_inflate` | large payload (64 KiB) through a dynamic block, to exercise repeated `cp_peak_bits` word loads and the `s->lookup` refill | [x] | `inflate::i30_large_payload` |

## Additional rows found while auditing the unchecked-index paths

These are *valid-shape* dynamic headers that the C accepts and that steer it
into code paths none of rows 1–47 reach.  They are listed here because the C
branches on them, and cross-referenced from `ERRORS.md` rows 31–36.

| #  | entry point(s) | configuration (options set + input shape) | [x] | test |
|----|----------------|--------------------------------------------|-----|------|
| 48 | `cp_inflate` | BTYPE 2, `HDIST` declared but **every distance code length 0** → `cp_build` returns `0` → `s->ndst == 0`.  Literal-only payloads decode normally; a match makes `cp_decode` read `s->dst[-1] == s->lit[287]`. | [x] | `oob_tables::oob01_empty_distance_tree_reads_past_dist_tables` |
| 49 | `cp_inflate` | as row 48, over every length symbol `257…285`, every extra-bit value at the bucket edges, every input alignment, and output buffers both large enough and one byte too small | [x] | `oob_tables::oob02_empty_distance_tree_matrix`, `oob_tables::oob03_empty_distance_tree_out_too_small` |
| 50 | `cp_inflate` | BTYPE 2 whose code-length program overshoots `lens[288+32]` by 1…20 bytes (symbol 16 / 17 / 18 as the final run) — lands in dead locals | [x] | `dynamic_overshoot::ov01_into_lenlens`, `ov02_into_sym_and_nlen` |
| 51 | `cp_inflate` | BTYPE 2 overshooting far enough to zero `ndst` (37…41 bytes past) — with literal-only, with a `distance > 1` match, and with a `distance == 1` (`memset` arm) match | [x] | `dynamic_overshoot::ov03_corrupts_ndst` |
| 52 | `cp_inflate` | BTYPE 2 overshooting far enough to zero `nlit` (42…45 bytes past) | [x] | `dynamic_overshoot::ov04_corrupts_nlit` |
| 53 | `cp_inflate` | BTYPE 2 overshooting into the run counters / the loop variable `n` / the HCLEN counter (46…65 bytes past) | [x] | `dynamic_overshoot::ov05`, `ov06`, `ov07` |
| 54 | `cp_inflate` | BTYPE 2 with the longest possible overshoot (66…138 bytes past), i.e. every run that could reach the saved frame pointer | [x] | `dynamic_overshoot::ov08_runaway_never_reaches_saved_rbp` |
| 55 | `cp_inflate` + globals | `cp_permutation_order[pos]` set to an out-of-range slot `19…63` (`pos ∈ {0,3,8}`), so `lenlens[slot]` in the C lands on `sym`/`nlen`/`ndst`/`nlit`/the run counters/`n`/`i` | [x] | `dynamic_overshoot::ov09_permutation_order_out_of_range` |
| 56 | `cp_inflate` + globals | `cp_fixed_table[pos]` set to an *invalid* code length `16…255` at 9 different positions, then a BTYPE=1 block | [x] | `oob_tables::oob04_code_length_ge_16_sweep` |

## How to run

```sh
cd translation
cargo build --release                     # the .so under test
cargo test  --release -- --test-threads=1 # everything
```

`tests/common/mod.rs` picks `target/release/libconvert_pix_lib.so` by default;
set `CP_RUST_SO=<path>` to point the whole suite at a different build (the debug
cdylib, which has overflow checks on, is also verified this way).
`CP_FORK_FUZZ_N`, `CP_FUZZ_N`, `CP_FORK_BOUND_N` and `CP_CHILD_TIMEOUT_US` scale
the randomised sweeps.
