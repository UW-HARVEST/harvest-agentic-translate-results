# Configuration Surface

The public API has one entry point, no mutable state, no runtime options,
modes, flags, formats, element counts, byte-order setting, or feature-gated C
branches. The implementation masks every stage to the low 16 bits, making the
16-bit cutoff the only value-shape distinction visible in the source.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `rev16` | No options; `a` is in the 16-bit domain (`0x00000000..=0x0000ffff`) | [x] |
| 2 | `rev16` | No options; at least one upper-half bit is set and the low half is arbitrary (`a & 0xffff0000 != 0`); C discards all upper-half bits | [x] |
| 3 | `rev16` | No options; boundary-focused low halves (`0`, `1`, `0x7fff`, `0x8000`, `0xffff`) combined with zero and randomized upper-half bits, including exact inputs `0x10000`, `0x10001`, and `u32::MAX` | [x] |

All rows pass in debug and release profiles with both default settings and
`--no-default-features`. `Cargo.toml` defines no named features, so these are
the complete effective feature combinations.
