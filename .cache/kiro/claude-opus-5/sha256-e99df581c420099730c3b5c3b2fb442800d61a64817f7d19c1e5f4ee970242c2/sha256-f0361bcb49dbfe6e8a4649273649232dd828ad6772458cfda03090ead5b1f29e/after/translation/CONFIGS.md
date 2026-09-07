# CONFIGS.md — configuration surface table (valid inputs)

## How this was derived

### Runtime options / modes / flags: **none**

`c_src/include/lib.h` declares exactly one function and no types, no context
struct, no setters, no globals. `grep -E 'enum|#if|#ifdef|static [^i]|extern'
c_src/src/lib.c` finds no configuration state at all. There is no build-time
configuration either: `CMakeLists.txt` has no `option()`, no
`target_compile_definitions`, and `translation/Cargo.toml` has **no `[features]`
section** (verified: `grep -A20 '\[features\]' Cargo.toml` → no match), so the
only feature combination is the default one.

Byte order is not an axis: the API consumes individual `uint8_t`s, never a
multi-byte scalar. Length/count is not an axis: no length parameter exists; the
read extent is fixed at 3 bytes (see `ERRORS.md` G4).

### Public entry points

| entry point | linkage | reached how |
|-------------|---------|-------------|
| `hdr_compare(const uint8_t *h1, const uint8_t *h2)` | exported (`T`) | called directly through the `.so` in every row below |
| `hdr_valid(const uint8_t *h)` | `static`, not exported | lowest-level function; driven through `hdr_compare`'s `h2` argument — rows 19–26 target it specifically |

There are no convenience/one-shot wrappers: `hdr_compare` *is* the lowest-level
public entry point, so every row calls it directly via `libloading`.

### Input-shape axes the C actually branches on

Read off the two `return` expressions in `c_src/src/lib.c`:

| axis | source expression | distinct classes |
|------|-------------------|------------------|
| `SYNC` | `h[0] == 0xff` | `0xff` / any of the other 255 |
| `CLASS` | `(h[1] & 0xF0) == 0xf0` \|\| `(h[1] & 0xFE) == 0xe2` | F-class (`0xF0..0xFF`) / E-class (`0xE2`,`0xE3`) / neither |
| `L` | `((h[1] >> 1) & 3)` | `0` (rejected) / `1` / `2` / `3` |
| `N` | `(h[2] >> 4)` | `0` (drives the `& 0xF0`-is-zero predicate) / `1..14` / `15` (rejected) |
| `S` | `((h[2] >> 2) & 3)` | `0` / `1` / `2` / `3` (rejected) |
| `B1CMP` | `((h1[1] ^ h2[1]) & 0xFE) == 0` | agree / differ |
| `B2CMP` | `((h1[2] ^ h2[2]) & 0x0C) == 0` | agree / differ |
| `NZ` | `!(((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0))` | both-zero / both-nonzero / disagree |
| `IGNORED` | bit `0x01` of `h[1]`; bits `0x03` of `h[2]`; **all** of `h1[0]` | must not affect the result |

Note the deliberate asymmetry in the C: **`h1` is never validity-checked.** Only
`h2` goes through `hdr_valid`. So `h1[2] >> 4 == 15` and `h1[1]` outside both
classes are perfectly legal inputs that can still yield `1`. Rows 12, 13 and 18
pin this down.

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, SplitMix64) that fill the free bits of the shape, not a
single hand-picked value.

## The table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `hdr_compare` | valid `h2`: SYNC=`0xff`, F-class, `L=1`, `N=0`, `S=0`; `h1` agrees on all compared bits, ignored bits randomized → expect `1` | [x] |
| 2 | `hdr_compare` | valid `h2`: F-class, `L=2`, `N=0`, `S=0`; matching `h1` | [x] |
| 3 | `hdr_compare` | valid `h2`: F-class, `L=3`, `N=0`, `S=0`; matching `h1` | [x] |
| 4 | `hdr_compare` | valid `h2`: E-class `h2[1]=0xE2` (`L=1` forced), `N=0`, `S=0`; matching `h1` | [x] |
| 5 | `hdr_compare` | valid `h2`: E-class `h2[1]=0xE3`, `N=0`, `S=0`; matching `h1` | [x] |
| 6 | `hdr_compare` | valid `h2`: F-class, `L∈{1,2,3}`, `N∈1..14` random, `S=0`; `h1[2]` high nibble non-zero (NZ=both-nonzero) | [x] |
| 7 | `hdr_compare` | boundary shape: `N=14` (max accepted), `S=2` (max accepted), F-class `L=3` | [x] |
| 8 | `hdr_compare` | boundary shape: `N=1` (min non-zero), `S=1`, F-class `L=1` | [x] |
| 9 | `hdr_compare` | full cross-product `CLASS∈{F,E} × L∈{1,2,3} × N∈{0,1,7,14} × S∈{0,1,2}`, matching `h1` (E-class pruned to `L=1`, the only value it can take) | [x] |
| 10 | `hdr_compare` | IGNORED axis: `h1[1]` and `h2[1]` differ **only** in bit `0x01` (masked out by `0xFE`), otherwise valid+matching → must still be `1` | [x] |
| 11 | `hdr_compare` | IGNORED axis: `h1[2]` and `h2[2]` differ **only** in bits `0x03`, same `0x0C` bits and same high-nibble-zeroness → must still be `1` | [x] |
| 12 | `hdr_compare` | asymmetry: `h1[2] >> 4 == 15` (a value `hdr_valid` would reject) while `h2[2]` has `N∈1..14`; NZ agrees → must be `1` | [x] |
| 13 | `hdr_compare` | IGNORED axis: `h1[0]` swept over all 256 values (incl. `0x00`, `0xff`) with an otherwise-matching pair → result independent of `h1[0]` | [x] |
| 14 | `hdr_compare` | aliasing: `h1 == h2`, same buffer, `h2` valid → must be `1` for every valid `h2` | [x] |
| 15 | `hdr_compare` | aliasing: `h1 == h2`, same buffer, `h2` **invalid** → must be `0` | [x] |
| 16 | `hdr_compare` | valid `h2`, `h1[1]` differs from `h2[1]` in a random non-empty subset of mask `0xFE` → expect `0` | [x] |
| 17 | `hdr_compare` | valid `h2`, byte-1 agrees, `h1[2]` differs in bit `0x04` only / bit `0x08` only / both → expect `0` (3 sub-shapes) | [x] |
| 18 | `hdr_compare` | valid `h2`, bytes agree on `0xFE`/`0x0C`, NZ **disagrees**: (a) `h2[2]` high nibble `0`, `h1[2]` high nibble non-zero; (b) the reverse → expect `0` | [x] |
| 19 | `hdr_compare` → `hdr_valid` | invalid `h2`: `h2[0] != 0xff`, swept over all 255 wrong sync bytes, rest of both headers randomized | [x] |
| 20 | `hdr_compare` → `hdr_valid` | invalid `h2`: `h2[1]` in neither class (all 238 such byte values) | [x] |
| 21 | `hdr_compare` → `hdr_valid` | invalid `h2`: F-class with `L=0` (`h2[1] ∈ {0xF0,0xF1,0xF8,0xF9}`) | [x] |
| 22 | `hdr_compare` → `hdr_valid` | invalid `h2`: `h2[2] >> 4 == 15` (`h2[2] ∈ 0xF0..0xFF`) | [x] |
| 23 | `hdr_compare` → `hdr_valid` | invalid `h2`: `(h2[2] >> 2) & 3 == 3` (`h2[2] & 0x0C == 0x0C`) | [x] |
| 24 | `hdr_compare` | **exhaustive** sweep of all 2^24 `h2` byte triples against a set of fixed `h1` values | [x] |
| 25 | `hdr_compare` | **exhaustive** sweep of all 2^16 `(h1[1], h1[2])` pairs for each of several valid and invalid `h2` | [x] |
| 26 | `hdr_compare` | unbiased random fuzz over all 5 read-relevant bytes (2,000,000 vectors, seeded) — hits the shapes no hand-written row anticipates | [x] |
| 27 | `hdr_compare` | pointer placement: identical logical headers located at every byte offset `0..7` in the backing buffer (alignment must not matter) and in two separate non-adjacent allocations | [x] |
| 28 | `hdr_compare` | read-extent: header placed so byte 3 onward is a `PROT_NONE` guard page; asserted for valid and invalid `h2` (no over-read in either implementation) | [x] |
| 29 | `hdr_compare` | short-circuit: `h1 = NULL` with invalid `h2` (all 5 invalidity reasons) → `0` without touching `h1` | [x] |
| 30 | binary/driver comparison | **N/A** — `CMakeLists.txt` declares only `add_library(... SHARED ...)`; `Cargo.toml` has `crate-type = ["cdylib"]` and no `[[bin]]`/`src/main.rs`. Neither project builds an executable, so there is no stdout to compare. | [x] |
| 31 | `hdr_compare` | **complete accept surface**: every valid `h2` (14 × 180 = 2520) × all 2^16 `(h1[1], h1[2])` = 165,150,720 pairs. Since the result depends on only 5 bytes and `h1[0]` is never read (row 13), this enumerates *every* input that can possibly return `1`. Accept count also checked against a closed form derived from the C. | [x] |
| 32 | `hdr_compare` | reject side: every **invalid** `(h2[1], h2[2])` under a correct sync byte × 48 seeded-random `h1` + the aliased `h1 == h2` → all must reject | [x] |

## Row → test mapping

| rows | test file |
|------|-----------|
| 1–18, 27 | `tests/phase_b_configs.rs` (17 tests) |
| 24, 25, 26, 31, 32 | `tests/phase_b_sweeps.rs` (5 tests) |
| 19–23 (invalid-`h2` shapes; identical to `ERRORS.md` rows 1–5), 28, 29 | `tests/phase_c_errors.rs` (12 tests) |
| 30 | N/A — no executable in either project |

Total volume actually compared through the two `.so`s: **~253 million** `hdr_compare`
call pairs (506 M FFI calls) per profile, zero divergences.
