# CONFIGS.md — Phase B configuration surface

## Axes the C code actually branches on

`c_src/src/lib.c` has **no** runtime options, no global state, no `#ifdef`
feature switches and no format/byte-order selectors. `translation/Cargo.toml`
declares **no `[features]` table**, so there is exactly one feature combination
(the default, which is empty). The axes are therefore purely *input shape*:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `convert_double_to_int` value class | in-range integral, in-range fractional (truncation toward zero, both signs), `±0.0`, subnormal, exactly `INT_MAX`/`INT_MIN`, one step past each, huge (`±1e300`), `±INFINITY`, `NaN` | `(int)value` line 30 |
| `find_value_in_buffer` size | `0`, `1`, small (`2..8`), `255`, `256`, large (`4096`) | `memchr(.., size)` line 35 |
| `find_value_in_buffer` match position | not present, position `0`, interior, last index | `if (result != NULL)` line 36 |
| `find_value_in_buffer` needle range | `0..=255`, `>255`, negative, `128` (signed-char boundary), `INT_MIN`/`INT_MAX` | `(char)search_val` line 34 |
| `create_numeric_buffer` size | `<=0`, `1`, `7` (< the `*7` stride), `256`, `1000` (wraps the `%256` cycle >3×) | loop line 49 |
| `create_numeric_buffer` seed sign/magnitude | `0`, positive small, positive `>=256`, negative, `INT_MAX`, `INT_MIN` (overflow) | `(seed + i*7) % 256` line 50 |
| `calculate_with_doubles` divisor | `b == 0` (guard taken) vs `b != 0` | `if (b != 0)` line 57 |
| `calculate_with_doubles` sign combos | `a`,`b` each `< 0`, `== 0`, `> 0` (incl. `-0.0` result) | line 58 |
| `calculate_with_doubles` exponent | `c % 10` over the full `-9..=9` range (C `%` truncates toward zero) | `pow(10.0, c % 10)` line 61 |
| `doubleneg` `!!` truthiness | each of `param1..param4` zero vs non-zero (2^4 = 16 combos) | lines 76, 81–83 |
| `doubleneg` buffer/search interaction | `param1` seeds the buffer, `param2..4 % 256` are the needles, `param2` also drives the overflowing `param1 + i*param2` sweep | lines 104–134 |

## Rows (each is a distinct combination; `[x]` = passes with randomized inputs)

Every row is exercised with **many** randomized inputs from a fixed-seed
xorshift PRNG (see `tests/common/mod.rs`) in addition to the listed boundary
values, and both libraries are called only through their `.so` exports.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `convert_double_to_int` | in-range integral values, both signs, incl. `0`, `±1`, `±42` | [x] |
| 2  | `convert_double_to_int` | in-range fractional values, both signs (truncation toward zero) | [x] |
| 3  | `convert_double_to_int` | `±0.0`, `f64::MIN_POSITIVE`, subnormal `5e-324` | [x] |
| 4  | `convert_double_to_int` | exact range endpoints `2147483647.0`, `-2147483648.0` | [x] |
| 5  | `convert_double_to_int` | one step past each endpoint: `2147483648.0`, `-2147483649.0`, `2147483647.5`, `-2147483648.5` | [x] |
| 6  | `convert_double_to_int` | huge magnitudes `±1e300`, `±2^40`, `±f64::MAX` | [x] |
| 7  | `convert_double_to_int` | `±INFINITY`, `NaN`, `-NaN` | [x] |
| 8  | `convert_double_to_int` | randomized: uniform bit patterns reinterpreted as `f64` (all classes at once, 20000 draws) | [x] |
| 9  | `convert_double_to_int` | randomized: values scaled across `1e-8 .. 1e12`, both signs (10000 draws) | [x] |
| 10 | `process_negation` | `0`, `1`, `-1`, `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` | [x] |
| 11 | `process_negation` | randomized full-range `i32` (20000 draws) | [x] |
| 12 | `create_numeric_buffer` | `size = 0` / negative / `INT_MIN`, seed arbitrary → buffer untouched (canary-checked) | [x] |
| 13 | `create_numeric_buffer` | `size = 1`, seeds `{0, 1, 255, 256, -1, INT_MAX, INT_MIN}` | [x] |
| 14 | `create_numeric_buffer` | `size = 7` (below the `i*7` stride), same seed set | [x] |
| 15 | `create_numeric_buffer` | `size = 256` (exactly one `% 256` cycle), same seed set | [x] |
| 16 | `create_numeric_buffer` | `size = 1000` (multiple wraps + `i*7` growth), same seed set | [x] |
| 17 | `create_numeric_buffer` | randomized `size ∈ 0..=1024` × randomized full-range `i32` seed (4000 draws), full buffer compared | [x] |
| 18 | `find_value_in_buffer` | `size = 0` with a non-null buffer, and with a NULL buffer | [x] |
| 19 | `find_value_in_buffer` | `size = 1`, needle present / absent | [x] |
| 20 | `find_value_in_buffer` | match at position `0`, interior, and last index (`size-1`) | [x] |
| 21 | `find_value_in_buffer` | needle absent from the whole buffer | [x] |
| 22 | `find_value_in_buffer` | needle sweep `0..=255` over a buffer containing every byte value (`size = 256`) | [x] |
| 23 | `find_value_in_buffer` | needle out of `unsigned char` range (`256..=511`, `1000`, `65536+k`) → low-byte semantics | [x] |
| 24 | `find_value_in_buffer` | negative needles (`-1..=-256`, `INT_MIN`) → low-byte semantics | [x] |
| 25 | `find_value_in_buffer` | signed-`char` boundary needles `127`, `128`, `129`, `255`, `-128`, `-129` | [x] |
| 26 | `find_value_in_buffer` | randomized buffer contents (`size ∈ 0..=4096`) × randomized full-range `i32` needle (8000 draws) | [x] |
| 27 | `find_value_in_buffer` ∘ `create_numeric_buffer` | composed: buffer produced by `create_numeric_buffer` (C-filled and Rust-filled) then searched by both, over randomized seeds/sizes/needles | [x] |
| 28 | `calculate_with_doubles` | `b == 0` guard taken, `a` and `c` swept (incl. negative `c`) → must be exactly `0.0`, sign of zero compared bitwise | [x] |
| 29 | `calculate_with_doubles` | `a == 0`, `b != 0` both signs → `+0.0` vs `-0.0` distinguished bitwise | [x] |
| 30 | `calculate_with_doubles` | `a`,`b` sign cross-product (`+/+`, `+/-`, `-/+`, `-/-`) with `c = 0` | [x] |
| 31 | `calculate_with_doubles` | exponent sweep `c ∈ -25..=25` (covers every `c % 10` value incl. negatives) with fixed non-trivial `a/b` | [x] |
| 32 | `calculate_with_doubles` | extreme operands `a,b ∈ {INT_MIN, INT_MIN+1, -1, 1, INT_MAX-1, INT_MAX}` cross-product, `c` swept | [x] |
| 33 | `calculate_with_doubles` | randomized full-range `a`,`b`,`c` (20000 draws), compared as raw bits | [x] |
| 34 | `doubleneg` | all 16 zero/non-zero truthiness combinations of `param1..param4`, return value + stdout | [x] |
| 35 | `doubleneg` | `param2 == 0` (divisor guard + constant `search_byte` sweep), return value + stdout | [x] |
| 36 | `doubleneg` | negative `param1` (negative buffer bytes) with assorted `param2..4`, return value + stdout | [x] |
| 37 | `doubleneg` | `param1`/`param2` at `INT_MIN`/`INT_MAX` (overflowing `param1 + i*param2`), return value + stdout | [x] |
| 38 | `doubleneg` | `param3` sweep `-25..=25` (drives `pow` exponent **and** a needle), return value + stdout | [x] |
| 39 | `doubleneg` | needle-not-found configurations (`param2/3/4 % 256` absent from the buffer) — provably unreachable (`gcd(7,256)=1` makes the buffer a permutation of all 256 bytes), and both libraries are asserted to agree on that; see ERRORS.md #27/#28 | [x] |
| 40 | `doubleneg` | randomized full-range `param1..param4` (600 draws), return value + full stdout byte-compared | [x] |
| 41 | binary driver | **n/a** — `c_src/CMakeLists.txt` builds only `add_library(... SHARED)`; there is no executable target and no `main`. Instead, stdout parity is checked for every `doubleneg` row above by redirecting fd 1 around each call. | [x] |
| 42 | feature combinations | **n/a / single combo** — `translation/Cargo.toml` has no `[features]`; the full suite is nevertheless re-run under `--no-default-features` and `--all-features` | [x] |
