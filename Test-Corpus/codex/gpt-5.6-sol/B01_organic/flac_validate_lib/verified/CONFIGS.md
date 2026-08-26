# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so there is exactly one valid feature
combination: the empty set, built with `--no-default-features`.

`c_src/CMakeLists.txt` defines no options or conditional sources. Its only
library target always compiles `c_src/src/lib.c`.

## Runtime Axes

The validator rows use these source-derived classes:

- `D16`: `bitdepth` in `1..=16`; `D31`: `17..=31`; `D32`: exactly `32`.
- `C2`: `channels == 2`; `CN`: `channels` is `1` or `3..=8`.
- `M0`: `channel_mode == 0`; `MV`: `1..=3`; `MX`: out-of-range `4..=255`.
- `R0`: `max_rice_value == 0`; `RE`: explicit value in `1..=30`.
- `PE`: `min_partition_order == max_partition_order`.
- `PN`: `min_partition_order < max_partition_order`, but `blocksize` is not
  divisible by `1 << (min_partition_order + 1)`.
- `PM`: `min_partition_order < max_partition_order`, and divisibility lets the
  loop increment through `max_partition_order`.
- `PS`: the loop increments at least once, then divisibility fails before
  `max_partition_order`.

Every validator row also randomizes `samplerate` over `1..=655350`, valid
values within each class, preexisting output fields, and valid block sizes
that preserve the selected partition shape. The cross-product is retained
because the options mutate shared state in sequence.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| 1 | `tflac_size_memory` | every `u32` block size, including zero, alignment edges, and arithmetic-wrap boundaries | [x] |
| 2 | `flac_validate` | D16/C2/M0/R0/PE | [x] |
| 3 | `flac_validate` | D16/C2/M0/R0/PN | [x] |
| 4 | `flac_validate` | D16/C2/M0/R0/PM | [x] |
| 5 | `flac_validate` | D16/C2/M0/R0/PS | [x] |
| 6 | `flac_validate` | D16/C2/M0/RE/PE | [x] |
| 7 | `flac_validate` | D16/C2/M0/RE/PN | [x] |
| 8 | `flac_validate` | D16/C2/M0/RE/PM | [x] |
| 9 | `flac_validate` | D16/C2/M0/RE/PS | [x] |
| 10 | `flac_validate` | D16/C2/MV/R0/PE | [x] |
| 11 | `flac_validate` | D16/C2/MV/R0/PN | [x] |
| 12 | `flac_validate` | D16/C2/MV/R0/PM | [x] |
| 13 | `flac_validate` | D16/C2/MV/R0/PS | [x] |
| 14 | `flac_validate` | D16/C2/MV/RE/PE | [x] |
| 15 | `flac_validate` | D16/C2/MV/RE/PN | [x] |
| 16 | `flac_validate` | D16/C2/MV/RE/PM | [x] |
| 17 | `flac_validate` | D16/C2/MV/RE/PS | [x] |
| 18 | `flac_validate` | D16/C2/MX/R0/PE | [x] |
| 19 | `flac_validate` | D16/C2/MX/R0/PN | [x] |
| 20 | `flac_validate` | D16/C2/MX/R0/PM | [x] |
| 21 | `flac_validate` | D16/C2/MX/R0/PS | [x] |
| 22 | `flac_validate` | D16/C2/MX/RE/PE | [x] |
| 23 | `flac_validate` | D16/C2/MX/RE/PN | [x] |
| 24 | `flac_validate` | D16/C2/MX/RE/PM | [x] |
| 25 | `flac_validate` | D16/C2/MX/RE/PS | [x] |
| 26 | `flac_validate` | D16/CN/M0/R0/PE | [x] |
| 27 | `flac_validate` | D16/CN/M0/R0/PN | [x] |
| 28 | `flac_validate` | D16/CN/M0/R0/PM | [x] |
| 29 | `flac_validate` | D16/CN/M0/R0/PS | [x] |
| 30 | `flac_validate` | D16/CN/M0/RE/PE | [x] |
| 31 | `flac_validate` | D16/CN/M0/RE/PN | [x] |
| 32 | `flac_validate` | D16/CN/M0/RE/PM | [x] |
| 33 | `flac_validate` | D16/CN/M0/RE/PS | [x] |
| 34 | `flac_validate` | D16/CN/MV/R0/PE | [x] |
| 35 | `flac_validate` | D16/CN/MV/R0/PN | [x] |
| 36 | `flac_validate` | D16/CN/MV/R0/PM | [x] |
| 37 | `flac_validate` | D16/CN/MV/R0/PS | [x] |
| 38 | `flac_validate` | D16/CN/MV/RE/PE | [x] |
| 39 | `flac_validate` | D16/CN/MV/RE/PN | [x] |
| 40 | `flac_validate` | D16/CN/MV/RE/PM | [x] |
| 41 | `flac_validate` | D16/CN/MV/RE/PS | [x] |
| 42 | `flac_validate` | D16/CN/MX/R0/PE | [x] |
| 43 | `flac_validate` | D16/CN/MX/R0/PN | [x] |
| 44 | `flac_validate` | D16/CN/MX/R0/PM | [x] |
| 45 | `flac_validate` | D16/CN/MX/R0/PS | [x] |
| 46 | `flac_validate` | D16/CN/MX/RE/PE | [x] |
| 47 | `flac_validate` | D16/CN/MX/RE/PN | [x] |
| 48 | `flac_validate` | D16/CN/MX/RE/PM | [x] |
| 49 | `flac_validate` | D16/CN/MX/RE/PS | [x] |
| 50 | `flac_validate` | D31/C2/M0/R0/PE | [x] |
| 51 | `flac_validate` | D31/C2/M0/R0/PN | [x] |
| 52 | `flac_validate` | D31/C2/M0/R0/PM | [x] |
| 53 | `flac_validate` | D31/C2/M0/R0/PS | [x] |
| 54 | `flac_validate` | D31/C2/M0/RE/PE | [x] |
| 55 | `flac_validate` | D31/C2/M0/RE/PN | [x] |
| 56 | `flac_validate` | D31/C2/M0/RE/PM | [x] |
| 57 | `flac_validate` | D31/C2/M0/RE/PS | [x] |
| 58 | `flac_validate` | D31/C2/MV/R0/PE | [x] |
| 59 | `flac_validate` | D31/C2/MV/R0/PN | [x] |
| 60 | `flac_validate` | D31/C2/MV/R0/PM | [x] |
| 61 | `flac_validate` | D31/C2/MV/R0/PS | [x] |
| 62 | `flac_validate` | D31/C2/MV/RE/PE | [x] |
| 63 | `flac_validate` | D31/C2/MV/RE/PN | [x] |
| 64 | `flac_validate` | D31/C2/MV/RE/PM | [x] |
| 65 | `flac_validate` | D31/C2/MV/RE/PS | [x] |
| 66 | `flac_validate` | D31/C2/MX/R0/PE | [x] |
| 67 | `flac_validate` | D31/C2/MX/R0/PN | [x] |
| 68 | `flac_validate` | D31/C2/MX/R0/PM | [x] |
| 69 | `flac_validate` | D31/C2/MX/R0/PS | [x] |
| 70 | `flac_validate` | D31/C2/MX/RE/PE | [x] |
| 71 | `flac_validate` | D31/C2/MX/RE/PN | [x] |
| 72 | `flac_validate` | D31/C2/MX/RE/PM | [x] |
| 73 | `flac_validate` | D31/C2/MX/RE/PS | [x] |
| 74 | `flac_validate` | D31/CN/M0/R0/PE | [x] |
| 75 | `flac_validate` | D31/CN/M0/R0/PN | [x] |
| 76 | `flac_validate` | D31/CN/M0/R0/PM | [x] |
| 77 | `flac_validate` | D31/CN/M0/R0/PS | [x] |
| 78 | `flac_validate` | D31/CN/M0/RE/PE | [x] |
| 79 | `flac_validate` | D31/CN/M0/RE/PN | [x] |
| 80 | `flac_validate` | D31/CN/M0/RE/PM | [x] |
| 81 | `flac_validate` | D31/CN/M0/RE/PS | [x] |
| 82 | `flac_validate` | D31/CN/MV/R0/PE | [x] |
| 83 | `flac_validate` | D31/CN/MV/R0/PN | [x] |
| 84 | `flac_validate` | D31/CN/MV/R0/PM | [x] |
| 85 | `flac_validate` | D31/CN/MV/R0/PS | [x] |
| 86 | `flac_validate` | D31/CN/MV/RE/PE | [x] |
| 87 | `flac_validate` | D31/CN/MV/RE/PN | [x] |
| 88 | `flac_validate` | D31/CN/MV/RE/PM | [x] |
| 89 | `flac_validate` | D31/CN/MV/RE/PS | [x] |
| 90 | `flac_validate` | D31/CN/MX/R0/PE | [x] |
| 91 | `flac_validate` | D31/CN/MX/R0/PN | [x] |
| 92 | `flac_validate` | D31/CN/MX/R0/PM | [x] |
| 93 | `flac_validate` | D31/CN/MX/R0/PS | [x] |
| 94 | `flac_validate` | D31/CN/MX/RE/PE | [x] |
| 95 | `flac_validate` | D31/CN/MX/RE/PN | [x] |
| 96 | `flac_validate` | D31/CN/MX/RE/PM | [x] |
| 97 | `flac_validate` | D31/CN/MX/RE/PS | [x] |
| 98 | `flac_validate` | D32/C2/M0/R0/PE | [x] |
| 99 | `flac_validate` | D32/C2/M0/R0/PN | [x] |
| 100 | `flac_validate` | D32/C2/M0/R0/PM | [x] |
| 101 | `flac_validate` | D32/C2/M0/R0/PS | [x] |
| 102 | `flac_validate` | D32/C2/M0/RE/PE | [x] |
| 103 | `flac_validate` | D32/C2/M0/RE/PN | [x] |
| 104 | `flac_validate` | D32/C2/M0/RE/PM | [x] |
| 105 | `flac_validate` | D32/C2/M0/RE/PS | [x] |
| 106 | `flac_validate` | D32/C2/MV/R0/PE | [x] |
| 107 | `flac_validate` | D32/C2/MV/R0/PN | [x] |
| 108 | `flac_validate` | D32/C2/MV/R0/PM | [x] |
| 109 | `flac_validate` | D32/C2/MV/R0/PS | [x] |
| 110 | `flac_validate` | D32/C2/MV/RE/PE | [x] |
| 111 | `flac_validate` | D32/C2/MV/RE/PN | [x] |
| 112 | `flac_validate` | D32/C2/MV/RE/PM | [x] |
| 113 | `flac_validate` | D32/C2/MV/RE/PS | [x] |
| 114 | `flac_validate` | D32/C2/MX/R0/PE | [x] |
| 115 | `flac_validate` | D32/C2/MX/R0/PN | [x] |
| 116 | `flac_validate` | D32/C2/MX/R0/PM | [x] |
| 117 | `flac_validate` | D32/C2/MX/R0/PS | [x] |
| 118 | `flac_validate` | D32/C2/MX/RE/PE | [x] |
| 119 | `flac_validate` | D32/C2/MX/RE/PN | [x] |
| 120 | `flac_validate` | D32/C2/MX/RE/PM | [x] |
| 121 | `flac_validate` | D32/C2/MX/RE/PS | [x] |
| 122 | `flac_validate` | D32/CN/M0/R0/PE | [x] |
| 123 | `flac_validate` | D32/CN/M0/R0/PN | [x] |
| 124 | `flac_validate` | D32/CN/M0/R0/PM | [x] |
| 125 | `flac_validate` | D32/CN/M0/R0/PS | [x] |
| 126 | `flac_validate` | D32/CN/M0/RE/PE | [x] |
| 127 | `flac_validate` | D32/CN/M0/RE/PN | [x] |
| 128 | `flac_validate` | D32/CN/M0/RE/PM | [x] |
| 129 | `flac_validate` | D32/CN/M0/RE/PS | [x] |
| 130 | `flac_validate` | D32/CN/MV/R0/PE | [x] |
| 131 | `flac_validate` | D32/CN/MV/R0/PN | [x] |
| 132 | `flac_validate` | D32/CN/MV/R0/PM | [x] |
| 133 | `flac_validate` | D32/CN/MV/R0/PS | [x] |
| 134 | `flac_validate` | D32/CN/MV/RE/PE | [x] |
| 135 | `flac_validate` | D32/CN/MV/RE/PN | [x] |
| 136 | `flac_validate` | D32/CN/MV/RE/PM | [x] |
| 137 | `flac_validate` | D32/CN/MV/RE/PS | [x] |
| 138 | `flac_validate` | D32/CN/MX/R0/PE | [x] |
| 139 | `flac_validate` | D32/CN/MX/R0/PN | [x] |
| 140 | `flac_validate` | D32/CN/MX/R0/PM | [x] |
| 141 | `flac_validate` | D32/CN/MX/R0/PS | [x] |
| 142 | `flac_validate` | D32/CN/MX/RE/PE | [x] |
| 143 | `flac_validate` | D32/CN/MX/RE/PN | [x] |
| 144 | `flac_validate` | D32/CN/MX/RE/PM | [x] |
| 145 | `flac_validate` | D32/CN/MX/RE/PS | [x] |
