# Configuration-Surface Table

Mechanically derived from all three exported entry points and the C branches
on `x < 0`, `fopen`, the `fgets` loop with a 100-byte buffer (99-byte maximum
payload per call), `ferror`, and the two composed `driver` result checks.

There are no runtime options, modes, flags, compile-time feature branches, or
Cargo features. The sole build configuration is the featureless library; it is
tested both normally and with `--no-default-features`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `forward_goto_example` | valid integer branch `x >= 0`, randomized across zero, small values, the multiplication-overflow boundary, and `INT_MAX` | [x] |
| 2 | `open_with_cleanup` | readable empty file; `fgets` loop executes zero times | [x] |
| 3 | `open_with_cleanup` | one short newline-terminated line, 1..98 bytes total; one `fgets` iteration | [x] |
| 4 | `open_with_cleanup` | one short final line without newline, 1..98 bytes; one `fgets` iteration followed by EOF | [x] |
| 5 | `open_with_cleanup` | multiple short lines; multiple `fgets` iterations split by newline | [x] |
| 6 | `open_with_cleanup` | exactly 99 non-newline bytes; maximum payload in one `fgets` iteration | [x] |
| 7 | `open_with_cleanup` | more than 99 bytes in a line; `fgets` splits it across multiple chunks | [x] |
| 8 | `open_with_cleanup` | readable bytes containing embedded NUL; `fgets` consumes them while `printf("%s")` truncates each printed chunk at NUL | [x] |
| 9 | `driver` | valid randomized `num >= 0` plus readable empty file | [x] |
| 10 | `driver` | valid randomized `num >= 0` plus one short newline-terminated line | [x] |
| 11 | `driver` | valid randomized `num >= 0` plus one short final line without newline | [x] |
| 12 | `driver` | valid randomized `num >= 0` plus multiple short lines | [x] |
| 13 | `driver` | valid randomized `num >= 0` plus exactly 99 non-newline bytes | [x] |
| 14 | `driver` | valid randomized `num >= 0` plus a line longer than 99 bytes | [x] |
| 15 | `driver` | valid randomized `num >= 0` plus readable bytes containing embedded NUL | [x] |
