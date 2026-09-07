# Configuration Surface

Public header inspection finds one entry point:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

There are no runtime modes, option setters, flags, enums, formats, element
types, byte-order options, compile-time `#ifdef` branches, Cargo features, or
binary drivers.

The C implementation branches only on input length:

- `len >= 8`: one or more slicing-by-8 iterations;
- remaining `len != 0`: zero or more byte-at-a-time tail iterations.

Randomized coverage for every row includes arbitrary byte values and arbitrary
initial CRC values across the full `uint16_t` range.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `crc16` | Empty input (`len == 0`): neither loop executes | [x] |
| 2 | `crc16` | Tail only (`len == 1..7`): byte loop executes, slicing loop does not | [x] |
| 3 | `crc16` | Exactly one full slice (`len == 8`): slicing loop executes once, no tail | [x] |
| 4 | `crc16` | One full slice plus tail (`len == 9..15`): both loops execute | [x] |
| 5 | `crc16` | Multiple full slices (`len >= 16`), covering both multiples of 8 and a final tail | [x] |
