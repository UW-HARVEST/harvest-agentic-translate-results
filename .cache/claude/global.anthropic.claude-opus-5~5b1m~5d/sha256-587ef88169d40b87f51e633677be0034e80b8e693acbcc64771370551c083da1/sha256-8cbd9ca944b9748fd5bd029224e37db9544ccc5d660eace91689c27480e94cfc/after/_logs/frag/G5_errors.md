| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = 32x`0x00` (blocklist[0], order 4) -> `has_small_order(p)` in x25519_ref10.c:106 | returns -1, `q` untouched |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `0100000000000000000000000000000000000000000000000000000000000000` (blocklist[1], order 1) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800` (blocklist[2], order 8) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157` (blocklist[3], order 8) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f` (p-1, order 2) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f` (= 0 mod p, order 4) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `p` = `eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f` (= 1 mod p, order 1) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | any of the 7 blocklist entries with bit 255 set (`p[31] |= 0x80`); `has_small_order` masks byte 31 with 0x7f so all 14 variants are blocked (e.g. `ecff..ffff`, `0000..0080`) | returns -1 |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | ladder output all-zero: `d |= q[i]` accumulator stays 0, `return -(1 & ((d - 1) >> 8))` (scalarmult_curve25519.c:24-27) | returns -1 (unreachable given the small-order pre-filter, but is a distinct branch) |
| G5 | crypto_scalarmult_curve25519 / crypto_scalarmult | `implementation->mult()` returns nonzero (scalarmult_curve25519.c:21, LCOV_EXCL) | returns -1 |
| G5 | crypto_scalarmult_curve25519_base / crypto_scalarmult_base | no rejection path exists; `n` = 32x`0x00` is clamped to `0x08..0x40` | always returns 0 (any scalar, incl. all-zero, all-0xff) |
| G5 | crypto_scalarmult_ed25519 / _noclamp | `p` non-canonical y >= p: `ge25519_is_canonical(p) == 0`, i.e. `p[1..30] == 0xff && (p[31]&0x7f) == 0x7f && p[0] >= 0xed`. Concrete: `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `eeff..ff7f`, `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` | returns -1 |
| G5 | crypto_scalarmult_ed25519 / _noclamp | `p` decodes to no point: `ge25519_frombytes(&P, p) != 0` (neither `vx^2-u` nor `vx^2+u` is zero); ~50% of random 32-byte strings, e.g. `0202...02` | returns -1 |
| G5 | crypto_scalarmult_ed25519 / _noclamp | `p` is one of the 8 small-order (torsion) ed25519 encodings -> `ge25519_has_small_order(&P) != 0`: `0100..00` (identity), `0000..00`, `0000..0080`, `ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, `26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05`, `26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85`, `c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a` | returns -1 |
| G5 | crypto_scalarmult_ed25519 / _noclamp | `p` is on the curve but of composite order (`ge25519_is_on_main_subgroup(&P) == 0`), e.g. a valid pk added to an order-8 point via crypto_core_ed25519_add | returns -1 |
| G5 | crypto_scalarmult_ed25519 | `n` = 32x`0x00` -> `sodium_is_zero(n, 32)` (scalarmult_ed25519_ref10.c:53). NOTE: clamping makes the internal scalar nonzero so `q` IS overwritten with a valid point before the -1 | returns -1, `q` clobbered with 8*P |
| G5 | crypto_scalarmult_ed25519_noclamp | `n` = 32x`0x00` -> `_crypto_scalarmult_ed25519_is_inf(q)` true (q == `0100..00`) AND `sodium_is_zero(n,32)` | returns -1 |
| G5 | crypto_scalarmult_ed25519_noclamp | `n` = L = `edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010` or any nonzero n ≡ 0 mod L (after `n[31] &= 127`) -> Q = identity -> `_is_inf(q)` | returns -1 |
| G5 | crypto_scalarmult_ed25519_base | `n` = 32x`0x00` -> `sodium_is_zero(n, 32)` | returns -1 (q clobbered) |
| G5 | crypto_scalarmult_ed25519_base_noclamp | `n` = 32x`0x00` -> `_is_inf(q)` and `sodium_is_zero(n,32)` | returns -1 |
| G5 | crypto_scalarmult_ed25519_base_noclamp | `n` = L (`edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010`) -> Q = identity | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` has bit 255 set -> `ristretto255_is_canonical` term `e = p[31]>>7` -> `ristretto255_frombytes() != 0`. Concrete: `0000000000000000000000000000000000000000000000000000000000000080`, `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` is a "negative" field element, i.e. `p[0]` odd -> `ristretto255_is_canonical` term `s[0] & 1`. Concrete: `0100000000000000000000000000000000000000000000000000000000000000`, `01ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, `ed57ffd8c914fb201471d1c3d245ce3c746fcbe63a3679d51b6a516ebebe0e20` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` is a non-canonical field encoding (>= p): `p[1..30]==0xff && (p[31]&0x7f)==0x7f && p[0]>=0xed`. Concrete: `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `f3ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, `00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` canonical but `v*u2^2` is a non-square -> `ristretto255_sqrt_ratio_m1` notsquare -> `-((1-notsquare)|...)`. Concrete: `26948d35ca62e643e26a83177332e6b6afeb9d08e4268b650f1f5bbd8d81d371`, `4eac077a713c57b4f4397629a4145982c661f48044dd3f96427d40b147d9742f` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` decodes but T = X*Y is negative -> `fe25519_isnegative(h->T)`. Concrete: `3eb858e78f5a7254d8c9731174a94f76755fd3941c0ac93735c07ba14579630e`, `a45fdc55c76448c049a1ab33f17023edfb2be3581e9c7aade8a6125215e04220` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | `p` decodes but Y == 0 -> `fe25519_iszero(h->Y)` in `ristretto255_frombytes` | returns -1 |
| G5 | crypto_scalarmult_ristretto255 | valid `p`, but `n` = 32x`0x00` (or n ≡ 0 mod L after `n[31] &= 127`, e.g. n = L) -> identity -> `sodium_is_zero(q, 32)` | returns -1, `q` = 32x`0x00` |
| G5 | crypto_scalarmult_ristretto255_base | `n` = 32x`0x00` -> `sodium_is_zero(q, 32)` | returns -1 |
| G5 | crypto_scalarmult_ristretto255_base | `n` = L = `edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010` (n ≡ 0 mod L) -> identity | returns -1 |
| G5 | crypto_box_beforenm / crypto_box_curve25519xsalsa20poly1305_beforenm | `pk` = any curve25519 small-order/blocklisted value (32x`0x00`, `0100..00`, `e0eb7a..b800`, `5f9c95..1157`, `ecff..7f`, `edff..7f`, `eeff..7f`) -> `crypto_scalarmult_curve25519 != 0` | returns -1, `k` untouched |
| G5 | crypto_box_curve25519xchacha20poly1305_beforenm | same small-order `pk` set as above | returns -1 |
| G5 | crypto_box_detached / crypto_box_curve25519xchacha20poly1305_detached | `crypto_box_beforenm(k, pk, sk) != 0` (small-order pk) | returns -1 |
| G5 | crypto_box_easy | `mlen > crypto_box_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX - 16 - 32) | `sodium_misuse()` -> abort/handler, no return |
| G5 | crypto_box_easy | inner `crypto_box_detached` fails (small-order pk) | returns -1 |
| G5 | crypto_box_easy_afternm | `mlen > crypto_box_MESSAGEBYTES_MAX` | `sodium_misuse()` |
| G5 | crypto_box_curve25519xchacha20poly1305_easy | `mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX` | `sodium_misuse()` |
| G5 | crypto_box_curve25519xchacha20poly1305_easy_afternm | `mlen > ..._MESSAGEBYTES_MAX` | `sodium_misuse()` |
| G5 | crypto_box_open_easy | `clen < crypto_box_MACBYTES` (clen in 0..15) | returns -1 |
| G5 | crypto_box_open_easy_afternm | `clen < crypto_box_MACBYTES` (clen in 0..15) | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_open_easy | `clen < 16` | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_open_easy_afternm | `clen < 16` | returns -1 |
| G5 | crypto_box_open_easy / crypto_box_open_detached | `crypto_box_beforenm(k, pk, sk) != 0` (sender pk small-order) | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_open_easy / _open_detached | `beforenm != 0` (sender pk small-order) | returns -1 |
| G5 | crypto_box_open_detached_afternm -> crypto_secretbox_open_detached | Poly1305 tag mismatch (`crypto_onetimeauth_poly1305_verify != 0`): any single bit flipped in `mac`, `c`, `n`, or `k`; also mac = 16x`0x00` | returns -1, `m` NOT written |
| G5 | crypto_box_open_easy / crypto_box_open_easy_afternm | MAC failure: flip a bit in `c[0..15]` (the MAC) or in `c[16..]` (the ciphertext) | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_open_detached_afternm | Poly1305 tag mismatch | returns -1 |
| G5 | crypto_box_open_easy | correct ciphertext but wrong recipient `sk` / wrong sender `pk` / wrong nonce | returns -1 (MAC failure) |
| G5 | crypto_box_curve25519xsalsa20poly1305 (NaCl zero-padded API) / crypto_box | `mlen < crypto_box_ZEROBYTES` (32); i.e. mlen in 0..31 -> `crypto_secretbox_xsalsa20poly1305` returns -1 | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305_afternm / crypto_box_afternm | `mlen < 32` | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305_open / crypto_box_open | `clen < 32` (clen in 0..31) | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305_open_afternm / crypto_box_open_afternm | `clen < 32` | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305_open / _open_afternm | Poly1305 verify failure on `c+16` over `clen-32` bytes (any bit flipped in c[16..]) | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305 / _open (NaCl API) | `beforenm != 0` (small-order pk) | returns -1 |
| G5 | crypto_box_curve25519xsalsa20poly1305 (NaCl API) | first 32 bytes of `m` not zero: NOT checked - silently produces a bogus box (no rejection); documented divergence risk | returns 0 |
| G5 | crypto_box_seal | `mlen > crypto_box_MESSAGEBYTES_MAX` | `sodium_misuse()` |
| G5 | crypto_box_seal | `crypto_box_keypair(epk, esk) != 0` (LCOV_EXCL, cannot fail: `crypto_scalarmult_curve25519_base` has no error path) | returns -1 |
| G5 | crypto_box_seal | recipient `pk` small-order (32x`0x00` etc.) -> inner `crypto_box_easy` -> `beforenm` -> -1; NOTE `memcpy(c, epk, 32)` still runs | returns -1, first 32 bytes of `c` written |
| G5 | crypto_box_curve25519xchacha20poly1305_seal | `mlen > ..._MESSAGEBYTES_MAX` | `sodium_misuse()` |
| G5 | crypto_box_curve25519xchacha20poly1305_seal | recipient `pk` small-order | returns -1 |
| G5 | crypto_box_seal_open | `clen < crypto_box_SEALBYTES` (= 32 + 16 = 48); clen in 0..47 | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_seal_open | `clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES` (48) | returns -1 |
| G5 | crypto_box_seal_open | embedded ephemeral pk `c[0..31]` is small-order (e.g. set c[0..31] = 32x`0x00`) -> `crypto_box_open_easy` -> `beforenm` -> -1 | returns -1 |
| G5 | crypto_box_seal_open | valid length but MAC failure: flip a bit in `c[32..47]` (MAC), in the ciphertext, in the embedded epk (changes the derived nonce), or use the wrong recipient `sk`/`pk` | returns -1 |
| G5 | crypto_box_curve25519xchacha20poly1305_seal_open | same MAC / wrong-key failures | returns -1 |
| G5 | crypto_kx_client_session_keys | `rx == NULL && tx == NULL` -> `sodium_misuse()` (crypto_kx.c:51-53) | `sodium_misuse()` -> abort/handler |
| G5 | crypto_kx_server_session_keys | `rx == NULL && tx == NULL` -> `sodium_misuse()` (crypto_kx.c:92-94) | `sodium_misuse()` |
| G5 | crypto_kx_client_session_keys | `server_pk` is a curve25519 small-order/blocklisted value (32x`0x00`, `0100..00`, `ecff..7f`, `edff..7f`, `eeff..7f`, `e0eb7a..b800`, `5f9c95..1157`) -> `crypto_scalarmult != 0` | returns -1, `rx`/`tx` untouched |
| G5 | crypto_kx_server_session_keys | `client_pk` is a curve25519 small-order/blocklisted value | returns -1 |
| G5 | crypto_kx_keypair / crypto_kx_seed_keypair | no failure path (`crypto_scalarmult_base` cannot fail); seed = 32x`0x00` is accepted | always returns 0 |
| G5 | crypto_sign_ed25519_verify_detached / crypto_sign_verify_detached | non-canonical S: `(sig[63] & 240) != 0 && sc25519_is_canonical(sig+32) == 0`, i.e. S >= L. Concrete S = L = `edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010`, or S = 32x`0xff` | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | non-canonical `pk`: `ge25519_is_canonical(pk) == 0`, i.e. `pk[1..30]==0xff && (pk[31]&0x7f)==0x7f && pk[0]>=0xed`. Concrete `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `ffff..ff` | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | `pk` not a curve point: `ge25519_frombytes_negate_vartime(&A, pk) != 0` (neither `vx^2-u` nor `vx^2+u` is 0); ~50% of random 32-byte strings | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | `pk` has small order: `ge25519_has_small_order(&A) != 0`; the 8 torsion encodings `0100..00`, `0000..00`, `0000..0080`, `ecff..ff7f`, `ecff..ffff`, `26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05`(+`..85`), `c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a`(+`..fa`) | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | R = `sig[0..31]` not a curve point: `ge25519_frombytes(&expected_r, sig) != 0` | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | R = `sig[0..31]` has small order: `ge25519_has_small_order(&expected_r) != 0` (e.g. R = `0100..00` or 32x`0x00`) | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | signature does not satisfy the cofactorless-with-small-order-tolerance check: `check = R - (S*B - h*A)` is not small order -> `return ge25519_has_small_order(&check) - 1`. Trigger: any bit flipped in `sig`, in `m`, or a mismatched `pk` | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | ED25519_COMPAT build only (not enabled by default): `sig[63] & 224` nonzero | returns -1 |
| G5 | crypto_sign_ed25519_verify_detached | valid non-ph signature presented to `crypto_sign_ed25519ph_final_verify` (prehashed=1 domain prefix mismatch) and vice versa | returns -1 |
| G5 | crypto_sign_ed25519_open / crypto_sign_open | `smlen < 64`; smlen in 0..63 | returns -1, `*mlen_p = 0`, `m` untouched |
| G5 | crypto_sign_ed25519_open / crypto_sign_open | `smlen - 64 > crypto_sign_ed25519_MESSAGEBYTES_MAX` (= SODIUM_SIZE_MAX - 64) | returns -1, `*mlen_p = 0` |
| G5 | crypto_sign_ed25519_open / crypto_sign_open | `crypto_sign_ed25519_verify_detached != 0` (any of the rejections above) | returns -1, `memset(m, 0, smlen-64)`, `*mlen_p = 0` |
| G5 | crypto_sign_ed25519 (combined sign) | `crypto_sign_ed25519_detached != 0 || siglen != 64` (LCOV_EXCL, unreachable: detached always returns 0) | returns -1, `*smlen_p = 0`, `memset(sm, 0, mlen+64)` |
| G5 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_frombytes_negate_vartime(&A, ed25519_pk) != 0` (pk not a curve point) | returns -1 |
| G5 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_has_small_order(&A) != 0`: pk in the 8 torsion set (`0100..00`, `0000..00`, `0000..0080`, `ecff..ff7f`, `ecff..ffff`, `26e8958f..fc05`/`..fc85`, `c7176a70..037a`/`..03fa`) | returns -1 |
| G5 | crypto_sign_ed25519_pk_to_curve25519 | `ge25519_is_on_main_subgroup(&A) == 0`: pk on curve but composite order | returns -1 |
| G5 | crypto_sign_ed25519_pk_to_curve25519 | NOTE: no `ge25519_is_canonical` check here (unlike verify_detached), so `edff..7f`-style non-canonical y is ACCEPTED if it decodes; divergence risk | returns 0 |
| G5 | crypto_sign_ed25519_sk_to_seed / _sk_to_pk / _sk_to_curve25519 | no rejection path at all (pure memmove / sha512+clamp) | always returns 0 |
| G5 | crypto_sign_ed25519_detached / crypto_sign_ed25519 / crypto_sign_ed25519ph_final_create | no rejection path; any 64-byte `sk` accepted incl. all-zero | always returns 0, `*siglen_p = 64` |
| G5 | crypto_sign_ed25519_seed_keypair / crypto_sign_ed25519_keypair | no rejection path; seed = 32x`0x00` accepted | always returns 0 |
| G5 | crypto_sign_ed25519ph_init / _update | no rejection path (`crypto_hash_sha512_update` always 0) | always returns 0 |
| G5 | crypto_core_ed25519_is_valid_point | `ge25519_is_canonical(p) == 0`: `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `eeff..ff7f`, `ffff..ffff` | returns 0 |
| G5 | crypto_core_ed25519_is_valid_point | `ge25519_frombytes(&p_p3, p) != 0`: p is not a valid y-coordinate (~50% of random strings) | returns 0 |
| G5 | crypto_core_ed25519_is_valid_point | `ge25519_is_on_curve(&p_p3) == 0` (redundant second check after frombytes) | returns 0 |
| G5 | crypto_core_ed25519_is_valid_point | `ge25519_has_small_order(&p_p3) != 0`: any of the 8 torsion encodings, incl. identity `0100..00` and 32x`0x00` | returns 0 |
| G5 | crypto_core_ed25519_is_valid_point | `ge25519_is_on_main_subgroup(&p_p3) == 0`: valid curve point of order 2L/4L/8L | returns 0 |
| G5 | crypto_core_ed25519_add | `ge25519_frombytes(&p_p3, p) != 0` (p not a decodable y) | returns -1, `r` untouched |
| G5 | crypto_core_ed25519_add | `ge25519_is_on_curve(&p_p3) == 0` | returns -1 |
| G5 | crypto_core_ed25519_add | `ge25519_frombytes(&q_p3, q) != 0` (q not decodable; note p is checked first, short-circuit order matters) | returns -1 |
| G5 | crypto_core_ed25519_add | `ge25519_is_on_curve(&q_p3) == 0` | returns -1 |
| G5 | crypto_core_ed25519_add | NOTE: no `is_canonical` / small-order / subgroup check -> non-canonical `edff..7f` and small-order/identity points are ACCEPTED here (unlike is_valid_point); divergence risk | returns 0 |
| G5 | crypto_core_ed25519_sub | `ge25519_frombytes(&p_p3, p) != 0` or `ge25519_is_on_curve(&p_p3) == 0` | returns -1 |
| G5 | crypto_core_ed25519_sub | `ge25519_frombytes(&q_p3, q) != 0` or `ge25519_is_on_curve(&q_p3) == 0` | returns -1 |
| G5 | crypto_core_ed25519_scalar_invert | `s` = 32x`0x00` -> `- sodium_is_zero(s, 32)` | returns -1, `recip` still written (garbage) |
| G5 | crypto_core_ed25519_scalar_invert | `s` = L or any nonzero s ≡ 0 mod L (e.g. `edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010`): NOT rejected because only the literal byte-zero test is done | returns 0 with a meaningless `recip`; divergence risk |
| G5 | crypto_core_ed25519_from_string / _from_string_nu | `hash_alg` not in {1 (H2CSHA256), 2 (H2CSHA512)} -> `core_h2c_string_to_hash` default branch sets `errno = EINVAL` | returns -1 |
| G5 | crypto_core_ed25519_scalar_from_string | `hash_alg` not in {1, 2} | returns -1 |
| G5 | _string_to_points (static, core_ed25519.c:72) | `n > 2U` -> `abort()` (LCOV_EXCL; unreachable, only called with n=1 or n=2) | `abort()` |
| G5 | core_h2c_string_to_hash_sha256 / _sha512 (core_h2c.c:26, :82) | `assert(h_len <= 0xff)` - only reachable if a caller requests > 255 output bytes; callers use 48 or 96 | assertion failure / abort in debug builds |
| G5 | ge25519_elligator2 (ed25519_ref10.c:2684, via crypto_core_ed25519_random / ge25519_from_uniform / ge25519_from_hash) | `ge25519_xmont_to_ymont(y, x) != 0` -> `abort()` (LCOV_EXCL, mathematically unreachable) | `abort()` |
| G5 | crypto_core_ed25519_random / _scalar_random / _scalar_negate / _scalar_complement / _scalar_add / _scalar_sub / _scalar_mul / _scalar_reduce | `void` return type - no rejection path exists at all; all 32/64-byte inputs accepted incl. 64x`0xff` for scalar_reduce | no error signalling possible |
| G5 | crypto_core_ed25519_scalar_is_canonical | `s >= L`: `s` = L itself (`edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010`), `s` = 32x`0xff`, `s` = `0000..0080` | returns 0 (not an error code; 1 == canonical) |
| G5 | crypto_core_ristretto255_is_valid_point | `ristretto255_frombytes(&p_p3, p) != 0`: high bit set (`0000..0080`, `ffff..ffff`) | returns 0 |
| G5 | crypto_core_ristretto255_is_valid_point | `p[0]` odd (negative field element): `0100..00`, `01ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, `ed57ffd8c914fb201471d1c3d245ce3c746fcbe63a3679d51b6a516ebebe0e20` | returns 0 |
| G5 | crypto_core_ristretto255_is_valid_point | `p` >= field prime (non-canonical): `edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f`, `f3ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`, `00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` | returns 0 |
| G5 | crypto_core_ristretto255_is_valid_point | `v*u2^2` non-square: `26948d35ca62e643e26a83177332e6b6afeb9d08e4268b650f1f5bbd8d81d371`, `4eac077a713c57b4f4397629a4145982c661f48044dd3f96427d40b147d9742f`, `de6a7b00deadc788eb6b6c8d20c0ae96c2f2019078fa604fee5b87d6e989ad7b` | returns 0 |
| G5 | crypto_core_ristretto255_is_valid_point | `fe25519_isnegative(h->T)` (negative x*y): `3eb858e78f5a7254d8c9731174a94f76755fd3941c0ac93735c07ba14579630e`, `a45fdc55c76448c049a1ab33f17023edfb2be3581e9c7aade8a6125215e04220` | returns 0 |
| G5 | crypto_core_ristretto255_is_valid_point | `fe25519_iszero(h->Y)` | returns 0 |
| G5 | crypto_core_ristretto255_add | `ristretto255_frombytes(&p_p3, p) != 0` (any of the bad-encoding classes above) | returns -1, `r` untouched |
| G5 | crypto_core_ristretto255_add | `ristretto255_frombytes(&q_p3, q) != 0` (p checked first) | returns -1 |
| G5 | crypto_core_ristretto255_sub | `ristretto255_frombytes(&p_p3, p) != 0` | returns -1 |
| G5 | crypto_core_ristretto255_sub | `ristretto255_frombytes(&q_p3, q) != 0` | returns -1 |
| G5 | crypto_core_ristretto255_from_hash | no rejection path: any 64 bytes (incl. 64x`0x00` and 64x`0xff`) map to a valid element | always returns 0 |
| G5 | crypto_core_ristretto255_from_string | `hash_alg` not in {1, 2} | returns -1 |
| G5 | crypto_core_ristretto255_scalar_from_string | `hash_alg` not in {1, 2} | returns -1 |
| G5 | crypto_core_ristretto255_scalar_invert | `s` = 32x`0x00` (delegates to crypto_core_ed25519_scalar_invert) | returns -1 |
| G5 | crypto_core_ristretto255_scalar_invert | `s` = L (nonzero but ≡ 0 mod L): NOT rejected | returns 0 with meaningless `recip`; divergence risk |
| G5 | crypto_core_ristretto255_scalar_is_canonical | `s >= L`: `s` = L, `s` = 32x`0xff` | returns 0 |
| G5 | crypto_core_ristretto255_random / _scalar_random / _scalar_negate / _scalar_complement / _scalar_add / _scalar_sub / _scalar_mul / _scalar_reduce | `void` return - no rejection path | no error signalling possible |
| G5 | crypto_core_ed25519_from_uniform / crypto_core_ed25519_from_hash | NOT PRESENT in libsodium 1.0.23 - removed from `include/sodium/crypto_core_ed25519.h`; replaced by `crypto_core_ed25519_from_string{,_nu}`. `ge25519_from_uniform`/`ge25519_from_hash` remain internal-only | n/a - no public entry point to test |
