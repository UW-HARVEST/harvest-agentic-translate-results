# Configuration surface

The C library has one public entry point, no runtime options or modes, and no
compile-time feature branches. The input is one 16-bit IEEE-754 half bit
pattern. The rows below mechanically partition the sign bit and the exponent /
fraction classes represented by the lookup-table indices and entries.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | positive zero: sign `0`, exponent `0`, fraction `0` | [x] |
| 2 | `half2float` | positive subnormal: sign `0`, exponent `0`, fraction `1..=1023` | [x] |
| 3 | `half2float` | positive normal: sign `0`, exponent `1..=30`, fraction `0..=1023` | [x] |
| 4 | `half2float` | positive infinity: sign `0`, exponent `31`, fraction `0` | [x] |
| 5 | `half2float` | positive NaN: sign `0`, exponent `31`, fraction `1..=1023` | [x] |
| 6 | `half2float` | negative zero: sign `1`, exponent `0`, fraction `0` | [x] |
| 7 | `half2float` | negative subnormal: sign `1`, exponent `0`, fraction `1..=1023` | [x] |
| 8 | `half2float` | negative normal: sign `1`, exponent `1..=30`, fraction `0..=1023` | [x] |
| 9 | `half2float` | negative infinity: sign `1`, exponent `31`, fraction `0` | [x] |
| 10 | `half2float` | negative NaN: sign `1`, exponent `31`, fraction `1..=1023` | [x] |
