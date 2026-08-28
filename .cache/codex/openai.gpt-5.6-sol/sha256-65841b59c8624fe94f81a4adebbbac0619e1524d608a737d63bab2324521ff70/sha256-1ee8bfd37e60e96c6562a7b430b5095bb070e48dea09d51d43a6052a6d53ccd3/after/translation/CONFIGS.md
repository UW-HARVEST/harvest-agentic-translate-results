# Configuration Surface

The public header declares one entry point. The C implementation contains no
runtime options, flags, modes, conditional branches, switches, compile-time
branches, variable lengths, element-type choices, or alternate formats.

The input shape is fixed: four `uint32_t` state words and one writable 16-byte
output. The function serializes each word least-significant byte first, in
`a`, `b`, `c`, `d` order. Full-domain randomized inputs include zero, one,
per-byte patterns, high-bit values, `UINT32_MAX` boundaries, and valid overlap
between the byte output and state storage.

`Cargo.toml` declares no features, so there is one feature configuration. It is
verified with both default feature handling and `--no-default-features`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `md5_digest` | No options; valid `tflac_md5 *`; writable `uint8_t[16]`; four arbitrary `uint32_t` words | [x] |
