# Configuration-Surface Table

Mechanical source scan covered the public header and every branch, loop, and
size-dependent operation in `../c_src/src/driver.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(int)` | No options or modes; one by-value C `int`. Exercise zero, signed extrema, sign-boundary values, repeated-byte and alternating-byte patterns, plus randomized values across all 32 bits. Output is the `sizeof(int)` native-endian object representation as lowercase hexadecimal followed by newline. | [x] |

There are no Cargo features, C preprocessor feature branches, runtime flags,
alternate formats, pointers, variable lengths, or additional public entry
points.

Verified with 20 boundary/pattern values and 10,000 fixed-seed randomized
values under both the default and `--no-default-features` Cargo configurations.
