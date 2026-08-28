# Configuration Surface

Mechanical scan scope: all six globally exported functions in
`../c_src/src/lib.c`, including the five not declared by the minimal public
header. There are no compile-time or Cargo feature flags.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| C01 | `convert_double_to_int` | finite, in-`int`-range doubles: negative/zero/positive, integral/fractional, and representable boundaries | [x] |
| C02 | `convert_double_to_int` | x86-64 cast edge shapes used by the C source: `NAN`, infinities, and finite values outside `int` range | [x] |
| C03 | `find_value_in_buffer` | empty search (`size == 0`), including a null pointer that `memchr` does not access | [x] |
| C04 | `find_value_in_buffer` | one-byte search with target present; `search_val` is converted through `char` | [x] |
| C05 | `find_value_in_buffer` | one-byte search with target absent | [x] |
| C06 | `find_value_in_buffer` | many-byte search with target present once or repeatedly; first offset must be returned | [x] |
| C07 | `find_value_in_buffer` | many-byte search with target absent, including `size` one past the internal 256-byte convenience-buffer size with sufficient backing storage | [x] |
| C08 | `process_negation` | input is zero | [x] |
| C09 | `process_negation` | input is nonzero (negative, positive, and integer boundaries) | [x] |
| C10 | `create_numeric_buffer` | `size <= 0`; loop executes zero times, including null buffer at size zero | [x] |
| C11 | `create_numeric_buffer` | `size == 1`; positive, zero, and negative seeds | [x] |
| C12 | `create_numeric_buffer` | `size > 1`; modulo wrap, signed seeds, and lengths around 256 | [x] |
| C13 | `calculate_with_doubles` | `b == 0`; division branch skipped for all signs of `a` and `c % 10` | [x] |
| C14 | `calculate_with_doubles` | `b != 0` and `c % 10 < 0` | [x] |
| C15 | `calculate_with_doubles` | `b != 0` and `c % 10 == 0` | [x] |
| C16 | `calculate_with_doubles` | `b != 0` and `c % 10 > 0` | [x] |
| C17 | `doubleneg` | `param2 == 0`; division skipped, with every zero/nonzero combination of `param1`, `param3`, and `param4` | [x] |
| C18 | `doubleneg` | `param2 != 0`; negative/zero/positive `param3 % 10`, all parameter signs, and zero/nonzero negation states | [x] |
| C19 | `doubleneg` | generated 256-byte buffer is a full byte permutation (`gcd(7, 256) == 1`), so all four value searches, direct byte-100 search, and ten combined searches take their found branches while offsets vary | [x] |
| C20 | `doubleneg` | integer boundary-shaped parameters that keep C arithmetic defined while stressing byte conversion and double-to-int boundaries | [x] |
