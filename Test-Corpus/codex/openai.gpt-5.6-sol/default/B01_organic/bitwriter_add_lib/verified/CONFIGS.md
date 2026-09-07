# Configuration surface

The public header exposes one entry point and no runtime option, mode, flag,
enum, feature, format, element type, byte-order selection, or buffer-consuming
operation. `bitwriter_add` reads `val`, `bits`, and `tot`; it leaves `pos`,
`len`, `buffer`, and the pointed-to buffer untouched.

Rows below are the cross-product pruned to branches and boundary shapes that
the C source distinguishes. Arithmetic in the loop condition is `uint32_t`
wrapping arithmetic. The ternary is
`b = ((63 - bw->bits) > bits) ? bits : (63 - bw->bits)`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `bitwriter_add` | No-loop path: wrapping `initial bits + input bits` is `0..63`, input `bits` is `1..63`; randomized value/state and non-wrapping `tot` | [x] |
| 2 | `bitwriter_add` | No-loop path with `tot` wrapping at `UINT32_MAX`; randomized value/state | [x] |
| 3 | `bitwriter_add` | Loop path with initial `bits <= 63`, sum `>= 64`, and ternary selecting `63 - initial bits`; covers sums exactly 64 and greater, then the 100-iteration cap | [x] |
| 4 | `bitwriter_add` | Loop path with initial `bits > 63` and small input, ternary selecting input `bits`; randomized value/state, then the 100-iteration cap | [x] |
| 5 | `bitwriter_add` | Native-width boundary `input bits == 64` (initial left shift count zero), randomized initial `bits` and state | [x] |
| 6 | `bitwriter_add` | Zero-width ABI boundary `input bits == 0`, randomized initial state on both no-loop and loop sides | [x] |
| 7 | `bitwriter_add` | One-past-width `input bits == 65`, randomized initial state on both wrapping-sum branch sides | [x] |
| 8 | `bitwriter_add` | Oversized `input bits`, including `UINT32_MAX`, with wrapping sums on both no-loop and loop sides | [x] |

## Public entry-point coverage

- `bitwriter_add`: rows 1-8

## Cargo feature combinations

`Cargo.toml` has no `[features]` table. The complete set is:

- default feature set (empty)
- `--no-default-features` (also empty; verified separately in Phase D)

## Binary targets

Neither build declares an executable target, so stdout comparison is not
applicable.
