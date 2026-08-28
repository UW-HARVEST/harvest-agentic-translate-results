# Configuration Surface

Source-derived axes:

- Public entry point: `dequantize_granule` (the complete public header surface).
- `total_bands`: empty (`0`), one (`1`), many, and the array-boundary value
  (`32`, because the loop reads `2 * total_bands` entries from `bitalloc[64]`).
- `group_size`: zero, one, the conventional grouped size `3`, and a larger
  value `4`; it controls both the inner sample loop and each outer-group base.
- Allocation mode: zero (`ba == 0`), direct (`1..16`), and grouped (`17..21`).
  Values above `21` can request shifts wider than the C `uint32_t` result and
  are outside the C implementation's defined arithmetic domain.
- Bit-reader shape: each starting bit offset (`bs.pos & 7 == 0..7`), reads
  contained in one byte or crossing bytes, exact-limit reads, and padded reads.
- Layout shape: one slot, paired channel slots, alternating `576`/`18` output
  offsets, and mixed modes over all 64 allocation slots.

There are no Cargo features. The only feature configurations are the equivalent
default and `--no-default-features` builds.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dequantize_granule` | `total_bands=0`; `group_size` in `0,1,3,4`; each starting bit offset | [x] |
| 2 | `dequantize_granule` | `group_size=0`; one/many bands; allocations only zero or direct (`0..16`) | [x] |
| 3 | `dequantize_granule` | `group_size=0`; one/many bands; grouped allocations (`17..21`) still consume one code per nonzero slot | [x] |
| 4 | `dequantize_granule` | one/many/max bands; all `bitalloc=0`; `group_size=1,3,4`; destination remains unchanged | [x] |
| 5 | `dequantize_granule` | direct boundary `bitalloc=1`; aligned and unaligned one-bit reads | [x] |
| 6 | `dequantize_granule` | direct `bitalloc=2..7`; every starting bit offset, including byte crossings | [x] |
| 7 | `dequantize_granule` | direct byte-width `bitalloc=8`; every starting bit offset | [x] |
| 8 | `dequantize_granule` | direct `bitalloc=9..15`; every starting bit offset and multi-byte reads | [x] |
| 9 | `dequantize_granule` | direct boundary `bitalloc=16`; every starting bit offset | [x] |
| 10 | `dequantize_granule` | grouped boundary `bitalloc=17` (`mod=3`, five-bit code); every starting bit offset | [x] |
| 11 | `dequantize_granule` | grouped `bitalloc=18` (`mod=5`, seven-bit code); every starting bit offset | [x] |
| 12 | `dequantize_granule` | grouped `bitalloc=19` (`mod=9`, ten-bit code); every starting bit offset | [x] |
| 13 | `dequantize_granule` | grouped `bitalloc=20` (`mod=17`, seventeen-bit code); every starting bit offset | [x] |
| 14 | `dequantize_granule` | grouped arithmetic boundary `bitalloc=21` (`mod=33`, thirty-one-bit code); every starting bit offset | [x] |
| 15 | `dequantize_granule` | paired channel slots with mixed zero/direct/grouped allocations; `group_size=1` | [x] |
| 16 | `dequantize_granule` | many bands with alternating `576`/`18` layout and mixed allocations; `group_size=3` | [x] |
| 17 | `dequantize_granule` | maximum `total_bands=32`, all 64 slots mixed, larger `group_size=4` | [x] |
| 18 | `dequantize_granule` | sufficient input with final successful read exactly at `bs.limit` | [x] |
| 19 | `dequantize_granule` | same valid shapes with `bs.limit` padded beyond the final read | [x] |
