# Configuration surface

The public callable ELF entry points are `cp_inflate` and `unfilter`.
Configuration rows below are derived from their `if`, `switch`, loop-bound,
alignment, block-shape, and predictor branches. "Narrow" means `w == 1`, so
`len == bpp`; "wide" means `w > 1`, so both the `x < bpp` and `x >= bpp`
regions can execute. Valid `unfilter` rows use positive `bpp`.

## `cp_inflate`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `cp_inflate` | stored block, empty payload, exact zero-byte output | [x] |
| 2 | `cp_inflate` | stored block, nonempty payload, exact output capacity | [x] |
| 3 | `cp_inflate` | stored block, nonempty payload, oversized output capacity | [x] |
| 4 | `cp_inflate` | fixed-Huffman block, empty payload | [x] |
| 5 | `cp_inflate` | fixed-Huffman block, literals only | [x] |
| 6 | `cp_inflate` | fixed-Huffman block, length/distance with distance `1` | [x] |
| 7 | `cp_inflate` | fixed-Huffman block, length/distance with distance greater than `1` | [x] |
| 8 | `cp_inflate` | dynamic-Huffman block, literals only | [x] |
| 9 | `cp_inflate` | dynamic-Huffman block, length/distance with distance `1` | [x] |
| 10 | `cp_inflate` | dynamic-Huffman block, length/distance with distance greater than `1` | [x] |
| 11 | `cp_inflate` | dynamic code-length alphabet uses literal lengths and repeat symbol `16` | [x] |
| 12 | `cp_inflate` | dynamic code-length alphabet uses zero repeat symbol `17` | [x] |
| 13 | `cp_inflate` | dynamic code-length alphabet uses zero repeat symbol `18` | [x] |
| 14 | `cp_inflate` | multiple blocks (`bfinal == 0` followed by another block) | [x] |
| 15 | `cp_inflate` | input address already 4-byte aligned (`first_bytes == 0`) | [x] |
| 16 | `cp_inflate` | input address offset requires one leading byte (`first_bytes == 1`) | [x] |
| 17 | `cp_inflate` | input address offset requires two leading bytes (`first_bytes == 2`) | [x] |
| 18 | `cp_inflate` | input address offset requires three leading bytes (`first_bytes == 3`) | [x] |
| 19 | `cp_inflate` | no final partial input word (`last_bytes == 0`) | [x] |
| 20 | `cp_inflate` | final partial input word has one byte | [x] |
| 21 | `cp_inflate` | final partial input word has two bytes | [x] |
| 22 | `cp_inflate` | final partial input word has three bytes | [x] |

## `unfilter`: no and one row

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 23 | `unfilter` | `h <= 0`; no filter byte is read | [x] |
| 24 | `unfilter` | one narrow row, first filter `0` (None) | [x] |
| 25 | `unfilter` | one narrow row, first filter `1` (Sub) | [x] |
| 26 | `unfilter` | one narrow row, first filter `2` (Up) | [x] |
| 27 | `unfilter` | one narrow row, first filter `3` (Average) | [x] |
| 28 | `unfilter` | one narrow row, first filter `4` (Paeth) | [x] |
| 29 | `unfilter` | one wide row, first filter `0` (None) | [x] |
| 30 | `unfilter` | one wide row, first filter `1` (Sub) | [x] |
| 31 | `unfilter` | one wide row, first filter `2` (Up) | [x] |
| 32 | `unfilter` | one wide row, first filter `3` (Average) | [x] |
| 33 | `unfilter` | one wide row, first filter `4` (Paeth) | [x] |

## `unfilter`: two rows

Each cell names `first-filter/later-filter`. Rows are split by width because
the later-row prefix and remainder loops differ at `x == bpp`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 34 | `unfilter` | two narrow rows, filters `0/0` | [x] |
| 35 | `unfilter` | two narrow rows, filters `0/1` | [x] |
| 36 | `unfilter` | two narrow rows, filters `0/2` | [x] |
| 37 | `unfilter` | two narrow rows, filters `0/3` | [x] |
| 38 | `unfilter` | two narrow rows, filters `0/4` | [x] |
| 39 | `unfilter` | two narrow rows, filters `1/0` | [x] |
| 40 | `unfilter` | two narrow rows, filters `1/1` | [x] |
| 41 | `unfilter` | two narrow rows, filters `1/2` | [x] |
| 42 | `unfilter` | two narrow rows, filters `1/3` | [x] |
| 43 | `unfilter` | two narrow rows, filters `1/4` | [x] |
| 44 | `unfilter` | two narrow rows, filters `2/0` | [x] |
| 45 | `unfilter` | two narrow rows, filters `2/1` | [x] |
| 46 | `unfilter` | two narrow rows, filters `2/2` | [x] |
| 47 | `unfilter` | two narrow rows, filters `2/3` | [x] |
| 48 | `unfilter` | two narrow rows, filters `2/4` | [x] |
| 49 | `unfilter` | two narrow rows, filters `3/0` | [x] |
| 50 | `unfilter` | two narrow rows, filters `3/1` | [x] |
| 51 | `unfilter` | two narrow rows, filters `3/2` | [x] |
| 52 | `unfilter` | two narrow rows, filters `3/3` | [x] |
| 53 | `unfilter` | two narrow rows, filters `3/4` | [x] |
| 54 | `unfilter` | two narrow rows, filters `4/0` | [x] |
| 55 | `unfilter` | two narrow rows, filters `4/1` | [x] |
| 56 | `unfilter` | two narrow rows, filters `4/2` | [x] |
| 57 | `unfilter` | two narrow rows, filters `4/3` | [x] |
| 58 | `unfilter` | two narrow rows, filters `4/4` | [x] |
| 59 | `unfilter` | two wide rows, filters `0/0` | [x] |
| 60 | `unfilter` | two wide rows, filters `0/1` | [x] |
| 61 | `unfilter` | two wide rows, filters `0/2` | [x] |
| 62 | `unfilter` | two wide rows, filters `0/3` | [x] |
| 63 | `unfilter` | two wide rows, filters `0/4` | [x] |
| 64 | `unfilter` | two wide rows, filters `1/0` | [x] |
| 65 | `unfilter` | two wide rows, filters `1/1` | [x] |
| 66 | `unfilter` | two wide rows, filters `1/2` | [x] |
| 67 | `unfilter` | two wide rows, filters `1/3` | [x] |
| 68 | `unfilter` | two wide rows, filters `1/4` | [x] |
| 69 | `unfilter` | two wide rows, filters `2/0` | [x] |
| 70 | `unfilter` | two wide rows, filters `2/1` | [x] |
| 71 | `unfilter` | two wide rows, filters `2/2` | [x] |
| 72 | `unfilter` | two wide rows, filters `2/3` | [x] |
| 73 | `unfilter` | two wide rows, filters `2/4` | [x] |
| 74 | `unfilter` | two wide rows, filters `3/0` | [x] |
| 75 | `unfilter` | two wide rows, filters `3/1` | [x] |
| 76 | `unfilter` | two wide rows, filters `3/2` | [x] |
| 77 | `unfilter` | two wide rows, filters `3/3` | [x] |
| 78 | `unfilter` | two wide rows, filters `3/4` | [x] |
| 79 | `unfilter` | two wide rows, filters `4/0` | [x] |
| 80 | `unfilter` | two wide rows, filters `4/1` | [x] |
| 81 | `unfilter` | two wide rows, filters `4/2` | [x] |
| 82 | `unfilter` | two wide rows, filters `4/3` | [x] |
| 83 | `unfilter` | two wide rows, filters `4/4` | [x] |

## `unfilter`: many rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 84 | `unfilter` | three or more narrow rows; every row filter independently ranges over `0..=4` | [x] |
| 85 | `unfilter` | three or more wide rows; every row filter independently ranges over `0..=4` | [x] |

Coverage is implemented in `tests/differential.rs`. Each checked row calls
both shared libraries through `libloading`; randomized rows use fixed seeds.
