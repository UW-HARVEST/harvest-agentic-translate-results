# Configuration Surface

The public API has one entry point and no runtime modes, flags, enums,
features, formats, element types, or byte-order options. The C control flow
distinguishes zero, one, and repeated loop iterations. Its capacity check
distinguishes the minimum valid capacity (`2 * bin_len + 1`) from invalid
capacities; larger valid capacities follow the same body but are retained here
to verify that trailing bytes remain untouched. Randomized non-empty rows cover
all byte and high/low-nibble values, including `0x00` and `0xff`.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|-------------------------------------------|---------|
| 1 | `bin2hex` | no options; empty input; minimum valid output capacity (1) | [x] |
| 2 | `bin2hex` | no options; empty input; output capacity greater than 1 | [x] |
| 3 | `bin2hex` | no options; one-byte input; minimum valid output capacity (3) | [x] |
| 4 | `bin2hex` | no options; one-byte input; output capacity greater than 3 | [x] |
| 5 | `bin2hex` | no options; many-byte input; minimum valid output capacity (`2n + 1`) | [x] |
| 6 | `bin2hex` | no options; many-byte input; output capacity greater than `2n + 1` | [x] |
