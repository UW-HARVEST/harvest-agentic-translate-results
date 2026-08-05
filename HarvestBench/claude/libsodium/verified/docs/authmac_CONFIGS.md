# authmac — Configuration / input-shape coverage

Family: AUTH / MAC / VERIFY. Each row is a meaningful configuration exercised
with many randomized inputs (fixed seeds) over varied lengths
(`0, 1, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 200, 255, 256,
257, 1000, 4096, 5000` — covering empty, tiny, and SHA-256/512 & poly1305 block
boundaries). All rows assert return code **and** output bytes match the C `.so`.

| # | entry point(s) | configuration (options + shape) | done |
|---|----------------|----------------------------------|------|
| 1 | `crypto_verify_16` | equal buffers (64 iters) ⇒ 0 | [x] |
| 2 | `crypto_verify_16` | random pairs (256) + every single-bit diff position ⇒ -1 | [x] |
| 3 | `crypto_verify_32` | equal + random + per-position single-bit diffs | [x] |
| 4 | `crypto_verify_64` | equal + random + per-position single-bit diffs | [x] |
| 5 | `crypto_verify_16/32/64_bytes` | size constant query = 16/32/64 | [x] |
| 6 | `crypto_auth` + `crypto_auth_verify` | default primitive (hmacsha512256), all lengths, good/tampered/wrong-key/truncated | [x] |
| 7 | `crypto_auth_hmacsha256` + `_verify` | one-shot 32B key, all lengths, 32B tag, error paths | [x] |
| 8 | `crypto_auth_hmacsha256_{init,update,final}` | streaming, random chunking, keylens {0,1,32,64,65,100,200} incl. `>64` rehash branch; equals one-shot | [x] |
| 9 | `crypto_auth_hmacsha512` + `_verify` | one-shot 32B key, all lengths, 64B tag, error paths | [x] |
| 10 | `crypto_auth_hmacsha512_{init,update,final}` | streaming, random chunking, varied keylens, one-shot equivalence | [x] |
| 11 | `crypto_auth_hmacsha512256` + `_verify` | one-shot 32B key, all lengths, 32B truncated tag, error paths | [x] |
| 12 | `crypto_auth_hmacsha512256_{init,update,final}` | streaming, random chunking, varied keylens, one-shot equivalence | [x] |
| 13 | `crypto_onetimeauth` + `_verify` | poly1305 default, 32B key, all lengths, 16B tag, error paths | [x] |
| 14 | `crypto_onetimeauth_{init,update,final}` | streaming, random chunking, one-shot equivalence | [x] |
| 15 | `crypto_onetimeauth_poly1305` + `_verify` | explicit poly1305, all lengths, error paths | [x] |
| 16 | `crypto_onetimeauth_poly1305_{init,update,final}` | streaming, random chunking, one-shot equivalence | [x] |
| 17 | zero-length `_update` no-op | HMAC streaming feeds an extra `update(_, 0)` — must be no-op | [x] |
| 18 | size/primitive constants | `*_bytes`, `*_keybytes`, `*_statebytes`, `*_primitive` for all variants | [x] |

## Key sub-shapes explicitly covered

- **Streaming vs one-shot equivalence:** for every HMAC and poly1305 variant,
  a randomly chunked init/update/final run is asserted equal to the one-shot
  output on both libraries.
- **HMAC key-length branches:** streaming init keylens span `0` (null-key
  no-op), `1`, `32`, `64` (block-size boundary), and `65/100/200` which trigger
  the `keylen > 64` "hash the key first" branch in `*_init`.
- **Empty / null input:** `inlen == 0` is passed with a NULL message pointer to
  match how a C caller would invoke it.
- **Constant-time verify:** verify results are compared for good tags,
  bit-flipped tags, wrong keys, and truncated messages.
