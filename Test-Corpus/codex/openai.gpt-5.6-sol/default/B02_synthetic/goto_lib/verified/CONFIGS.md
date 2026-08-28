# Configuration Surface

The C library has no runtime options, modes, flags, enums, `switch` statements,
conditional-compilation branches, or Cargo features. Its effective feature
matrix has one configuration: no declared features (default and
`--no-default-features` are equivalent).

The source branches on the sign of `x`, whether `fopen` succeeds, the number of
successful `fgets` iterations with a 100-byte buffer (at most 99 data bytes per
call), and `ferror`. Valid rows below cover the nonnegative integer classes and
the readable-file shapes. Embedded NUL is distinct because each chunk is
emitted with `printf("%s", buffer)`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `forward_goto_example` | `x = 0` boundary | [x] |
| 2 | `forward_goto_example` | `1 <= x <= INT_MAX / 2` (doubling is representable) | [x] |
| 3 | `forward_goto_example` | `INT_MAX / 2 < x <= INT_MAX` (compiled C integer wrap behavior) | [x] |
| 4 | `open_with_cleanup` | readable empty file; zero `fgets` iterations | [x] |
| 5 | `open_with_cleanup` | readable nonempty file emitted by one `fgets` iteration (1-99 bytes, with or without newline) | [x] |
| 6 | `open_with_cleanup` | readable file emitted by multiple `fgets` iterations (multiple lines and/or chunks over 99 bytes) | [x] |
| 7 | `open_with_cleanup` | readable file containing an embedded NUL in an `fgets` chunk | [x] |
| 8 | `driver` | `x = 0`; readable empty file | [x] |
| 9 | `driver` | `x = 0`; one emitted file chunk | [x] |
| 10 | `driver` | `x = 0`; multiple emitted file chunks | [x] |
| 11 | `driver` | `x = 0`; embedded NUL in a file chunk | [x] |
| 12 | `driver` | representable positive doubling; readable empty file | [x] |
| 13 | `driver` | representable positive doubling; one emitted file chunk | [x] |
| 14 | `driver` | representable positive doubling; multiple emitted file chunks | [x] |
| 15 | `driver` | representable positive doubling; embedded NUL in a file chunk | [x] |
| 16 | `driver` | wrapping positive doubling; readable empty file | [x] |
| 17 | `driver` | wrapping positive doubling; one emitted file chunk | [x] |
| 18 | `driver` | wrapping positive doubling; multiple emitted file chunks | [x] |
| 19 | `driver` | wrapping positive doubling; embedded NUL in a file chunk | [x] |
