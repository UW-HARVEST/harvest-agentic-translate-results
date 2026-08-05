# streamcore configurations

Each row is a meaningful configuration exercised in `tests/streamcore.rs`.
All configs run many randomized inputs (fixed seeds) across lengths
`{0,1,7,31,63,64,65,127,128,129,191,256,1000,4096}` with random keys, nonces,
and (where applicable) random initial counters. `_xor` / `_xor_ic` configs also
assert xor-then-xor == identity within EACH library.

| # | entry point(s) | configuration (options+shape) | [x] |
|---|----------------|-------------------------------|-----|
| 1 | *_keybytes/_noncebytes/_messagebytes_max, *_outputbytes/_inputbytes/_constbytes, crypto_core_keccak1600_statebytes, crypto_stream_primitive | all size accessors + primitive string agree | [x] |
| 2 | crypto_stream_salsa20 | keystream, key=32 nonce=8 | [x] |
| 3 | crypto_stream_salsa2012 | keystream, key=32 nonce=8 | [x] |
| 4 | crypto_stream_salsa208 | keystream, key=32 nonce=8 | [x] |
| 5 | crypto_stream_chacha20 | keystream, key=32 nonce=8 | [x] |
| 6 | crypto_stream_chacha20_ietf | keystream, key=32 nonce=12 | [x] |
| 7 | crypto_stream_xchacha20 | keystream, key=32 nonce=24 | [x] |
| 8 | crypto_stream_xsalsa20 | keystream, key=32 nonce=24 | [x] |
| 9 | crypto_stream (xsalsa20 generic) | keystream, key=32 nonce=24 | [x] |
| 10 | crypto_stream_salsa20_xor | xor + roundtrip | [x] |
| 11 | crypto_stream_salsa2012_xor | xor + roundtrip | [x] |
| 12 | crypto_stream_salsa208_xor | xor + roundtrip | [x] |
| 13 | crypto_stream_chacha20_xor | xor + roundtrip | [x] |
| 14 | crypto_stream_chacha20_ietf_xor | xor + roundtrip | [x] |
| 15 | crypto_stream_xchacha20_xor | xor + roundtrip | [x] |
| 16 | crypto_stream_xsalsa20_xor | xor + roundtrip | [x] |
| 17 | crypto_stream_xor (generic) | xor + roundtrip | [x] |
| 18 | crypto_stream_salsa20_xor_ic | xor with random u64 ic (small + full-range) + roundtrip | [x] |
| 19 | crypto_stream_chacha20_xor_ic | xor with random u64 ic + roundtrip | [x] |
| 20 | crypto_stream_xchacha20_xor_ic | xor with random u64 ic + roundtrip | [x] |
| 21 | crypto_stream_xsalsa20_xor_ic | xor with random u64 ic + roundtrip | [x] |
| 22 | crypto_stream_chacha20_ietf_xor_ic | xor with in-window random u32 ic + roundtrip | [x] |
| 23 | crypto_stream_salsa20 / _xor / _xor_ic | relations: xor==xor_ic(0), keystream==xor(zeros), both libs | [x] |
| 24 | crypto_core_salsa20 | out=64, in=16 k=32, const NULL and non-NULL | [x] |
| 25 | crypto_core_salsa2012 | out=64, const NULL and non-NULL | [x] |
| 26 | crypto_core_salsa208 | out=64, const NULL and non-NULL | [x] |
| 27 | crypto_core_hsalsa20 | out=32, const NULL and non-NULL | [x] |
| 28 | crypto_core_hchacha20 | out=32, const NULL and non-NULL | [x] |
| 29 | crypto_core_keccak1600_* | init -> xor_bytes(rand off/len) -> permute_24 -> extract_bytes, full 224B state compared | [x] |
| 30 | crypto_core_keccak1600_* | same pipeline with permute_12 | [x] |
| 31 | crypto_stream_chacha20_ietf_xor_ic | error: out-of-range ic aborts (SIGABRT) in both libs; boundary ic does not | [x] |
