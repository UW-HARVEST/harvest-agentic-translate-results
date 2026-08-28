# Configuration Surface

Mechanical scan scope: the sole public declaration in
`../c_src/include/lib.h` and both loops in `../c_src/src/lib.c`.

There are no Cargo features and no C preprocessor feature branches. The input
bytes and initial CRC span their full `uint8_t` and `uint16_t` domains; tests
randomize both for every applicable row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `crc16` | `len == 0`; neither loop executes; null and non-null `d`; boundary initial CRC values | [x] |
| 2 | `crc16` | `1 <= len <= 7`; byte-at-a-time tail loop only | [x] |
| 3 | `crc16` | `len == 8`; one slicing-by-eight iteration, no tail | [x] |
| 4 | `crc16` | `9 <= len <= 15`; one slicing-by-eight iteration followed by tail bytes | [x] |
| 5 | `crc16` | `len >= 16` and divisible by 8; multiple slicing-by-eight iterations, no tail | [x] |
| 6 | `crc16` | `len >= 16` and not divisible by 8; multiple slicing-by-eight iterations followed by tail bytes | [x] |
