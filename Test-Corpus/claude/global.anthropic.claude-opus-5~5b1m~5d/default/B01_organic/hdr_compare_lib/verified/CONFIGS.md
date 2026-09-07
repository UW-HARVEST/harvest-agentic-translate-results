# CONFIGS.md — Phase B configuration surface table (VALID inputs)

Mechanically derived from the branches the C actually takes in
`c_src/src/lib.c`.

## Full public entry-point set

`c_src/include/lib.h` exposes exactly one entry point, and it is already the
lowest level one — there are no convenience wrappers above it and no other
public function below it:

| entry point | signature | notes |
|-------------|-----------|-------|
| `hdr_compare` | `int hdr_compare(const uint8_t *h1, const uint8_t *h2)` | the only exported symbol |
| `hdr_valid` | `static int hdr_valid(const uint8_t *h)` | not exported; reachable **only** as the first conjunct of `hdr_compare`, so it is driven indirectly by every row below |

## Runtime options / modes

There is **no** runtime option, mode, flag, setter, context struct, global,
`#ifdef`, or byte-order switch anywhere in `c_src/`. The library is a pure
function of its two input buffers. Consequently the configuration surface is
entirely made of **input shapes**, i.e. the bit-fields the C branches on.

## Axes the C code actually distinguishes

Extracted from each operand of each `&&` / `||` / `^` in `lib.c`:

| axis | source line | distinct values the code separates |
|------|-------------|-------------------------------------|
| A. `h2[0]` sync byte | `lib.c:4` | `0xff` / anything else |
| B. `h2[1]` sync form | `lib.c:4` | form-F: `(h2[1] & 0xF0) == 0xf0`; form-E: `(h2[1] & 0xFE) == 0xe2` (i.e. `0xe2`/`0xe3`); both (impossible); neither |
| C. `h2[1]` layer field `(h2[1] >> 1) & 3` | `lib.c:5` | `0` (rejected) / `1` / `2` / `3` |
| D. `h2[1]` bit 0 (protection bit) | masked away by `& 0xFE` on `lib.c:10` | `0` / `1` — must be a **don't-care** for the `h1`/`h2` comparison |
| E. `h2[2]` bitrate nibble `h2[2] >> 4` | `lib.c:5`, `lib.c:12` | `0` ("free"), `1..14` (normal), `15` (rejected) |
| F. `h2[2]` sample-rate index `(h2[2] >> 2) & 3` | `lib.c:6`, `lib.c:11` | `0` / `1` / `2` valid, `3` rejected |
| G. `h2[2]` bits 1..0 (padding / private) | never inspected | must be a **don't-care** |
| H. `h1[1]` vs `h2[1]` relation | `lib.c:10` | equal on `0xFE` mask / differing on `0xFE` mask |
| I. `h1[2]` vs `h2[2]` sample-rate relation | `lib.c:11` | equal on `0x0C` mask / differing |
| J. `h1[2]` vs `h2[2]` free-bitrate relation | `lib.c:12` | both free / both non-free / exactly one free |
| K. `h1[0]` | never inspected | must be a **don't-care** |
| L. buffer length | pointers only, no length param | 3 bytes exactly / longer with garbage tail |

## Configuration rows (pruned cross-product of the axes above)

Each row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, SplitMix64) that satisfy the row's constraints, plus
the exhaustive sweeps in rows 17-19.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hdr_compare` | Fully matching pair: `h2` valid form-F, layer∈{1,2,3}, bitrate 1..14, srate∈{0,1,2}; `h1[1]==h2[1]`, `h1[2]==h2[2]` → expect `1` | [x] |
| 2 | `hdr_compare` | Same as row 1 but sync form-E (`h2[1] & 0xFE == 0xe2`) → expect `1` | [x] |
| 3 | `hdr_compare` | Valid `h2`, layer field = 1 specifically, all other axes random-valid, matching `h1` | [x] |
| 4 | `hdr_compare` | Valid `h2`, layer field = 2 specifically, matching `h1` | [x] |
| 5 | `hdr_compare` | Valid `h2`, layer field = 3 specifically, matching `h1` | [x] |
| 6 | `hdr_compare` | Axis D don't-care: `h1[1] == h2[1] ^ 0x01` (protection bit differs only) → still expect `1` | [x] |
| 7 | `hdr_compare` | Axis G don't-care: `h1[2]` and `h2[2]` differ **only** in bits 1..0 (padding/private), both non-free → expect `1` | [x] |
| 8 | `hdr_compare` | Axis K don't-care: `h1[0]` randomized over all 256 values (incl. `0x00`, `0xff`) while the pair otherwise matches → result must be independent of `h1[0]` | [x] |
| 9 | `hdr_compare` | Axis E, both-free branch of J: `h2[2] >> 4 == 0` and `h1[2] >> 4 == 0`, same srate → expect `1` | [x] |
| 10 | `hdr_compare` | Axis J both-non-free but *different* bitrate nibbles (`h1[2]>>4` and `h2[2]>>4` both in `1..15`, unequal) → expect `1` (bitrate value itself is not compared) | [x] |
| 11 | `hdr_compare` | Axis J one-free / one-non-free (mismatch branch) → expect `0` | [x] |
| 12 | `hdr_compare` | Axis F: srate index equal in both headers, swept over `{0,1,2}`, bitrate nibbles random-valid | [x] |
| 13 | `hdr_compare` | Axis E boundary: `h2[2] >> 4 == 14` (max valid) and `h1[2] >> 4 == 14`, srate valid | [x] |
| 14 | `hdr_compare` | Axis L: both buffers allocated to exactly 3 bytes (no readable 4th byte) | [x] |
| 15 | `hdr_compare` | Axis L: both buffers 16 bytes with random garbage in bytes 3..15 → result identical to the 3-byte case | [x] |
| 16 | `hdr_compare` | Fully unconstrained fuzz: all of `h1[0..4]`, `h2[0..4]` uniform random (mixes valid & invalid; ~500 000 iterations) | [x] |
| 17 | `hdr_compare` | **Exhaustive** over `(h1[1], h2[1])` = all 65 536 pairs, with `h2[0]=0xff` and `(h1[2], h2[2])` swept over a fixed representative set | [x] |
| 18 | `hdr_compare` | **Exhaustive** over `(h1[2], h2[2])` = all 65 536 pairs, with `h2[0]=0xff` and `h1[1]==h2[1]` swept over all 256 values of `h2[1]` (256 × 65 536 = 16 777 216 calls) | [x] |
| 19 | `hdr_compare` | **Exhaustive** over `h2[0]` = all 256 values, crossed with a valid remainder — isolates axis A | [x] |
| 20 | `hdr_compare` | Structured "realistic MPEG frame header" corpus: `0xFF 0xFB 0x90 0x64`, `0xFF 0xFA …`, `0xFF 0xF3 …`, `0xFF 0xE3 …`, `0xFF 0xE2 …`, each compared against itself and against every other member of the corpus (all ordered pairs) | [x] |
| 21 | `hdr_compare` | Joint 4-byte high-volume sweep: 20 000 000 randomized draws over the full `(h1[1], h1[2], h2[1], h2[2])` space with `h2[0] = 0xff` — validates the factorization assumption that rows 17-19 rely on, without assuming it | [x] |

## Notes on coverage completeness

Rows 17 + 18 + 19 together are effectively an exhaustive proof: `hdr_compare`
reads only 5 bytes (`h1[1]`, `h1[2]`, `h2[0]`, `h2[1]`, `h2[2]`), and the
expression decomposes so that `h2[0]` is independent of the rest, `(h1[1],h2[1])`
is independent of `(h1[2],h2[2])`, and row 18 covers all `h2[1]` values crossed
with all `(h1[2],h2[2])` pairs. Row 16's unconstrained fuzz cross-checks the
independence assumption without relying on it, and row 21 hammers the joint
4-byte space with 20 M draws.

There is no binary/driver target in `c_src/CMakeLists.txt`, so no stdout
comparison applies. `translation/Cargo.toml` declares no `[features]`, so this
table is the complete configuration surface for all builds.
