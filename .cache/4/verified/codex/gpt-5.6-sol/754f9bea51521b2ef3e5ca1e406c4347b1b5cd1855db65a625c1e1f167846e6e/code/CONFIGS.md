# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, no implicit optional-dependency
features, and no default features. `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. The complete build
configuration set is therefore:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| B1 | `cargo ... --no-default-features` (empty feature set) | default | [x] |

## Runtime Axes

All rows below call the sole public entry point, `read_side_info`, directly
through each shared object. IDs expand these C-source-derived axes:

- Mode `N`: `hdr[1] & 0x08 == 0`; 1 granule for mono and 2 for non-mono.
- Mode `M`: `hdr[1] & 0x08 != 0`; 2 granules for mono and 4 for non-mono.
- `s0`...`s7`: the computed `sr_idx` and corresponding scalefactor table row.
  Header encodings that keep the C array index in bounds produce `s0`...`s5`
  for mode N and `s2`...`s7` for mode M.
- Channel `m`/`s`: `(hdr[3] & 0xC0) == 0xC0` (mono) or not (non-mono).
- Non-M mode `lo`/`hi`: `scalefac_compress < 500` or `>= 500`, which controls
  computed `preflag`. Mode M reads `preflag` from the bitstream instead.
- Shape `n`: normal window; `w`: switched block type 1 or 3; `p`: switched
  block type 2, non-mixed short; `x`: switched block type 2, mixed; `t`:
  switched block type 3. The named shape is used for the first granule; later
  granules randomize across all five shapes to cover branch-order interactions.
  The `w` rows fix accepted non-short type 1 while `t` fixes type 3, so both
  accepted values are explicit.
- Every row includes randomized zero/nonzero `main_data_begin`,
  zero/nonzero `part_23_length`, field boundary values, arbitrary initial
  bytes, and all starting bit alignments `bs.pos & 7 == 0...7`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C001 | `read_side_info` | `N-s0-m-lo-n` | [x] |
| C002 | `read_side_info` | `N-s0-m-lo-w` | [x] |
| C003 | `read_side_info` | `N-s0-m-lo-p` | [x] |
| C004 | `read_side_info` | `N-s0-m-lo-x` | [x] |
| C005 | `read_side_info` | `N-s0-m-lo-t` | [x] |
| C006 | `read_side_info` | `N-s0-m-hi-n` | [x] |
| C007 | `read_side_info` | `N-s0-m-hi-w` | [x] |
| C008 | `read_side_info` | `N-s0-m-hi-p` | [x] |
| C009 | `read_side_info` | `N-s0-m-hi-x` | [x] |
| C010 | `read_side_info` | `N-s0-m-hi-t` | [x] |
| C011 | `read_side_info` | `N-s0-s-lo-n` | [x] |
| C012 | `read_side_info` | `N-s0-s-lo-w` | [x] |
| C013 | `read_side_info` | `N-s0-s-lo-p` | [x] |
| C014 | `read_side_info` | `N-s0-s-lo-x` | [x] |
| C015 | `read_side_info` | `N-s0-s-lo-t` | [x] |
| C016 | `read_side_info` | `N-s0-s-hi-n` | [x] |
| C017 | `read_side_info` | `N-s0-s-hi-w` | [x] |
| C018 | `read_side_info` | `N-s0-s-hi-p` | [x] |
| C019 | `read_side_info` | `N-s0-s-hi-x` | [x] |
| C020 | `read_side_info` | `N-s0-s-hi-t` | [x] |
| C021 | `read_side_info` | `N-s1-m-lo-n` | [x] |
| C022 | `read_side_info` | `N-s1-m-lo-w` | [x] |
| C023 | `read_side_info` | `N-s1-m-lo-p` | [x] |
| C024 | `read_side_info` | `N-s1-m-lo-x` | [x] |
| C025 | `read_side_info` | `N-s1-m-lo-t` | [x] |
| C026 | `read_side_info` | `N-s1-m-hi-n` | [x] |
| C027 | `read_side_info` | `N-s1-m-hi-w` | [x] |
| C028 | `read_side_info` | `N-s1-m-hi-p` | [x] |
| C029 | `read_side_info` | `N-s1-m-hi-x` | [x] |
| C030 | `read_side_info` | `N-s1-m-hi-t` | [x] |
| C031 | `read_side_info` | `N-s1-s-lo-n` | [x] |
| C032 | `read_side_info` | `N-s1-s-lo-w` | [x] |
| C033 | `read_side_info` | `N-s1-s-lo-p` | [x] |
| C034 | `read_side_info` | `N-s1-s-lo-x` | [x] |
| C035 | `read_side_info` | `N-s1-s-lo-t` | [x] |
| C036 | `read_side_info` | `N-s1-s-hi-n` | [x] |
| C037 | `read_side_info` | `N-s1-s-hi-w` | [x] |
| C038 | `read_side_info` | `N-s1-s-hi-p` | [x] |
| C039 | `read_side_info` | `N-s1-s-hi-x` | [x] |
| C040 | `read_side_info` | `N-s1-s-hi-t` | [x] |
| C041 | `read_side_info` | `N-s2-m-lo-n` | [x] |
| C042 | `read_side_info` | `N-s2-m-lo-w` | [x] |
| C043 | `read_side_info` | `N-s2-m-lo-p` | [x] |
| C044 | `read_side_info` | `N-s2-m-lo-x` | [x] |
| C045 | `read_side_info` | `N-s2-m-lo-t` | [x] |
| C046 | `read_side_info` | `N-s2-m-hi-n` | [x] |
| C047 | `read_side_info` | `N-s2-m-hi-w` | [x] |
| C048 | `read_side_info` | `N-s2-m-hi-p` | [x] |
| C049 | `read_side_info` | `N-s2-m-hi-x` | [x] |
| C050 | `read_side_info` | `N-s2-m-hi-t` | [x] |
| C051 | `read_side_info` | `N-s2-s-lo-n` | [x] |
| C052 | `read_side_info` | `N-s2-s-lo-w` | [x] |
| C053 | `read_side_info` | `N-s2-s-lo-p` | [x] |
| C054 | `read_side_info` | `N-s2-s-lo-x` | [x] |
| C055 | `read_side_info` | `N-s2-s-lo-t` | [x] |
| C056 | `read_side_info` | `N-s2-s-hi-n` | [x] |
| C057 | `read_side_info` | `N-s2-s-hi-w` | [x] |
| C058 | `read_side_info` | `N-s2-s-hi-p` | [x] |
| C059 | `read_side_info` | `N-s2-s-hi-x` | [x] |
| C060 | `read_side_info` | `N-s2-s-hi-t` | [x] |
| C061 | `read_side_info` | `N-s3-m-lo-n` | [x] |
| C062 | `read_side_info` | `N-s3-m-lo-w` | [x] |
| C063 | `read_side_info` | `N-s3-m-lo-p` | [x] |
| C064 | `read_side_info` | `N-s3-m-lo-x` | [x] |
| C065 | `read_side_info` | `N-s3-m-lo-t` | [x] |
| C066 | `read_side_info` | `N-s3-m-hi-n` | [x] |
| C067 | `read_side_info` | `N-s3-m-hi-w` | [x] |
| C068 | `read_side_info` | `N-s3-m-hi-p` | [x] |
| C069 | `read_side_info` | `N-s3-m-hi-x` | [x] |
| C070 | `read_side_info` | `N-s3-m-hi-t` | [x] |
| C071 | `read_side_info` | `N-s3-s-lo-n` | [x] |
| C072 | `read_side_info` | `N-s3-s-lo-w` | [x] |
| C073 | `read_side_info` | `N-s3-s-lo-p` | [x] |
| C074 | `read_side_info` | `N-s3-s-lo-x` | [x] |
| C075 | `read_side_info` | `N-s3-s-lo-t` | [x] |
| C076 | `read_side_info` | `N-s3-s-hi-n` | [x] |
| C077 | `read_side_info` | `N-s3-s-hi-w` | [x] |
| C078 | `read_side_info` | `N-s3-s-hi-p` | [x] |
| C079 | `read_side_info` | `N-s3-s-hi-x` | [x] |
| C080 | `read_side_info` | `N-s3-s-hi-t` | [x] |
| C081 | `read_side_info` | `N-s4-m-lo-n` | [x] |
| C082 | `read_side_info` | `N-s4-m-lo-w` | [x] |
| C083 | `read_side_info` | `N-s4-m-lo-p` | [x] |
| C084 | `read_side_info` | `N-s4-m-lo-x` | [x] |
| C085 | `read_side_info` | `N-s4-m-lo-t` | [x] |
| C086 | `read_side_info` | `N-s4-m-hi-n` | [x] |
| C087 | `read_side_info` | `N-s4-m-hi-w` | [x] |
| C088 | `read_side_info` | `N-s4-m-hi-p` | [x] |
| C089 | `read_side_info` | `N-s4-m-hi-x` | [x] |
| C090 | `read_side_info` | `N-s4-m-hi-t` | [x] |
| C091 | `read_side_info` | `N-s4-s-lo-n` | [x] |
| C092 | `read_side_info` | `N-s4-s-lo-w` | [x] |
| C093 | `read_side_info` | `N-s4-s-lo-p` | [x] |
| C094 | `read_side_info` | `N-s4-s-lo-x` | [x] |
| C095 | `read_side_info` | `N-s4-s-lo-t` | [x] |
| C096 | `read_side_info` | `N-s4-s-hi-n` | [x] |
| C097 | `read_side_info` | `N-s4-s-hi-w` | [x] |
| C098 | `read_side_info` | `N-s4-s-hi-p` | [x] |
| C099 | `read_side_info` | `N-s4-s-hi-x` | [x] |
| C100 | `read_side_info` | `N-s4-s-hi-t` | [x] |
| C101 | `read_side_info` | `N-s5-m-lo-n` | [x] |
| C102 | `read_side_info` | `N-s5-m-lo-w` | [x] |
| C103 | `read_side_info` | `N-s5-m-lo-p` | [x] |
| C104 | `read_side_info` | `N-s5-m-lo-x` | [x] |
| C105 | `read_side_info` | `N-s5-m-lo-t` | [x] |
| C106 | `read_side_info` | `N-s5-m-hi-n` | [x] |
| C107 | `read_side_info` | `N-s5-m-hi-w` | [x] |
| C108 | `read_side_info` | `N-s5-m-hi-p` | [x] |
| C109 | `read_side_info` | `N-s5-m-hi-x` | [x] |
| C110 | `read_side_info` | `N-s5-m-hi-t` | [x] |
| C111 | `read_side_info` | `N-s5-s-lo-n` | [x] |
| C112 | `read_side_info` | `N-s5-s-lo-w` | [x] |
| C113 | `read_side_info` | `N-s5-s-lo-p` | [x] |
| C114 | `read_side_info` | `N-s5-s-lo-x` | [x] |
| C115 | `read_side_info` | `N-s5-s-lo-t` | [x] |
| C116 | `read_side_info` | `N-s5-s-hi-n` | [x] |
| C117 | `read_side_info` | `N-s5-s-hi-w` | [x] |
| C118 | `read_side_info` | `N-s5-s-hi-p` | [x] |
| C119 | `read_side_info` | `N-s5-s-hi-x` | [x] |
| C120 | `read_side_info` | `N-s5-s-hi-t` | [x] |
| C121 | `read_side_info` | `M-s2-m-n` | [x] |
| C122 | `read_side_info` | `M-s2-m-w` | [x] |
| C123 | `read_side_info` | `M-s2-m-p` | [x] |
| C124 | `read_side_info` | `M-s2-m-x` | [x] |
| C125 | `read_side_info` | `M-s2-m-t` | [x] |
| C126 | `read_side_info` | `M-s2-s-n` | [x] |
| C127 | `read_side_info` | `M-s2-s-w` | [x] |
| C128 | `read_side_info` | `M-s2-s-p` | [x] |
| C129 | `read_side_info` | `M-s2-s-x` | [x] |
| C130 | `read_side_info` | `M-s2-s-t` | [x] |
| C131 | `read_side_info` | `M-s3-m-n` | [x] |
| C132 | `read_side_info` | `M-s3-m-w` | [x] |
| C133 | `read_side_info` | `M-s3-m-p` | [x] |
| C134 | `read_side_info` | `M-s3-m-x` | [x] |
| C135 | `read_side_info` | `M-s3-m-t` | [x] |
| C136 | `read_side_info` | `M-s3-s-n` | [x] |
| C137 | `read_side_info` | `M-s3-s-w` | [x] |
| C138 | `read_side_info` | `M-s3-s-p` | [x] |
| C139 | `read_side_info` | `M-s3-s-x` | [x] |
| C140 | `read_side_info` | `M-s3-s-t` | [x] |
| C141 | `read_side_info` | `M-s4-m-n` | [x] |
| C142 | `read_side_info` | `M-s4-m-w` | [x] |
| C143 | `read_side_info` | `M-s4-m-p` | [x] |
| C144 | `read_side_info` | `M-s4-m-x` | [x] |
| C145 | `read_side_info` | `M-s4-m-t` | [x] |
| C146 | `read_side_info` | `M-s4-s-n` | [x] |
| C147 | `read_side_info` | `M-s4-s-w` | [x] |
| C148 | `read_side_info` | `M-s4-s-p` | [x] |
| C149 | `read_side_info` | `M-s4-s-x` | [x] |
| C150 | `read_side_info` | `M-s4-s-t` | [x] |
| C151 | `read_side_info` | `M-s5-m-n` | [x] |
| C152 | `read_side_info` | `M-s5-m-w` | [x] |
| C153 | `read_side_info` | `M-s5-m-p` | [x] |
| C154 | `read_side_info` | `M-s5-m-x` | [x] |
| C155 | `read_side_info` | `M-s5-m-t` | [x] |
| C156 | `read_side_info` | `M-s5-s-n` | [x] |
| C157 | `read_side_info` | `M-s5-s-w` | [x] |
| C158 | `read_side_info` | `M-s5-s-p` | [x] |
| C159 | `read_side_info` | `M-s5-s-x` | [x] |
| C160 | `read_side_info` | `M-s5-s-t` | [x] |
| C161 | `read_side_info` | `M-s6-m-n` | [x] |
| C162 | `read_side_info` | `M-s6-m-w` | [x] |
| C163 | `read_side_info` | `M-s6-m-p` | [x] |
| C164 | `read_side_info` | `M-s6-m-x` | [x] |
| C165 | `read_side_info` | `M-s6-m-t` | [x] |
| C166 | `read_side_info` | `M-s6-s-n` | [x] |
| C167 | `read_side_info` | `M-s6-s-w` | [x] |
| C168 | `read_side_info` | `M-s6-s-p` | [x] |
| C169 | `read_side_info` | `M-s6-s-x` | [x] |
| C170 | `read_side_info` | `M-s6-s-t` | [x] |
| C171 | `read_side_info` | `M-s7-m-n` | [x] |
| C172 | `read_side_info` | `M-s7-m-w` | [x] |
| C173 | `read_side_info` | `M-s7-m-p` | [x] |
| C174 | `read_side_info` | `M-s7-m-x` | [x] |
| C175 | `read_side_info` | `M-s7-m-t` | [x] |
| C176 | `read_side_info` | `M-s7-s-n` | [x] |
| C177 | `read_side_info` | `M-s7-s-w` | [x] |
| C178 | `read_side_info` | `M-s7-s-p` | [x] |
| C179 | `read_side_info` | `M-s7-s-x` | [x] |
| C180 | `read_side_info` | `M-s7-s-t` | [x] |
