# kemip — Configuration / input-shape coverage

Family: `crypto_kem_mlkem768`, `crypto_kem_xwing`, `crypto_ipcrypt`.

Each meaningful configuration is driven through DETERMINISTIC entry points
(`*_seed_keypair`, `*_enc_deterministic`, and the inherently-deterministic
ipcrypt block functions), so C and Rust outputs are compared byte-for-byte over
128 randomized inputs per config (fixed-seed PRNG). `[x]` = test passing.

| # | entry point(s) | configuration (options + shape) | [x] |
|---|----------------|--------------------------------|-----|
| 1 | `crypto_kem_mlkem768_seed_keypair` + `_enc_deterministic` + `_dec` | full valid KEM cycle: 64-byte kp seed, 32-byte enc seed; assert pk/sk/ct/ss byte-equal + dec==enc | [x] |
| 2 | `crypto_kem_mlkem768_{seed_keypair,enc_deterministic,dec}` | cross-lib: Rust-encaps -> C-decaps and C-encaps -> Rust-decaps yield same ss | [x] |
| 3 | `crypto_kem_xwing_seed_keypair` + `_enc_deterministic` + `_dec` | full valid KEM cycle: 32-byte kp seed, 64-byte enc seed; pk/sk/ct/ss byte-equal + dec==enc | [x] |
| 4 | `crypto_kem_xwing_{seed_keypair,enc_deterministic,dec}` | cross-lib: Rust-encaps -> C-decaps and C-encaps -> Rust-decaps yield same ss | [x] |
| 5 | `crypto_ipcrypt_encrypt` / `crypto_ipcrypt_decrypt` | deterministic single-block (16B) AES; two fixed keys, many inputs; encrypt byte-equal + roundtrip | [x] |
| 6 | `crypto_ipcrypt_nd_encrypt` / `crypto_ipcrypt_nd_decrypt` | nd variant: 16B key, 8B tweak, 16B in -> 24B out (tweak prepended); byte-equal + roundtrip | [x] |
| 7 | `crypto_ipcrypt_ndx_encrypt` / `crypto_ipcrypt_ndx_decrypt` | ndx variant: 32B key, 16B tweak, 16B in -> 32B out (XEX); byte-equal + roundtrip | [x] |
| 8 | `crypto_ipcrypt_ndx_{encrypt,decrypt}` | ndx degenerate-key collision branch (equal key halves -> diff==0 fallback) | [x] |
| 9 | `crypto_ipcrypt_pfx_encrypt` / `crypto_ipcrypt_pfx_decrypt` | pfx variant: 32B key, 16B in/out; alternating general vs IPv4-mapped input (prefix_start 0 vs 96) | [x] |
| 10 | `crypto_ipcrypt_pfx_{encrypt,decrypt}` | pfx degenerate-key collision branch (equal key halves -> fallback) | [x] |
| 11 | `crypto_kem_mlkem768_enc_deterministic` | error: all-`0xFF` (non-canonical) pk -> both return -1 | [x] |
| 12 | `crypto_kem_mlkem768_enc_deterministic` | error: random pk, return code parity per input (accept -> compare ct/ss) | [x] |
| 13 | `crypto_kem_xwing_enc_deterministic` | error: non-canonical embedded ML-KEM pk -> both return -1 | [x] |
| 14 | `crypto_kem_mlkem768_dec` | implicit rejection: tampered ct -> rc 0, pseudo-random ss byte-equal, != true ss | [x] |
| 15 | `crypto_kem_xwing_dec` | tampered ct in ML-KEM portion -> rc parity, ss byte-equal | [x] |

Coverage notes:
- Both KEMs expose `keypair` (randomized) and `enc` (randomized); these wrap the
  deterministic entry points with `randombytes_buf`, so byte-for-byte comparison
  is only meaningful via the `*_seed_keypair` / `*_enc_deterministic` forms used
  above. Randomized wrappers are exercised indirectly (same code path) and via
  cross-lib roundtrip semantics.
- ipcrypt pfx IPv4-mapped path (`is_ipv4_mapped`, `prefix_start = 96`) is covered
  by config #9's alternating inputs.
