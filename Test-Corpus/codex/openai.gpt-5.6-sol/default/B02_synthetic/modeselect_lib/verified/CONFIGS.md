# Configuration Surface

This table is derived from all seven `nm -D` exports and the branch/data-shape
operations in `src/lib.c`: four `strcmp` branches, six `switch` outcomes,
double-to-int conversion classes, signed offset shapes, all `time_t` bytes,
`mode_selector % 4`, `complexity % 5`, and `seed % 24`.

There are no preprocessor features or runtime option setters. Cargo.toml has no
`[features]` table, so the only feature configuration is the default/no-feature
build.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `classify_mode` | mode is exactly `standard` | [x] |
| 2 | `classify_mode` | mode is exactly `enhanced` | [x] |
| 3 | `classify_mode` | mode is exactly `turbo` | [x] |
| 4 | `classify_mode` | mode is exactly `extreme` | [x] |
| 5 | `classify_mode` | mode is any other non-null C string, including empty | [x] |
| 6 | `apply_multiplier` | `level == 0`; randomized safe-range base | [x] |
| 7 | `apply_multiplier` | `level == 1`; randomized safe-range base | [x] |
| 8 | `apply_multiplier` | `level == 2`; randomized safe-range base | [x] |
| 9 | `apply_multiplier` | `level == 3`; randomized safe-range base | [x] |
| 10 | `apply_multiplier` | `level == 4`; randomized safe-range base | [x] |
| 11 | `apply_multiplier` | `level` is outside 0..=4, both signs | [x] |
| 12 | `convert_time_factor` | zero, including negative zero | [x] |
| 13 | `convert_time_factor` | positive finite input with scaled result inside `int` range, including fractions | [x] |
| 14 | `convert_time_factor` | negative finite input with scaled result inside `int` range, including fractions | [x] |
| 15 | `convert_time_factor` | scaled result exactly `INT_MIN` | [x] |
| 16 | `convert_time_factor` | scaled result at the largest representable/truncating positive edge | [x] |
| 17 | `convert_time_factor` | finite scaled result outside `int` range, both signs | [x] |
| 18 | `convert_time_factor` | NaN and positive/negative infinity | [x] |
| 19 | `convert_negative_overflow` | zero, including negative zero | [x] |
| 20 | `convert_negative_overflow` | positive finite input whose scaled result is inside `int` range | [x] |
| 21 | `convert_negative_overflow` | negative finite input whose scaled result is inside `int` range | [x] |
| 22 | `convert_negative_overflow` | scaled result exactly `INT_MIN` | [x] |
| 23 | `convert_negative_overflow` | scaled result at the largest representable/truncating positive edge | [x] |
| 24 | `convert_negative_overflow` | finite scaled result outside `int` range, both signs | [x] |
| 25 | `convert_negative_overflow` | NaN and positive/negative infinity | [x] |
| 26 | `get_modified_time` | zero days, zero hours | [x] |
| 27 | `get_modified_time` | zero days, positive hours | [x] |
| 28 | `get_modified_time` | zero days, negative hours | [x] |
| 29 | `get_modified_time` | positive days, zero hours | [x] |
| 30 | `get_modified_time` | positive days, positive hours | [x] |
| 31 | `get_modified_time` | positive days, negative hours | [x] |
| 32 | `get_modified_time` | negative days, zero hours | [x] |
| 33 | `get_modified_time` | negative days, positive hours | [x] |
| 34 | `get_modified_time` | negative days, negative hours | [x] |
| 35 | `hash_time_value` | zero `time_t` | [x] |
| 36 | `hash_time_value` | positive `time_t`, randomized all-byte patterns | [x] |
| 37 | `hash_time_value` | negative `time_t`, randomized all-byte patterns | [x] |
| 38 | `hash_time_value` | `time_t` minimum and maximum | [x] |
| 39 | `hash_time_value` | repeated and alternating byte-edge patterns | [x] |
| 40 | `modeselect` | mode remainder 0, complexity remainder 0; seed/time axes randomized | [x] |
| 41 | `modeselect` | mode remainder 0, complexity remainder 1; seed/time axes randomized | [x] |
| 42 | `modeselect` | mode remainder 0, complexity remainder 2; seed/time axes randomized | [x] |
| 43 | `modeselect` | mode remainder 0, complexity remainder 3; seed/time axes randomized | [x] |
| 44 | `modeselect` | mode remainder 0, complexity remainder 4; seed/time axes randomized | [x] |
| 45 | `modeselect` | mode remainder 0, negative complexity/default branch; seed/time axes randomized | [x] |
| 46 | `modeselect` | mode remainder 1, complexity remainder 0; seed/time axes randomized | [x] |
| 47 | `modeselect` | mode remainder 1, complexity remainder 1; seed/time axes randomized | [x] |
| 48 | `modeselect` | mode remainder 1, complexity remainder 2; seed/time axes randomized | [x] |
| 49 | `modeselect` | mode remainder 1, complexity remainder 3; seed/time axes randomized | [x] |
| 50 | `modeselect` | mode remainder 1, complexity remainder 4; seed/time axes randomized | [x] |
| 51 | `modeselect` | mode remainder 1, negative complexity/default branch; seed/time axes randomized | [x] |
| 52 | `modeselect` | mode remainder 2, complexity remainder 0; seed/time axes randomized | [x] |
| 53 | `modeselect` | mode remainder 2, complexity remainder 1; seed/time axes randomized | [x] |
| 54 | `modeselect` | mode remainder 2, complexity remainder 2; seed/time axes randomized | [x] |
| 55 | `modeselect` | mode remainder 2, complexity remainder 3; seed/time axes randomized | [x] |
| 56 | `modeselect` | mode remainder 2, complexity remainder 4; seed/time axes randomized | [x] |
| 57 | `modeselect` | mode remainder 2, negative complexity/default branch; seed/time axes randomized | [x] |
| 58 | `modeselect` | mode remainder 3, complexity remainder 0; seed/time axes randomized | [x] |
| 59 | `modeselect` | mode remainder 3, complexity remainder 1; seed/time axes randomized | [x] |
| 60 | `modeselect` | mode remainder 3, complexity remainder 2; seed/time axes randomized | [x] |
| 61 | `modeselect` | mode remainder 3, complexity remainder 3; seed/time axes randomized | [x] |
| 62 | `modeselect` | mode remainder 3, complexity remainder 4; seed/time axes randomized | [x] |
| 63 | `modeselect` | mode remainder 3, negative complexity/default branch; seed/time axes randomized | [x] |

For rows 40-63, randomized seeds cover zero, both signs, and every reachable
`seed % 24` remainder (-23..=23). Randomized time offsets cover zero and both
signs. Nonnegative mode selectors are used because negative remainders index
before the C `modes` array and invoke undefined behavior rather than a defined
configuration. Complexity uses nonnegative representatives for remainders
0..=4 and negative representatives for the `switch` default branch.
