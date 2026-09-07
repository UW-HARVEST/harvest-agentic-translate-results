# ERRORS.md — Phase A error-surface table

Derived mechanically from the complete C source (`c_src/src/lib.c`, 9 lines;
`c_src/include/lib.h`, 3 lines). Grep used to enumerate every rejection
mechanism:

```sh
grep -nE 'return[[:space:]]+(-1|NULL|0)|assert|RETURN_ERROR|errno|ERROR|abort|exit|if[[:space:]]*\(|switch|#ifdef|#if |#ifndef|goto' -r c_src/src c_src/include
# -> no matches
```

## Result of the mechanical scan

| mechanism searched for | occurrences in C source |
|------------------------|-------------------------|
| error-return macros (`RETURN_ERROR`, …) | 0 |
| `return -1` / `return NULL` / sentinel returns | 0 (the single `return a;` returns a computed value) |
| error enums / status codes | 0 (return type is `uint32_t`, a value, not a status) |
| `assert` | 0 |
| explicit range / bounds checks | 0 |
| null-pointer checks | 0 (the API takes no pointers) |
| min/max constants | 0 |
| `if` / `switch` / `goto` / early return | 0 (function is straight-line and branchless) |
| `#if` / `#ifdef` / `#ifndef` | 0 |
| allocation, I/O, or any other fallible operation | 0 |
| integer division or modulo (UB on 0) | 0 |
| signed overflow / UB-capable shift (shift count is a literal 1/2/4/8 on an unsigned value) | 0 |

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | — | **(no rows)** — the C API has no rejection path | — |

`rev16` is a **total function** over its entire domain: every one of the
2^32 possible `uint32_t` arguments is valid and produces a defined value. There
is no invalid input, no out-of-range condition, no sentinel, and no error code.
Consequently there is nothing to check off in this table.

## Generic FFI boundaries still exercised in Phase C

Even with an empty table, the task requires covering the generic boundaries any
C API has. The ones that are *meaningful for this signature* are enumerated
below and are covered by `tests/differential.rs`
(`phase_c_generic_boundaries`); each asserts C and Rust agree bit-for-bit on
the returned `uint32_t`.

| # | boundary class | concrete inputs | status |
|---|----------------|-----------------|--------|
| C1 | zero / minimum value | `0x0000_0000` | [x] |
| C2 | maximum value | `0xFFFF_FFFF` | [x] |
| C3 | one step past the "documented" 16-bit range the masks imply | `0x0001_0000`, `0x0000_FFFF`, `0x0001_FFFF` | [x] |
| C4 | high-half saturated, low half swept (upper bits must be discarded) | `0xFFFF_0000 \| lo` for all `lo` in `0..=0xFFFF` | [x] |
| C5 | every single-bit value across the full 32-bit width (incl. bits 16..31 that have no valid effect) | `1u32 << k`, `k` in `0..32` | [x] |
| C6 | every single-bit-clear value (`!(1<<k)`) | `!(1u32 << k)`, `k` in `0..32` | [x] |
| C7 | sign-bit / `INT_MAX` / `INT_MIN` reinterpretations passed as `unsigned` | `0x8000_0000`, `0x7FFF_FFFF`, `-1 as u32`, `-2147483648 as u32` | [x] |
| C8 | nibble/byte-boundary values one step past each mask edge | `0x00FF`,`0x0100`,`0x0F0F`,`0xF0F0`,`0x5555`,`0xAAAA`,`0x3333`,`0xCCCC`,`0xFF00` and each `±1` | [x] |

Not applicable to this API and therefore intentionally absent: null pointers
(no pointer parameters), zero/oversized lengths (no length parameters), buffers
(no buffer parameters), and out-of-range enum values (no enum parameters — the
sole parameter is `uint32_t`, whose entire value range is valid; the "any int
crosses the boundary" concern is nevertheless covered exhaustively by the
32-bit sweeps above and by the full-range randomized sweep in Phase B).

## Gate

- [x] Every row in the error-surface table has a passing error-path
      differential test — vacuously satisfied (0 rows), with the generic
      boundary rows C1–C8 tested and passing in addition.
