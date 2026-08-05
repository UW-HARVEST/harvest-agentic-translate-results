# pubkey family — configuration coverage

Each meaningful entry-point / input-shape combination exercised by
`tests/pubkey.rs`, checked off once its test passes. All valid-path configs run
many randomized inputs from fixed seeds and compare return code + output buffers
byte-for-byte between the C and Rust `.so`.

| # | entry point(s) | configuration (options + shape) | [x] |
|---|----------------|----------------------------------|-----|
| 1 | `crypto_scalarmult_curve25519` (+ `_base`) | base(random scalar) then mult(scalar, derived pubkey); 400 iters | [x] |
| 2 | `crypto_scalarmult` / `crypto_scalarmult_base` (frontend) | delegates to curve25519; 300 iters | [x] |
| 3 | `crypto_scalarmult_curve25519` | all-zero small-order point → zero output → `-1` | [x] |
| 4 | `crypto_scalarmult_ed25519` + `_noclamp` | valid main-subgroup point, random scalar (clamp vs noclamp) | [x] |
| 5 | `crypto_scalarmult_ed25519_base` + `_base_noclamp` | random scalar, base point | [x] |
| 6 | `crypto_scalarmult_ed25519` | random (invalid) point → `-1` parity | [x] |
| 7 | `crypto_scalarmult_ed25519` / `_base_noclamp` | zero scalar → infinity → `-1` | [x] |
| 8 | `crypto_scalarmult_ristretto255` (+ `_base`) | valid ristretto point, random scalar | [x] |
| 9 | `crypto_scalarmult_ristretto255` | random (invalid) encoding → `-1` parity | [x] |
| 10 | `crypto_sign_ed25519_seed_keypair` | fixed seed → deterministic (pk, sk) identical across libs | [x] |
| 11 | `crypto_sign_ed25519` + `_open` | full sign/open roundtrip, message len 0..300 | [x] |
| 12 | `crypto_sign_ed25519_detached` + `_verify_detached` | detached sign then verify (good) | [x] |
| 13 | `crypto_sign_ed25519_verify_detached` | tampered signature byte / tampered message → `-1` | [x] |
| 14 | `crypto_sign_ed25519_open` | tampered signed msg → `-1`+zeroed; `smlen < 64` → `-1` | [x] |
| 15 | `crypto_sign_ed25519_sk_to_seed` + `_sk_to_pk` | recover seed & pk from sk | [x] |
| 16 | `crypto_sign_ed25519_sk_to_curve25519` + `_pk_to_curve25519` | ed25519 → curve25519 key conversion | [x] |
| 17 | `crypto_box` xsalsa20poly1305: `seed_keypair`, `beforenm`, `easy`/`open_easy` | seeded keypairs; shared key; easy roundtrip; tampered ct → `-1` | [x] |
| 18 | `crypto_box` xsalsa20poly1305: `easy_afternm`/`open_easy_afternm` | precomputed shared key path | [x] |
| 19 | `crypto_box` xsalsa20poly1305: `detached`/`open_detached` | separate MAC output; roundtrip | [x] |
| 20 | `crypto_box` xsalsa20poly1305: `seal`/`seal_open` | anonymous seal; roundtrip + C-opens-Rust interop | [x] |
| 21 | `crypto_box` xchacha20poly1305: full family (same 4 shapes as #17-20) | seed_keypair/beforenm/easy/afternm/detached/seal | [x] |
| 22 | `crypto_box_open_easy` | `clen` in 0..MACBYTES → `-1` parity | [x] |
| 23 | `crypto_kx_seed_keypair` | deterministic (pk, sk) from seed | [x] |
| 24 | `crypto_kx_client_session_keys` + `crypto_kx_server_session_keys` | full handshake; client.rx==server.tx and client.tx==server.rx | [x] |
| 25 | `crypto_kx_client_session_keys` | zero/bad server pubkey → `-1` parity | [x] |
| 26 | `crypto_core_ed25519_is_valid_point` | valid points → 1; random → 0 parity | [x] |
| 27 | `crypto_core_ed25519_add` / `_sub` | valid point pairs; random pairs → `-1` parity | [x] |
| 28 | `crypto_core_ed25519_scalar_{add,sub,mul,negate,complement,reduce,invert,is_canonical}` | random scalars; 64-byte reduce; invert(0) → `-1` | [x] |
| 29 | `crypto_core_ed25519_from_string` + `_scalar_from_string` | hash-to-curve, ctx/msg random len, hash_alg = SHA256 & SHA512 | [x] |
| 30 | `crypto_core_ristretto255_is_valid_point` | valid → 1; random → 0 parity | [x] |
| 31 | `crypto_core_ristretto255_add` / `_sub` | valid pairs; random → `-1` parity | [x] |
| 32 | `crypto_core_ristretto255_from_hash` | 64-byte input → valid point; parity | [x] |
| 33 | `crypto_core_ristretto255_scalar_{add,sub,mul,negate,complement,reduce,invert,is_canonical}` | random scalars; 64-byte reduce | [x] |
