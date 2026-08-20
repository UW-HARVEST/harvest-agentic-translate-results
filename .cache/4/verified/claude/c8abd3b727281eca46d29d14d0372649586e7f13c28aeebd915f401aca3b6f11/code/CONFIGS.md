# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from what the C
actually branches on.

## Build-time configuration surface

| axis | values | source |
|------|--------|--------|
| Cargo `[features]` | **none — the section does not exist in `Cargo.toml`** | `Cargo.toml` |
| Cargo `crate-type` | `["cdylib"]` only | `Cargo.toml` `[lib]` |
| CMake options | none; `add_library(SHARED src/lib.c)`, no `option()`, no `target_compile_definitions`, no generator expressions | `c_src/CMakeLists.txt` |
| C preprocessor conditionals | **none** — `grep -c '#if' c_src/src/lib.c c_src/include/lib.h` → `0` | `c_src/` |

**Enumeration of every valid feature combination: exactly ONE — the empty set.**
`cargo check --no-default-features` and `cargo check` are the same build. There
is no second configuration to repeat Phases B–C under; the "every feature
combination" gate is satisfied by the single combination. This is verified
programmatically by `scripts/check_all_features.sh`.

## Runtime configuration surface

| axis | values | source |
|------|--------|--------|
| runtime options / modes / flags | **none** — the API is a single pure function with one argument and no setters, no context struct, no globals | `c_src/include/lib.h` |
| public entry points | **one: `hdr_bitrate`** — it *is* the lowest-level entry point; there is no convenience wrapper layer above it and nothing below it | `c_src/include/lib.h` |

## Input-shape axes the C actually branches on

The whole function is the single expression
`2 * halfrate[i][j][k]` over `static const uint8_t halfrate[2][3][15]`, so the
axes are exactly the three index sub-fields of the input bytes:

| axis | expression | distinct values | meaning (MPEG audio frame header) |
|------|-----------|-----------------|-----------------------------------|
| `i` | `!!(h[1] & 0x8)` | 2 (`0`,`1`) | MPEG version ID bit |
| `j` | `((h[1] >> 1) & 3) - 1` | 4 (`-1`,`0`,`1`,`2`) | 2-bit layer field (`0` ⇒ `j == -1`) |
| `k` | `h[2] >> 4` | 16 (`0..=15`) | 4-bit bitrate index |
| unread bytes | `h[0]`, `h[3]`… | arbitrary | must not affect output |
| low bits | `h[1] & 0x1`, `h[1] & 0xF0`, `h[2] & 0x0F` | arbitrary | masked off; must not affect output |

Cross-product = `2 * 4 * 16 = 128` index triples, i.e. **128 distinct
configurations**, and the *entire* space of inputs that can change the result is
`h[1] * h[2] = 256 * 256 = 65536` byte pairs. That is small enough to test
**exhaustively**, which strictly subsumes randomized property testing. Rows
below group the 128 triples by the behaviour class the flat offset
`flat = i*45 + j*15 + k` falls into; every row is additionally covered
exhaustively by `config_exhaustive_all_65536_h1_h2_pairs`.

## CONFIGURATION-SURFACE TABLE

`flat` = flat byte offset into the 90-byte table. In-table ⇔ `0 <= flat <= 89`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `hdr_bitrate` | `i=0, j=0` (`h[1]&0x0E == 0x02`), `k=0..14` → `flat=0..14`, row `halfrate[0][0]` = MPEG1 Layer III | [x] |
| C2 | `hdr_bitrate` | `i=0, j=1` (`h[1]&0x0E == 0x04`), `k=0..14` → `flat=15..29`, row `halfrate[0][1]` | [x] |
| C3 | `hdr_bitrate` | `i=0, j=2` (`h[1]&0x0E == 0x06`), `k=0..14` → `flat=30..44`, row `halfrate[0][2]` | [x] |
| C4 | `hdr_bitrate` | `i=1, j=0` (`h[1]&0x0E == 0x0A`), `k=0..14` → `flat=45..59`, row `halfrate[1][0]` | [x] |
| C5 | `hdr_bitrate` | `i=1, j=1` (`h[1]&0x0E == 0x0C`), `k=0..14` → `flat=60..74`, row `halfrate[1][1]` | [x] |
| C6 | `hdr_bitrate` | `i=1, j=2` (`h[1]&0x0E == 0x0E`), `k=0..14` → `flat=75..89`, row `halfrate[1][2]` (contains the max entry `224` ⇒ result `448`) | [x] |
| C7 | `hdr_bitrate` | `i=1, j=-1` (`h[1]&0x0E == 0x08`), `k=0..14` → `flat=30..44`: out-of-range `j` that still lands **in-table**, aliasing row `halfrate[0][2]` | [x] |
| C8 | `hdr_bitrate` | `i=0, j=-1` (`h[1]&0x0E == 0x00`), `k=15` → `flat=0`: aliases `halfrate[0][0][0]` = `0` | [x] |
| C9 | `hdr_bitrate` | any `(i,j)` with `k=15` and `(i,j) != (1,2)` → `flat` = first byte of the next row = `0` | [x] |
| C10 | `hdr_bitrate` | boundary values of `k`: `k=0` (first entry, `0` in every row) and `k=14` (last valid entry of each row) across all 8 `(i,j)` groups | [x] |
| C11 | `hdr_bitrate` | all 8 `(i,j)` groups × all 16 `k` = **all 128 index triples**, each with randomized don't-care bits | [x] |
| C12 | `hdr_bitrate` | don't-care-bit independence: fixed `(i,j,k)`, randomized `h[0]`, `h[1]&0xF0`, `h[1]&0x01`, `h[2]&0x0F`, `h[3..]` — result must be invariant | [x] |
| C13 | `hdr_bitrate` | **exhaustive**: every one of the 65536 `(h[1],h[2])` pairs, `h[0]`/`h[3]` seeded-random | [x] |
| C14 | `hdr_bitrate` | pointer shape: buffer longer than 3 bytes, `h` at an offset inside a larger allocation, and every alignment `h as usize % 8 == 0..7` (function takes `const uint8_t*`, so no alignment requirement) | [x] |
| C15 | `hdr_bitrate` | repeated / interleaved calls with the same and alternating inputs — confirms the `static const` table needs no init and the function is stateless & idempotent | [x] |
| C16 | `hdr_bitrate` | full-range return-value sanity: the set of *all* values C can return over the whole input space is exactly `{0,8,16,…}` — compare the complete C output multiset against Rust | [x] |

All 16 rows are checked off by the tests in `tests/differential.rs`; C13 alone
covers the entire reachable input space byte-for-byte.
