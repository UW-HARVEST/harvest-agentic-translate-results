# Configuration Surface

The public header exposes one entry point:

```c
int dequantize_granule(float *grbuf, bs_t *bs, L12_scale_info *sci,
                       int group_size);
```

Mechanically derived branch axes:

- `group_size`: zero, one, or many output samples per group.
- `sci->total_bands`: zero, one, or many bands; the loop reads exactly
  `2 * total_bands` entries from `bitalloc[64]`, so the maximum in-array value
  is 32.
- `sci->bitalloc[i]`: zero (skip), 1 (direct one-bit edge), 2..15 (direct),
  16 (direct upper edge), or 17 and above (grouped-code branch).
- Grouped-code formula subshapes: `ba=17` (`mod=3`, 5-bit code), `ba=18`
  (`mod=5`, 7-bit code), `ba=19` (`mod=9`, 10-bit code), `ba=20`
  (`mod=17`, 17-bit code), and `ba=21` (`mod=33`, 31-bit code).
- Bit-reader position: byte-aligned or each unaligned offset 1..7.
- Bit-reader width: contained in the current byte, crossing one byte boundary,
  or crossing multiple byte boundaries.
- Bitstream capacity: sufficient for every read, exactly exhausted, or
  exceeded (the exceeded case is E01 in `ERRORS.md`).
- Band composition: homogeneous allocation classes or mixed skipped/direct/
  grouped entries, which also exercises the alternating `choff` destination
  layout (`576`, then `-558` via `choff = 18 - choff`).
- `scf`, `stereo_bands`, and `scfcod` are public state but are never read by
  the C implementation; inert-value variation is included in mixed cases.

There are no Cargo features and no C preprocessor feature branches.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| C01 | `dequantize_granule` | `total_bands=0`, `group_size=0`; empty operation | [x] |
| C02 | `dequantize_granule` | `total_bands=0`, `group_size=1` and many; return-only operation | [x] |
| C03 | `dequantize_granule` | one band pair, all `bitalloc=0`, `group_size=1` and many | [x] |
| C04 | `dequantize_granule` | one band pair, direct `ba=1`, aligned, read contained in one byte | [x] |
| C05 | `dequantize_granule` | one band pair, direct `ba=2..7`, aligned and contained reads | [x] |
| C06 | `dequantize_granule` | one band pair, direct `ba=2..7`, unaligned offsets 1..7, including one-boundary crossings | [x] |
| C07 | `dequantize_granule` | one band pair, direct `ba=8..15`, aligned and unaligned, one/multiple-boundary reads | [x] |
| C08 | `dequantize_granule` | one band pair, direct upper edge `ba=16`, aligned and unaligned | [x] |
| C09 | `dequantize_granule` | one band pair, grouped `ba=17` (`mod=3`, 5-bit code), all alignments | [x] |
| C10 | `dequantize_granule` | one band pair, grouped `ba=18` (`mod=5`, 7-bit code), all alignments | [x] |
| C11 | `dequantize_granule` | one band pair, grouped `ba=19` (`mod=9`, 10-bit code), all alignments | [x] |
| C12 | `dequantize_granule` | one band pair, grouped `ba=20` (`mod=17`, 17-bit code), all alignments | [x] |
| C13 | `dequantize_granule` | one band pair, grouped `ba=21` (`mod=33`, 31-bit code), all alignments | [x] |
| C14 | `dequantize_granule` | multiple bands with homogeneous direct allocations; `group_size=1` | [x] |
| C15 | `dequantize_granule` | multiple bands with homogeneous direct allocations; `group_size=3` and `12` | [x] |
| C16 | `dequantize_granule` | multiple bands with homogeneous grouped allocations; `group_size=1`, `3`, and `12` | [x] |
| C17 | `dequantize_granule` | multiple bands mixing skipped, direct, and grouped allocations; varied inert fields | [x] |
| C18 | `dequantize_granule` | maximum in-array band count `total_bands=32`, mixed allocations, `group_size=1` | [x] |
| C19 | `dequantize_granule` | maximum in-array band count `total_bands=32`, mixed allocations, `group_size=3` and `12` | [x] |
| C20 | `dequantize_granule` | sufficient bitstream ending exactly at `limit` on the final read | [x] |
| C21 | `dequantize_granule` | randomized aligned bitstreams across every allocation class and group shape | [x] |
| C22 | `dequantize_granule` | randomized unaligned bitstreams across offsets 1..7, every allocation class, and group shape | [x] |
| C23 | `dequantize_granule` | `group_size=0` with active direct versus grouped allocations; direct consumes no bits while grouped consumes one code per entry and group | [x] |
