# Configuration Surface

The public header declares only `void driver(char c)`. The implementation has
no runtime option, flag, mode, conditional, switch, or compile-time feature
branch. It unconditionally selects locale `"C"` and applies all twelve
classification operations plus both case conversions.

The rows below partition the complete 256-value `char` bit-pattern domain by
the combinations distinguished by the C-locale `ctype` operations invoked in
`driver.c`. Byte notation describes the external FFI bit pattern; on this
build target plain `char` is signed, so `0x80..0xff` arrive as negative values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | fixed C locale; non-whitespace controls: `0x00..0x08`, `0x0e..0x1f`, `0x7f` | [x] |
| 2 | `driver` | fixed C locale; horizontal tab: `0x09` (control + space + blank) | [x] |
| 3 | `driver` | fixed C locale; other whitespace controls: `0x0a..0x0d` (control + space) | [x] |
| 4 | `driver` | fixed C locale; ASCII space: `0x20` (space + blank + printing) | [x] |
| 5 | `driver` | fixed C locale; punctuation: `0x21..0x2f`, `0x3a..0x40`, `0x5b..0x60`, `0x7b..0x7e` | [x] |
| 6 | `driver` | fixed C locale; decimal/hex digits: `0x30..0x39` | [x] |
| 7 | `driver` | fixed C locale; uppercase hexadecimal letters: `A..F` | [x] |
| 8 | `driver` | fixed C locale; other uppercase letters: `G..Z` | [x] |
| 9 | `driver` | fixed C locale; lowercase hexadecimal letters: `a..f` | [x] |
| 10 | `driver` | fixed C locale; other lowercase letters: `g..z` | [x] |
| 11 | `driver` | fixed C locale; negative promoted `char` values from bytes `0x80..0xfe` | [x] |
| 12 | `driver` | fixed C locale; byte `0xff`, promoted to `EOF` (`-1`) on this signed-`char` target | [x] |

There are no Cargo features and CMake defines no optional build configuration,
so this table is the complete configuration matrix for both libraries.
