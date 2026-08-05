# kemip — Error / rejection paths

Family: `crypto_kem_mlkem768`, `crypto_kem_xwing`, `crypto_ipcrypt`.

Every distinct rejection or non-normal-success behavior in the C source
(`c_src/libsodium/crypto_kem/**`, `c_src/libsodium/crypto_ipcrypt/**`) and the
return code the C code produces, verified to match the Rust `.so` byte-for-byte.

| # | function | trigger | expected C result |
|---|----------|---------|--------------------|
| 1 | `crypto_kem_mlkem768_enc_deterministic` | public key not canonical: any decoded coefficient >= q (3329). `polyvec_frombytes` + `polyvec_is_canonical` == 0. All-`0xFF` pk decodes to coeffs `0xFFF` = 4095 >= 3329. | returns `-1`; `ct`/`ss` left unmodified |
| 2 | `crypto_kem_mlkem768_enc_deterministic` | fully random 1184-byte pk (almost always non-canonical) | returns `-1` (same on both libs for the identical random inputs; return code compared each iter) |
| 3 | `crypto_kem_mlkem768_dec` | tampered ciphertext (flip 1 bit) — FO transform implicit rejection: `sodium_memcmp(ct, cmp)` fails, `cmov` swaps in `k_bar = SHAKE256(z ‖ ct)` | returns `0` (NO error); shared secret is the pseudo-random reject value, differs from true ss, identical across both libs |
| 4 | `crypto_kem_xwing_enc_deterministic` | embedded ML-KEM pk (first 1184 of 1216 bytes) non-canonical -> inner `crypto_kem_mlkem768_enc_deterministic` returns non-zero | returns `-1` |
| 5 | `crypto_kem_xwing_dec` | tampered ciphertext (flip a bit in the ML-KEM portion) — ML-KEM half uses implicit rejection, combiner always runs | returns `0`; shared secret differs from true ss but is identical across both libs |
| 6 | `crypto_ipcrypt_ndx_encrypt` / `crypto_ipcrypt_ndx_decrypt` | degenerate key: `tkeys[ROUNDS/2] == rkeys[ROUNDS/2]` (`diff == 0`), e.g. both 16-byte key halves equal. Fallback re-expands `rkeys` from `k[i] ^ 0x5a`. | no error (void fn); output identical across libs, roundtrip holds |
| 7 | `crypto_ipcrypt_pfx_encrypt` / `crypto_ipcrypt_pfx_decrypt` | same degenerate-key fallback (`k1keys[5] == k2keys[5]`) | no error (void fn); output identical across libs, roundtrip holds |

Notes:
- The ipcrypt API is all `void` — there are no return-code error paths. The only
  branch-level "error-ish" behavior is the degenerate-key fallback (#6, #7),
  which is covered by dedicated collision-branch tests.
- `__attribute__((nonnull))` / `warn_unused_result` are compile-time hints; they
  are not runtime rejections and cannot be triggered through the FFI boundary
  (passing null would be UB in both libs), so they are not tested as error paths.
- ML-KEM / X-Wing decapsulation never returns a hard error on bad ciphertext by
  design (IND-CCA implicit rejection); the `return -1` lines in `kem_xwing.c` dec
  and in `mlkem768_ref_dec`'s non-canonical branch are marked `LCOV_EXCL` in the C
  source (unreachable for well-formed secret keys) — represented by #3/#5.
