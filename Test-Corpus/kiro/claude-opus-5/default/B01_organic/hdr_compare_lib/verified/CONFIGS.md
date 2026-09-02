# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

## Mechanical derivation of the axes

The public API (`c_src/include/lib.h`) is one entry point:

```c
int hdr_compare(const uint8_t *h1, const uint8_t *h2);
```

There is **no** runtime option, mode, flag, setter, context/handle struct, global
variable, byte-order switch, or `#ifdef` in the C source — so there are no
"options set" axes. The library is a pure function of the input bytes, and its
entire configuration surface is the **shape of the two 3-byte inputs**. `hdr_valid`
is `static` (not part of the ABI) but is the lowest-level unit of behaviour; it is
exercised *through* `hdr_compare`'s first term, which is the only way an external
caller can reach it, so the table drives the real entry point rather than a
convenience wrapper.

The axes are exactly the sub-expressions the C branches on:

| axis | source expression | distinct classes the C treats differently |
|------|-------------------|-------------------------------------------|
| A. `h2` sync byte | `h2[0] == 0xff` | `0xff` / anything else |
| B. `h2[1]` class | `(h2[1] & 0xF0) == 0xf0` vs `(h2[1] & 0xFE) == 0xe2` | high-nibble class `[0xf0,0xff]` (16) / MPEG-2.5 class `{0xe2,0xe3}` (2) / neither (238) |
| C. `h2[1]` layer field | `((h2[1] >> 1) & 3)` | `0` (reserved → reject) / `1` / `2` / `3` |
| D. `h2[2]` bitrate nibble | `(h2[2] >> 4)` | `15` (reject) / `0` (free format) / `1..14` |
| E. `h2[2]` samplerate field | `((h2[2] >> 2) & 3)` | `3` (reserved → reject) / `0` / `1` / `2` |
| F. `h1[1]` vs `h2[1]` | `(h1[1] ^ h2[1]) & 0xFE` | equal-except-bit-0 (bit 0 same / bit 0 flipped) / differing in ≥1 masked bit |
| G. `h1[2]` vs `h2[2]` samplerate bits | `(h1[2] ^ h2[2]) & 0x0C` | equal / differing |
| H. free-format nibble agreement | `((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0)` | both zero / both non-zero / exactly one zero |
| I. pointer shape | which bytes are dereferenced | distinct buffers / aliased (`h1 == h2`) / `h1` unreadable while `h2` invalid (short-circuit) / `h1[0]` arbitrary (never read) |

Derived constants (computed from the predicates, used to size the tests):
**14** of 256 `h2[1]` values pass axes B+C — `{e2,e3,f2,f3,f4,f5,f6,f7,fa,fb,fc,fd,fe,ff}`;
**180** of 256 `h2[2]` values pass axes D+E; so **2 520** of 65 536 `h2` tails are
valid, and for each there are exactly **2** accepting `h1[1]` values and
**16** accepting `h1[2]` values.

## Configuration table

Each row is a combination of the axes above that the C distinguishes. Every row is
driven through the `hdr_compare` export of **both** `.so` files with **many
randomized inputs** (fixed seed, SplitMix64) plus the row's boundary values, and
the returned `int`s must be byte-identical. Rows 1–4 are the exhaustive sweeps
that subsume the cross-product; rows 5–24 are the targeted combinations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `hdr_compare` | **Exhaustive** over the whole meaningful space: all 2^32 combinations of `(h1[1], h1[2], h2[1], h2[2])` with `h2[0] = 0xff`, split into 256 sharded sub-sweeps | [x] |
| 2 | `hdr_compare` | **Exhaustive** over all 256 values of `h2[0]` × all 65 536 `h2[1..3]` tails, with `h1` tail both matching and mismatching (axis A × B × C × D × E) | [x] |
| 3 | `hdr_compare` | **Exhaustive** over all 2^16 `(h1[1], h1[2])` for every one of the 2 520 valid `h2` tails (axes F × G × H under every accepting `h2`) | [x] |
| 4 | `hdr_compare` | **Exhaustive** over all 2^16 `h2` tails for a representative set of `h1` tails, `h2[0] = 0xff` (rejection classes B/C/D/E in full) | [x] |
| 5 | `hdr_compare` | Fully random 5 relevant bytes, uniform (mostly-rejecting mass), 20 M randomized draws | [x] |
| 6 | `hdr_compare` | Random but *biased to accept*: `h2[0]=0xff`, `h2[1]` drawn from the 14 valid values, `h2[2]` from the 180 valid values, `h1` tail drawn near-matching | [x] |
| 7 | `hdr_compare` | Axis A: `h2[0] = 0xff` (sync ok) with an otherwise perfectly matching valid header → accept | [x] |
| 8 | `hdr_compare` | Axis A: all 255 non-`0xff` `h2[0]` values with an otherwise perfectly matching valid header → reject | [x] |
| 9 | `hdr_compare` | Axis B class 1: `h2[1] ∈ [0xf0,0xff]` (all 16), each × all 256 `h2[2]` × matching `h1` | [x] |
| 10 | `hdr_compare` | Axis B class 2: `h2[1] ∈ {0xe2,0xe3}` (MPEG-2.5 branch, reached only via the `& 0xFE` test) × all 256 `h2[2]` × matching `h1` | [x] |
| 11 | `hdr_compare` | Axis B class 3: `h2[1]` in neither class (all 238 values) × valid `h2[2]` × matching `h1` | [x] |
| 12 | `hdr_compare` | Axis C: `((h2[1]>>1)&3)` = 1, 2, 3 in turn, `h2[1]` in both B classes where possible | [x] |
| 13 | `hdr_compare` | Axis C boundary: `((h2[1]>>1)&3) == 0`, i.e. `h2[1] ∈ {0xf0,0xf1,0xf8,0xf9}` — passes axis B but fails C | [x] |
| 14 | `hdr_compare` | Axis D: `(h2[2]>>4) == 0` (free format) on **both** headers, all 16 low-nibble values each | [x] |
| 15 | `hdr_compare` | Axis D: `(h2[2]>>4)` = 1…14, all 14 values × all 16 low nibbles × matching `h1[2]` | [x] |
| 16 | `hdr_compare` | Axis D boundary: `(h2[2]>>4) == 15`, all 16 values `0xf0..0xff` | [x] |
| 17 | `hdr_compare` | Axis E: `((h2[2]>>2)&3)` = 0, 1, 2 with matching and mismatching `h1[2]` bits 2–3 | [x] |
| 18 | `hdr_compare` | Axis E boundary: `((h2[2]>>2)&3) == 3`, all 64 such `h2[2]` values (incl. those where axis D also fires) | [x] |
| 19 | `hdr_compare` | Axis F: `h1[1] == h2[1]` and `h1[1] == h2[1] ^ 0x01` (bit 0 is the ignored padding bit) — the 2 accepting values, for all 14 valid `h2[1]` | [x] |
| 20 | `hdr_compare` | Axis F: `h1[1] = h2[1] ^ (1 << k)` for k = 1…7 (each masked bit flipped individually) | [x] |
| 21 | `hdr_compare` | Axis G: `h1[2] = h2[2] ^ (1<<2)`, `^(1<<3)`, `^(0x0C)` — the samplerate-bit mismatches | [x] |
| 22 | `hdr_compare` | Axis H: all four (h1 nibble zero?, h2 nibble zero?) combinations, incl. the two "exactly one zero" rejects, with axes F/G held passing | [x] |
| 23 | `hdr_compare` | Axis I: aliased pointers `h1 == h2` across all 65 536 tails and all 256 `h2[0]` | [x] |
| 24 | `hdr_compare` | Axis I: `h1[0]` swept over all 256 values (never read by the C) and `h1` placed at an unmapped, non-null address with `h2` invalid (short-circuit ⇒ no read past `h1[0]`) | [x] |
| 25 | `hdr_compare` | **Exhaustive** closure of the gap left by rows 1–2: all 256 `h2[0]` × all 65 536 `h1` tails × 16 `h2` tails, one per acceptance/rejection class (268 M pairs) | [x] |
| 26 | `hdr_compare` | **Exhaustive** deep sweep for 5 representative bad sync bytes: all 65 536 `h2` tails × all 256 `h1[1]` values with `h1[2]` cycling (84 M pairs), asserting the C's verdict is `0` throughout | [x] |

## Why rows 1 + 2 + 24 + 25 + 26 are jointly exhaustive

`hdr_compare`'s result is a function of at most five bytes: `h2[0]`, `h2[1]`,
`h2[2]`, `h1[1]`, `h1[2]` (`h1[0]` appears in no term of the C). Row 13 of
`ERRORS.md` proves with a `PROT_NONE` guard page that neither implementation
reads any further byte, and row 24 proves neither reads `h1[0]`. Within that
5-byte space:

* row 1 covers **all 2^32** combinations of the four bytes that matter when
  `h2[0] == 0xff` — i.e. the entire accepting region and every rejection reachable
  with a good sync byte, with nothing sampled;
* rows 2, 25 and 26 cover the `h2[0] != 0xff` region from all three directions
  (every `h2[0]` × every `h2` tail, every `h2[0]` × every `h1` tail, and every
  `h2` tail × a broad `h1` spread for representative bad sync bytes).

So the only inputs not literally enumerated are `(bad h2[0], arbitrary h2 tail,
arbitrary h1 tail)` triples, and those are covered on two of their three axes
simultaneously by rows 25/26 with the C's verdict asserted to be `0`.

## Feature combinations

`Cargo.toml` has no `[features]` table, so `--no-default-features` and the default
build are the same single configuration; it is verified explicitly (see
`VERIFICATION.md`).

## Test mapping

| rows | test file | test names |
|------|-----------|------------|
| 1 | `tests/phase_b_exhaustive.rs` | `cfg_row01_exhaustive_shard00` … `shard15` |
| 2–4, 25, 26 | `tests/phase_b_exhaustive.rs` | `cfg_row02_…`, `cfg_row03_…`, `cfg_row04_…`, `cfg_row25_…`, `cfg_row26_…` |
| 5–24 | `tests/phase_b_configs.rs` | `cfg_row05_…` … `cfg_row24_…` |
