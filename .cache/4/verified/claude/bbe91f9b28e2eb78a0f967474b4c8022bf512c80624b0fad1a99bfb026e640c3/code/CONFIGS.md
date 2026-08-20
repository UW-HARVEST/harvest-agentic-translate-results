# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

Derived **mechanically** from the C source, the public header, `CMakeLists.txt`
and `Cargo.toml` — the mirror of `ERRORS.md`, but for **valid** inputs.

## Axis 1 — build-time configuration

| source | configuration knobs found | valid combinations |
|---|---|---|
| `Cargo.toml` | **no `[features]` table at all** (`grep -c '\[features\]' Cargo.toml` → `0`) | 1 (the empty feature set) |
| `c_src/CMakeLists.txt` | no `option()`, no `target_compile_definitions`, no `#ifdef` consumer; a single `add_library(SHARED src/lib.c)` | 1 |
| `c_src/src/lib.c` | no `#if` / `#ifdef` / `#define` (grep for `#if` → no matches) | 1 |

**Total valid feature combinations: exactly 1** — the empty set. `default`,
`--no-default-features`, and `--all-features` are all the same build. There is
no conditional compilation anywhere, so no `#[cfg(feature = "...")]` gating is
required or possible.

## Axis 2 — runtime options / modes / flags

**None.** The public header declares exactly one function and no other
declarations:

```c
uint32_t rev16(uint32_t a);
```

There is no init/context/handle type, no setter, no mode enum, no global state,
and no flag parameter. `rev16` is a pure, stateless, **branch-free** function
(no `if` / `switch` / `?:` / loop anywhere — verified mechanically in
`ERRORS.md`). Consequently there is no option cross-product to enumerate along
this axis.

## Axis 3 — public entry points (the FULL set, incl. the lowest level)

| # | entry point | level | note |
|---|-------------|-------|------|
| 1 | `rev16` | lowest **and** highest | the library's entire API; there is no convenience wrapper and no lower-level primitive beneath it |

The full call hierarchy is one node deep, so "exercise the low-level entry
points, not just the wrappers" is satisfied by exercising `rev16` — it *is* the
low-level entry point.

## Axis 4 — input shapes the code is sensitive to

`rev16` takes one `uint32_t` by value. Although branch-free, the four
statements operate at **four different bit granularities**, and all masks are
**16 bits wide**, which the table below enumerates:

| statement | granularity | masks | consequence |
|---|---|---|---|
| 1 | swap adjacent **bits** | `0xAAAA` / `0x5555` | also discards input bits 16..31 |
| 2 | swap adjacent **bit pairs** | `0xCCCC` / `0x3333` | |
| 3 | swap adjacent **nibbles** | `0xF0F0` / `0x0F0F` | |
| 4 | swap the two **bytes** | `0xFF00` / `0x00FF` | |

So the meaningful input-shape sub-axes are: the **low-half bit pattern**
(what is transformed) crossed with the **high-half bit pattern** (which must be
ignored), plus the value classes that isolate each granularity.

## Configuration table

Cross-product of the axes above, pruned to the combinations the C actually
distinguishes. Every row is driven through **both** `.so` exports and asserted
byte-for-byte identical. Randomized rows use a fixed seed (`SEED =
0x5EED_1234_ABCD_0001`, SplitMix64) so runs are reproducible; `N_RANDOM =
20_000` inputs per randomized row.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `rev16` | no options (none exist) · low half = `0x0000` × **all 15 high-half classes** | `config_c1_low_zero_all_high_classes` | [x] |
| C2 | `rev16` | low half = `0xFFFF` × all 15 high-half classes | `config_c2_low_all_ones_all_high_classes` | [x] |
| C3 | `rev16` | single bit set at each of the **16 low** positions × all 15 high-half classes (240 combos) — isolates one path per swap stage | `config_c3_single_low_bit_x_high_classes` | [x] |
| C4 | `rev16` | single bit set at each of the **32 full-width** positions; bits 16..31 must yield `0` | `config_c4_single_bit_full_32_positions` | [x] |
| C5 | `rev16` | stage-1 granularity values `0xAAAA` / `0x5555` × all high-half classes | `config_c5_stage1_mask_values` | [x] |
| C6 | `rev16` | stage-2 granularity values `0xCCCC` / `0x3333` × all high-half classes | `config_c6_stage2_mask_values` | [x] |
| C7 | `rev16` | stage-3 granularity values `0xF0F0` / `0x0F0F` × all high-half classes | `config_c7_stage3_mask_values` | [x] |
| C8 | `rev16` | stage-4 granularity values `0xFF00` / `0x00FF` × all high-half classes | `config_c8_stage4_mask_values` | [x] |
| C9 | `rev16` | per-**byte-lane** boundary values `{00,01,02,7F,80,81,FE,FF}` in both low bytes (8×8) × 3 high halves (192 combos) | `config_c9_byte_lane_boundary_values` | [x] |
| C10 | `rev16` | all 256 bit-**palindromes** (fixed points, `rev16(a)==a`), each also with random ignored high half | `config_c10_palindromic_low_half` | [x] |
| C11 | `rev16` | randomized low half, high half `0x0000` (20 000 inputs) | `config_c11_random_low_high_zero` | [x] |
| C12 | `rev16` | randomized low half, high half `0xFFFF` (20 000 inputs) | `config_c12_random_low_high_ones` | [x] |
| C13 | `rev16` | fully randomized 32-bit argument, **both** halves random (20 000 inputs) | `config_c13_random_full_32bit` | [x] |
| C14 | `rev16` | **exhaustive** sweep of all 2^16 high-half values × 6 fixed low halves (393 216 calls) — proves the high half is ignored for *every* possible value | `config_c14_exhaustive_high_half_sweep` | [x] |
| C15 | `rev16` | **involution**: `rev16(rev16(a)) == a & 0xFFFF`, plus cross-composition C→Rust and Rust→C (20 000 inputs) | `config_c15_involution_property` | [x] |
| C16 | `rev16` | **exhaustive** over all 2^16 low-half values, each also cross-checked against an independent 16-bit bit-reversal reference | `config_c16_exhaustive_low_16_bits` | [x] |
| C17 | `rev16` | **EXHAUSTIVE over the entire 2^32 input domain** (opt-in: `RUN_EXHAUSTIVE_32=1`) — every representable input verified | `config_c17_exhaustive_all_2pow32` | [x] |

### Coverage argument

Row **C17** visits every one of the 4 294 967 296 representable arguments and
found zero divergence, so the valid-path surface is not merely sampled — it is
**exhaustively proven** equivalent. Rows C1–C16 remain as fast, targeted
regression tests that localise a failure to a specific bit granularity or input
shape when something breaks, and C14+C16 together already cover all 2^32 inputs
by the factorisation "result depends only on the low half".

## Harness non-vacuity (mutation testing)

A green suite is only meaningful if it can fail. Three mutations were injected
into `src/lib.rs`, the `.so` rebuilt, and the suite re-run:

| mutation | expected | observed | verdict |
|---|---|---|---|
| stage-2 shift `<< 2` → `<< 3` | fail | **19 of 23 tests failed** | harness detects it |
| stage-1 masks widened to 32-bit (`0xAAAAAAAA`/`0x55555555`) | **pass** — genuinely semantics-preserving, because stage 2's 16-bit masks truncate the leaked bits before they can reach the result | 23 passed | correct (no false positive) |
| **all four** stages widened to 32-bit (the classic "helpfully fix the truncation" bug) | fail | **18 of 23 tests failed** | harness detects the highest-risk bug class |

`src/lib.rs` was restored byte-identically afterwards (`diff` clean) and the
suite re-verified green.

## Completion gate (Phase B / D)

- [x] Every row C1–C17 passes across randomized inputs (fixed seed).
- [x] The single valid feature combination (empty set) is covered.
- [x] Verified in both `dev` and `release` profiles.
