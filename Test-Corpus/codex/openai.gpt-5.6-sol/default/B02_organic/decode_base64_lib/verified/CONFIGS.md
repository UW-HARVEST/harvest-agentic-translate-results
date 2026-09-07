# Configuration surface

The public API has one entry point and no runtime options, modes, flags,
element-type choices, byte-order choices, compile-time feature branches, or
public state. The rows below enumerate the combinations distinguished by the
actual filtering, quartet-width, character-decoding, and padding branches in
`c_src/src/lib.c`. Null and empty inputs are rejection cases in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `decode_base64` | nonempty source; filtering leaves zero base64 characters (all bytes ignored) | [x] |
| 2 | `decode_base64` | filtered length is `1 mod 4`; missing `c2`, `c3`, and `c4` default to `A` | [x] |
| 3 | `decode_base64` | filtered length is `2 mod 4`; missing `c3` and `c4` default to `A` | [x] |
| 4 | `decode_base64` | filtered length is `3 mod 4`; missing `c4` defaults to `A` | [x] |
| 5 | `decode_base64` | one complete quartet with no `=` padding | [x] |
| 6 | `decode_base64` | complete quartet with `c3 == '='` and `c4 == '='`; both conditional output branches are suppressed | [x] |
| 7 | `decode_base64` | complete quartet with data in `c3` and `c4 == '='`; only the third output byte is suppressed | [x] |
| 8 | `decode_base64` | complete quartet with `c3 == '='` and data in `c4`; second output byte is suppressed but third is emitted | [x] |
| 9 | `decode_base64` | `=` occurs in `c1`; `decode('=')` takes the default value `63` | [x] |
| 10 | `decode_base64` | `=` occurs in `c2`; `decode('=')` takes the default value `63` | [x] |
| 11 | `decode_base64` | multiple quartets, including independently padded/unpadded quartets | [x] |
| 12 | `decode_base64` | ignored non-base64 ASCII bytes occur before, between, and after accepted bytes | [x] |
| 13 | `decode_base64` | accepted bytes exercise every decode class and its boundaries: `A-Z`, `a-z`, `0-9`, `+`, `/`, and `=` | [x] |
| 14 | `decode_base64` | non-ASCII bytes (`0x80-0xff`) are present and ignored by `is_base64` | [x] |
| 15 | `decode_base64` | long nonempty source with many quartets and ignored bytes (oversized generic boundary/stress shape; no C maximum exists) | [x] |

Cargo declares no features, so the only feature configuration is the default
empty feature set (equivalent to `--no-default-features`).
