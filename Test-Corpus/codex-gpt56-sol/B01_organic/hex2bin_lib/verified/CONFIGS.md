# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` defines a feature, option,
conditional source, or preprocessor definition. There is exactly one valid
feature combination:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| B01 | `--no-default-features` (empty feature set) | default and only CMake configuration |

## Runtime and Input Configurations

The only public entry point in `c_src/include/lib.h` is the low-level
`hex2bin` function. Rows below are the pruned cross-product of branches in
`c_src/src/lib.c`: loop shape, numeric/upper/lower character class, output
capacity, parser state, ignore pointer/membership, termination location, and
end-pointer mode.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| C01 | `hex2bin` | Empty input; zero output capacity; `hex_end_p == NULL`. | [x] |
| C02 | `hex2bin` | Empty input; nonzero output capacity; non-null `hex_end_p`, which must point to input offset zero. | [x] |
| C03 | `hex2bin` | One output byte from two decimal nibbles; exact output capacity; `hex_end_p == NULL`. | [x] |
| C04 | `hex2bin` | One output byte using uppercase `A`-`F`; exact output capacity; non-null `hex_end_p`. | [x] |
| C05 | `hex2bin` | One output byte using lowercase `a`-`f`; excess output capacity; both end-pointer modes. | [x] |
| C06 | `hex2bin` | Many output bytes with mixed numeric, uppercase, and lowercase nibbles; exact output capacity. | [x] |
| C07 | `hex2bin` | Many output bytes with mixed nibble classes; excess output capacity and non-null `hex_end_p`. | [x] |
| C08 | `hex2bin` | Non-null `ignore`; one listed invalid separator at `state == 0`; complete consumption with `hex_end_p == NULL`. | [x] |
| C09 | `hex2bin` | Non-null `ignore`; repeated and varied listed separators at the beginning, between bytes, and at the end; non-null `hex_end_p`. | [x] |
| C10 | `hex2bin` | Non-null `ignore` whose string contains valid hex characters; valid hex bytes are parsed rather than ignored. | [x] |
| C11 | `hex2bin` | Non-null `ignore` and a NUL byte in length-delimited input at `state == 0`; `strchr(ignore, 0)` matches the ignore terminator, so the NUL is skipped. | [x] |
| C12 | `hex2bin` | Invalid byte at input offset zero; `ignore == NULL`; non-null `hex_end_p`; successful zero-byte prefix result. | [x] |
| C13 | `hex2bin` | Invalid byte after one or many complete bytes; `ignore == NULL`; non-null `hex_end_p`; successful partial-prefix result. | [x] |
| C14 | `hex2bin` | Invalid byte absent from a non-null `ignore` string after complete bytes; non-null `hex_end_p`; successful partial-prefix result. | [x] |
| C15 | `hex2bin` | Invalid boundary bytes around accepted classes (`/`, `:`, `@`, `G`, `` ` ``, `g`) and high-bit bytes; non-null `hex_end_p`. | [x] |
| C16 | `hex2bin` | Safe null pointers: `hex == NULL` with `hex_len == 0`, and `bin == NULL` when empty input prevents a write. | [x] |
| C17 | `hex2bin` | Zero output capacity with invalid input at offset zero and non-null `hex_end_p`; capacity is never consulted and the result is zero. | [x] |
| C18 | `hex2bin` | Large allocated, valid, even-length input; exact and excess capacities; verifies the unrestricted `size_t` length path. | [x] |
