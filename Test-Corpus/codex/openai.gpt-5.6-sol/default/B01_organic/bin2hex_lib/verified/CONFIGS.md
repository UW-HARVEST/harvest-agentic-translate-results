# Configuration surface

Mechanically derived from the public header and branches in
`../c_src/src/lib.c`.

- Public entry points: `bin2hex` only.
- Runtime options, modes, flags, enums, formats, element types, and byte-order
  choices: none.
- Input-shape axes distinguished by the code: the `while (i < bin_len)` loop
  separates empty, one-element, and many-element inputs; the capacity guard
  separates the minimum valid output capacity (`2 * bin_len + 1`) from larger
  capacities.
- Byte values are randomized across the full `uint8_t` range, covering digit
  and alphabetic high/low nibbles used by the branchless conversion arithmetic.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `bin2hex` | empty input; minimum valid output capacity `1` | [x] |
| 2 | `bin2hex` | empty input; output capacity greater than `1` | [x] |
| 3 | `bin2hex` | one byte; minimum valid output capacity `3`; randomized byte over `0x00..=0xff` | [x] |
| 4 | `bin2hex` | one byte; output capacity greater than `3`; randomized byte over `0x00..=0xff` | [x] |
| 5 | `bin2hex` | many bytes (`2..=4096`); minimum valid capacity `2 * bin_len + 1`; randomized bytes over `0x00..=0xff` | [x] |
| 6 | `bin2hex` | many bytes (`2..=4096`); capacity greater than `2 * bin_len + 1`; randomized bytes over `0x00..=0xff` | [x] |

There are no Cargo features in `Cargo.toml`; the only build configuration is
the default/no-feature configuration.
