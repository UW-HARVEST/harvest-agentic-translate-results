# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section, no optional dependencies, and no
default feature declaration. `c_src/CMakeLists.txt` has no options or
conditional compilation. There is exactly one valid build-time combination:

| # | Cargo feature set | C configuration | checked |
|---|-------------------|-----------------|---------|
| B1 | empty (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

These rows come from the `strcmp` chain, `switch` cases, conversion operations,
native `time_t` representation, and `% 4`, `% 5`, and `% 24` operations in
`c_src/src/lib.c`. Randomized cases use a fixed seed and preserve the listed
shape or residue.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|-------------------------------------------|---------|
| 1 | `classify_mode` | exact NUL-terminated string `standard` | [x] |
| 2 | `classify_mode` | exact NUL-terminated string `enhanced` | [x] |
| 3 | `classify_mode` | exact NUL-terminated string `turbo` | [x] |
| 4 | `classify_mode` | exact NUL-terminated string `extreme` | [x] |
| 5 | `apply_multiplier` | arbitrary `base`; `level == 0` | [x] |
| 6 | `apply_multiplier` | arbitrary `base`; `level == 1` | [x] |
| 7 | `apply_multiplier` | arbitrary `base`; `level == 2` | [x] |
| 8 | `apply_multiplier` | arbitrary `base`; `level == 3` | [x] |
| 9 | `apply_multiplier` | arbitrary `base`; `level == 4` | [x] |
| 10 | `convert_time_factor` | `factor == 0.0` (including signed zero) | [x] |
| 11 | `convert_time_factor` | positive finite factor whose scaled value is in `int` range | [x] |
| 12 | `convert_time_factor` | negative finite factor whose scaled value is in `int` range | [x] |
| 13 | `convert_time_factor` | scaled value at the positive `int` boundary | [x] |
| 14 | `convert_time_factor` | scaled value at the negative `int` boundary | [x] |
| 15 | `convert_time_factor` | finite positive factor whose scaled value exceeds `INT_MAX` | [x] |
| 16 | `convert_time_factor` | finite negative factor whose scaled value is below `INT_MIN` | [x] |
| 17 | `convert_time_factor` | positive or negative infinity | [x] |
| 18 | `convert_time_factor` | NaN payloads | [x] |
| 19 | `convert_negative_overflow` | `value == 0.0` (including signed zero) | [x] |
| 20 | `convert_negative_overflow` | positive finite value whose scaled value is in `int` range | [x] |
| 21 | `convert_negative_overflow` | negative finite value whose scaled value is in `int` range | [x] |
| 22 | `convert_negative_overflow` | scaled value at the positive `int` boundary | [x] |
| 23 | `convert_negative_overflow` | scaled value at the negative `int` boundary | [x] |
| 24 | `convert_negative_overflow` | finite input producing scaled positive overflow | [x] |
| 25 | `convert_negative_overflow` | finite input producing scaled negative overflow | [x] |
| 26 | `convert_negative_overflow` | positive or negative infinity | [x] |
| 27 | `convert_negative_overflow` | NaN payloads | [x] |
| 28 | `get_modified_time` | zero day and hour offsets | [x] |
| 29 | `get_modified_time` | positive day/hour offsets | [x] |
| 30 | `get_modified_time` | negative day/hour offsets | [x] |
| 31 | `get_modified_time` | mixed-sign day/hour offsets | [x] |
| 32 | `get_modified_time` | `int` boundary offsets exercising C arithmetic wrap | [x] |
| 33 | `hash_time_value` | zero `time_t` | [x] |
| 34 | `hash_time_value` | positive `time_t` with one and many nonzero bytes | [x] |
| 35 | `hash_time_value` | negative `time_t` | [x] |
| 36 | `hash_time_value` | `time_t` minimum and maximum | [x] |
| 37 | `hash_time_value` | arbitrary native-endian byte patterns | [x] |
| 38 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 39 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 40 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 41 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 42 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 43 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 44 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 45 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 46 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 47 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 48 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 49 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 50 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 51 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 52 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 53 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 54 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 55 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 56 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 57 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 58 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 59 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 60 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 61 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 62 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 63 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 64 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 65 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 66 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 67 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 68 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 69 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 70 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 71 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 72 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 73 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 74 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 75 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 76 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 77 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 78 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 79 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 80 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 81 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 82 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 83 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 84 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 85 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 86 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 87 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 88 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 89 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 90 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 91 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 92 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 93 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 94 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 95 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 96 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 97 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 98 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 99 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 100 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 101 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 102 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 103 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 104 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 105 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 106 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 107 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 108 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 109 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 110 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 111 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 112 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 113 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 114 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 115 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 116 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 117 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 118 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 119 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 120 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 121 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 122 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 123 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 124 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 125 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 126 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 127 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 128 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 129 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 130 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 131 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 132 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 133 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 134 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 135 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 136 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 137 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 138 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 139 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 140 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 141 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 142 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 143 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 144 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 145 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 146 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 147 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 148 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 149 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 150 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 151 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 152 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 153 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 154 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 155 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 156 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 157 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 0`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 158 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 159 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 160 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 161 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 162 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 163 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 164 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 165 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 166 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 167 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 168 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 169 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 170 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 171 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 172 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 173 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 174 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 175 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 176 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 177 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 178 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 179 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 180 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 181 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 182 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 183 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 184 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 185 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 186 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 187 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 188 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 189 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 190 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 191 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 192 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 193 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 194 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 195 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 196 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 197 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 198 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 199 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 200 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 201 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 202 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 203 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 204 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 205 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 206 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 207 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 208 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 209 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 210 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 211 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 212 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 213 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 214 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 215 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 216 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 217 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 218 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 219 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 220 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 221 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 222 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 223 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 224 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 225 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 226 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 227 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 228 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 229 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 230 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 231 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 232 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 233 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 234 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 235 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 236 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 237 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 238 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 239 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 240 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 241 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 242 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 243 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 244 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 245 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 246 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 247 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 248 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 249 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 250 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 251 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 252 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 253 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 254 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 255 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 256 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 257 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 258 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 259 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 260 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 261 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 262 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 263 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 264 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 265 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 266 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 267 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 268 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 269 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 270 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 271 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 272 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 273 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 274 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 275 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 276 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 277 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 1`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 278 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 279 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 280 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 281 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 282 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 283 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 284 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 285 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 286 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 287 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 288 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 289 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 290 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 291 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 292 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 293 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 294 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 295 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 296 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 297 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 298 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 299 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 300 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 301 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 302 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 303 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 304 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 305 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 306 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 307 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 308 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 309 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 310 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 311 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 312 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 313 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 314 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 315 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 316 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 317 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 318 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 319 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 320 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 321 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 322 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 323 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 324 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 325 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 326 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 327 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 328 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 329 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 330 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 331 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 332 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 333 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 334 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 335 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 336 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 337 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 338 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 339 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 340 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 341 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 342 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 343 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 344 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 345 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 346 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 347 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 348 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 349 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 350 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 351 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 352 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 353 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 354 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 355 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 356 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 357 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 358 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 359 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 360 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 361 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 362 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 363 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 364 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 365 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 366 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 367 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 368 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 369 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 370 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 371 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 372 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 373 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 374 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 375 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 376 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 377 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 378 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 379 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 380 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 381 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 382 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 383 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 384 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 385 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 386 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 387 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 388 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 389 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 390 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 391 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 392 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 393 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 394 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 395 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 396 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 397 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 2`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 398 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 399 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 400 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 401 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 402 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 403 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 404 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 405 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 406 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 407 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 408 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 409 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 410 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 411 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 412 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 413 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 414 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 415 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 416 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 417 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 418 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 419 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 420 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 421 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 0`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 422 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 423 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 424 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 425 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 426 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 427 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 428 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 429 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 430 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 431 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 432 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 433 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 434 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 435 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 436 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 437 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 438 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 439 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 440 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 441 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 442 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 443 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 444 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 445 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 1`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 446 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 447 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 448 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 449 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 450 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 451 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 452 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 453 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 454 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 455 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 456 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 457 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 458 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 459 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 460 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 461 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 462 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 463 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 464 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 465 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 466 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 467 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 468 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 469 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 2`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 470 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 471 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 472 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 473 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 474 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 475 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 476 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 477 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 478 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 479 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 480 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 481 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 482 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 483 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 484 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 485 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 486 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 487 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 488 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 489 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 490 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 491 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 492 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 493 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 3`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
| 494 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 0`; arbitrary `time_offset` | [x] |
| 495 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 1`; arbitrary `time_offset` | [x] |
| 496 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 2`; arbitrary `time_offset` | [x] |
| 497 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 3`; arbitrary `time_offset` | [x] |
| 498 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 4`; arbitrary `time_offset` | [x] |
| 499 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 5`; arbitrary `time_offset` | [x] |
| 500 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 6`; arbitrary `time_offset` | [x] |
| 501 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 7`; arbitrary `time_offset` | [x] |
| 502 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 8`; arbitrary `time_offset` | [x] |
| 503 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 9`; arbitrary `time_offset` | [x] |
| 504 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 10`; arbitrary `time_offset` | [x] |
| 505 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 11`; arbitrary `time_offset` | [x] |
| 506 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 12`; arbitrary `time_offset` | [x] |
| 507 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 13`; arbitrary `time_offset` | [x] |
| 508 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 14`; arbitrary `time_offset` | [x] |
| 509 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 15`; arbitrary `time_offset` | [x] |
| 510 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 16`; arbitrary `time_offset` | [x] |
| 511 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 17`; arbitrary `time_offset` | [x] |
| 512 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 18`; arbitrary `time_offset` | [x] |
| 513 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 19`; arbitrary `time_offset` | [x] |
| 514 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 20`; arbitrary `time_offset` | [x] |
| 515 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 21`; arbitrary `time_offset` | [x] |
| 516 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 22`; arbitrary `time_offset` | [x] |
| 517 | `modeselect` (composed pipeline) | nonnegative `mode_selector % 4 == 3`; nonnegative `complexity % 5 == 4`; nonnegative `seed % 24 == 23`; arbitrary `time_offset` | [x] |
