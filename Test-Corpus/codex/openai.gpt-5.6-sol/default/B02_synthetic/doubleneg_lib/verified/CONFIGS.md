# Configuration-surface table

Mechanical scan covered all six dynamic entry points and every branch, loop
shape, fixed size, modulo, special floating-point value, and zero/nonzero
condition in `c_src/src/lib.c`. There are no Cargo features.

For rows described as randomized, the differential test uses a fixed seed and
many values. `c % 10` residues mean every C remainder from `-9` through `9`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `convert_double_to_int` | finite integral values in the `int` range, including `INT_MIN`, zero, and `INT_MAX` | [x] |
| 2 | `convert_double_to_int` | positive and negative finite fractional values; truncation toward zero | [x] |
| 3 | `convert_double_to_int` | finite values one step outside and far outside the `int` range, matching the compiled C target behavior | [x] |
| 4 | `convert_double_to_int` | `+INFINITY`, `-INFINITY`, and positive/negative NaNs | [x] |
| 5 | `find_value_in_buffer` | empty prefix (`size == 0`) | [x] |
| 6 | `find_value_in_buffer` | one-byte prefix, target present at offset 0 | [x] |
| 7 | `find_value_in_buffer` | one-byte prefix, target absent | [x] |
| 8 | `find_value_in_buffer` | many-byte prefix, first matching byte at beginning, middle, or end | [x] |
| 9 | `find_value_in_buffer` | many-byte prefix, target absent | [x] |
| 10 | `find_value_in_buffer` | `search_val` outside byte range; C `(char)` conversion and `memchr` byte conversion | [x] |
| 11 | `process_negation` | input is zero | [x] |
| 12 | `process_negation` | input is positive nonzero | [x] |
| 13 | `process_negation` | input is negative nonzero | [x] |
| 14 | `create_numeric_buffer` | negative `size`; loop executes zero times | [x] |
| 15 | `create_numeric_buffer` | zero `size`; loop executes zero times | [x] |
| 16 | `create_numeric_buffer` | one-byte output with positive, zero, and negative seeds | [x] |
| 17 | `create_numeric_buffer` | many-byte output with positive, zero, and negative seeds; modulo wrap through byte values | [x] |
| 18 | `calculate_with_doubles` | `b == 0`, with every `c % 10` residue; initialized zero remains zero after `pow` scaling | [x] |
| 19 | `calculate_with_doubles` | `b != 0`, `c % 10 == -9`, randomized operand signs and magnitudes | [x] |
| 20 | `calculate_with_doubles` | `b != 0`, `c % 10 == -8`, randomized operand signs and magnitudes | [x] |
| 21 | `calculate_with_doubles` | `b != 0`, `c % 10 == -7`, randomized operand signs and magnitudes | [x] |
| 22 | `calculate_with_doubles` | `b != 0`, `c % 10 == -6`, randomized operand signs and magnitudes | [x] |
| 23 | `calculate_with_doubles` | `b != 0`, `c % 10 == -5`, randomized operand signs and magnitudes | [x] |
| 24 | `calculate_with_doubles` | `b != 0`, `c % 10 == -4`, randomized operand signs and magnitudes | [x] |
| 25 | `calculate_with_doubles` | `b != 0`, `c % 10 == -3`, randomized operand signs and magnitudes | [x] |
| 26 | `calculate_with_doubles` | `b != 0`, `c % 10 == -2`, randomized operand signs and magnitudes | [x] |
| 27 | `calculate_with_doubles` | `b != 0`, `c % 10 == -1`, randomized operand signs and magnitudes | [x] |
| 28 | `calculate_with_doubles` | `b != 0`, `c % 10 == 0`, randomized operand signs and magnitudes | [x] |
| 29 | `calculate_with_doubles` | `b != 0`, `c % 10 == 1`, randomized operand signs and magnitudes | [x] |
| 30 | `calculate_with_doubles` | `b != 0`, `c % 10 == 2`, randomized operand signs and magnitudes | [x] |
| 31 | `calculate_with_doubles` | `b != 0`, `c % 10 == 3`, randomized operand signs and magnitudes | [x] |
| 32 | `calculate_with_doubles` | `b != 0`, `c % 10 == 4`, randomized operand signs and magnitudes | [x] |
| 33 | `calculate_with_doubles` | `b != 0`, `c % 10 == 5`, randomized operand signs and magnitudes | [x] |
| 34 | `calculate_with_doubles` | `b != 0`, `c % 10 == 6`, randomized operand signs and magnitudes | [x] |
| 35 | `calculate_with_doubles` | `b != 0`, `c % 10 == 7`, randomized operand signs and magnitudes | [x] |
| 36 | `calculate_with_doubles` | `b != 0`, `c % 10 == 8`, randomized operand signs and magnitudes | [x] |
| 37 | `calculate_with_doubles` | `b != 0`, `c % 10 == 9`, randomized operand signs and magnitudes | [x] |
| 38 | `doubleneg` | zero mask `0000` for `(param1,param2,param3,param4)` | [x] |
| 39 | `doubleneg` | zero mask `0001` | [x] |
| 40 | `doubleneg` | zero mask `0010` | [x] |
| 41 | `doubleneg` | zero mask `0011` | [x] |
| 42 | `doubleneg` | zero mask `0100` | [x] |
| 43 | `doubleneg` | zero mask `0101` | [x] |
| 44 | `doubleneg` | zero mask `0110` | [x] |
| 45 | `doubleneg` | zero mask `0111` | [x] |
| 46 | `doubleneg` | zero mask `1000` | [x] |
| 47 | `doubleneg` | zero mask `1001` | [x] |
| 48 | `doubleneg` | zero mask `1010` | [x] |
| 49 | `doubleneg` | zero mask `1011` | [x] |
| 50 | `doubleneg` | zero mask `1100` | [x] |
| 51 | `doubleneg` | zero mask `1101` | [x] |
| 52 | `doubleneg` | zero mask `1110` | [x] |
| 53 | `doubleneg` | zero mask `1111` | [x] |

For zero masks, bit order is `(param1,param2,param3,param4)`, `0` means the
parameter is exactly zero, and `1` means nonzero. Each nonzero-mask row covers
random positive/negative values, all applicable `param3 % 10` residues, the
four fixed buffer searches, the direct byte-100 search, and all ten combined
search iterations. The generated buffer always has 256 bytes.

The CMake and Cargo manifests define shared libraries only; there is no binary
driver requiring a stdout comparison. The `doubleneg` FFI test captures and
compares its complete stdout byte-for-byte.
