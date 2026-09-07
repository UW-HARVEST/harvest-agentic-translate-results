# Configuration surface

Mechanically derived from the public header and every input-dependent branch in
`c_src/src/lib.c`. The only public entry point is the lowest-level function
`read_side_info`.

Configuration shorthand:

- `M25/sf0`: MPEG-2.5 header, sample-rate code 0 or 1 (both select table index 0).
- `M25/sf1`: MPEG-2.5 header, sample-rate code 2 (table index 1).
- `M2/sf2..4`: MPEG-2 header, sample-rate codes 0..2.
- `M2/sf5`: MPEG-2 header, sample-rate code 3.
- `R/sf2..5`: reserved version bits `01`, sample-rate codes 0..3. Bit 3
  still selects the C implementation's MPEG-1 field widths and explicit
  preflag path.
- `M1/sf5..7`: MPEG-1 header, sample-rate codes 0..2.
- `M1/sf8`: MPEG-1 header, sample-rate code 3. The C stores the computed
  one-past scalefactor-table pointer; tests compare all defined scalar bytes,
  pointer nullness, and pointer aliasing without dereferencing that pointer.
- `mono` is `hdr[3] & 0xc0 == 0xc0`; `stereo` is every other channel-mode value.
- `long`: `window_switching_flag=0`.
- `start`: switched `block_type=1`, with `mixed_block_flag` randomized 0/1.
- `short`: switched `block_type=2`, `mixed_block_flag=0`.
- `mixed`: switched `block_type=2`, `mixed_block_flag=1`.
- `stop`: switched `block_type=3`, with `mixed_block_flag` randomized 0/1.
- `pf0`/`pf1` for M25/M2 mean `scalefac_compress < 500` / `500..511`,
  selecting derived `preflag=0` / `1`.
- MPEG-1 reads an explicit preflag bit; every M1 row randomizes it across 0/1.
- Every row randomizes all non-branch field values, SCFSI patterns,
  `main_data_begin`, reservoir-valid `part_23_length`, table selections, gains,
  region counts, and initial bit offsets 0..7 (the `get_bits` alignment branch)
  with a fixed seed.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `read_side_info` | M25/sf0, mono, long, pf0 | [x] |
| 2 | `read_side_info` | M25/sf0, mono, long, pf1 | [x] |
| 3 | `read_side_info` | M25/sf0, mono, start, pf0 | [x] |
| 4 | `read_side_info` | M25/sf0, mono, start, pf1 | [x] |
| 5 | `read_side_info` | M25/sf0, mono, short, pf0 | [x] |
| 6 | `read_side_info` | M25/sf0, mono, short, pf1 | [x] |
| 7 | `read_side_info` | M25/sf0, mono, mixed, pf0 | [x] |
| 8 | `read_side_info` | M25/sf0, mono, mixed, pf1 | [x] |
| 9 | `read_side_info` | M25/sf0, mono, stop, pf0 | [x] |
| 10 | `read_side_info` | M25/sf0, mono, stop, pf1 | [x] |
| 11 | `read_side_info` | M25/sf0, stereo, long, pf0 | [x] |
| 12 | `read_side_info` | M25/sf0, stereo, long, pf1 | [x] |
| 13 | `read_side_info` | M25/sf0, stereo, start, pf0 | [x] |
| 14 | `read_side_info` | M25/sf0, stereo, start, pf1 | [x] |
| 15 | `read_side_info` | M25/sf0, stereo, short, pf0 | [x] |
| 16 | `read_side_info` | M25/sf0, stereo, short, pf1 | [x] |
| 17 | `read_side_info` | M25/sf0, stereo, mixed, pf0 | [x] |
| 18 | `read_side_info` | M25/sf0, stereo, mixed, pf1 | [x] |
| 19 | `read_side_info` | M25/sf0, stereo, stop, pf0 | [x] |
| 20 | `read_side_info` | M25/sf0, stereo, stop, pf1 | [x] |
| 21 | `read_side_info` | M25/sf1, mono, long, pf0 | [x] |
| 22 | `read_side_info` | M25/sf1, mono, long, pf1 | [x] |
| 23 | `read_side_info` | M25/sf1, mono, start, pf0 | [x] |
| 24 | `read_side_info` | M25/sf1, mono, start, pf1 | [x] |
| 25 | `read_side_info` | M25/sf1, mono, short, pf0 | [x] |
| 26 | `read_side_info` | M25/sf1, mono, short, pf1 | [x] |
| 27 | `read_side_info` | M25/sf1, mono, mixed, pf0 | [x] |
| 28 | `read_side_info` | M25/sf1, mono, mixed, pf1 | [x] |
| 29 | `read_side_info` | M25/sf1, mono, stop, pf0 | [x] |
| 30 | `read_side_info` | M25/sf1, mono, stop, pf1 | [x] |
| 31 | `read_side_info` | M25/sf1, stereo, long, pf0 | [x] |
| 32 | `read_side_info` | M25/sf1, stereo, long, pf1 | [x] |
| 33 | `read_side_info` | M25/sf1, stereo, start, pf0 | [x] |
| 34 | `read_side_info` | M25/sf1, stereo, start, pf1 | [x] |
| 35 | `read_side_info` | M25/sf1, stereo, short, pf0 | [x] |
| 36 | `read_side_info` | M25/sf1, stereo, short, pf1 | [x] |
| 37 | `read_side_info` | M25/sf1, stereo, mixed, pf0 | [x] |
| 38 | `read_side_info` | M25/sf1, stereo, mixed, pf1 | [x] |
| 39 | `read_side_info` | M25/sf1, stereo, stop, pf0 | [x] |
| 40 | `read_side_info` | M25/sf1, stereo, stop, pf1 | [x] |
| 41 | `read_side_info` | M2/sf2, mono, long, pf0 | [x] |
| 42 | `read_side_info` | M2/sf2, mono, long, pf1 | [x] |
| 43 | `read_side_info` | M2/sf2, mono, start, pf0 | [x] |
| 44 | `read_side_info` | M2/sf2, mono, start, pf1 | [x] |
| 45 | `read_side_info` | M2/sf2, mono, short, pf0 | [x] |
| 46 | `read_side_info` | M2/sf2, mono, short, pf1 | [x] |
| 47 | `read_side_info` | M2/sf2, mono, mixed, pf0 | [x] |
| 48 | `read_side_info` | M2/sf2, mono, mixed, pf1 | [x] |
| 49 | `read_side_info` | M2/sf2, mono, stop, pf0 | [x] |
| 50 | `read_side_info` | M2/sf2, mono, stop, pf1 | [x] |
| 51 | `read_side_info` | M2/sf2, stereo, long, pf0 | [x] |
| 52 | `read_side_info` | M2/sf2, stereo, long, pf1 | [x] |
| 53 | `read_side_info` | M2/sf2, stereo, start, pf0 | [x] |
| 54 | `read_side_info` | M2/sf2, stereo, start, pf1 | [x] |
| 55 | `read_side_info` | M2/sf2, stereo, short, pf0 | [x] |
| 56 | `read_side_info` | M2/sf2, stereo, short, pf1 | [x] |
| 57 | `read_side_info` | M2/sf2, stereo, mixed, pf0 | [x] |
| 58 | `read_side_info` | M2/sf2, stereo, mixed, pf1 | [x] |
| 59 | `read_side_info` | M2/sf2, stereo, stop, pf0 | [x] |
| 60 | `read_side_info` | M2/sf2, stereo, stop, pf1 | [x] |
| 61 | `read_side_info` | M2/sf3, mono, long, pf0 | [x] |
| 62 | `read_side_info` | M2/sf3, mono, long, pf1 | [x] |
| 63 | `read_side_info` | M2/sf3, mono, start, pf0 | [x] |
| 64 | `read_side_info` | M2/sf3, mono, start, pf1 | [x] |
| 65 | `read_side_info` | M2/sf3, mono, short, pf0 | [x] |
| 66 | `read_side_info` | M2/sf3, mono, short, pf1 | [x] |
| 67 | `read_side_info` | M2/sf3, mono, mixed, pf0 | [x] |
| 68 | `read_side_info` | M2/sf3, mono, mixed, pf1 | [x] |
| 69 | `read_side_info` | M2/sf3, mono, stop, pf0 | [x] |
| 70 | `read_side_info` | M2/sf3, mono, stop, pf1 | [x] |
| 71 | `read_side_info` | M2/sf3, stereo, long, pf0 | [x] |
| 72 | `read_side_info` | M2/sf3, stereo, long, pf1 | [x] |
| 73 | `read_side_info` | M2/sf3, stereo, start, pf0 | [x] |
| 74 | `read_side_info` | M2/sf3, stereo, start, pf1 | [x] |
| 75 | `read_side_info` | M2/sf3, stereo, short, pf0 | [x] |
| 76 | `read_side_info` | M2/sf3, stereo, short, pf1 | [x] |
| 77 | `read_side_info` | M2/sf3, stereo, mixed, pf0 | [x] |
| 78 | `read_side_info` | M2/sf3, stereo, mixed, pf1 | [x] |
| 79 | `read_side_info` | M2/sf3, stereo, stop, pf0 | [x] |
| 80 | `read_side_info` | M2/sf3, stereo, stop, pf1 | [x] |
| 81 | `read_side_info` | M2/sf4, mono, long, pf0 | [x] |
| 82 | `read_side_info` | M2/sf4, mono, long, pf1 | [x] |
| 83 | `read_side_info` | M2/sf4, mono, start, pf0 | [x] |
| 84 | `read_side_info` | M2/sf4, mono, start, pf1 | [x] |
| 85 | `read_side_info` | M2/sf4, mono, short, pf0 | [x] |
| 86 | `read_side_info` | M2/sf4, mono, short, pf1 | [x] |
| 87 | `read_side_info` | M2/sf4, mono, mixed, pf0 | [x] |
| 88 | `read_side_info` | M2/sf4, mono, mixed, pf1 | [x] |
| 89 | `read_side_info` | M2/sf4, mono, stop, pf0 | [x] |
| 90 | `read_side_info` | M2/sf4, mono, stop, pf1 | [x] |
| 91 | `read_side_info` | M2/sf4, stereo, long, pf0 | [x] |
| 92 | `read_side_info` | M2/sf4, stereo, long, pf1 | [x] |
| 93 | `read_side_info` | M2/sf4, stereo, start, pf0 | [x] |
| 94 | `read_side_info` | M2/sf4, stereo, start, pf1 | [x] |
| 95 | `read_side_info` | M2/sf4, stereo, short, pf0 | [x] |
| 96 | `read_side_info` | M2/sf4, stereo, short, pf1 | [x] |
| 97 | `read_side_info` | M2/sf4, stereo, mixed, pf0 | [x] |
| 98 | `read_side_info` | M2/sf4, stereo, mixed, pf1 | [x] |
| 99 | `read_side_info` | M2/sf4, stereo, stop, pf0 | [x] |
| 100 | `read_side_info` | M2/sf4, stereo, stop, pf1 | [x] |
| 101 | `read_side_info` | M1/sf5, mono, long, explicit preflag 0/1 | [x] |
| 102 | `read_side_info` | M1/sf5, mono, start, explicit preflag 0/1 | [x] |
| 103 | `read_side_info` | M1/sf5, mono, short, explicit preflag 0/1 | [x] |
| 104 | `read_side_info` | M1/sf5, mono, mixed, explicit preflag 0/1 | [x] |
| 105 | `read_side_info` | M1/sf5, mono, stop, explicit preflag 0/1 | [x] |
| 106 | `read_side_info` | M1/sf5, stereo, long, explicit preflag 0/1 | [x] |
| 107 | `read_side_info` | M1/sf5, stereo, start, explicit preflag 0/1 | [x] |
| 108 | `read_side_info` | M1/sf5, stereo, short, explicit preflag 0/1 | [x] |
| 109 | `read_side_info` | M1/sf5, stereo, mixed, explicit preflag 0/1 | [x] |
| 110 | `read_side_info` | M1/sf5, stereo, stop, explicit preflag 0/1 | [x] |
| 111 | `read_side_info` | M1/sf6, mono, long, explicit preflag 0/1 | [x] |
| 112 | `read_side_info` | M1/sf6, mono, start, explicit preflag 0/1 | [x] |
| 113 | `read_side_info` | M1/sf6, mono, short, explicit preflag 0/1 | [x] |
| 114 | `read_side_info` | M1/sf6, mono, mixed, explicit preflag 0/1 | [x] |
| 115 | `read_side_info` | M1/sf6, mono, stop, explicit preflag 0/1 | [x] |
| 116 | `read_side_info` | M1/sf6, stereo, long, explicit preflag 0/1 | [x] |
| 117 | `read_side_info` | M1/sf6, stereo, start, explicit preflag 0/1 | [x] |
| 118 | `read_side_info` | M1/sf6, stereo, short, explicit preflag 0/1 | [x] |
| 119 | `read_side_info` | M1/sf6, stereo, mixed, explicit preflag 0/1 | [x] |
| 120 | `read_side_info` | M1/sf6, stereo, stop, explicit preflag 0/1 | [x] |
| 121 | `read_side_info` | M1/sf7, mono, long, explicit preflag 0/1 | [x] |
| 122 | `read_side_info` | M1/sf7, mono, start, explicit preflag 0/1 | [x] |
| 123 | `read_side_info` | M1/sf7, mono, short, explicit preflag 0/1 | [x] |
| 124 | `read_side_info` | M1/sf7, mono, mixed, explicit preflag 0/1 | [x] |
| 125 | `read_side_info` | M1/sf7, mono, stop, explicit preflag 0/1 | [x] |
| 126 | `read_side_info` | M1/sf7, stereo, long, explicit preflag 0/1 | [x] |
| 127 | `read_side_info` | M1/sf7, stereo, start, explicit preflag 0/1 | [x] |
| 128 | `read_side_info` | M1/sf7, stereo, short, explicit preflag 0/1 | [x] |
| 129 | `read_side_info` | M1/sf7, stereo, mixed, explicit preflag 0/1 | [x] |
| 130 | `read_side_info` | M1/sf7, stereo, stop, explicit preflag 0/1 | [x] |
| 131 | `read_side_info` | M2/sf5, mono, long, pf0 | [x] |
| 132 | `read_side_info` | M2/sf5, mono, long, pf1 | [x] |
| 133 | `read_side_info` | M2/sf5, mono, start, pf0 | [x] |
| 134 | `read_side_info` | M2/sf5, mono, start, pf1 | [x] |
| 135 | `read_side_info` | M2/sf5, mono, short, pf0 | [x] |
| 136 | `read_side_info` | M2/sf5, mono, short, pf1 | [x] |
| 137 | `read_side_info` | M2/sf5, mono, mixed, pf0 | [x] |
| 138 | `read_side_info` | M2/sf5, mono, mixed, pf1 | [x] |
| 139 | `read_side_info` | M2/sf5, mono, stop, pf0 | [x] |
| 140 | `read_side_info` | M2/sf5, mono, stop, pf1 | [x] |
| 141 | `read_side_info` | M2/sf5, stereo, long, pf0 | [x] |
| 142 | `read_side_info` | M2/sf5, stereo, long, pf1 | [x] |
| 143 | `read_side_info` | M2/sf5, stereo, start, pf0 | [x] |
| 144 | `read_side_info` | M2/sf5, stereo, start, pf1 | [x] |
| 145 | `read_side_info` | M2/sf5, stereo, short, pf0 | [x] |
| 146 | `read_side_info` | M2/sf5, stereo, short, pf1 | [x] |
| 147 | `read_side_info` | M2/sf5, stereo, mixed, pf0 | [x] |
| 148 | `read_side_info` | M2/sf5, stereo, mixed, pf1 | [x] |
| 149 | `read_side_info` | M2/sf5, stereo, stop, pf0 | [x] |
| 150 | `read_side_info` | M2/sf5, stereo, stop, pf1 | [x] |
| 151 | `read_side_info` | R/sf2, mono, long, explicit preflag 0/1 | [x] |
| 152 | `read_side_info` | R/sf2, mono, start, explicit preflag 0/1 | [x] |
| 153 | `read_side_info` | R/sf2, mono, short, explicit preflag 0/1 | [x] |
| 154 | `read_side_info` | R/sf2, mono, mixed, explicit preflag 0/1 | [x] |
| 155 | `read_side_info` | R/sf2, mono, stop, explicit preflag 0/1 | [x] |
| 156 | `read_side_info` | R/sf2, stereo, long, explicit preflag 0/1 | [x] |
| 157 | `read_side_info` | R/sf2, stereo, start, explicit preflag 0/1 | [x] |
| 158 | `read_side_info` | R/sf2, stereo, short, explicit preflag 0/1 | [x] |
| 159 | `read_side_info` | R/sf2, stereo, mixed, explicit preflag 0/1 | [x] |
| 160 | `read_side_info` | R/sf2, stereo, stop, explicit preflag 0/1 | [x] |
| 161 | `read_side_info` | R/sf3, mono, long, explicit preflag 0/1 | [x] |
| 162 | `read_side_info` | R/sf3, mono, start, explicit preflag 0/1 | [x] |
| 163 | `read_side_info` | R/sf3, mono, short, explicit preflag 0/1 | [x] |
| 164 | `read_side_info` | R/sf3, mono, mixed, explicit preflag 0/1 | [x] |
| 165 | `read_side_info` | R/sf3, mono, stop, explicit preflag 0/1 | [x] |
| 166 | `read_side_info` | R/sf3, stereo, long, explicit preflag 0/1 | [x] |
| 167 | `read_side_info` | R/sf3, stereo, start, explicit preflag 0/1 | [x] |
| 168 | `read_side_info` | R/sf3, stereo, short, explicit preflag 0/1 | [x] |
| 169 | `read_side_info` | R/sf3, stereo, mixed, explicit preflag 0/1 | [x] |
| 170 | `read_side_info` | R/sf3, stereo, stop, explicit preflag 0/1 | [x] |
| 171 | `read_side_info` | R/sf4, mono, long, explicit preflag 0/1 | [x] |
| 172 | `read_side_info` | R/sf4, mono, start, explicit preflag 0/1 | [x] |
| 173 | `read_side_info` | R/sf4, mono, short, explicit preflag 0/1 | [x] |
| 174 | `read_side_info` | R/sf4, mono, mixed, explicit preflag 0/1 | [x] |
| 175 | `read_side_info` | R/sf4, mono, stop, explicit preflag 0/1 | [x] |
| 176 | `read_side_info` | R/sf4, stereo, long, explicit preflag 0/1 | [x] |
| 177 | `read_side_info` | R/sf4, stereo, start, explicit preflag 0/1 | [x] |
| 178 | `read_side_info` | R/sf4, stereo, short, explicit preflag 0/1 | [x] |
| 179 | `read_side_info` | R/sf4, stereo, mixed, explicit preflag 0/1 | [x] |
| 180 | `read_side_info` | R/sf4, stereo, stop, explicit preflag 0/1 | [x] |
| 181 | `read_side_info` | R/sf5, mono, long, explicit preflag 0/1 | [x] |
| 182 | `read_side_info` | R/sf5, mono, start, explicit preflag 0/1 | [x] |
| 183 | `read_side_info` | R/sf5, mono, short, explicit preflag 0/1 | [x] |
| 184 | `read_side_info` | R/sf5, mono, mixed, explicit preflag 0/1 | [x] |
| 185 | `read_side_info` | R/sf5, mono, stop, explicit preflag 0/1 | [x] |
| 186 | `read_side_info` | R/sf5, stereo, long, explicit preflag 0/1 | [x] |
| 187 | `read_side_info` | R/sf5, stereo, start, explicit preflag 0/1 | [x] |
| 188 | `read_side_info` | R/sf5, stereo, short, explicit preflag 0/1 | [x] |
| 189 | `read_side_info` | R/sf5, stereo, mixed, explicit preflag 0/1 | [x] |
| 190 | `read_side_info` | R/sf5, stereo, stop, explicit preflag 0/1 | [x] |
| 191 | `read_side_info` | M1/sf8, mono, long, explicit preflag 0/1 | [x] |
| 192 | `read_side_info` | M1/sf8, mono, start, explicit preflag 0/1 | [x] |
| 193 | `read_side_info` | M1/sf8, mono, short, explicit preflag 0/1 | [x] |
| 194 | `read_side_info` | M1/sf8, mono, mixed, explicit preflag 0/1 | [x] |
| 195 | `read_side_info` | M1/sf8, mono, stop, explicit preflag 0/1 | [x] |
| 196 | `read_side_info` | M1/sf8, stereo, long, explicit preflag 0/1 | [x] |
| 197 | `read_side_info` | M1/sf8, stereo, start, explicit preflag 0/1 | [x] |
| 198 | `read_side_info` | M1/sf8, stereo, short, explicit preflag 0/1 | [x] |
| 199 | `read_side_info` | M1/sf8, stereo, mixed, explicit preflag 0/1 | [x] |
| 200 | `read_side_info` | M1/sf8, stereo, stop, explicit preflag 0/1 | [x] |
