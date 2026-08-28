# Configuration Surface

The public API has one entry point, no runtime options, and no compile-time
feature switches. The C lookup-table indices distinguish sign, exponent class,
and zero/nonzero fraction as follows.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `half2float` | positive zero: sign `0`, exponent `0`, fraction `0` | [x] |
| 2 | `half2float` | negative zero: sign `1`, exponent `0`, fraction `0` | [x] |
| 3 | `half2float` | positive subnormal: sign `0`, exponent `0`, fraction `1..=1023` | [x] |
| 4 | `half2float` | negative subnormal: sign `1`, exponent `0`, fraction `1..=1023` | [x] |
| 5 | `half2float` | positive normal: sign `0`, exponent `1..=30`, fraction `0..=1023` | [x] |
| 6 | `half2float` | negative normal: sign `1`, exponent `1..=30`, fraction `0..=1023` | [x] |
| 7 | `half2float` | positive infinity: sign `0`, exponent `31`, fraction `0` | [x] |
| 8 | `half2float` | negative infinity: sign `1`, exponent `31`, fraction `0` | [x] |
| 9 | `half2float` | positive NaN: sign `0`, exponent `31`, fraction `1..=1023` | [x] |
| 10 | `half2float` | negative NaN: sign `1`, exponent `31`, fraction `1..=1023` | [x] |
