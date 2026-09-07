# Configuration Surface

Derived mechanically from all exported entry points, string comparisons,
switch arms, `%` remainder operations, floating conversions, time offset
arithmetic, `sizeof(time_t)` byte iteration, and public call composition in
`../c_src/src/lib.c`.

There are no Cargo features, preprocessor feature flags, runtime option
structures, element formats, byte-order options, or executable targets.

For rows 43–62, each listed mode/complexity branch is crossed with all seed
hour remainders `0..=23`, seed conversion classes (zero and nonzero), and
negative/zero/positive time offsets using fixed-seed randomized representatives.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `classify_mode` | exact string `"standard"` | [x] |
| 2 | `classify_mode` | exact string `"enhanced"` | [x] |
| 3 | `classify_mode` | exact string `"turbo"` | [x] |
| 4 | `classify_mode` | exact string `"extreme"` | [x] |
| 5 | `apply_multiplier` | `level == 0`, only `+0x05` | [x] |
| 6 | `apply_multiplier` | `level == 1`, fall through 1→0 | [x] |
| 7 | `apply_multiplier` | `level == 2`, fall through 2→0 | [x] |
| 8 | `apply_multiplier` | `level == 3`, fall through 3→0 | [x] |
| 9 | `apply_multiplier` | `level == 4`, fall through 4→0 | [x] |
| 10 | `convert_time_factor` | zero | [x] |
| 11 | `convert_time_factor` | positive finite scaled value inside `int` range | [x] |
| 12 | `convert_time_factor` | negative finite scaled value inside `int` range | [x] |
| 13 | `convert_time_factor` | positive fractional scaled value (truncation) | [x] |
| 14 | `convert_time_factor` | negative fractional scaled value (truncation) | [x] |
| 15 | `convert_time_factor` | scaled value exactly `INT_MIN` | [x] |
| 16 | `convert_time_factor` | largest representable scaled integers below `INT_MAX + 1` | [x] |
| 17 | `convert_time_factor` | positive finite overflow | [x] |
| 18 | `convert_time_factor` | negative finite overflow | [x] |
| 19 | `convert_time_factor` | NaN | [x] |
| 20 | `convert_time_factor` | positive/negative infinity | [x] |
| 21 | `convert_negative_overflow` | zero | [x] |
| 22 | `convert_negative_overflow` | source negative, scaled result positive and inside `int` range | [x] |
| 23 | `convert_negative_overflow` | source positive, scaled result negative and inside `int` range | [x] |
| 24 | `convert_negative_overflow` | positive fractional scaled result (truncation) | [x] |
| 25 | `convert_negative_overflow` | negative fractional scaled result (truncation) | [x] |
| 26 | `convert_negative_overflow` | scaled value exactly `INT_MIN` | [x] |
| 27 | `convert_negative_overflow` | largest representable scaled integers below `INT_MAX + 1` | [x] |
| 28 | `convert_negative_overflow` | positive finite overflow result | [x] |
| 29 | `convert_negative_overflow` | negative finite overflow result | [x] |
| 30 | `convert_negative_overflow` | NaN | [x] |
| 31 | `convert_negative_overflow` | positive/negative infinity | [x] |
| 32 | `get_modified_time` | zero day and zero hour offsets | [x] |
| 33 | `get_modified_time` | positive day, zero hour | [x] |
| 34 | `get_modified_time` | negative day, zero hour | [x] |
| 35 | `get_modified_time` | zero day, positive hour | [x] |
| 36 | `get_modified_time` | zero day, negative hour | [x] |
| 37 | `get_modified_time` | positive/negative mixed offsets and safe arithmetic boundaries | [x] |
| 38 | `hash_time_value` | `time_t == 0` (all zero bytes) | [x] |
| 39 | `hash_time_value` | positive values with varied bytes | [x] |
| 40 | `hash_time_value` | negative values with sign-extension bytes | [x] |
| 41 | `hash_time_value` | `time_t` minimum and maximum | [x] |
| 42 | `hash_time_value` | fixed-seed randomized full-width byte patterns | [x] |
| 43 | `modeselect` and all callees | `mode_selector % 4 == 0`, `complexity % 5 == 0`; full secondary cross-product described above | [x] |
| 44 | `modeselect` and all callees | `mode_selector % 4 == 0`, `complexity % 5 == 1`; full secondary cross-product described above | [x] |
| 45 | `modeselect` and all callees | `mode_selector % 4 == 0`, `complexity % 5 == 2`; full secondary cross-product described above | [x] |
| 46 | `modeselect` and all callees | `mode_selector % 4 == 0`, `complexity % 5 == 3`; full secondary cross-product described above | [x] |
| 47 | `modeselect` and all callees | `mode_selector % 4 == 0`, `complexity % 5 == 4`; full secondary cross-product described above | [x] |
| 48 | `modeselect` and all callees | `mode_selector % 4 == 1`, `complexity % 5 == 0`; full secondary cross-product described above | [x] |
| 49 | `modeselect` and all callees | `mode_selector % 4 == 1`, `complexity % 5 == 1`; full secondary cross-product described above | [x] |
| 50 | `modeselect` and all callees | `mode_selector % 4 == 1`, `complexity % 5 == 2`; full secondary cross-product described above | [x] |
| 51 | `modeselect` and all callees | `mode_selector % 4 == 1`, `complexity % 5 == 3`; full secondary cross-product described above | [x] |
| 52 | `modeselect` and all callees | `mode_selector % 4 == 1`, `complexity % 5 == 4`; full secondary cross-product described above | [x] |
| 53 | `modeselect` and all callees | `mode_selector % 4 == 2`, `complexity % 5 == 0`; full secondary cross-product described above | [x] |
| 54 | `modeselect` and all callees | `mode_selector % 4 == 2`, `complexity % 5 == 1`; full secondary cross-product described above | [x] |
| 55 | `modeselect` and all callees | `mode_selector % 4 == 2`, `complexity % 5 == 2`; full secondary cross-product described above | [x] |
| 56 | `modeselect` and all callees | `mode_selector % 4 == 2`, `complexity % 5 == 3`; full secondary cross-product described above | [x] |
| 57 | `modeselect` and all callees | `mode_selector % 4 == 2`, `complexity % 5 == 4`; full secondary cross-product described above | [x] |
| 58 | `modeselect` and all callees | `mode_selector % 4 == 3`, `complexity % 5 == 0`; full secondary cross-product described above | [x] |
| 59 | `modeselect` and all callees | `mode_selector % 4 == 3`, `complexity % 5 == 1`; full secondary cross-product described above | [x] |
| 60 | `modeselect` and all callees | `mode_selector % 4 == 3`, `complexity % 5 == 2`; full secondary cross-product described above | [x] |
| 61 | `modeselect` and all callees | `mode_selector % 4 == 3`, `complexity % 5 == 3`; full secondary cross-product described above | [x] |
| 62 | `modeselect` and all callees | `mode_selector % 4 == 3`, `complexity % 5 == 4`; full secondary cross-product described above | [x] |

