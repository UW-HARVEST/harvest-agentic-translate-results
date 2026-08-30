# CONFIGS.md — configuration-surface table (valid inputs)

## Axes the C actually branches on

**Public entry points** (2): `cp_inflate(void*,int,void*,int)` — the low-level
one — and `load_png_mem(const uint8_t*,int)` — the wrapper. Both are tested
directly through `dlsym`.

**Runtime "options"**: the library has no option struct; its only mutable
configuration is the seven exported globals. Six are lookup tables that the
decoder re-reads on *every* call, so a consumer can retune them:
`cp_fixed_table` (both halves), `cp_permutation_order`, `cp_len_extra_bits`,
`cp_len_base`, `cp_dist_extra_bits`, `cp_dist_base`. The seventh,
`cp_error_reason`, is an output.

**`cp_inflate` input shapes** (`if`/`switch` branches in
`cp_inflate`/`cp_peak_bits`/`cp_stored`/`cp_block`):
`in % 4` ∈ {0,1,2,3} (`first_bytes`), `(in_bytes-first_bytes) % 4` ∈ {0,1,2,3}
(`final_word_available`), `btype` ∈ {0,1,2}, `bfinal` single vs. multi-block,
symbol `<256` / `>256` / `==256`, `backwards_distance == 1` (`memset`) vs. `> 1`
(byte loop), length/distance symbols with 0 vs. >0 extra bits, `out_bytes`
exactly-fits vs. slack.

**`load_png_mem` input shapes**: colour type ∈ {0,2,3,4,6} → `bpp` ∈
{1,3,1,2,4}; `PLTE` present/absent; `tRNS` present/absent and shorter/longer
than the palette; filter byte ∈ {0,1,2,3,4} on row 0 (where 2 is a no-op and
1/3/4 start at `x = bpp`) and on rows ≥ 1 (different code); `w` = 1 / small /
`w*bpp` crossing several words; `h` = 1 / many; 1 vs. many vs. empty `IDAT`
chunks; ancillary chunks before/between/after; `cp_chunk` vs. `cp_find` walking
(`IHDR` uses `cp_chunk`, `PLTE`/`tRNS`/first `IDAT` use `cp_find`, subsequent
`IDAT`s use `cp_chunk`).

Cargo features: `translation/Cargo.toml` declares **no `[features]`**, so there
is exactly one feature combination (`default` == no features). Phase D still
runs `--no-default-features` to prove it.

## Rows

Every row is exercised with many randomized inputs (fixed seed) in
`tests/phase_b_inflate.rs` / `tests/phase_b_png.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `cp_inflate` | stored block (`btype=0`), `bfinal=1`, `LEN` == exactly the bytes remaining, `in%4=0`, random payload/length | [x] |
| 2 | `cp_inflate` | stored block, `in%4` ∈ {1,2,3} (non-zero `first_bytes`) | [x] |
| 3 | `cp_inflate` | stored block, `(in_bytes-first_bytes)%4` ∈ {1,2,3} (`final_word_available=1`) | [x] |
| 4 | `cp_inflate` | fixed block (`btype=1`), literals only, all 4 `in%4`, `out_bytes` exact | [x] |
| 5 | `cp_inflate` | fixed block, literals only, `out_bytes` with slack | [x] |
| 6 | `cp_inflate` | fixed block, back-references with `backwards_distance == 1` (`memset` path) | [x] |
| 7 | `cp_inflate` | fixed block, back-references with `backwards_distance > 1` (byte-copy path), overlapping and non-overlapping | [x] |
| 8 | `cp_inflate` | fixed block, length symbols with extra bits (265–284) and 258-length symbol 285 | [x] |
| 9 | `cp_inflate` | fixed block, distance symbols with extra bits (4–29) | [x] |
| 10 | `cp_inflate` | fixed block, symbols from the 9-bit half of the fixed literal table (144–255) and the 8-bit tail (280–287) | [x] |
| 11 | `cp_inflate` | dynamic block (`btype=2`), random complete code-length alphabet, literals only | [x] |
| 12 | `cp_inflate` | dynamic block using run-length code-length symbols 16, 17 and 18 | [x] |
| 13 | `cp_inflate` | dynamic block, `nlit`/`ndst`/`nlen` at their extremes (257/1/4 and 288/32/19) | [x] |
| 14 | `cp_inflate` | dynamic block with back-references (exercises `s->dst` built by `cp_build(0,…)`) | [x] |
| 15 | `cp_inflate` | multi-block stream: several `bfinal=0` blocks then a final one, mixing `btype` 1 and 2 | [x] |
| 16 | `cp_inflate` | multi-block stream ending with a stored block (the only position `cp_stored` accepts) | [x] |
| 17 | `cp_inflate` | after mutating `cp_len_base` / `cp_len_extra_bits` (retuned length table) | [x] |
| 18 | `cp_inflate` | after mutating `cp_dist_base` / `cp_dist_extra_bits` | [x] |
| 19 | `cp_inflate` | after mutating `cp_fixed_table` (still all lengths ≤ 15) — changes `btype=1` decoding | [x] |
| 20 | `cp_inflate` | after mutating `cp_permutation_order` (permuted code-length order) — changes `btype=2` decoding | [x] |
| 21 | `load_png_mem` | colour type 0 (grey, bpp 1), filter 0 everywhere, random `w`×`h` | [x] |
| 22 | `load_png_mem` | colour type 2 (RGB, bpp 3), filter 0 | [x] |
| 23 | `load_png_mem` | colour type 4 (grey+alpha, bpp 2), filter 0 | [x] |
| 24 | `load_png_mem` | colour type 6 (RGBA, bpp 4), filter 0 | [x] |
| 25 | `load_png_mem` | colour type 3 (indexed, bpp 1) + `PLTE`, no `tRNS` | [x] |
| 26 | `load_png_mem` | colour type 3 + `PLTE` + `tRNS` shorter than the palette (`cp_get_alpha_for_indexed_image` both branches) | [x] |
| 27 | `load_png_mem` | colour type 3 + `PLTE` + `tRNS` covering the whole palette | [x] |
| 28 | `load_png_mem` | colour type 3 with indices past the end of `PLTE` (C reads out of the chunk — buffer over-allocated so both read the same bytes) | [x] |
| 29 | `load_png_mem` | every colour type × row-0 filter ∈ {0,1,2,3,4} | [x] |
| 30 | `load_png_mem` | every colour type × rows ≥ 1 filter ∈ {0,1,2,3,4}, randomized per row | [x] |
| 31 | `load_png_mem` | `h == 1` (the `for y in 1..h` loop never runs) | [x] |
| 32 | `load_png_mem` | `w == 1` (`len == bpp`, so the `x = bpp; x < len` loops never run) | [x] |
| 33 | `load_png_mem` | 1×1 image, every colour type | [x] |
| 34 | `load_png_mem` | wide image (`w*bpp` spanning many 4-byte words), `h` small | [x] |
| 35 | `load_png_mem` | tall image (`h` large, `w` small) | [x] |
| 36 | `load_png_mem` | IDAT split across 2..5 chunks (first via `cp_find`, rest via `cp_chunk`) | [x] |
| 37 | `load_png_mem` | zero-length IDAT chunk mixed in with non-empty ones | [x] |
| 38 | `load_png_mem` | ancillary chunks (`gAMA`, `pHYs`, `tEXt`, `bKGD`) before `PLTE`, between `PLTE` and `IDAT`, and after `IDAT` | [x] |
| 39 | `load_png_mem` | `PLTE` present on a non-indexed colour type (found but unused) | [x] |
| 40 | `load_png_mem` | `tRNS` present on a non-indexed colour type | [x] |
| 41 | `load_png_mem` | `tRNS` appearing *before* `PLTE` (`cp_find` restarts from `first`, so `PLTE` is found and `tRNS` is then searched from after it) | [x] |
| 42 | `load_png_mem` | `IHDR` chunk longer than 13 bytes (`minlen` satisfied, `len+12` skip) | [x] |
| 43 | `load_png_mem` | zlib header CINFO 0..7 × FCHECK/FLEVEL bits (`data[0] & 0xf0 <= 0x70`, `data[1] & 0x20 == 0`) | [x] |
| 44 | `load_png_mem` | DEFLATE payload as a single stored block | [x] |
| 45 | `load_png_mem` | DEFLATE payload as fixed-Huffman blocks with back-references | [x] |
| 46 | `load_png_mem` | DEFLATE payload as dynamic-Huffman blocks | [x] |
| 47 | `load_png_mem` | DEFLATE payload as several blocks (mixed `btype`) | [x] |
| 48 | `load_png_mem` | inflated stream stops well before `out_end` (always: the raw block is `h*(1+w*bpp)` bytes while `out_bytes` is `(w+1)*h*4`), plus streams that write 1..`h*(bpp-1)` extra bytes into the slack that is still inside the `img.pix` allocation | [x] |
| 49 | `load_png_mem` | `pix_bytes` vs `cp_out_size(bpp)` offset (`out` starts inside `img.pix`, not at its base) for bpp 1/2/3 | [x] |
| 50 | `load_png_mem` | repeated calls on the same buffer, and interleaved calls between the two libraries (no hidden state) | [x] |
| 51 | `load_png_mem` | with each of the six tables mutated (rows 17–20 applied through the PNG wrapper) | [x] |
| 52 | `load_png_mem` + `cp_inflate` | `cp_error_reason` read after a *successful* call (stale value is preserved) | [x] |


## Verified

Every row above is exercised by `tests/phase_b_inflate.rs` (rows 1-20) and
`tests/phase_b_png.rs` (rows 21-52) with randomized inputs from a fixed seed,
and `tests/fuzz.rs` adds ~3000 mutated inputs on top. All rows pass; the `[x]`
marks are the result, not an intention.

## Deliberately excluded (the C is nondeterministic here)

Two input classes make the reference C library disagree *with itself* between
runs, so they are not compared (see the end of `README.md`):

* **Heap overflows.** `out_end` inside `load_png_mem` is `(w+1)*h*(4-bpp)` bytes
  past the end of the `img.pix` allocation, so a DEFLATE stream that produces
  more than `(w+1)*h*bpp` bytes corrupts the heap; and `cp_stored`'s `memcpy`
  ignores `out_end` entirely. Whether glibc aborts depends on the heap layout.
* **Reads of uninitialised `malloc` memory.** A stream producing *fewer* than
  `h*(1 + w*bpp)` bytes leaves part of the scanline block unwritten, which
  `cp_unfilter` and `cp_convert` then read.

`tests/fuzz.rs` filters both automatically by running the C corpus twice and
only comparing the cases the C reproduced.
