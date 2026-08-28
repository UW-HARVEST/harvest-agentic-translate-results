# Configuration Surface

The only public entry point is the low-level `hex2bin` function; there are no
wrappers, compile-time feature flags, enums, or other runtime options. Rows
below enumerate the cross-product portions that the C branches distinguish:
loop count, character classifier, output capacity, ignore-pointer state,
nibble state, termination mode, and endpoint-pointer state.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C01 | `hex2bin` | Empty input (`hex_len == 0`) with null `hex`, null `bin`, null `ignore`, and null `hex_end_p` | [x] |
| C02 | `hex2bin` | Empty input with non-null `hex_end_p`; endpoint is set to the input base without dereferencing it | [x] |
| C03 | `hex2bin` | One output byte, decimal digit nibbles only, exact output capacity, null endpoint | [x] |
| C04 | `hex2bin` | One output byte containing uppercase `A..F` nibbles, spare output capacity, non-null endpoint | [x] |
| C05 | `hex2bin` | One output byte containing lowercase `a..f` nibbles and mixed digit/alpha pairs | [x] |
| C06 | `hex2bin` | Many output bytes with mixed digit, uppercase, and lowercase nibbles; exact capacity | [x] |
| C07 | `hex2bin` | Many output bytes with mixed valid nibbles and spare capacity (`bin_maxlen > decoded length`) | [x] |
| C08 | `hex2bin` | Non-null ignore string, with leading, trailing, repeated, and inter-byte ignored separators while `state == 0` | [x] |
| C09 | `hex2bin` | Non-null ignore string present but no input byte matches it | [x] |
| C10 | `hex2bin` | Non-null ignore string containing multiple distinct separator bytes, each skipped only at byte boundaries | [x] |
| C11 | `hex2bin` | Non-null ignore string whose terminator makes input NUL match `strchr(ignore, 0)` and be skipped at a byte boundary | [x] |
| C12 | `hex2bin` | Non-null ignore string containing a high-bit byte (`0x80..0xff`) that is skipped at a byte boundary | [x] |
| C13 | `hex2bin` | Non-hex terminator at a byte boundary with non-null endpoint after a decoded prefix; successful partial parse | [x] |
| C14 | `hex2bin` | Non-hex terminator as the first input byte with non-null endpoint; successful zero-byte partial parse | [x] |
| C15 | `hex2bin` | High-bit non-hex terminator at a byte boundary with non-null endpoint | [x] |
| C16 | `hex2bin` | Explicit `hex_len` selects an even-length valid prefix from a larger backing buffer | [x] |
| C17 | `hex2bin` | Full valid input with non-null endpoint; endpoint is exactly `hex + hex_len` | [x] |
| C18 | `hex2bin` | Huge declared lengths in safe short-circuit forms: `bin_maxlen == usize::MAX` for a small valid input, and `hex_len == usize::MAX` when byte zero is an immediate terminator | [x] |

Feature combinations: only the empty feature set exists in `Cargo.toml`; it
passes both default and explicit `--no-default-features` runs.
