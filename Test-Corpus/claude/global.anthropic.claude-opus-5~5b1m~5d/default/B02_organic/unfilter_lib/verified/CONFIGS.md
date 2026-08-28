# CONFIGS.md — Phase A: the configuration-surface table (valid inputs)

Mechanically derived from the branches that `c_src/src/lib.c` actually takes.
There is no init/options struct in this library: the "configuration" is
carried entirely by

* which exported entry point is called — `unfilter` or `cp_inflate`;
* the *scalar arguments* (`w`, `h`, `bpp` / `in_bytes`, `out_bytes`, and the
  4-byte **alignment of the `in` pointer**, which `cp_inflate` branches on
  through `first_bytes`);
* the *shape of the data*: the per-row filter byte (a 5-way `switch`), and,
  for `cp_inflate`, the DEFLATE block type (a 4-way `switch`), the Huffman
  table kind, and every length/distance code class;
* the **mutable exported tables** (`cp_fixed_table`, `cp_permutation_order`,
  `cp_len_extra_bits`, `cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base`),
  which are writable `D`-section globals and are therefore genuine runtime
  options an external caller can flip.

Every row is exercised with **many randomized inputs** (fixed seed, SplitMix64
PRNG) — never a single hand-picked value. Both libraries are called through
`libloading` on their respective `.so`, in a forked child with a shared-memory
scratch buffer, so that the input bytes, the buffer contents *around* the
nominal region and the 4-byte alignment are bit-identical for the two runs.
The whole scratch region, the return value and `cp_error_reason` are compared.

Cargo feature combinations: `Cargo.toml` declares one optional feature,
`c-asserts` (on by default), which mirrors the C build's `NDEBUG` switch. The
matrix is therefore {default (= `c-asserts`), `--no-default-features`,
`--no-default-features --features c-asserts`} × {dev, release profile}, and
`run_all.sh` enumerates it mechanically out of `Cargo.toml`. The harness picks
the C library that corresponds to the Rust feature set automatically
(`c_ref()`: the assert-enabled CMake build, or a `-DNDEBUG` build of the same
source).

## Group 1 — `unfilter` (the only entry point declared by `include/lib.h`)

Axes: `h` (≤0 / 1 / ≥2) × row-0 filter (0..4) × row-y filter (0..4) ×
`bpp` (0, 1..8, > `len`, negative) × `w` (0, 1, ≥2, negative) ×
`len = w*bpp` (0 / >0 / <0).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1  | `unfilter` | `h = 0`, random `w`/`bpp`/data — early-out, zero memory accesses | [x] |
| 2  | `unfilter` | `h < 0` (incl. `INT_MIN`), random `w`/`bpp`/data — early-out | [x] |
| 3  | `unfilter` | `h = 1`, row-0 filter `0` (None), `bpp ∈ 1..8`, `w ∈ 1..16` | [x] |
| 4  | `unfilter` | `h = 1`, row-0 filter `1` (Sub), `bpp < len` — loop `x = bpp..len` | [x] |
| 5  | `unfilter` | `h = 1`, row-0 filter `1`, `bpp >= len` — loop body never runs | [x] |
| 6  | `unfilter` | `h = 1`, row-0 filter `2` (Up) — quirk: **no-op** on row 0 | [x] |
| 7  | `unfilter` | `h = 1`, row-0 filter `3` (Average), `bpp < len` — `raw[x-bpp]/2` only | [x] |
| 8  | `unfilter` | `h = 1`, row-0 filter `4` (Paeth), `bpp < len` — `cp_paeth(a,0,0)` | [x] |
| 9  | `unfilter` | `h = 2`, row-0 filter × row-1 filter — full 5×5 cross product, random data | [x] |
| 10 | `unfilter` | `h ∈ 3..12`, **independently random filter byte per row** (the composed pipeline: every row's result feeds the next row's `prev[]`) | [x] |
| 11 | `unfilter` | `bpp = 1` (`len = w`) — `prev[x-bpp]` is the immediately preceding byte | [x] |
| 12 | `unfilter` | `bpp = 2, 3, 4` (the real PNG channel counts) × all filters × `h ∈ 1..8` | [x] |
| 13 | `unfilter` | `bpp = len` exactly (`w = 1`) — first loops cover the whole row, second loops empty | [x] |
| 14 | `unfilter` | `bpp > len` (`w = 1`, `bpp > w*bpp` impossible for `w=1`; reached with `w = 0` and `bpp > 0`, and with `w` negative) — `x` ends up `> len` after the first loop | [x] |
| 15 | `unfilter` | `bpp = 0` ⇒ `len = 0`: every loop is empty, but the filter byte is still consumed per row | [x] |
| 16 | `unfilter` | `w = 0` ⇒ `len = 0`, `bpp > 0`: rows are 1 byte each (filter byte only); `case 2/3/4` first loops still run `x = 0..bpp` and touch the *next* row's bytes | [x] |
| 17 | `unfilter` | `bpp < 0` (`len < 0` when `w > 0`): `x` starts negative ⇒ reads/writes **before** `raw` | [x] |
| 18 | `unfilter` | `w < 0`, `bpp > 0` ⇒ `len < 0`: all `x < len` loops empty, `x < bpp` loops still run | [x] |
| 19 | `unfilter` | `w < 0` **and** `bpp < 0` ⇒ `len > 0` with negative strides | [x] |
| 20 | `unfilter` | non-zero pointer offset into the scratch region (`raw` at offsets 0..7 — alignment must be irrelevant) | [x] |
| 21 | `unfilter` | large-ish shape sweep: `w ∈ 1..64`, `h ∈ 1..64`, `bpp ∈ 1..8`, random filters and data (value-dependent Paeth/Average paths) | [x] |
| 22 | `unfilter` | data patterns that stress `cp_paeth`'s three-way tie-breaks: all-`0x00`, all-`0xFF`, `0x80`, ramps, and random — every `(a,b,c)` branch of `(pa<=pb && pa<=pc) ? a : (pb<=pc) ? b : c` | [x] |

## Group 2 — `cp_inflate`, block-type / stream-shape axes

Axes: btype (0/1/2/3) × block count (1 / 2 / 3 mixed) × `in % 4` (0..3) ×
`(in_bytes - first_bytes) % 4` (0..3) × `out_bytes` (exact / larger) ×
symbol classes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 23 | `cp_inflate` | btype 1 (fixed Huffman), literals only, all literal values `0x00..0xFF` — covers the 8-bit (`0..143`) *and* 9-bit (`144..255`) code lengths of `cp_fixed_table` | [x] |
| 24 | `cp_inflate` | btype 1, empty block (EOB immediately), `out_bytes ≥ 0` | [x] |
| 25 | `cp_inflate` | btype 1, matches with `distance == 1` — the `memset(dst, *src, length)` fast path | [x] |
| 26 | `cp_inflate` | btype 1, matches with `distance > 1` and `distance >= length` — non-overlapping byte copy | [x] |
| 27 | `cp_inflate` | btype 1, matches with `1 < distance < length` — **overlapping** forward byte copy (the `while (length--) *dst++ = *src++;` default arm) | [x] |
| 28 | `cp_inflate` | btype 1, every length code `257..285` (`cp_len_base` 3..258, `cp_len_extra_bits` 0..5) with randomized extra bits | [x] |
| 29 | `cp_inflate` | btype 1, every distance code `0..29` (`cp_dist_base` 1..24577, `cp_dist_extra_bits` 0..13) with randomized extra bits, output buffer large enough for the distance | [x] |
| 30 | `cp_inflate` | btype 1, length codes `280..285` (the 8-bit fixed codes) and `257..279` (7-bit) mixed in one block | [x] |
| 31 | `cp_inflate` | btype 2 (dynamic), HLIT = 257 (minimum) .. 288 (maximum), HDIST = 1 .. 32, HCLEN = 4 .. 19 | [x] |
| 32 | `cp_inflate` | btype 2, code-length sequence built only from literal CL symbols `0..15` (no runs) | [x] |
| 33 | `cp_inflate` | btype 2, code-length sequence using CL symbol **16** (copy previous, 3..6 times) | [x] |
| 34 | `cp_inflate` | btype 2, code-length sequence using CL symbol **17** (3..10 zeroes) | [x] |
| 35 | `cp_inflate` | btype 2, code-length sequence using CL symbol **18** (11..138 zeroes) | [x] |
| 36 | `cp_inflate` | btype 2, single distance code (`ndst = 1`) and literal-only payload | [x] |
| 37 | `cp_inflate` | btype 2, code lengths up to 15 bits (deep tree ⇒ `cp_build`'s `len > 9` path, which skips the 9-bit `s->lookup` fill) | [x] |
| 38 | `cp_inflate` | btype 0 (stored), `LEN` == number of remaining input bytes (the only shape the inverted `bits_left/8 <= LEN` check accepts), `LEN ∈ 0..2051`, × all four `in` alignments. The *content* is additionally pinned when `(in_bytes - first_bytes) % 4 == 0`, i.e. when `cp_ptr`'s source pointer is exact — see the note at the end of `ERRORS.md` | [x] |
| 39 | `cp_inflate` | btype 0 as the **final** block preceded by a non-final btype 1 block — exercises the `cp_read_bits(s, s->count & 7)` re-alignment after a bit-packed block | [x] |
| 40 | `cp_inflate` | multi-block streams: `bfinal = 0` blocks of mixed types (1,2 and 1,1 and 2,2 and 2,1) followed by a final block; output accumulates across blocks and back-references reach into the previous block's output | [x] |
| 41 | `cp_inflate` | `in` misaligned by 0, 1, 2, 3 bytes ⇒ `first_bytes = 0..3`, the "load the leading bytes into `bits` by hand" path | [x] |
| 42 | `cp_inflate` | `(in_bytes - first_bytes) % 4 = 0, 1, 2, 3` ⇒ `last_bytes`/`final_word` path (incl. the quirk `count += bits_left`, which double-counts the already-buffered bits) | [x] |
| 43 | `cp_inflate` | `in_bytes` smaller than 4 (no full word at all — everything comes from `first_bytes`/`final_word`) | [x] |
| 44 | `cp_inflate` | `out_bytes` exactly equal to the decompressed size (tightest non-erroring case) | [x] |
| 45 | `cp_inflate` | `out_bytes` much larger than needed — the tail of the buffer must be left untouched | [x] |
| 46 | `cp_inflate` | realistic streams from a third-party DEFLATE encoder (`flate2`, `rust_backend`) over random / repetitive / text-like payloads of 0..8 KiB, levels 0 (stored), 1, 6, 9 | [x] |
| 47 | `cp_inflate` | same as 46 but with `in` copied to every 4-byte alignment and `in_bytes` covering all four `last_bytes` residues | [x] |

## Group 3 — the mutable exported tables as runtime options

`cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`,
`cp_dist_extra_bits` and `cp_dist_base` are all `D`-section (writable)
globals of identical size in both libraries; a caller can legally rewrite them
before calling `cp_inflate`, and the decoder reads them at run time.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 48 | data symbols | default contents of all six tables compared byte-for-byte between the two `.so`s | [x] |
| 49 | `cp_inflate` + `cp_fixed_table` | `cp_fixed_table` rewritten to a *different but still complete* code-length assignment in both libraries, then a btype 1 stream encoded against it | [x] |
| 50 | `cp_inflate` + `cp_permutation_order` | `cp_permutation_order` replaced by a random permutation **of `0..18`** in both libraries (with `HCLEN = 19` so every entry is used), then a btype 2 stream encoded against the new order. Entries `>= 19` are deliberately *not* tested: `lenlens[cp_permutation_order[i]]` would then write outside `uint8_t lenlens[19]` in the C build - see `ERRORS.md`, section "unavoidable divergences" | [x] |
| 51 | `cp_inflate` + `cp_len_base`/`cp_len_extra_bits` | length tables rewritten identically (shifted bases / different extra-bit counts), btype 1 stream with matches | [x] |
| 52 | `cp_inflate` + `cp_dist_base`/`cp_dist_extra_bits` | distance tables rewritten identically, btype 1 stream with matches | [x] |
| 53 | data symbols | after a full `cp_inflate` run the six tables must still be unmodified (the library must not write to them) | [x] |

## Group 4 — internal (`static`) entry points, driven through the exported ones

The low-level functions are `static`, so the *only* way an external caller can
reach them is through `cp_inflate`. These rows make sure each of them is
driven directly in every mode it has, rather than only incidentally:

| # | internal entry point | configuration reached through | [x] |
|---|----------------------|-------------------------------|-----|
| 54 | `cp_build(s, …)` (with `s != NULL`, fills `s->lookup`) | btype 1 (`cp_fixed`: lit table) and btype 2 (`cp_dynamic`: lit table) | [x] |
| 55 | `cp_build(0, …)` (with `s == NULL`, skips `s->lookup`) | btype 1 (dist table), btype 2 (CL table + dist table) | [x] |
| 56 | `cp_build` with code lengths ≤ 9 (lookup filled) and 10..15 (lookup skipped) | rows 32/37 | [x] |
| 57 | `cp_build` return value `first[15]` (excludes the count of 15-bit codes — a quirk) | row 37 (tree with 15-bit codes ⇒ `nlit` smaller than the real symbol count) | [x] |
| 58 | `cp_decode` binary search: `lo == hi == 0` (empty tree ⇒ `tree[-1]`, i.e. the `u32` in front of `dst[]` **inside `cp_state_t`**) | btype 2 with all 32 distance code lengths zeroed, then a match symbol. With the asserts live this aborts on row 27 of `ERRORS.md` — in *both* libraries, with the same diagnostic, which is what proves the struct layout and the sub-array pointer arithmetic agree | [x] |
| 59 | `cp_peak_bits` word path vs. final-word path vs. no-load path | rows 41, 42, 43 | [x] |
| 60 | `cp_consume_bits` with `num_bits_to_read = 0` (mask `(1<<0)-1 = 0`) | any length/distance code with 0 extra bits (rows 28, 29) | [x] |
| 61 | `cp_read_bits` with `n = 16` (`LEN`/`NLEN`) and `n = 1..7` | rows 38, 31 | [x] |
| 62 | `cp_rev16` | exercised for every code emitted in rows 23..47; additionally pinned by comparing the two libraries' `cp_build` outputs indirectly through `nlit`/`ndst`-dependent decoding | [x] |
| 63 | `cp_paeth` all three branches | row 22 | [x] |
| 64 | `cp_ptr` with `count/8 ∈ 0..4` | row 38/39 (stored blocks at different bit offsets) | [x] |

## Group 5 — randomized corrupt input (`tests/inflate_fuzz.rs`)

Not "configurations" in the option sense, but the same axes driven at random,
and the place where the interaction bugs actually live.  Every divergence is
checked against the undefined-behaviour oracle described in `ERRORS.md`.

| # | entry point(s) | configuration | [x] |
|---|----------------|---------------|-----|
| 65 | `cp_inflate` | 1200 short random byte strings × 5 `out_bytes` × 4 alignments | [x] |
| 66 | `cp_inflate` | 500 long (40..440 byte) random strings — full word loads, multi-block paths | [x] |
| 67 | `cp_inflate` | 2581 truncations of valid fixed / dynamic / stored streams | [x] |
| 68 | `cp_inflate` | 1200 single-bit mutations of valid fixed / dynamic / stored streams | [x] |
| 69 | `cp_inflate` | 400 random dynamic-block *headers* with random (often Kraft-incomplete) code-length vectors, `nlit ∈ 257..288`, `ndst ∈ 1..32`, max length 1..15, every CL run mode | [x] |
| 70 | `unfilter` | 4000 random `(w, h, bpp)` in `-6..40 × -2..20 × -6..10` with random filter bytes, 25 % of them *invalid* | [x] |
