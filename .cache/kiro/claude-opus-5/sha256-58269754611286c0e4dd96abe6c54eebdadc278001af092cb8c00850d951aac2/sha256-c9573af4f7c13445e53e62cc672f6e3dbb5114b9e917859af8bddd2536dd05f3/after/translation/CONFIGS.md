# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the branch points in `c_src/src/lib.c`.

`c_src/src/lib.c` has no `#ifdef`s, but the C build *does* have two compile-time
configurations: with and without `NDEBUG`, which decides whether `assert()` is
compiled in. The reference build (the command given in the task) has asserts
live. The Rust crate mirrors this with a single feature, `c_asserts`, on by
default — so the complete feature-combination set is:

| combination | Rust build | matching C build |
|-------------|------------|------------------|
| `default` (`c_asserts`) | `cargo build --release` | `c_src/build` (no `NDEBUG`) |
| none | `cargo build --release --no-default-features` | `c_ndebug_build` (`-DNDEBUG`) |

`run_all.sh` builds both C variants, both Rust variants, and runs every row
below against each matching pair. Everything else in this table is *runtime*
state: the values the two exported functions branch on, plus the seven exported
**writable** data objects the algorithm reads at run time.

## Axes actually branched on by the C

### `cp_inflate(void *in, int in_bytes, void *out, int out_bytes)`

| axis | values the C distinguishes | C site |
|------|----------------------------|--------|
| `in` pointer alignment mod 4 | 0, 1, 2, 3 → `first_bytes`, which bytes are pre-loaded into `s->bits`, and where `s->words` starts | `cp_inflate` L310-311 |
| `(in_bytes - first_bytes) & 3` | 0, 1, 2, 3 → `last_bytes`, `final_word_available`, `final_word` | L313, L318-321 |
| `word_count` | 0 (all input in first/final bytes) vs ≥1 (word refill path in `cp_peak_bits`) | L312, L95-99 |
| `btype` | 0 `cp_stored`, 1 `cp_fixed`, 2 `cp_dynamic`, 3 error | L332-361 |
| `bfinal` | 0 → loop again; 1 → stop. 1 block vs N blocks, mixed types | L329, L366 |
| `out_bytes` | exact-fit vs slack (drives `out_end` checks E3/E5) | L325 |

### `cp_stored` (btype 0)

| axis | values | C site |
|------|--------|--------|
| pre-alignment discard | `s->count & 7` ∈ 0..7 | L167 |
| `LEN` | 0, 1, 2, 3, 4, large; `LEN` vs remaining `bits_left/8` | L168-188 |

### `cp_fixed` / `cp_block` (btype 1)

| axis | values | C site |
|------|--------|--------|
| symbol class | `<256` literal, `==256` end-of-block, `>256` length | L250-303 |
| literal value | 0..143 (8-bit code) vs 144..255 (9-bit code) — different `cp_fixed_table` lengths | `cp_fixed_table` |
| length symbol | 257..264 (0 extra bits), 265..284 (1..5 extra bits), 285 (`len_base` 258, 0 extra) | `cp_len_extra_bits`/`cp_len_base` |
| distance symbol | 0..3 (0 extra bits), 4..29 (1..13 extra bits) | `cp_dist_extra_bits`/`cp_dist_base` |
| `backwards_distance` | `== 1` → `memset` path; `!= 1` → byte-copy loop | L293-301 |
| overlap | `distance < length` (self-overlapping copy) vs `distance >= length` | L299-300 |

### `cp_dynamic` / `cp_build` (btype 2)

| axis | values | C site |
|------|--------|--------|
| `nlit` | 257 (`HLIT=0`) .. 288 (`HLIT=31`) | L216 |
| `ndst` | 1 (`HDIST=0`) .. 32 (`HDIST=31`) | L217 |
| `nlen` | 4 (`HCLEN=0`) .. 19 (`HCLEN=15`) | L218 |
| code-length symbol | 0..15 literal, 16 (copy prev 3-6), 17 (zeros 3-10), 18 (zeros 11-138) | L224-245 |
| code bit length | ≤9 → `cp_build` also fills `s->lookup`; 10..15 → tree only | L149-156 |
| distance-tree size | `ndst == 1` (single/degenerate distance code) vs many | L246 |

### `convert_pix(int bpp, int w, int h, uint8_t *src, cp_pixel_t *dst)`

| axis | values | C site |
|------|--------|--------|
| `bpp` | 1 (gray), 2 (gray+alpha), 3 (rgb), 4 (rgba), anything else → no store | L477-491 |
| `w` | ≤0, 1, 2, many | L476 |
| `h` | ≤0, 1, 2, many | L474 |

### Exported writable data objects (a real configuration axis)

`cp_fixed_table`, `cp_permutation_order`, `cp_len_extra_bits`, `cp_len_base`,
`cp_dist_extra_bits`, `cp_dist_base` are non-`const` globals that the decoder
reads on every call, and `cp_error_reason` is written on every failure. A caller
can mutate them, which changes the decode. Their initial bytes must also match.

## Row table

Each row is checked by a differential test that drives **both** `.so`s through
their exported symbols with many randomized inputs (fixed seed
`0x9E3779B97F4A7C15`), asserting byte-identical `out` buffers, identical return
codes, and identical `cp_error_reason`. Rows C01-C06 live in
`tests/phase_b_tables.rs` (their own process, since they mutate globals of the
shared `.so`), C39 in `tests/phase_b_zlib.rs`, everything else in
`tests/phase_b_valid.rs`. All rows pass under **both** feature combinations.

Where a stream is *valid* the tests additionally self-check the C's output
against an independent reference expansion of the program that was encoded, so a
passing row means "C and Rust agree **and** the C really decompressed
correctly". Two documented exceptions, where the C deviates from a reference
inflate and only the C-vs-Rust comparison is meaningful:

* a stored block is only accepted as the last thing in a stream (its
  `bits_left / 8 <= LEN` check), so rows C15/C16 and part of C38 are *rejection*
  comparisons;
* `cp_stored` recovers the payload address with `cp_ptr()`, which points into the
  input buffer and ignores bytes that live only in `s->bits` / `s->final_word`,
  so a stored payload inside the final partial word is copied as zeros. Row C39
  reports how many zlib records this affects (12 of 1150).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| **Data symbols** | | | |
| C01 | data symbols | initial contents of all 6 tables byte-compared (`cp_fixed_table` 320 B, `cp_permutation_order` 19 B, `cp_len_extra_bits` 31 B, `cp_len_base` 31×u32, `cp_dist_extra_bits` 32 B, `cp_dist_base` 32×u32) | [x] |
| C02 | data symbols | `cp_error_reason` is NULL/unwritten before the first call in both | [x] |
| C03 | `cp_inflate` + `cp_permutation_order` | mutate `cp_permutation_order` identically in both libs (rotated order), decode a dynamic block encoded with the rotated order | [x] |
| C04 | `cp_inflate` + `cp_len_base`/`cp_len_extra_bits` | mutate the length tables identically in both, decode a fixed block containing length codes | [x] |
| C05 | `cp_inflate` + `cp_dist_base`/`cp_dist_extra_bits` | mutate the distance tables identically in both, decode a fixed block containing matches | [x] |
| C06 | `cp_inflate` + `cp_fixed_table` | mutate `cp_fixed_table` identically in both (swap two length groups, all lengths still <16) and decode a stream re-encoded with the mutated fixed code | [x] |
| **`convert_pix`** | | | |
| C07 | `convert_pix` | `bpp=1`, `(w,h)` ∈ {(1,1),(2,1),(1,2),(7,5),(64,33)}, random `src` | [x] |
| C08 | `convert_pix` | `bpp=2`, same `(w,h)` set, random `src` | [x] |
| C09 | `convert_pix` | `bpp=3`, same `(w,h)` set, random `src` | [x] |
| C10 | `convert_pix` | `bpp=4`, same `(w,h)` set, random `src` | [x] |
| C11 | `convert_pix` | `bpp` ∈ {1,2,3,4} × `w` ∈ {1..8} × `h` ∈ {1..8}, full cross product, random `src` | [x] |
| C12 | `convert_pix` | `bpp` ∈ {1,2,3,4}, `h=0` and `w=0` (dst must stay untouched) | [x] |
| **`cp_inflate` — stored blocks (btype 0)** | | | |
| C13 | `cp_inflate` | single final stored block, `LEN` ∈ {0,1,2,3,4,5,7,8,15,16,17,255,256,1024}, random payload, `in` alignment 0 | [x] |
| C14 | `cp_inflate` | single final stored block, random `LEN`, `in` alignment ∈ {0,1,2,3} (cross product with `in_bytes & 3` ∈ {0,1,2,3}) | [x] |
| C15 | `cp_inflate` | two stored blocks (`bfinal=0` then `1`), random LENs | [x] |
| C16 | `cp_inflate` | N∈{3..6} stored blocks, random LENs, random alignment | [x] |
| **`cp_inflate` — fixed blocks (btype 1)** | | | |
| C17 | `cp_inflate` | fixed block, literals only, all in 0..143 (8-bit codes), random length 0..300 | [x] |
| C18 | `cp_inflate` | fixed block, literals only, all in 144..255 (9-bit codes) | [x] |
| C19 | `cp_inflate` | fixed block, literals only, random full-range bytes (mixes 8- and 9-bit codes) | [x] |
| C20 | `cp_inflate` | fixed block, one match with `dist == 1` (memset path), `len` ∈ {3,4,5,10,258} | [x] |
| C21 | `cp_inflate` | fixed block, one match with `dist > 1`, non-overlapping (`dist >= len`) | [x] |
| C22 | `cp_inflate` | fixed block, one match with `dist > 1`, overlapping (`1 < dist < len`) | [x] |
| C23 | `cp_inflate` | fixed block, length symbols sweeping all of 257..285 (every extra-bit width 0..5, incl. `len==258`) | [x] |
| C24 | `cp_inflate` | fixed block, distance symbols sweeping all of 0..29 (every extra-bit width 0..13), with a large enough already-emitted prefix | [x] |
| C25 | `cp_inflate` | fixed block, random mixed literal/match program, random `in` alignment ∈ {0..3} | [x] |
| C26 | `cp_inflate` | multiple fixed blocks (2..5), `bfinal` only on the last, matches crossing block boundaries | [x] |
| **`cp_inflate` — dynamic blocks (btype 2)** | | | |
| C27 | `cp_inflate` | dynamic block, code lengths emitted literally (no 16/17/18), `nlit=257`, `ndst=1`, `nlen=19` | [x] |
| C28 | `cp_inflate` | dynamic block, `nlen` swept 4..19 (`HCLEN` 0..15) | [x] |
| C29 | `cp_inflate` | dynamic block, `nlit` swept 257..288, `ndst` swept 1..32 (random pairs) | [x] |
| C30 | `cp_inflate` | dynamic block using code-length symbol **16** (repeat previous 3..6), never at `n==0` (avoids U1) | [x] |
| C31 | `cp_inflate` | dynamic block using code-length symbol **17** (3..10 zeros) | [x] |
| C32 | `cp_inflate` | dynamic block using code-length symbol **18** (11..138 zeros) | [x] |
| C33 | `cp_inflate` | dynamic block whose literal code has max length **≤9** (all symbols hit `s->lookup`) | [x] |
| C34 | `cp_inflate` | dynamic block whose literal code has lengths **10..15** (tree-only path in `cp_build`) | [x] |
| C35 | `cp_inflate` | dynamic block with `ndst == 1` (degenerate single distance code) and matches | [x] |
| C36 | `cp_inflate` | dynamic block with matches: `dist==1`, `dist>1` non-overlapping, and overlapping | [x] |
| C37 | `cp_inflate` | dynamic block, random alphabet + random program, `in` alignment ∈ {0..3} × `in_bytes&3` ∈ {0..3} | [x] |
| C38 | `cp_inflate` | mixed multi-block stream: stored → fixed → dynamic → stored (final), random payloads | [x] |
| C39 | `cp_inflate` | real zlib-produced raw-deflate streams (`python3 zlib`, `wbits=-15`, levels 0..9) over random and highly-repetitive payloads | [x] |
| C40 | `cp_inflate` | `out_bytes` exactly equal to the decompressed size (boundary of E3/E5) vs `out_bytes` with slack, for the C17/C25/C37 programs | [x] |
