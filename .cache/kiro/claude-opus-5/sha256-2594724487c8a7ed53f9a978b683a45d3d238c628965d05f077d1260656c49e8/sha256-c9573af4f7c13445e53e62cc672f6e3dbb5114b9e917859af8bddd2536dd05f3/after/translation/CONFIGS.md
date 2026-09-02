# CONFIGS.md — configuration-surface table

Derived mechanically from the branch/index structure of `c_src/src/lib.c`.

## Public entry points (full set, lowest level included)

`c_src/include/lib.h` declares exactly one function and it *is* the lowest-level
entry point — there are no convenience wrappers, no init/teardown, no opaque
context object and no setter functions:

```c
unsigned hdr_bitrate(const uint8_t *h);
```

So the entry-point axis has a single value, and every row below drives that
function directly through the `.so` export.

## Axes the C actually distinguishes

The body contains no `if` / `switch` / `#ifdef`, so there are no runtime
option/mode/flag axes. All branching is *data* branching through three index
expressions:

| axis | expression in C | distinct values | effect |
|------|-----------------|-----------------|--------|
| A. version bit | `!!((h[1]) & 0x8)` → `i` | 2 (`0`, `1`) | selects `halfrate[i]`, i.e. flat offset `+0` or `+45` |
| B. layer field | `(((h[1]) >> 1) & 3) - 1` → `j` | 4 (`-1`, `0`, `1`, `2`) | selects the row, `+15*j`; value `-1` (layer field `0b00`) escapes the declared bounds |
| C. bitrate nibble | `((h[2]) >> 4)` → `k` | 16 (`0..15`) | byte within the row; value `15` escapes the declared 15-byte row |
| D. ignored input bits | — | — | `h[0]`, `h[1] & 0x01`, `h[1] & 0xF0`, `h[2] & 0x0F`, and `h[3]`+ are **never read**; changing them must not change the result |
| E. buffer shape | — | — | no length parameter; C touches exactly `h[1]` and `h[2]`, so buffer length ≥ 3, pointer alignment, and offset-into-a-larger-buffer must all be irrelevant |

Pruning C: the code treats `k` differently for every value, but three classes
carry distinct *meaning* and distinct in/out-of-bounds behaviour: `k = 0`
(the "free" bitrate slot, always table value `0`), `k = 1..14` (in-range table
entries), `k = 15` (the "bad" index, one past the row). Rows are the cross
product A × B × {k=0, k∈1..14, k=15} = 2 × 4 × 3 = 24, plus rows for axes D and E.

Each row is exercised with **many randomized inputs** (fixed seed
`0x5DEECE66D`, SplitMix64) over the free bits of that configuration — the
ignored bits of `h[1]`/`h[2]`, all of `h[0]`/`h[3..]`, buffer length, and
buffer offset — and over the full `k` sub-range where the row spans one.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the complete
feature-combination set is the single default configuration (equivalently
`--no-default-features`, which is identical here). Both are run; see the
`run_all.sh` output recorded in the completion gate.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `hdr_bitrate` | A: `i=0` (`h[1]&0x8 == 0`); B: layer field `0b00` ⇒ `j=-1`; C: `k=0` ⇒ flat offset `-15`, read before the table | [x] |
| C02 | `hdr_bitrate` | A: `i=0`; B: `j=-1`; C: `k∈1..14` ⇒ flat offsets `-14..-1`, all before the table | [x] |
| C03 | `hdr_bitrate` | A: `i=0`; B: `j=-1`; C: `k=15` ⇒ flat offset `0`, aliases `halfrate[0][0][0]` | [x] |
| C04 | `hdr_bitrate` | A: `i=0`; B: layer field `0b01` ⇒ `j=0`; C: `k=0` ⇒ `halfrate[0][0][0]` | [x] |
| C05 | `hdr_bitrate` | A: `i=0`; B: `j=0`; C: `k∈1..14` ⇒ `halfrate[0][0][1..14]` (MPEG-2/2.5 Layer III row) | [x] |
| C06 | `hdr_bitrate` | A: `i=0`; B: `j=0`; C: `k=15` ⇒ flat offset `15`, aliases `halfrate[0][1][0]` | [x] |
| C07 | `hdr_bitrate` | A: `i=0`; B: layer field `0b10` ⇒ `j=1`; C: `k=0` ⇒ `halfrate[0][1][0]` | [x] |
| C08 | `hdr_bitrate` | A: `i=0`; B: `j=1`; C: `k∈1..14` ⇒ `halfrate[0][1][1..14]` (MPEG-2/2.5 Layer II row) | [x] |
| C09 | `hdr_bitrate` | A: `i=0`; B: `j=1`; C: `k=15` ⇒ flat offset `30`, aliases `halfrate[0][2][0]` | [x] |
| C10 | `hdr_bitrate` | A: `i=0`; B: layer field `0b11` ⇒ `j=2`; C: `k=0` ⇒ `halfrate[0][2][0]` | [x] |
| C11 | `hdr_bitrate` | A: `i=0`; B: `j=2`; C: `k∈1..14` ⇒ `halfrate[0][2][1..14]` (MPEG-2/2.5 Layer I row) | [x] |
| C12 | `hdr_bitrate` | A: `i=0`; B: `j=2`; C: `k=15` ⇒ flat offset `45`, aliases `halfrate[1][0][0]` | [x] |
| C13 | `hdr_bitrate` | A: `i=1` (`h[1]&0x8 != 0`); B: `j=-1`; C: `k=0` ⇒ flat offset `30`, aliases `halfrate[0][2][0]` | [x] |
| C14 | `hdr_bitrate` | A: `i=1`; B: `j=-1`; C: `k∈1..14` ⇒ flat offsets `31..44`, alias `halfrate[0][2][1..14]` | [x] |
| C15 | `hdr_bitrate` | A: `i=1`; B: `j=-1`; C: `k=15` ⇒ flat offset `45`, aliases `halfrate[1][0][0]` | [x] |
| C16 | `hdr_bitrate` | A: `i=1`; B: `j=0`; C: `k=0` ⇒ `halfrate[1][0][0]` | [x] |
| C17 | `hdr_bitrate` | A: `i=1`; B: `j=0`; C: `k∈1..14` ⇒ `halfrate[1][0][1..14]` (MPEG-1 Layer III row) | [x] |
| C18 | `hdr_bitrate` | A: `i=1`; B: `j=0`; C: `k=15` ⇒ flat offset `60`, aliases `halfrate[1][1][0]` | [x] |
| C19 | `hdr_bitrate` | A: `i=1`; B: `j=1`; C: `k=0` ⇒ `halfrate[1][1][0]` | [x] |
| C20 | `hdr_bitrate` | A: `i=1`; B: `j=1`; C: `k∈1..14` ⇒ `halfrate[1][1][1..14]` (MPEG-1 Layer II row) | [x] |
| C21 | `hdr_bitrate` | A: `i=1`; B: `j=1`; C: `k=15` ⇒ flat offset `75`, aliases `halfrate[1][2][0]` | [x] |
| C22 | `hdr_bitrate` | A: `i=1`; B: `j=2`; C: `k=0` ⇒ `halfrate[1][2][0]` | [x] |
| C23 | `hdr_bitrate` | A: `i=1`; B: `j=2`; C: `k∈1..14` ⇒ `halfrate[1][2][1..14]` (MPEG-1 Layer I row) | [x] |
| C24 | `hdr_bitrate` | A: `i=1`; B: `j=2`; C: `k=15` ⇒ flat offset `90`, one byte past the end of the whole table | [x] |
| C25 | `hdr_bitrate` | D: ignored bits — for a fixed `(i, layer, k)`, randomize `h[0]`, `h[1] & 0x01`, `h[1] & 0xF0`, `h[2] & 0x0F`; result must be invariant and equal in C and Rust | [x] |
| C26 | `hdr_bitrate` | D/E: trailing bytes `h[3..]` randomized, buffer lengths 3..64; result must be invariant | [x] |
| C27 | `hdr_bitrate` | E: pointer offset into a larger buffer, offsets 0..15 (all alignments mod 16), same 3 header bytes at each offset | [x] |
| C28 | `hdr_bitrate` | E: buffer of exactly 3 bytes ending immediately before an unmapped guard page — proves both read only `h[1]`, `h[2]` and neither over-reads | [x] |
| C29 | `hdr_bitrate` | E: repeated invocation / statelessness — the same input called many times interleaved with other inputs yields the same value (C table is `static`, Rust table is a `static`; neither may accumulate state) | [x] |
| C30 | `hdr_bitrate` | Full cross product, exhaustive: all 256 × 256 `(h[1], h[2])` values with randomized surrounding bytes — the complete A × B × C space with no pruning | [x] |

## Verification gate

- [x] Every row above passes across randomized inputs (fixed seed) under the
      default feature set.
- [x] Every row above passes under `--no-default-features` (identical set here,
      as no features are declared).

## How the suite was validated (test sensitivity)

A green differential suite proves nothing unless it can fail. Two findings:

### Pitfall: `cargo test` does not rebuild a `cdylib`

With `crate-type = ["cdylib"]` and no test target linking the library, `cargo
test` recompiles the crate for the harness but leaves
`target/<profile>/libhdr_bitrate_lib.so` untouched. An initial run of this suite
passed against a *stale* `.so`, and every injected bug passed too. Two mitigations
are now in place:

* `assert_rust_so_is_fresh()` in `tests/differential.rs` fails the run if the
  `.so` is older than any crate source.
* `run_all.sh` runs `cargo build` before every `cargo test`, for each profile and
  feature combination.

### Mutation results (each mutant rebuilt via `run_all.sh`)

| mutant | injected change | result |
|--------|-----------------|--------|
| M2 | clamp bitrate nibble `15 → 14` | caught |
| M3 | last table byte `224 → 225` | caught |
| M4 | return `half` instead of `2 * half` | caught |
| M5 | read `h[3]` instead of `h[2]` | caught |
| M6 | remove the `#[no_mangle]` export | caught (symbol-parity test) |
| M7 | version mask `0x8 → 0x4` | caught |
| M9 | out-of-table fallback `0 → 1` | caught by C01, C02, C24, C25–C27, C29, C30, E2, E4, E5, E6 |
| M10 | `u8::wrapping_sub` for `layer - 1` (yields `255`, not `-1`) | caught |
| M11 | invert version row selection | caught |
| M1 | `isize::saturating_sub(1)` for `layer - 1` | not caught — **behaviourally equivalent**: `isize` saturates at `isize::MIN`, so `0 - 1` is still `-1` |
| M8 | clamp negative flat offset to `0` | not caught — **behaviourally equivalent**: offsets `-15..-1` read zero padding and `HALFRATE[0]` is also `0` |

The two uncaught mutants are equivalent mutants, not coverage gaps.

## Notes on the out-of-bounds reads

`c_src/src/lib.c` indexes `halfrate[2][3][15]` with a middle index that can be
`-1` and a last index that can be `15`, so flat offsets span `-15 ..= 90` against
a 90-byte table. What the C reads outside the table is a property of the built
object, so it was measured rather than assumed:

* `.rodata` starts at `0x2000`, a page boundary, and the table is at its very
  start (`objdump -s -j .rodata`, `readelf -S`). Offsets `-15..-1` therefore land
  in the zero tail-padding of the preceding `R E` segment page ⇒ `0`.
* The table occupies `0x2000..0x205A`; `.eh_frame_hdr` begins at `0x205C`, so
  offset `90` (`0x205A`) is alignment padding ⇒ `0`.

The Rust translation returns `0` for any offset outside the flat table, which
matches. Rather than relying on that reasoning, `c30_e7_exhaustive_all_header_bytes`
compares all 65 536 `(h[1], h[2])` pairs against the actual built C `.so`.
