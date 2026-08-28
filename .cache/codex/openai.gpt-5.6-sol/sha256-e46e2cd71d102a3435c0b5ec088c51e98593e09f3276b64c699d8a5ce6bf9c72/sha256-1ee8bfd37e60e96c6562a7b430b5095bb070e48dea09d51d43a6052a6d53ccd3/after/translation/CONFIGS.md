# Configuration Surface

The crate declares no Cargo features and the C API exposes no runtime mode or
option setters. Rows below enumerate the cross-product combinations that the C
source treats differently through loops and pointer comparisons. Randomized
values are used within every applicable row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `create_block` | arbitrary `int` id, NUL-terminated name of 0..31 bytes, arbitrary `uint8_t` flags | [x] |
| 2 | `allocate_block`, `free_block` | `count == 0` (empty allocation), arbitrary init value | [x] |
| 3 | `allocate_block`, `free_block` | `count == 1` (single element), arbitrary init value | [x] |
| 4 | `allocate_block`, `free_block` | `count > 1` (many elements), arbitrary init value with defined C additions | [x] |
| 5 | `free_block` | `mb == NULL` | [x] |
| 6 | `free_block` | nonnull `mb`, `mb->data == NULL` | [x] |
| 7 | `free_block` | nonnull `mb`, nonnull `mb->data` | [x] |
| 8 | `compute_hash` | `mb1 < mb2`, `mb1->data < mb2->data` | [x] |
| 9 | `compute_hash` | `mb1 < mb2`, `mb1->data == mb2->data` | [x] |
| 10 | `compute_hash` | `mb1 < mb2`, `mb1->data > mb2->data` | [x] |
| 11 | `compute_hash` | `mb1 == mb2` (therefore data pointers equal) | [x] |
| 12 | `compute_hash` | `mb1 > mb2`, `mb1->data < mb2->data` | [x] |
| 13 | `compute_hash` | `mb1 > mb2`, `mb1->data == mb2->data` | [x] |
| 14 | `compute_hash` | `mb1 > mb2`, `mb1->data > mb2->data` | [x] |
| 15 | `betagamma` | `(param1 % 10) + 5 == 0`; empty internal arrays, fixed three-block flag matrix, randomized remaining params | [x] |
| 16 | `betagamma` | `(param1 % 10) + 5 == 1`; one-element internal arrays, fixed three-block flag matrix, randomized remaining params | [x] |
| 17 | `betagamma` | `(param1 % 10) + 5` in `2..=14`; many-element internal arrays, fixed three-block flag matrix, randomized params | [x] |
