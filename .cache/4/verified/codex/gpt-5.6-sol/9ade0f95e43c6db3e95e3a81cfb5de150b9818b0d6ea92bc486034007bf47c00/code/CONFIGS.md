# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options or conditional compilation. There is exactly one valid Rust feature
combination:

| # | Rust feature set | C configuration | [ ] |
|---|------------------|-----------------|-----|
| B1 | empty set (`--no-default-features`) | default, no compile definitions | [x] |

The CMake target is an executable only. For FFI testing, the same unmodified C
source and default preprocessor configuration is also compiled as
`c_src/build/libdriver_c.so`.

## Runtime configurations

The public surface consists of the low-level `driver(char)` function and the
composed `main(void)` entry point. There are no runtime options, modes, or
flags. The rows below partition the complete 8-bit `char` input domain by every
distinct combination of ctype classifications and case-conversion behavior in
the C source. This platform uses signed `char`, as reflected by the generated C
object code.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | byte `0x00..0x08` or `0x0e..0x1f`: non-whitespace control | [x] |
| C2 | `driver` | byte `0x09`: control, whitespace, and blank | [x] |
| C3 | `driver` | byte `0x0a..0x0d`: control and whitespace, not blank | [x] |
| C4 | `driver` | byte `0x20`: whitespace, blank, and printing | [x] |
| C5 | `driver` | ASCII punctuation: graphical, printing, and punctuation | [x] |
| C6 | `driver` | byte `0x30..0x39`: decimal digit and hexadecimal digit | [x] |
| C7 | `driver` | byte `A..F`: uppercase hexadecimal letter, converted to lowercase | [x] |
| C8 | `driver` | byte `G..Z`: uppercase non-hexadecimal letter, converted to lowercase | [x] |
| C9 | `driver` | byte `a..f`: lowercase hexadecimal letter, converted to uppercase | [x] |
| C10 | `driver` | byte `g..z`: lowercase non-hexadecimal letter, converted to uppercase | [x] |
| C11 | `driver` | byte `0x7f`: DEL control | [x] |
| C12 | `driver` | byte `0x80..0xfe`: negative signed `char`, no C-locale classifications | [x] |
| C13 | `driver` | byte `0xff`: signed `char` value `-1` / ctype EOF value | [x] |
| C14 | `main` | empty stdin: `getchar()` returns EOF and converts it to byte `0xff` | [x] |
| C15 | `main` | exactly one input byte, randomized across the full `0x00..0xff` domain | [x] |
| C16 | `main` | multiple input bytes: only the randomized first byte is consumed | [x] |

## Completion

- [x] Every runtime configuration row passes byte-for-byte differential tests.
- [x] The build-time configuration passes checks, tests, and symbol parity.
