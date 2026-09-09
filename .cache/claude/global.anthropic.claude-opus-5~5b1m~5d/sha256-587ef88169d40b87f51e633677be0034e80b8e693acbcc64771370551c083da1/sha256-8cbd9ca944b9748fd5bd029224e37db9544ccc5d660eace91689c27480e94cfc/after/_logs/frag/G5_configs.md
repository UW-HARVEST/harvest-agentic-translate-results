| G5 | crypto_box_keypair | fresh random keypair; check pk == crypto_scalarmult_base(sk) and pk/sk lengths 32/32 | [ ] |
| G5 | crypto_box_seed_keypair | seed = 32x`0x00` | [ ] |
| G5 | crypto_box_seed_keypair | seed = 32x`0xff` | [ ] |
| G5 | crypto_box_seed_keypair | seed = `00 01 02 .. 1f` (incrementing); verify sk = SHA512(seed)[0..31] (NOT clamped) and pk = X25519 base mult | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_seed_keypair | seed = `00..1f`; must produce byte-identical pk/sk to the xsalsa20 variant (same derivation) | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_keypair | fresh random keypair | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | fixed seed keypairs A/B, nonce = 24x`0x00`, mlen = 0 (clen = 16) | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 1 | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 31 | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 32 (exactly the salsa20 block0 payload boundary 64-ZEROBYTES) | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 33 (first byte spilling into the `xor_ic(..., ic=1)` path) | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 64 | [ ] |
| G5 | crypto_box_easy + crypto_box_open_easy | mlen = 1000 (multi-block) | [ ] |
| G5 | crypto_box_easy | nonce = `01 00 .. 00`, nonce = 24x`0xff`, nonce = random - same key/msg, distinct ciphertexts | [ ] |
| G5 | crypto_box_easy / crypto_box_open_easy | in-place: c == m buffer overlap (the memmove branch in crypto_secretbox_detached), mlen = 1000 | [ ] |
| G5 | crypto_box_open_easy | `m == NULL` (verify-only mode: crypto_secretbox_open_detached returns 0 early without decrypting) | [ ] |
| G5 | crypto_box_detached + crypto_box_open_detached | mac in a separate buffer; mlen in {0,1,31,32,33,64,1000}; mac must equal the first 16 bytes of the _easy ciphertext | [ ] |
| G5 | crypto_box_open_detached | `m == NULL`, valid mac (verify-only) | [ ] |
| G5 | crypto_box_beforenm | A.sk x B.pk and B.sk x A.pk must yield the identical 32-byte `k` (HSalsa20(0^16, X25519(sk,pk))) | [ ] |
| G5 | crypto_box_easy_afternm + crypto_box_open_easy_afternm | k from crypto_box_beforenm; mlen in {0,1,31,32,33,64,1000}; output must equal crypto_box_easy | [ ] |
| G5 | crypto_box_detached_afternm + crypto_box_open_detached_afternm | k from beforenm; mlen in {0,1,32,1000} | [ ] |
| G5 | crypto_box (NaCl zero-padded) + crypto_box_open | m has crypto_box_ZEROBYTES(32) leading zeros; total mlen in {32, 33, 64, 96, 1032}; c[0..15] must be zero (BOXZEROBYTES) | [ ] |
| G5 | crypto_box_afternm + crypto_box_open_afternm | k from beforenm; zero-padded m, mlen in {32, 64, 1032}; must match crypto_box() output | [ ] |
| G5 | crypto_box_zerobytes / _boxzerobytes / _macbytes / _noncebytes / _publickeybytes / _secretkeybytes / _seedbytes / _beforenmbytes / _sealbytes / _messagebytes_max / _primitive | constant getters: 32, 16, 16, 24, 32, 32, 32, 32, 48, SIZE_MAX-48, "curve25519xsalsa20poly1305" | [ ] |
| G5 | crypto_box_curve25519xsalsa20poly1305 low-level family (`_seed_keypair`, `_keypair`, `_beforenm`, `_afternm`, `_open_afternm`, `_`, `_open`) | same matrix as the default crypto_box_* aliases; verify byte-identical results (crypto_box.c is a pure forwarding layer) | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_easy + _open_easy | fixed seed keypairs, nonce = 24x`0x00`, mlen in {0,1,31,32,33,64,1000}; ciphertexts must DIFFER from the xsalsa20 variant | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_detached + _open_detached | separate mac buffer; mlen in {0,1,32,1000} | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_beforenm | HChacha20(0^16, X25519(sk,pk)); A/B symmetry | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_easy_afternm + _open_easy_afternm | k from _beforenm; mlen in {0,1,32,33,1000} | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_detached_afternm + _open_detached_afternm | k from _beforenm; mlen in {0,32,1000} | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305 constant getters | `_sealbytes`(48), `_macbytes`(16), `_noncebytes`(24), `_beforenmbytes`(32), `_publickeybytes`/`_secretkeybytes`/`_seedbytes`(32), `_messagebytes_max`; NOTE this primitive has NO ZEROBYTES/BOXZEROBYTES/NaCl-style API | [ ] |
| G5 | crypto_box_seal + crypto_box_seal_open | recipient = fixed seed keypair; mlen in {0,1,31,32,33,64,1000}; clen = mlen + 48 | [ ] |
| G5 | crypto_box_seal_open | c[0..31] is the ephemeral pk; nonce = BLAKE2b-24(epk || recipient_pk); verify structural round-trip only (seal is non-deterministic) | [ ] |
| G5 | crypto_box_curve25519xchacha20poly1305_seal + _seal_open | mlen in {0,1,32,1000} | [ ] |
| G5 | crypto_kx_keypair | fresh random keypair; pk == crypto_scalarmult_base(sk) | [ ] |
| G5 | crypto_kx_seed_keypair | seed = 32x`0x00`; sk = BLAKE2b-32(seed), pk = X25519 base | [ ] |
| G5 | crypto_kx_seed_keypair | seed = 32x`0xff` | [ ] |
| G5 | crypto_kx_seed_keypair | seed = `00 01 .. 1f` | [ ] |
| G5 | crypto_kx_client_session_keys + crypto_kx_server_session_keys | matching client/server seed keypairs; assert client.rx == server.tx and client.tx == server.rx (32 bytes each) | [ ] |
| G5 | crypto_kx_client_session_keys | `rx = buf, tx = NULL` (tx aliased to rx) | [ ] |
| G5 | crypto_kx_client_session_keys | `rx = NULL, tx = buf` (rx aliased to tx) | [ ] |
| G5 | crypto_kx_server_session_keys | `rx = buf, tx = NULL` and `rx = NULL, tx = buf` | [ ] |
| G5 | crypto_kx_client_session_keys | swapped roles: calling client_session_keys on both sides must NOT agree (rx/tx ordering differs) | [ ] |
| G5 | crypto_kx_publickeybytes / _secretkeybytes / _seedbytes / _sessionkeybytes / _primitive | getters: 32, 32, 32, 32, "x25519blake2b" | [ ] |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | n = `01 00..00` | [ ] |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | n = `08 00..00` | [ ] |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | n = 32x`0x00` (clamped to 0x08..0x40 -> still returns 0) | [ ] |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | n = 32x`0xff` (high bit set, exercises `n[31] &= 127; n[31] |= 64`) | [ ] |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | n = random 32 bytes; also n with `n[31] |= 0x80` must give the same q as `n[31] &= 0x7f` | [ ] |
| G5 | crypto_scalarmult_curve25519_base | RFC 7748 vector: n = `a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4` -> q = `8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a` | [ ] |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | RFC 7748 X25519 vector: n = `a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4`, p = `e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c` -> q = `c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552` | [ ] |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | n = `01 00..00`, p = a valid non-small-order public key (q == p) | [ ] |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | n = `08 00..00`, p = valid pk | [ ] |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | n = random, p = valid pk; ECDH symmetry: mult(a, base(b)) == mult(b, base(a)) | [ ] |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | n with high bit set (`n[31] |= 0x80`) and low bits set (`n[0] |= 0x07`) - clamping must make it equal to the clamped form | [ ] |
| G5 | crypto_scalarmult_bytes / _scalarbytes / _primitive / crypto_scalarmult_curve25519_bytes / _scalarbytes | getters: 32, 32, "curve25519" | [ ] |
| G5 | crypto_scalarmult_ed25519_base | n = `01 00..00` (clamped -> 8*B) | [ ] |
| G5 | crypto_scalarmult_ed25519_base | n = `08 00..00` | [ ] |
| G5 | crypto_scalarmult_ed25519_base | n = random 32 bytes | [ ] |
| G5 | crypto_scalarmult_ed25519_base | n = 32x`0xff` (high bit set; clamped `n[0] &= 248`, `n[31] |= 64`, then `n[31] &= 127`) | [ ] |
| G5 | crypto_scalarmult_ed25519_base_noclamp | n = `01 00..00` -> q must equal the ed25519 base point `5866666666666666666666666666666666666666666666666666666666666666` | [ ] |
| G5 | crypto_scalarmult_ed25519_base_noclamp | n = `08 00..00` -> must equal crypto_scalarmult_ed25519_base(`01 00..00`) | [ ] |
| G5 | crypto_scalarmult_ed25519_base_noclamp | n = random scalar reduced mod L; also n with `n[31] |= 0x80` (masked by `t[31] &= 127`, so must match) | [ ] |
| G5 | crypto_scalarmult_ed25519 | n = `01 00..00` (clamped -> 8), p = valid prime-order point (e.g. crypto_core_ed25519_random output or an ed25519 pk) | [ ] |
| G5 | crypto_scalarmult_ed25519 | n = `08 00..00`, p = valid prime-order point | [ ] |
| G5 | crypto_scalarmult_ed25519 | n = random, p = valid prime-order point; check mult(a, base(b)) == mult(b, base(a)) for clamped scalars | [ ] |
| G5 | crypto_scalarmult_ed25519_noclamp | n = `01 00..00`, p = valid point -> q == p | [ ] |
| G5 | crypto_scalarmult_ed25519_noclamp | n = `08 00..00`, p = valid point; and n = random reduced scalar | [ ] |
| G5 | crypto_scalarmult_ed25519_noclamp | n with high bit set (masked out by `t[31] &= 127`) - must equal the masked-scalar result | [ ] |
| G5 | crypto_scalarmult_ed25519_bytes / _scalarbytes | getters: 32, 32 | [ ] |
| G5 | crypto_scalarmult_ristretto255_base | n = `01 00..00` -> the ristretto255 basepoint encoding `e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76` | [ ] |
| G5 | crypto_scalarmult_ristretto255_base | n = `08 00..00`; also n = 2..15 (the draft's multiples-of-basepoint vectors) | [ ] |
| G5 | crypto_scalarmult_ristretto255_base | n = random scalar; n with `n[31] |= 0x80` must equal the masked form (`t[31] &= 127`) | [ ] |
| G5 | crypto_scalarmult_ristretto255 | n = `01 00..00`, p = valid ristretto encoding -> q == p | [ ] |
| G5 | crypto_scalarmult_ristretto255 | n = `08 00..00`, p = valid ristretto encoding | [ ] |
| G5 | crypto_scalarmult_ristretto255 | n = random, p = crypto_core_ristretto255_random(); commutativity mult(a, base(b)) == mult(b, base(a)) | [ ] |
| G5 | crypto_scalarmult_ristretto255 | n = L-1 (`ecd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010`), p = basepoint -> q == -basepoint | [ ] |
| G5 | crypto_scalarmult_ristretto255_bytes / _scalarbytes | getters: 32, 32 | [ ] |
| G5 | crypto_sign_ed25519_keypair | fresh random keypair; sk[0..31] == seed, sk[32..63] == pk | [ ] |
| G5 | crypto_sign_ed25519_seed_keypair | seed = 32x`0x00` (RFC 8032 TEST 1: sk seed `9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60` for the canonical vector - also run seed = that value -> pk = `d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`) | [ ] |
| G5 | crypto_sign_ed25519_seed_keypair | seed = 32x`0xff` | [ ] |
| G5 | crypto_sign_ed25519_seed_keypair | seed = `00 01 .. 1f` | [ ] |
| G5 | crypto_sign_ed25519_detached + crypto_sign_ed25519_verify_detached | fixed seed keypair; mlen = 0 (RFC 8032 TEST 1: sig = `e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b`) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 1 (RFC 8032 TEST 2: m = `72`) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 2 (RFC 8032 TEST 3: m = `af82`) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 32 | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 64 (exactly one SHA-512 block worth of message tail) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 127 (SHA-512 block-boundary minus 1) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 128 (SHA-512 block boundary) | [ ] |
| G5 | crypto_sign_ed25519_detached + _verify_detached | mlen = 1000 | [ ] |
| G5 | crypto_sign_ed25519_detached | `siglen_p == NULL` (skip the length write) | [ ] |
| G5 | crypto_sign_ed25519 + crypto_sign_ed25519_open (combined) | mlen in {0,1,32,64,127,128,1000}; smlen == mlen+64; sm[64..] == m | [ ] |
| G5 | crypto_sign_ed25519 + crypto_sign_ed25519_open | in-place / overlapping sm and m (`memmove(sm+64, m, mlen)` path), mlen = 1000 | [ ] |
| G5 | crypto_sign_ed25519_open | `m == NULL` and `mlen_p == NULL` (both optional-output branches), valid signature | [ ] |
| G5 | crypto_sign_ed25519_open | `smlen == 64` exactly (zero-length message) | [ ] |
| G5 | crypto_sign_ed25519ph_init/_update/_final_create + _final_verify | single update, mlen in {0,1,32,64,127,128,1000}; sig must equal detached-sign of SHA512(m) with the `SigEd25519 no Ed25519 collisions\x01\x00` DOM2 prefix | [ ] |
| G5 | crypto_sign_ed25519ph_* | multi-chunk: update(m[0..0]) then update(m[1..]) - must equal the single-update signature | [ ] |
| G5 | crypto_sign_ed25519ph_* | multi-chunk: 3 updates of 127/1/128 bytes crossing the SHA-512 block boundary | [ ] |
| G5 | crypto_sign_ed25519ph_* | zero updates at all (final_create straight after init, i.e. sign of SHA512("")) | [ ] |
| G5 | crypto_sign_ed25519ph_final_create | `siglen_p == NULL` | [ ] |
| G5 | crypto_sign_init / crypto_sign_update / crypto_sign_final_create / crypto_sign_final_verify | the generic aliases; must be byte-identical to the ed25519ph forms | [ ] |
| G5 | crypto_sign_ed25519ph_statebytes / crypto_sign_statebytes | getter == sizeof(crypto_sign_ed25519ph_state) (SHA-512 state) | [ ] |
| G5 | crypto_sign_ed25519_sk_to_seed | sk from seed_keypair(seed = `00..1f`) -> must return exactly that seed | [ ] |
| G5 | crypto_sign_ed25519_sk_to_pk | sk from seed_keypair -> must return sk[32..63] == pk | [ ] |
| G5 | crypto_sign_ed25519_pk_to_curve25519 | pk from a valid ed25519 keypair (prime-order, non-small-order); verify against x25519 pk derived from sk_to_curve25519 | [ ] |
| G5 | crypto_sign_ed25519_sk_to_curve25519 | sk from a valid ed25519 keypair -> SHA512(seed)[0..31] clamped (`&248`, `&127`, `|64`) | [ ] |
| G5 | crypto_sign_ed25519_pk_to_curve25519 + _sk_to_curve25519 | cross-check: crypto_scalarmult_base(sk_to_curve25519(sk)) == pk_to_curve25519(pk) | [ ] |
| G5 | crypto_sign_ed25519_bytes / _seedbytes / _publickeybytes / _secretkeybytes / _messagebytes_max / crypto_sign_primitive | getters: 64, 32, 32, 64, SIZE_MAX-64, "ed25519" | [ ] |
| G5 | crypto_core_ed25519_is_valid_point | p = crypto_core_ed25519_random() output (always a prime-order point after cofactor clearing) -> 1 | [ ] |
| G5 | crypto_core_ed25519_is_valid_point | p = a crypto_sign_ed25519 public key -> 1 | [ ] |
| G5 | crypto_core_ed25519_is_valid_point | p = crypto_scalarmult_ed25519_base(random scalar) -> 1 | [ ] |
| G5 | crypto_core_ed25519_add | p, q both from crypto_core_ed25519_random(); check add(p,q) == add(q,p) (commutativity) | [ ] |
| G5 | crypto_core_ed25519_add | p = valid point, q = identity encoding `0100000000000000000000000000000000000000000000000000000000000000` -> r == p (identity is accepted: add() has no small-order check) | [ ] |
| G5 | crypto_core_ed25519_sub | p, q from random(); check sub(add(p,q), q) == p | [ ] |
| G5 | crypto_core_ed25519_sub | p == q -> r == identity `0100..00` | [ ] |
| G5 | crypto_core_ed25519_random | 32-byte output; structural check only (non-deterministic): is_valid_point(out) == 1 | [ ] |
| G5 | crypto_core_ed25519_scalar_random | structural (non-deterministic): out[31] <= 0x1f, scalar_is_canonical(out) == 1, out != 0 | [ ] |
| G5 | crypto_core_ed25519_scalar_invert | s = `01 00..00` -> recip == `01 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_invert | s = `02 00..00`; verify scalar_mul(s, recip) == `01 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_invert | s = random canonical scalar; verify scalar_mul(s, invert(s)) == 1 | [ ] |
| G5 | crypto_core_ed25519_scalar_negate | s = `01 00..00` -> L-1 = `ecd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010` | [ ] |
| G5 | crypto_core_ed25519_scalar_negate | s = 32x`0x00` -> 32x`0x00`; s = random; verify scalar_add(s, negate(s)) == 0 | [ ] |
| G5 | crypto_core_ed25519_scalar_complement | s = `01 00..00`, s = 32x`0x00`, s = random; verify scalar_add(s, complement(s)) == `01 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_add | x = `01 00..00`, y = `01 00..00` -> `02 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_add | x = L-1, y = `01 00..00` -> 32x`0x00` (wrap through the 64-byte `sodium_add` + `sc25519_reduce` path) | [ ] |
| G5 | crypto_core_ed25519_scalar_add | x = y = 32x`0xff` (non-canonical inputs are accepted and reduced) | [ ] |
| G5 | crypto_core_ed25519_scalar_sub | x = `02 00..00`, y = `01 00..00` -> `01 00..00`; x = 0, y = `01 00..00` -> L-1 | [ ] |
| G5 | crypto_core_ed25519_scalar_mul | x = `02 00..00`, y = `03 00..00` -> `06 00..00`; x = L-1, y = L-1 -> `01 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_mul | x = random, y = `01 00..00` -> x (identity element) | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce | 64-byte s = 64x`0x00` -> 32x`0x00` | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce | 64-byte s = 64x`0xff` (maximum non-reduced input) | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce | 64-byte s = L padded with 32 zeros -> 32x`0x00` | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce | 64-byte s = (L-1) padded with 32 zeros -> L-1 (already reduced, unchanged) | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce | 64-byte s = `01` followed by 63x`0x00` -> `01 00..00` | [ ] |
| G5 | crypto_core_ed25519_scalar_reduce (NONREDUCEDSCALARBYTES path) | random 64 bytes; and 2^317-ish inputs (only bytes 0..39 nonzero) per the header's uniformity note | [ ] |
| G5 | crypto_core_ed25519_scalar_is_canonical | s = 32x`0x00` -> 1; s = `01 00..00` -> 1; s = L-1 -> 1 | [ ] |
| G5 | crypto_core_ed25519_from_string_nu | ctx = "QUUX-V01-CS02-with-edwards25519_XMD:SHA-512_ELL2_NU_", msg = "" / "abc", hash_alg = crypto_core_ed25519_H2CSHA512 (2) | [ ] |
| G5 | crypto_core_ed25519_from_string | ctx = "...RO_", msg = "" / "abc" / 1000 bytes, hash_alg = 2 (two-point + add path) | [ ] |
| G5 | crypto_core_ed25519_from_string / _from_string_nu | hash_alg = crypto_core_ed25519_H2CSHA256 (1) | [ ] |
| G5 | crypto_core_ed25519_from_string / _from_string_nu | ctx_len > 255 (triggers the "H2C-OVERSIZE-DST-" pre-hash branch); ctx = 300 bytes | [ ] |
| G5 | crypto_core_ed25519_from_string / _from_string_nu | ctx = NULL, ctx_len = 0 | [ ] |
| G5 | crypto_core_ed25519_scalar_from_string | ctx/msg as above, hash_alg 1 and 2; ctx_len > 255 | [ ] |
| G5 | crypto_core_ed25519_bytes / _uniformbytes / _hashbytes / _scalarbytes / _nonreducedscalarbytes | getters: 32, 32, 64, 32, 64 | [ ] |
| G5 | crypto_core_ristretto255_is_valid_point | p = crypto_core_ristretto255_random() output -> 1 | [ ] |
| G5 | crypto_core_ristretto255_is_valid_point | p = the basepoint `e2f2ae0a6abc4e71a884a961c500515f58e30b6aa582dd8db6a65945e08d2d76` -> 1 | [ ] |
| G5 | crypto_core_ristretto255_is_valid_point | p = identity 32x`0x00` -> 1 (the ristretto identity IS a valid encoding) | [ ] |
| G5 | crypto_core_ristretto255_add | p, q from random(); commutativity add(p,q) == add(q,p) | [ ] |
| G5 | crypto_core_ristretto255_add | p = basepoint, q = 32x`0x00` (identity) -> r == p | [ ] |
| G5 | crypto_core_ristretto255_add | p = basepoint, q = basepoint -> the 2*B vector `6a493210f7499cd17fecb510ae0cea23a110e8d5b901f8acadd3095c73a3b919` | [ ] |
| G5 | crypto_core_ristretto255_sub | p, q from random(); sub(add(p,q), q) == p | [ ] |
| G5 | crypto_core_ristretto255_sub | p == q -> 32x`0x00` (identity) | [ ] |
| G5 | crypto_core_ristretto255_from_hash | r = 64x`0x00` | [ ] |
| G5 | crypto_core_ristretto255_from_hash | r = 64x`0xff` | [ ] |
| G5 | crypto_core_ristretto255_from_hash | r = SHA-512("Ristretto is traditionally a small shot of espresso coffee") (the draft's one-way-map vector) -> `3066f82a1a747d45120d1740f14358531a8f04bbffe6a819f86dfe50f44a0a46` | [ ] |
| G5 | crypto_core_ristretto255_from_hash | r = 64 incrementing bytes `00 01 .. 3f`; result must satisfy is_valid_point == 1 | [ ] |
| G5 | crypto_core_ristretto255_random | structural (non-deterministic): is_valid_point(out) == 1, out[31] & 0x80 == 0, out[0] even | [ ] |
| G5 | crypto_core_ristretto255_scalar_random | structural (non-deterministic): canonical, nonzero, out[31] <= 0x1f | [ ] |
| G5 | crypto_core_ristretto255_scalar_invert | s = `01 00..00`, s = `02 00..00`, s = random canonical; verify scalar_mul(s, invert(s)) == `01 00..00` | [ ] |
| G5 | crypto_core_ristretto255_scalar_negate | s = `01 00..00` -> L-1; s = 0 -> 0; s = random | [ ] |
| G5 | crypto_core_ristretto255_scalar_complement | s = `01 00..00`, s = 0, s = random | [ ] |
| G5 | crypto_core_ristretto255_scalar_add | x = y = `01 00..00`; x = L-1, y = `01 00..00` -> 0 | [ ] |
| G5 | crypto_core_ristretto255_scalar_sub | x = `02 00..00`, y = `01 00..00`; x = 0, y = `01 00..00` -> L-1 | [ ] |
| G5 | crypto_core_ristretto255_scalar_mul | x = `02 00..00`, y = `03 00..00` -> `06 00..00`; x = y = L-1 -> `01 00..00` | [ ] |
| G5 | crypto_core_ristretto255_scalar_reduce | 64-byte input: 64x`0x00`, 64x`0xff`, L||0^32, (L-1)||0^32, `01`||0^63 | [ ] |
| G5 | crypto_core_ristretto255_scalar_reduce (NONREDUCEDSCALARBYTES path) | random 64 bytes; must be byte-identical to crypto_core_ed25519_scalar_reduce | [ ] |
| G5 | crypto_core_ristretto255_scalar_is_canonical | s = 0 -> 1; s = `01 00..00` -> 1; s = L-1 -> 1 | [ ] |
| G5 | crypto_core_ristretto255_from_string | ctx/msg pairs, hash_alg = 1 and 2; ctx_len > 255; ctx = NULL/0 | [ ] |
| G5 | crypto_core_ristretto255_scalar_from_string | ctx/msg pairs, hash_alg = 1 and 2 | [ ] |
| G5 | crypto_core_ristretto255_bytes / _hashbytes / _scalarbytes / _nonreducedscalarbytes | getters: 32, 64, 32, 64 | [ ] |
| G5 | cross-module consistency | crypto_scalarmult_ed25519_base_noclamp(scalar) vs ge25519 point built by crypto_core_ed25519_add repeated - same result for small scalars | [ ] |
| G5 | cross-module consistency | crypto_box_beforenm(pk_B, sk_A) == HSalsa20(0^16, crypto_scalarmult(sk_A, pk_B)) computed via crypto_core_hsalsa20 | [ ] |
| G5 | cross-module consistency | crypto_kx session keys == BLAKE2b-64(X25519(sk,pk) || client_pk || server_pk) split into rx/tx | [ ] |

<!--
NON-DETERMINISTIC ENTRY POINTS (call randombytes internally -> need a seeded/deterministic
randombytes implementation, or structural comparison instead of byte equality):

  * crypto_box_keypair
  * crypto_box_curve25519xsalsa20poly1305_keypair
  * crypto_box_curve25519xchacha20poly1305_keypair
  * crypto_box_seal                                  (generates an ephemeral keypair; c[0..31] and the
                                                      derived nonce differ every call -> compare only via
                                                      crypto_box_seal_open round-trip, or fix randombytes)
  * crypto_box_curve25519xchacha20poly1305_seal      (same)
  * crypto_kx_keypair
  * crypto_sign_ed25519_keypair                      (crypto_sign_keypair)
  * crypto_core_ed25519_random
  * crypto_core_ed25519_scalar_random
  * crypto_core_ristretto255_random
  * crypto_core_ristretto255_scalar_random           (delegates to crypto_core_ed25519_scalar_random)

DETERMINISTIC (safe for byte-for-byte C-vs-Rust comparison):
  * all *_seed_keypair variants (crypto_box, crypto_kx, crypto_sign) - pure hash + scalarmult
  * crypto_sign_ed25519_detached / crypto_sign_ed25519 / crypto_sign_ed25519ph_final_create -
    deterministic in this tree: ED25519_NONDETERMINISTIC is NOT defined anywhere in the build
    (only referenced inside `#ifdef` in crypto_sign/ed25519/ref10/sign.c:35,63). If the Rust port
    enables synthetic nonces, signatures will diverge.
  * ED25519_COMPAT is likewise NOT defined, so the S-canonicality branch in open.c:35-41 is the
    active one (the `sig[63] & 224` branch at open.c:31 is dead code here).
  * all crypto_scalarmult_*, crypto_core_*_add/sub/scalar_*/from_hash/from_string, all *bytes()
    getters, sk_to_seed/sk_to_pk/pk_to_curve25519/sk_to_curve25519, all _easy/_detached/_afternm
    encrypt+open paths.

API-SURFACE NOTES FOR 1.0.23:
  * crypto_core_ed25519_from_uniform() and crypto_core_ed25519_from_hash() are NOT in the public
    header any more; the map-to-curve surface is crypto_core_ed25519_from_string{,_nu}() and
    crypto_core_ristretto255_from_hash()/_from_string(). ge25519_from_uniform/ge25519_from_hash
    survive only as internal helpers (include/sodium/private/ed25519_ref10.h).
  * crypto_box_curve25519xchacha20poly1305 has NO NaCl-style zero-padded API (no ZEROBYTES /
    BOXZEROBYTES / bare `crypto_box_..._()` / `_open()` / `_afternm()` / `_open_afternm()`);
    those exist only for curve25519xsalsa20poly1305 and the default crypto_box_* aliases.
  * crypto_box_curve25519xsalsa20poly1305 has no _easy/_detached/_seal of its own - the default
    crypto_box_easy/_detached/_seal in crypto_box_easy.c / crypto_box_seal.c ARE that primitive.
  * hash_alg for every *_from_string* entry point is only valid as 1 (H2CSHA256) or 2 (H2CSHA512).
-->
