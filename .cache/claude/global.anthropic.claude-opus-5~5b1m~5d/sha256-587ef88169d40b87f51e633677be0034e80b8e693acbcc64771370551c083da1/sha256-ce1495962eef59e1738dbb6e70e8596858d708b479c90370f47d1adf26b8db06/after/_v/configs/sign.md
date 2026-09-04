| sign-1 | crypto_scalarmult_bytes, crypto_scalarmult_scalarbytes, crypto_scalarmult_primitive | value + string content ("curve25519") vs C | [x] |
| sign-2 | crypto_scalarmult_curve25519_bytes, crypto_scalarmult_curve25519_scalarbytes | constant getters (32/32) | [x] |
| sign-3 | crypto_scalarmult_ed25519_bytes, crypto_scalarmult_ed25519_scalarbytes | constant getters (32/32) | [x] |
| sign-4 | crypto_scalarmult_ristretto255_bytes, crypto_scalarmult_ristretto255_scalarbytes | constant getters (32/32) | [x] |
| sign-5 | crypto_sign_bytes, _seedbytes, _publickeybytes, _secretkeybytes, _messagebytes_max, _statebytes, _primitive | generic dispatcher getters (64/32/32/64/SIZE_MAX-64/208/"ed25519") | [x] |
| sign-6 | crypto_sign_ed25519_bytes, _seedbytes, _publickeybytes, _secretkeybytes, _messagebytes_max, crypto_sign_ed25519ph_statebytes | ed25519 getters; statebytes == crypto_sign_statebytes | [x] |
| sign-7 | crypto_scalarmult_curve25519_ref10_implementation (exported data object) | `both_data!`, both function pointers (`mult`, `mult_base`) invoked directly: 24 random scalars, 24 random scalar/point pairs | [x] |
| sign-8 | crypto_scalarmult_curve25519_ref10_implementation.mult | all 7 blocklist entries + their bit-255-set variants through the raw function pointer | [x] |
| sign-9 | _crypto_scalarmult_curve25519_pick_best_implementation | called 3x on both libs (HAVE_AVX_ASM undefined ⇒ always ref10); curve25519 mult + blocklist rejection re-verified afterwards | [x] |
| sign-10 | crypto_scalarmult_curve25519_base, crypto_scalarmult_base | 40 scalars: all-zero, all-0xff, all-0x01, L, 2L, 7L, low-3-bits-only, bit-255-only, pre-clamped, 30 random; clamp-invariance (n vs clamp(n)) | [x] |
| sign-11 | crypto_scalarmult_curve25519_base | output aliases the scalar (`q == n`, the C uses `t = q` as scratch), 8 random scalars, full canary buffer compared | [x] |
| sign-12 | crypto_scalarmult_curve25519, crypto_scalarmult | 30 random scalar × random point pairs; dispatcher output compared against the curve25519 entry point | [x] |
| sign-13 | crypto_scalarmult_curve25519 | X25519 ECDH: 16 pairs, `a*base(b) == b*base(a)` | [x] |
| sign-14 | crypto_scalarmult_curve25519 | non-canonical point encodings p+2 … p+18 (byte 31 = 0x7f) accepted and equal to the canonical encoding of 2 … 18 | [x] |
| sign-15 | crypto_scalarmult_curve25519 | bit 255 of the point set vs cleared (masked away by fe25519_frombytes), 8 random points | [x] |
| sign-16 | crypto_scalarmult_curve25519 | scalar clamping: raw n vs clamp(n) = (n[0]&248, n[31]&127\|64), 12 random pairs | [x] |
| sign-17 | crypto_scalarmult_curve25519 | output aliases the scalar (`q == n`) and the point (`q == p`), 8 pairs each | [x] |
| sign-18 | crypto_scalarmult_curve25519 vs implementation->mult | 270 candidate points (blocklist ± deltas, p…p+20, 100 random) comparing raw `mult` result and wrapper return, searching for the all-zero-output rejection | [x] |
| sign-19 | crypto_scalarmult_ed25519_base | 38 scalars: zero, 0xff…, 0x01…, k·L (k=1..7), 1, bit-255-only, L\|2^255, 25 random | [x] |
| sign-20 | crypto_scalarmult_ed25519_base_noclamp | same 38 scalars; k·L and scalars that mask to 0 give the identity ⇒ -1 with identity bytes written | [x] |
| sign-21 | crypto_scalarmult_ed25519_base vs _base_noclamp | clamp equivalence: base(n) == base_noclamp(n[0]&248, n[31]\|64 then &127), 12 random | [x] |
| sign-22 | crypto_scalarmult_ed25519_base, _base_noclamp | output aliases the scalar (`q == n`), 6 random scalars × 2 variants | [x] |
| sign-23 | crypto_scalarmult_ed25519 | 71 points (12 valid main-subgroup, 12 small-order encodings, 7 non-canonical y≥p, 40 random) × 6 scalars (0, 0xff…, L, 3L, random, 1) | [x] |
| sign-24 | crypto_scalarmult_ed25519_noclamp | same 71×6 matrix; k·L on a main-subgroup point ⇒ identity ⇒ -1 | [x] |
| sign-25 | crypto_scalarmult_ed25519 vs _noclamp | clamp equivalence on 6 valid points; `q == n` and `q == p` aliasing | [x] |
| sign-26 | crypto_scalarmult_ristretto255_base | 36 scalars: zero, 0xff…, k·L (k=1..7), bit-255-only, 1, 25 random; identity result ⇒ -1 with all-zero output | [x] |
| sign-27 | crypto_scalarmult_ristretto255 | 66 points (12 valid, all-zero identity encoding, all-0xff, 12 ed25519 small-order encodings, 40 random) × 5 scalars (0, 0xff…, L, 2L, random) | [x] |
| sign-28 | crypto_scalarmult_ristretto255, _base | output aliases the scalar (`q == n`) and the point (`q == p`), 5 each | [x] |
| sign-29 | crypto_sign_ed25519_seed_keypair, crypto_sign_seed_keypair | 28 seeds (zero, 0xff…, L, 25 random); deterministic ⇒ pk and sk bytes compared; sk[0..32]==seed, sk[32..64]==pk | [x] |
| sign-30 | crypto_sign_ed25519_keypair, crypto_sign_keypair | RNG-driven ⇒ return code only; per-library self-consistency (sk[32..]==pk) | [x] |
| sign-31 | crypto_sign_ed25519_sk_to_pk, crypto_sign_ed25519_sk_to_seed | 28 secret keys, canary-padded output, result matched against the keypair | [x] |
| sign-32 | crypto_sign_ed25519, crypto_sign | mlen ∈ {0,1,31,32,33,64,127,128,1000} × 2 keys, smlen_p non-NULL | [x] |
| sign-33 | crypto_sign_ed25519 | same message set with `smlen_p == NULL` (output must be identical) | [x] |
| sign-34 | crypto_sign_ed25519 | in-place signing: `m == sm + crypto_sign_BYTES` (memmove overlap), all message lengths | [x] |
| sign-35 | crypto_sign_ed25519_open, crypto_sign_open | valid signed message, `m != NULL, mlen_p != NULL`, all message lengths | [x] |
| sign-36 | crypto_sign_ed25519_open | `m == NULL, mlen_p != NULL` | [x] |
| sign-37 | crypto_sign_ed25519_open | `m != NULL, mlen_p == NULL` | [x] |
| sign-38 | crypto_sign_ed25519_open | `m == NULL, mlen_p == NULL` | [x] |
| sign-39 | crypto_sign_ed25519_open | in-place open (`m == sm`, memmove overlap) | [x] |
| sign-40 | crypto_sign_ed25519_open | every one of the 64 signature bytes flipped ⇒ -1, `*mlen_p = 0`, `m` zeroed over exactly mlen bytes (canary intact) | [x] |
| sign-41 | crypto_sign_ed25519_open | message body tampered at offset 0, mlen/2, mlen-1 | [x] |
| sign-42 | crypto_sign_ed25519_open | signature verified against a different public key | [x] |
| sign-43 | crypto_sign_ed25519_detached, crypto_sign_detached | mlen ∈ {0,1,31,32,33,64,127,128,1000}, siglen_p non-NULL (⇒ 64) | [x] |
| sign-44 | crypto_sign_ed25519_detached | same set with `siglen_p == NULL` | [x] |
| sign-45 | _crypto_sign_ed25519_detached | prehashed ∈ {0, 1, 2, -1} (C int, any non-zero is "true") × all message lengths; ph=0 must equal the public wrapper | [x] |
| sign-46 | _crypto_sign_ed25519_verify_detached | prehashed ∈ {0, 1, 2, -1} round-trip against the matching signature | [x] |
| sign-47 | crypto_sign_ed25519_verify_detached, crypto_sign_verify_detached | valid signature, all message lengths | [x] |
| sign-48 | crypto_sign_ed25519_verify_detached | 64 signature bytes × 3 bit masks (0x01/0x40/0x80) flipped ⇒ -1 | [x] |
| sign-49 | crypto_sign_ed25519_verify_detached | message tampered at 3 offsets; message length truncated by 1 | [x] |
| sign-50 | crypto_sign_ed25519_verify_detached | mlen = 0 with `m == NULL` | [x] |
| sign-51 | crypto_sign_ed25519ph_init, _update, _final_create | mlen ∈ {0,1,31,32,33,64,127,128,1000} × 2 passes, random chunk splits; FULL 208-byte state buffer compared after init and after every update (crypto_sign_ed25519ph_state is padding-free: uint64[8] + uint64[2] + uint8[128]) | [x] |
| sign-52 | crypto_sign_ed25519ph_final_create | `siglen_p == NULL` vs non-NULL on identical states | [x] |
| sign-53 | crypto_sign_ed25519ph_update | zero-length updates prepended and appended to the chunk list (pass 1) | [x] |
| sign-54 | crypto_sign_ed25519ph_final_verify | matching signature ⇒ 0; each of the 64 signature bytes flipped ⇒ -1; wrong public key ⇒ -1; state buffer compared each time | [x] |
| sign-55 | crypto_sign_init, crypto_sign_update, crypto_sign_final_create, crypto_sign_final_verify | generic dispatcher, single one-shot update; signature must equal the chunked ed25519ph one | [x] |
| sign-56 | _crypto_sign_ed25519_ref10_hinit | prehashed ∈ {0, 1, 2, -1, INT_MIN, INT_MAX}: full sha512 state compared, then finalized (ph=0 ⇒ SHA-512(""), ph!=0 ⇒ DOM2PREFIX absorbed); all non-zero values equivalent | [x] |
| sign-57 | crypto_sign_ed25519_pk_to_curve25519 | 20 valid ed25519 public keys; result == crypto_scalarmult_curve25519_base(sk_to_curve25519(sk)) | [x] |
| sign-58 | crypto_sign_ed25519_pk_to_curve25519 | 79 invalid keys: 12 small-order encodings, 7 non-canonical y≥p, 60 random (off-curve / off-main-subgroup) | [x] |
| sign-59 | crypto_sign_ed25519_sk_to_curve25519 | 20 valid secret keys + 20 random + all-zero + all-0xff (never fails) | [x] |
| sign-60 | crypto_sign_ed25519_open, crypto_sign_open | smlen = 0 … 63 (short), with mlen_p non-NULL and NULL, m non-NULL and NULL | [x] |
| sign-61 | crypto_sign_ed25519_detached / _verify_detached (cross-library) | keys from each library's own RNG, signed by C and verified by Rust and vice versa, mlen ∈ {0,1,33,128} | [x] |
