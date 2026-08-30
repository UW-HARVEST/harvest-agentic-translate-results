# Configuration Surface

The public header contains one entry point, `void driver(char)`. The
implementation unconditionally selects the `"C"` locale and has no runtime
options, flags, modes, sizes, pointers, counts, formats, or compile-time
feature branches. The rows below partition the full 8-bit `char` domain by the
distinct combinations observed by the C-locale classification and conversion
operations called in `driver.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | C locale; signed negative `char` values (`0x80..0xff`) | [x] |
| 2 | `driver` | C locale; NUL (`0x00`) | [x] |
| 3 | `driver` | C locale; horizontal tab (`0x09`: control + space + blank) | [x] |
| 4 | `driver` | C locale; newline-style whitespace (`0x0a..0x0d`: control + space) | [x] |
| 5 | `driver` | C locale; other controls (`0x01..0x08`, `0x0e..0x1f`, `0x7f`) | [x] |
| 6 | `driver` | C locale; space (`0x20`: space + blank + printing) | [x] |
| 7 | `driver` | C locale; decimal digit (`0x30..0x39`: alphanumeric + digit + hexadecimal) | [x] |
| 8 | `driver` | C locale; uppercase hexadecimal letter (`A..F`) | [x] |
| 9 | `driver` | C locale; uppercase non-hexadecimal letter (`G..Z`) | [x] |
| 10 | `driver` | C locale; lowercase hexadecimal letter (`a..f`) | [x] |
| 11 | `driver` | C locale; lowercase non-hexadecimal letter (`g..z`) | [x] |
| 12 | `driver` | C locale; printable punctuation (`0x21..0x7e`, excluding alphanumerics) | [x] |

Together these rows cover all 256 argument bit patterns. Because the source
defines no Cargo features or C preprocessor configurations, this is the only
feature configuration.
