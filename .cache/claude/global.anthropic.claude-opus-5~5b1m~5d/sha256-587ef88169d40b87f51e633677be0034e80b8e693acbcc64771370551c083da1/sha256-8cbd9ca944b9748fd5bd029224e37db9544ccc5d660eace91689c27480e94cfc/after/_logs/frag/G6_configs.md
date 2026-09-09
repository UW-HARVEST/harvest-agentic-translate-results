| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 0 -> output buffer untouched (ChaCha20-IETF with 0 length) | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 1 | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 63 (sub-block) | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 64 (exact block boundary) | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 65 (block boundary + 1, forces counter increment) | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0x00, size = 1000 | [ ] |
| G6 | randombytes_buf_deterministic | seed = 32x 0xff, sizes 0/1/63/64/65/1000 | [ ] |
| G6 | randombytes_buf_deterministic | seed = 0x00..0x1f (incrementing bytes), sizes 0/1/63/64/65/1000 | [ ] |
| G6 | randombytes_buf_deterministic | seed = SHA-256-like fixed vector (e.g. 32 bytes of 0x42), sizes 0/1/63/64/65/1000 | [ ] |
| G6 | randombytes_buf_deterministic | prefix property: output for size=N must be a byte-exact prefix of the output for size=M when N < M with the same seed (fixed nonce `"LibsodiumDRG"` + 0x00 padding, counter starts at 0) | [ ] |
| G6 | randombytes_buf_deterministic | two seeds differing in exactly one bit, size 64 — outputs must differ in both C and Rust the same way | [ ] |
| G6 | randombytes_seedbytes | no args -> must return 32 (== `randombytes_SEEDBYTES` == `crypto_stream_chacha20_ietf_KEYBYTES`) | [ ] |
| G6 | randombytes_uniform | upper_bound = 0 -> returns 0, consumes NO randomness (verifiable with a counting/deterministic implementation) | [ ] |
| G6 | randombytes_uniform | upper_bound = 1 -> returns 0, consumes NO randomness | [ ] |
| G6 | randombytes_uniform | upper_bound = 2 with a deterministic implementation whose `random()` yields a fixed sequence: min = (1U + ~2) % 2 = 0, so no rejection, result = r % 2 | [ ] |
| G6 | randombytes_uniform | upper_bound = 3: min = (1U + ~3) % 3 = 4294967293 % 3 = 1; feed r = 0 (rejected) then r = 7 -> result 1. Tests the rejection-sampling arithmetic exactly | [ ] |
| G6 | randombytes_uniform | upper_bound = 10: min = 4294967286 % 10 = 6; feed r = 3 (rejected), r = 25 -> 5 | [ ] |
| G6 | randombytes_uniform | upper_bound = 0x80000001 (2**31+1, documented worst case): min = 0x7FFFFFFE = 2147483646; feed r = 0 (rejected), r = 0x80000002 -> 1 | [ ] |
| G6 | randombytes_uniform | upper_bound = 0xFFFFFFFF: min = 1 % 0xFFFFFFFF = 1; feed r = 0 (rejected), r = 1 -> 1 | [ ] |
| G6 | randombytes_uniform | upper_bound = 0x80000000 (power of two): min = 0x80000000 % 0x80000000 = 0, never rejects; r = 0xdeadbeef -> 0x5eadbeef | [ ] |
| G6 | randombytes_uniform | upper_bound = 256 (power of two): min = 0, result = r & 0xff | [ ] |
| G6 | randombytes_uniform | implementation with a NON-NULL `.uniform` hook -> must delegate and bypass both the `< 2` guard and the rejection loop (e.g. hook returning `upper_bound` verbatim, then `randombytes_uniform(0)` must return 0 from the hook, not from the guard) | [ ] |
| G6 | randombytes_implementation_name | fresh state (nothing set) -> auto-init to the default and return `"sysrandom"` (non-Emscripten builds; `RANDOMBYTES_DEFAULT_IMPLEMENTATION = &randombytes_sysrandom_implementation`) | [ ] |
| G6 | randombytes_set_implementation + randombytes_implementation_name | `randombytes_set_implementation(&randombytes_internal_implementation)` then name -> `"internal"` | [ ] |
| G6 | randombytes_set_implementation + randombytes_implementation_name | `randombytes_set_implementation(&randombytes_sysrandom_implementation)` then name -> `"sysrandom"` | [ ] |
| G6 | randombytes_set_implementation | switch internal -> sysrandom -> internal in sequence; name after each switch; return value must be 0 every time | [ ] |
| G6 | randombytes_set_implementation | set a fully custom deterministic implementation (name `"det"`, `random` = counter, `stir`/`uniform`/`close` = NULL, `buf` = ChaCha20 keystream); then `randombytes_implementation_name()` == `"det"`, `randombytes_stir()` is a no-op, `randombytes_close()` returns 0 | [ ] |
| G6 | randombytes_set_implementation(NULL) + randombytes_implementation_name | returns 0, then the next call re-inits to the default and reports `"sysrandom"` | [ ] |
| G6 | randombytes_buf | size = 0 with a custom implementation whose `buf` hook records calls -> hook must NOT be invoked | [ ] |
| G6 | randombytes_buf | sizes 1, 31, 32, 33, 256, 257, 1000 with a deterministic custom implementation -> byte-for-byte identical output | [ ] |
| G6 | randombytes_random | with a deterministic custom implementation returning a fixed sequence -> straight pass-through of `implementation->random()` | [ ] |
| G6 | randombytes / randombytes_buf (NaCl compat alias) | `randombytes(buf, len)` for len = 0, 1, 64, 1000 must be identical to `randombytes_buf(buf, (size_t) len)` | [ ] |
| G6 | randombytes_stir | called twice in a row on `randombytes_sysrandom_implementation` -> idempotent, `void`, no observable difference (second call sees `stream.initialized != 0`) | [ ] |
| G6 | randombytes_stir | called on `randombytes_internal_implementation` -> re-seeds (`stream.nonce = sodium_hrtime()`, `rnd32` zeroed, `rnd32_outleft = 0`, key re-read from the entropy source); calling it twice must be safe | [ ] |
| G6 | randombytes_stir | implementation with `.stir == NULL` -> no-op, no crash | [ ] |
| G6 | randombytes_close | sysrandom on Linux with getrandom available -> returns 0; a second consecutive call also returns 0 (idempotent) | [ ] |
| G6 | randombytes_close | internal implementation on Linux with getrandom available -> returns 0 after a `randombytes_stir()`, and `stream` is zeroised; a second call still returns 0 | [ ] |
| G6 | randombytes_close | implementation with `.close == NULL` -> returns 0 | [ ] |
| G6 | randombytes_close then randombytes_buf | close, then a fresh `randombytes_buf` must transparently re-stir and still produce output (no error) | [ ] |
| G6 | randombytes_internal_implementation (struct fields) | verify `.implementation_name` -> `"internal"`, `.random` non-NULL, `.stir` non-NULL, `.uniform == NULL`, `.buf` non-NULL, `.close` non-NULL | [ ] |
| G6 | randombytes_sysrandom_implementation (struct fields) | verify `.implementation_name` -> `"sysrandom"`, `.random` non-NULL, `.stir` non-NULL, `.uniform == NULL`, `.buf` non-NULL, `.close` non-NULL | [ ] |
| G6 | randombytes_salsa20_implementation (compat macro) | must alias `randombytes_internal_implementation` (same address / same name string) | [ ] |
| G6 | randombytes_internal_implementation.buf via randombytes_buf | fork-safety: same process, repeated calls of sizes 1/32/64/1024 must never repeat output and never abort (exercises the `rnd32` refill + key ratchet path) | [ ] |
| G6 | randombytes_internal_implementation.random via randombytes_random | drain the 32-bit pool across a refill boundary: `sizeof rnd32` = 16*32 = 512, `rnd32_outleft` starts at 512 - 32 = 480, so 120 consecutive `randombytes_random()` calls force exactly one refill | [ ] |
| G6 | crypto_kem_mlkem768_seed_keypair | seed = 64x 0x00 (d = 0^32, z = 0^32) -> DETERMINISTIC pk (1184 B) and sk (2400 B); check the sk layout: sk[0..1152] = skpv, sk[1152..2336] = pk, sk[2336..2368] = SHA3-256(pk), sk[2368..2400] = z | [ ] |
| G6 | crypto_kem_mlkem768_seed_keypair | seed = 0x00..0x3f (incrementing) -> deterministic pk/sk | [ ] |
| G6 | crypto_kem_mlkem768_seed_keypair | seed = 64x 0xff -> deterministic pk/sk (stresses `rej_uniform` refill counts in `gen_matrix`) | [ ] |
| G6 | crypto_kem_mlkem768_seed_keypair | ACVP/FIPS-203 known-answer seeds (d,z) -> compare pk/sk against the published vectors AND against Rust | [ ] |
| G6 | crypto_kem_mlkem768_enc_deterministic | pk from seed=0^64, coins/seed = 32x 0x00 -> deterministic ct (1088 B) + ss (32 B) | [ ] |
| G6 | crypto_kem_mlkem768_enc_deterministic | pk from seed=0^64, seed = 32x 0xff -> deterministic ct + ss | [ ] |
| G6 | crypto_kem_mlkem768_enc_deterministic | pk from seed=0x00..0x3f, seed = 0x00..0x1f -> deterministic ct + ss | [ ] |
| G6 | crypto_kem_mlkem768_enc_deterministic + crypto_kem_mlkem768_dec | full derandomised round trip: ss from enc must equal ss from dec, for at least 3 distinct (keypair seed, enc seed) pairs | [ ] |
| G6 | crypto_kem_mlkem768_dec | valid ct from `_enc_deterministic` -> returns 0, ss = SHA3-512(m ‖ hpk)[0..32]; the "success" branch of `cmov` (fail_mask = 0) | [ ] |
| G6 | crypto_kem_mlkem768_dec | IMPLICIT REJECTION: flip bit 0 of ct[0] -> returns 0 with ss = SHAKE256(sk[2368..2400] ‖ ct)[0..32]; C and Rust must produce the SAME pseudorandom 32 bytes | [ ] |
| G6 | crypto_kem_mlkem768_dec | implicit rejection at other positions: flip ct[543] (mid `polyvec_compress` region) and ct[1087] (last byte of the dv-compressed poly) -> two more distinct pseudorandom shared secrets, must match | [ ] |
| G6 | crypto_kem_mlkem768_dec | ct = 1088x 0x00 (all-zero ciphertext, valid encoding but not a real ct) -> returns 0 with the implicit-rejection secret | [ ] |
| G6 | crypto_kem_mlkem768_dec | ct = 1088x 0xff -> returns 0 with the implicit-rejection secret (exercises `poly_decompress_du`/`_dv` on maximal input) | [ ] |
| G6 | crypto_kem_mlkem768_dec | sk with a deliberately wrong embedded `hpk` (sk[2336..2368] zeroed) but otherwise valid -> returns 0 with the implicit-rejection secret (no hash check exists) | [ ] |
| G6 | crypto_kem_mlkem768_keypair + _enc + _dec | fully random round trip (non-deterministic): enc/dec must agree on ss; only the agreement invariant is comparable, not the bytes | [ ] |
| G6 | crypto_kem_mlkem768_publickeybytes / _secretkeybytes / _ciphertextbytes / _sharedsecretbytes / _seedbytes | no args -> 1184 / 2400 / 1088 / 32 / 64 | [ ] |
| G6 | crypto_kem_xwing_seed_keypair | seed = 32x 0x00 -> DETERMINISTIC pk (1216 B = 1184 mlkem ‖ 32 x25519) and sk (32 B, == the seed verbatim) | [ ] |
| G6 | crypto_kem_xwing_seed_keypair | seed = 0x00..0x1f -> deterministic pk/sk; verify `sk == seed` and pk[1184..1216] == X25519 base * (SHAKE256(seed,96)[64..96]) | [ ] |
| G6 | crypto_kem_xwing_seed_keypair | seed = 32x 0xff -> deterministic pk/sk | [ ] |
| G6 | crypto_kem_xwing_seed_keypair | X-Wing draft/RFC KAT seed -> compare pk against the published vector AND against Rust; also verify the SHAKE256 96-byte expansion split (64 B mlkem seed ‖ 32 B x25519 scalar) | [ ] |
| G6 | crypto_kem_xwing_enc_deterministic | pk from seed=0^32, seed = 64x 0x00 (32 B mlkem coins ‖ 32 B ephemeral x25519 scalar) -> deterministic ct (1120 B) + ss (32 B). NOTE the enc seed is 64 bytes, not `crypto_kem_xwing_SEEDBYTES` (32) | [ ] |
| G6 | crypto_kem_xwing_enc_deterministic | pk from seed=0^32, seed = 0x00..0x3f -> deterministic ct + ss; verify ct = ct_mlkem(1088) ‖ ct_x25519(32) | [ ] |
| G6 | crypto_kem_xwing_enc_deterministic | pk from seed=0xff^32, seed = 64x 0xaa -> deterministic ct + ss | [ ] |
| G6 | crypto_kem_xwing_enc_deterministic + crypto_kem_xwing_dec | derandomised round trip for >= 3 (keypair seed, enc seed) pairs -> ss must match on both sides and equal the C reference bytes | [ ] |
| G6 | crypto_kem_xwing_dec | valid ct -> returns 0, ss = SHA3-256(ss_mlkem ‖ ss_x25519 ‖ ct_x25519 ‖ pk_x25519 ‖ 5c 2e 2f 2f 5e 5c); exercises the `combiner` and the on-the-fly re-expansion of the decaps key from the 32-byte sk | [ ] |
| G6 | crypto_kem_xwing_dec | IMPLICIT REJECTION: corrupt ct[0] (ML-KEM part only, leave ct[1088..1120] intact) -> returns 0 with a DIFFERENT but fully deterministic ss; C and Rust must agree byte-for-byte | [ ] |
| G6 | crypto_kem_xwing_dec | corrupt ct[1087] (last byte of the ML-KEM part) -> returns 0 with a second deterministic implicit-rejection ss | [ ] |
| G6 | crypto_kem_xwing_dec | corrupt ct[1088..1120] to a VALID non-low-order X25519 point (e.g. increment ct[1088]) -> returns 0 with a deterministic ss that differs from the honest one (both the mlkem re-encryption and the x25519 shared secret change) | [ ] |
| G6 | crypto_kem_xwing_keypair + _enc + _dec | fully random round trip; only the ss-agreement invariant is comparable | [ ] |
| G6 | crypto_kem_xwing_publickeybytes / _secretkeybytes / _ciphertextbytes / _sharedsecretbytes / _seedbytes | no args -> 1216 / 32 / 1120 / 32 / 32 | [ ] |
| G6 | crypto_kem_seed_keypair / crypto_kem_keypair / crypto_kem_enc / crypto_kem_dec (generic dispatch) | must be exact pass-throughs of the corresponding `crypto_kem_xwing_*`: same pk/sk for the same 32-byte seed, same ct/ss for the same inputs | [ ] |
| G6 | crypto_kem_publickeybytes / _secretkeybytes / _ciphertextbytes / _sharedsecretbytes / _seedbytes / _primitive | no args -> 1216 / 32 / 1120 / 32 / 32 / `"xwing"`; must equal the xwing getters exactly (macros are aliases) | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 16x 0x00, in = IPv4-mapped 0.0.0.0 = `00*10 ff ff 00 00 00 00` -> deterministic 16-byte out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = IPv4-mapped 127.0.0.1 = `00*10 ff ff 7f 00 00 01` -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = IPv4-mapped 192.168.1.1 = `00*10 ff ff c0 a8 01 01` -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = IPv4-mapped 255.255.255.255 = `00*10 ff ff ff ff ff ff` -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = native IPv6 `2001:0db8:0000:0000:0000:0000:0000:0001` (16 B, NOT ipv4-mapped) -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = native IPv6 `::1` = 15x 0x00 ‖ 0x01 -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 0x00..0x0f, in = 16x 0xff (all-ones IPv6) -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_encrypt | key = 16x 0xff, in = 16x 0x00 -> deterministic out (single-block AES-128 KAT) | [ ] |
| G6 | crypto_ipcrypt_decrypt | inverse of every `crypto_ipcrypt_encrypt` row above: `decrypt(encrypt(ip, k), k) == ip` (round trip) | [ ] |
| G6 | crypto_ipcrypt_decrypt | key = 16x 0x00, in = 16x 0x00 (decrypt an arbitrary 16-byte block) -> deterministic out, must match Rust | [ ] |
| G6 | crypto_ipcrypt_encrypt / _decrypt | bijectivity spot-check: 256 distinct inputs under one fixed key must map to 256 distinct outputs, identically in C and Rust | [ ] |
| G6 | crypto_ipcrypt_nd_encrypt | EXPLICIT tweak t = 8x 0x00, key = 16x 0x00, in = IPv4-mapped 127.0.0.1 -> deterministic 24-byte out; verify `out[0..8] == t` verbatim | [ ] |
| G6 | crypto_ipcrypt_nd_encrypt | EXPLICIT tweak t = 0x00..0x07, key = 0x00..0x0f, in = IPv4-mapped 192.168.1.1 -> deterministic 24-byte out | [ ] |
| G6 | crypto_ipcrypt_nd_encrypt | EXPLICIT tweak t = 8x 0xff, key = 0x00..0x0f, in = native IPv6 `2001:db8::1` -> deterministic 24-byte out | [ ] |
| G6 | crypto_ipcrypt_nd_encrypt | same key + same input, two DIFFERENT explicit tweaks -> `out[8..24]` must differ (tweak actually enters the AES tweak schedule) | [ ] |
| G6 | crypto_ipcrypt_nd_decrypt | feed the 24-byte output of every `_nd_encrypt` row back -> must recover the original 16-byte IP (tweak is read from `in[0..8]`) | [ ] |
| G6 | crypto_ipcrypt_nd_decrypt | hand-crafted 24-byte input (tweak 8x 0x00 ‖ ciphertext 16x 0x00), key = 16x 0x00 -> deterministic 16-byte out, must match Rust | [ ] |
| G6 | crypto_ipcrypt_ndx_encrypt | EXPLICIT tweak t = 16x 0x00, key = 32x 0x00 (degenerate: both halves equal -> triggers the `diff == 0` re-derivation `k[i] ^ 0x5a`), in = IPv4-mapped 127.0.0.1 -> deterministic 32-byte out; verify `out[0..16] == t` | [ ] |
| G6 | crypto_ipcrypt_ndx_encrypt | EXPLICIT tweak t = 0x00..0x0f, key = 0x00..0x1f (distinct halves, normal path), in = IPv4-mapped 192.168.1.1 -> deterministic 32-byte out | [ ] |
| G6 | crypto_ipcrypt_ndx_encrypt | EXPLICIT tweak t = 16x 0xff, key = 0x00..0x1f, in = native IPv6 `2001:db8::1` -> deterministic 32-byte out | [ ] |
| G6 | crypto_ipcrypt_ndx_encrypt | key whose two 16-byte halves are IDENTICAL but non-zero (e.g. 0x00..0x0f repeated twice) -> exercises the `d == 0` degenerate-key branch with a non-trivial key | [ ] |
| G6 | crypto_ipcrypt_ndx_decrypt | feed the 32-byte output of every `_ndx_encrypt` row back -> must recover the original 16-byte IP (tweak read from `in[0..16]`) | [ ] |
| G6 | crypto_ipcrypt_ndx_decrypt | hand-crafted 32-byte input (tweak 16x 0x00 ‖ ciphertext 16x 0x00), key = 0x00..0x1f -> deterministic 16-byte out | [ ] |
| G6 | crypto_ipcrypt_pfx_encrypt | key = 0x00..0x1f, in = IPv4-mapped 192.168.1.1 -> deterministic 16-byte out; `is_ipv4_mapped()` true so `prefix_start = 96`, out[0..10] = 0, out[10] = out[11] = 0xff (IPv4-mapped prefix preserved) | [ ] |
| G6 | crypto_ipcrypt_pfx_encrypt | key = 0x00..0x1f, in = native IPv6 `2001:db8::1` -> `prefix_start = 0`, all 128 bit positions processed -> deterministic 16-byte out | [ ] |
| G6 | crypto_ipcrypt_pfx_encrypt | key = 32x 0x00 (degenerate halves -> `k2keys` re-expanded from `k[i] ^ 0x5a`), in = IPv4-mapped 10.0.0.1 -> deterministic out | [ ] |
| G6 | crypto_ipcrypt_pfx_encrypt | prefix-preservation property: two IPv4-mapped addresses sharing an /24 prefix must have outputs sharing the corresponding encrypted prefix bits; identical behaviour required in Rust | [ ] |
| G6 | crypto_ipcrypt_pfx_decrypt | round trip `pfx_decrypt(pfx_encrypt(ip, k), k) == ip` for the IPv4-mapped and the native-IPv6 rows above | [ ] |
| G6 | crypto_ipcrypt_keygen / _nd_keygen / _ndx_keygen / _pfx_keygen | with a deterministic `randombytes` implementation installed -> keys of exactly 16 / 16 / 32 / 32 bytes, byte-for-byte reproducible | [ ] |
| G6 | crypto_ipcrypt_bytes / _keybytes / _nd_keybytes / _nd_tweakbytes / _nd_inputbytes / _nd_outputbytes / _ndx_keybytes / _ndx_tweakbytes / _ndx_inputbytes / _ndx_outputbytes / _pfx_keybytes / _pfx_bytes | no args -> 16 / 16 / 16 / 8 / 16 / 24 / 32 / 16 / 16 / 32 / 32 / 16 | [ ] |
| G6 | _crypto_ipcrypt_pick_best_implementation + crypto_ipcrypt_encrypt | force each backend in turn (soft / AES-NI / ARM crypto) and check that all three produce IDENTICAL output for a fixed key+input; the Rust port only needs to match the soft backend's values | [ ] |

<!--
NON-DETERMINISTIC ENTRY POINTS IN G6 AND HOW TO MAKE THEM COMPARABLE

Inherently non-deterministic (they draw from the OS entropy source and CANNOT be
compared byte-for-byte between C and Rust as-is):
  * randombytes_random, randombytes_buf, randombytes(), randombytes_uniform
  * randombytes_stir (reseeds from the OS; also reads a high-resolution clock into
    stream.nonce in the internal implementation)
  * randombytes_internal_random / randombytes_internal_random_buf /
    randombytes_internal_random_stir (via randombytes_internal_implementation)
  * randombytes_sysrandom / randombytes_sysrandom_buf / randombytes_sysrandom_stir
    (via randombytes_sysrandom_implementation)
  * crypto_kem_mlkem768_keypair, crypto_kem_mlkem768_enc
  * crypto_kem_xwing_keypair, crypto_kem_xwing_enc
  * crypto_kem_keypair, crypto_kem_enc  (dispatch to xwing)
  * crypto_ipcrypt_keygen, crypto_ipcrypt_nd_keygen, crypto_ipcrypt_ndx_keygen,
    crypto_ipcrypt_pfx_keygen (all just wrap randombytes_buf)

How to make each deterministic:

1. Install a custom deterministic randombytes implementation on BOTH sides before
   the test:
       static const randombytes_implementation det_impl = {
           det_name,        /* returns "det" */
           det_random,      /* 32-bit counter or a ChaCha20 keystream word */
           NULL,            /* stir: no-op */
           NULL,            /* uniform: NULL -> exercise the default rejection loop */
           det_buf,         /* fills buf with a fixed ChaCha20/counter keystream */
           NULL             /* close */
       };
       randombytes_set_implementation(&det_impl);
   randombytes_set_implementation() always returns 0 and performs NO validation, so
   this is the sanctioned hook. With it installed, every *_keypair / *_enc / *_keygen
   above becomes fully deterministic and directly byte-comparable, and
   randombytes_random()/randombytes_uniform() become comparable too (which is the only
   way to test randombytes_uniform's rejection-sampling arithmetic: choose det_random
   values that land below `min` to force at least one rejection).

2. Prefer the *_derand / *_deterministic / *_seed_keypair variants where they exist -
   these need no hook at all and are the highest-value differential targets:
       crypto_kem_mlkem768_seed_keypair(pk, sk, seed[64])
       crypto_kem_mlkem768_enc_deterministic(ct, ss, pk, seed[32])
       crypto_kem_xwing_seed_keypair(pk, sk, seed[32])
       crypto_kem_xwing_enc_deterministic(ct, ss, pk, seed[64])
       crypto_kem_seed_keypair(pk, sk, seed[32])        /* == xwing */
       randombytes_buf_deterministic(buf, size, seed[32])
   NOTE: libsodium spells these `_seed_keypair` and `_enc_deterministic`, NOT
   `_keypair_derand` / `_enc_derand`. Note also the asymmetric seed sizes:
   xwing seed_keypair takes 32 bytes but xwing enc_deterministic takes 64;
   mlkem768 seed_keypair takes 64 but mlkem768 enc_deterministic takes 32.

3. Decapsulation (crypto_kem_mlkem768_dec, crypto_kem_xwing_dec, crypto_kem_dec) and
   every crypto_ipcrypt_* encrypt/decrypt entry point are ALREADY fully deterministic -
   no randomness is consumed. In particular the implicit-rejection path of
   mlkem768_ref_dec is a pure function of (z = sk[2368..2400], ct) via
   SHAKE256, so a corrupted ciphertext MUST yield the same 32-byte pseudorandom
   shared secret in C and in Rust; this is a hard equality assertion, not a
   "both fail" assertion (dec returns 0, never -1).

4. The nd / ndx / pfx ipcrypt variants are "non-deterministic" only in the sense that
   callers normally pass a fresh random tweak. Pass an EXPLICIT tweak (8 bytes for nd,
   16 bytes for ndx) and they are fully deterministic. pfx takes no tweak at all.

5. randombytes_close() / randombytes_stir() return values depend on which entropy
   backend was compiled in and on whether the stream was ever stirred; pin the platform
   (Linux + getrandom, so close() -> 0) or compare only the C and Rust results built for
   the same platform, not against a hardcoded expectation.

6. NOT PRESENT in libsodium 1.0.23 (no differential-test surface):
   crypto_ipcrypt_str_to_ip16 / crypto_ipcrypt_ip16_to_str or any other IP-text
   parsing/formatting helper. The whole c_src/libsodium tree contains no
   inet_pton/inet_ntop/str_to_ip references and crypto_ipcrypt.h exposes only the
   16-byte binary API. Also absent: randombytes_random_stir / randombytes_random_close
   (the real names are randombytes_stir / randombytes_close), and
   crypto_kem_*_keypair_derand / _enc_derand (see item 2 for the real names).
   Conversely, the pfx variant (crypto_ipcrypt_pfx_keygen/_encrypt/_decrypt,
   PFX_KEYBYTES = 32, PFX_BYTES = 16) IS present and was not in the original G6 brief -
   it is included above.
-->
