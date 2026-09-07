# Configuration Surface

Mechanically derived from the public header, all global functions in
`../c_src/src/driver.c`, and every `if`, `switch`, and preprocessor
conditional in the C implementation. The implementation contains no runtime
options, modes, flags, data-shape branches, or feature conditionals. On this
platform, C `char` is signed and has an 8-bit object representation.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printHexCharLine` | Direct low-level call; every one of the 256 possible `char` object representations, in fixed-seed randomized order | [x] |
| 2 | `driver` | Composed public call (`char` increment followed by `printHexCharLine`); every one of the 256 possible `char` object representations, in fixed-seed randomized order | [x] |

`driver` is the sole function declared in the public header.
`printHexCharLine` is nevertheless part of the externally callable ABI
because the C shared object exports it.

Each row passed with its complete 256-value domain plus 4,096 additional
fixed-seed randomized inputs, comparing captured stdout byte-for-byte.
