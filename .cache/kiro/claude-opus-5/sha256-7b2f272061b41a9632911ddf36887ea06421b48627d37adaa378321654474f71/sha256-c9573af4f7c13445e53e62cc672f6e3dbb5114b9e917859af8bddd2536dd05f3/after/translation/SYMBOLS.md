# SYMBOLS.md — exported-symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from:
```
nm -D --defined-only c_src/build/libsodium.so            | awk "{print \$3}" | sort -u
nm -D --defined-only translation/target/release/liblibsodium.so | awk "{print \$3}" | sort -u
```

## Result

| metric | value |
|---|---|
| symbols exported by C `.so`    | 890 |
| symbols exported by Rust `.so` | 890 |
| **in C but MISSING from Rust** | **0** |
| in Rust but not in C (extra)   | 0 |
| non-libc undefined symbols in Rust `.so` | 0 |

`nm -D` diff is **empty in both directions**. No stubs were added; the crate
already contained a `#[no_mangle] extern "C"` wrapper for every C export.

Undefined symbols in the Rust `.so` are all libc / libgcc-unwind imports
(`memcpy`, `malloc`, `__errno_location`, `_Unwind_*`, …) — `ldd` resolves only
`libgcc_s.so.1` and `libc.so.6`. Zero non-libc undefined symbols.

## Fixes applied in Phase A

The crate did **not** compile as delivered: 45 `E0761` errors. Two complete,
parallel translations of every module had been left in the tree — a *flat* one
(`src/crypto_hash/sha256.rs`) and a *directory* one mirroring the C file layout
(`src/crypto_hash/sha256/{mod.rs,hash_sha256.rs,cp/…}`), so every `pub mod X;`
resolved ambiguously.

The flat set is the one the module tree actually references
(`lib.rs` declares `sodium_codecs`/`sodium_core`/… as top-level flat files, and
`crypto_core/mod.rs` declares `core_ed25519`/`ed25519_ref10` — names that exist
only in the flat layout). The shadowing directories, plus four orphan directories
never declared by any parent (`src/sodium/`, `src/crypto_core/ed25519/`,
`src/crypto_kdf/hkdf/`, `src/crypto_pwhash/scryptsalsa208sha256/`), were removed.
Both sets contained the identical number of `no_mangle` wrappers per module, so
no exported symbol was lost — confirmed by the 0/0 `nm -D` diff above.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Rather than assume that,
`tools/feature_matrix.sh` runs the whole suite three ways — default,
`--no-default-features`, `--all-features` — and all three give
`C=890 Rust=890 missing=0 extra=0` and a fully green suite.

## Completion gate (Phase D)

| gate | status | evidence |
|---|---|---|
| `nm -D`: 0 missing, 0 extra, 0 non-libc undefined in the Rust `.so` | **PASS** | 890 / 890 both directions; `ldd` resolves only `libc` + `libgcc_s`; asserted from inside the suite by `t00_harness::symbol_parity` |
| Phase B: every `CONFIGS.md` row passes over randomized inputs | **PASS** | 581 / 581 rows checked off, see `CONFIGS.md` |
| Phase C: every `ERRORS.md` row has a passing differential test | **PASS** | 378 / 378 rows checked off, see `ERRORS.md` |
| Every exported symbol is named by at least one differential test | **PASS** | 890 / 890, `tools/build_coverage.py` |
| All of the above under every feature combination | **PASS** | 3 / 3 configurations, `tools/feature_matrix.sh` |

Suite: **191 `#[test]` functions across 15 files**, 153 s wall time
(`cargo test --release -- --test-threads=1`).

Every assertion goes through `libloading` against the two `.so` files; no Rust
function is ever called directly, so the `#[no_mangle]` export wrappers are on
the tested path.

### Does the suite actually detect divergence?

Passing tests only mean something if they can fail. `tools/mutation_test.py`
injects **31 small semantic bugs** into the Rust translation — one per module
family (wrong rotation constant, corrupted BLAKE2b sigma row, wrong SHA round
constant, wrong SHAKE domain byte/rate, wrong Poly1305 `r` clamp, wrong SipHash
rotation, shifted hex table, wrong ML-KEM modulus, wrong Argon2 block size,
wrong HKDF counter start, wrong Ed25519 scalar clamp, …) — rebuilds, and
requires the corresponding test target to turn **red**.

```
detected: 31   NOT detected: 0   skipped: 0
```

Three earlier candidate mutations were *equivalent mutants* (no observable
effect for any runnable input) and were replaced with real ones rather than
recorded as coverage gaps; that analysis is in the script's comments.

## Divergences found and fixed

One real divergence in the translation, plus one behavioural fact that shapes
several tests:

1. **`crypto_generichash_blake2b_final` / `crypto_generichash_final` were
   missing a live C assertion.** The C is
   ```c
   int crypto_generichash_blake2b_final(..., const size_t outlen) {
       assert(outlen <= UINT8_MAX);
       return blake2b_final(..., (uint8_t) outlen);
   }
   ```
   and `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so no `-DNDEBUG` is
   added and **`assert()` is live**. Unlike every other public BLAKE2b entry
   point, `_final` range-checks nothing itself, so `outlen = 257` hits the
   assertion and aborts in C, while the Rust silently truncated to `1` and
   returned 0. Fixed by reproducing the abort (`sodium_core::sodium_assert_fail`),
   pinned by `t05_mac::generichash_final_out_of_range_outlen_aborts_identically`
   over `outlen` 0/65/100/255/256/257/1000.

2. **`crypto_pwhash_argon2{i,id}_str` / `argon2*_hash_encoded` abort on a
   too-small `encodedlen`.** `argon2_encode_string` emits its base64 fields with
   `sodium_bin2base64(dst, dst_len, ...)`, and that function calls
   `sodium_misuse()` — it never returns NULL — when the destination is too
   small. So an `encodedlen` big enough for the
   `$argon2i$v=19$m=..,t=..,p=..$` prefix but too small for the base64 salt
   terminates the process, while the smallest capacities fail cleanly through
   the `SS()` macro's own length check. Both libraries already agreed; the
   boundary is now pinned by
   `t14_remaining_exports::argon2_hash_encoded_small_buffer_aborts_identically`.

Two rows also had to be corrected against the C rather than trusted from the
initial analysis: `crypto_ipcrypt` has **no** error surface at all (every
operational entry point returns `void` and there is no IP-string parsing in this
version), and `crypto_shorthash` has none either — both verified by grep, not
assumed. Conversely the initial analysis claimed the BLAKE2b `sodium_misuse()`
guards were unreachable behind the public wrappers; they are not —
`crypto_generichash(out, outlen, in, inlen, NULL, keylen>0)` reaches
`blake2b()`'s `key == NULL && keylen > 0` trap, and that is now tested.

### Full symbol list (890 symbols, all present in both `.so`s)

Regenerate everything in this file, plus `ERRORS.md` / `CONFIGS.md`, with:

```sh
./tools/run_tests.sh            # build both .so, gate on nm -D, run the suite
python3 tools/build_coverage.py # map every table row to its covering #[test]
python3 tools/assemble_tables.py
./tools/feature_matrix.sh       # Phase D: every feature combination
python3 tools/mutation_test.py  # prove the suite detects divergence
```

| # | symbol | in C .so | in Rust .so |
|---|--------|----------|-------------|
| 1 | `_crypto_aead_aegis128l_pick_best_implementation` | yes | yes |
| 2 | `_crypto_aead_aegis256_pick_best_implementation` | yes | yes |
| 3 | `_crypto_generichash_blake2b_pick_best_implementation` | yes | yes |
| 4 | `_crypto_ipcrypt_pick_best_implementation` | yes | yes |
| 5 | `_crypto_onetimeauth_poly1305_pick_best_implementation` | yes | yes |
| 6 | `_crypto_pwhash_argon2_pick_best_implementation` | yes | yes |
| 7 | `_crypto_scalarmult_curve25519_pick_best_implementation` | yes | yes |
| 8 | `_crypto_sign_ed25519_detached` | yes | yes |
| 9 | `_crypto_sign_ed25519_ref10_hinit` | yes | yes |
| 10 | `_crypto_sign_ed25519_verify_detached` | yes | yes |
| 11 | `_crypto_stream_chacha20_pick_best_implementation` | yes | yes |
| 12 | `_crypto_stream_salsa20_pick_best_implementation` | yes | yes |
| 13 | `_sodium_alloc_init` | yes | yes |
| 14 | `_sodium_argon2_ctx` | yes | yes |
| 15 | `_sodium_argon2_decode_string` | yes | yes |
| 16 | `_sodium_argon2_encode_string` | yes | yes |
| 17 | `_sodium_argon2_fill_memory_blocks` | yes | yes |
| 18 | `_sodium_argon2_fill_segment_ref` | yes | yes |
| 19 | `_sodium_argon2_finalize` | yes | yes |
| 20 | `_sodium_argon2_hash` | yes | yes |
| 21 | `_sodium_argon2_initialize` | yes | yes |
| 22 | `_sodium_argon2_validate_inputs` | yes | yes |
| 23 | `_sodium_argon2_verify` | yes | yes |
| 24 | `_sodium_argon2i_hash_encoded` | yes | yes |
| 25 | `_sodium_argon2i_hash_raw` | yes | yes |
| 26 | `_sodium_argon2i_verify` | yes | yes |
| 27 | `_sodium_argon2id_hash_encoded` | yes | yes |
| 28 | `_sodium_argon2id_hash_raw` | yes | yes |
| 29 | `_sodium_argon2id_verify` | yes | yes |
| 30 | `_sodium_blake2b` | yes | yes |
| 31 | `_sodium_blake2b_compress_ref` | yes | yes |
| 32 | `_sodium_blake2b_final` | yes | yes |
| 33 | `_sodium_blake2b_init` | yes | yes |
| 34 | `_sodium_blake2b_init_key` | yes | yes |
| 35 | `_sodium_blake2b_init_key_salt_personal` | yes | yes |
| 36 | `_sodium_blake2b_init_param` | yes | yes |
| 37 | `_sodium_blake2b_init_salt_personal` | yes | yes |
| 38 | `_sodium_blake2b_long` | yes | yes |
| 39 | `_sodium_blake2b_pick_best_implementation` | yes | yes |
| 40 | `_sodium_blake2b_salt_personal` | yes | yes |
| 41 | `_sodium_blake2b_update` | yes | yes |
| 42 | `_sodium_core_h2c_string_to_hash` | yes | yes |
| 43 | `_sodium_escrypt_PBKDF2_SHA256` | yes | yes |
| 44 | `_sodium_escrypt_alloc_region` | yes | yes |
| 45 | `_sodium_escrypt_free_local` | yes | yes |
| 46 | `_sodium_escrypt_free_region` | yes | yes |
| 47 | `_sodium_escrypt_gensalt_r` | yes | yes |
| 48 | `_sodium_escrypt_init_local` | yes | yes |
| 49 | `_sodium_escrypt_kdf_nosse` | yes | yes |
| 50 | `_sodium_escrypt_parse_setting` | yes | yes |
| 51 | `_sodium_escrypt_r` | yes | yes |
| 52 | `_sodium_fe25519_frombytes` | yes | yes |
| 53 | `_sodium_fe25519_invert` | yes | yes |
| 54 | `_sodium_fe25519_tobytes` | yes | yes |
| 55 | `_sodium_ge25519_clear_cofactor` | yes | yes |
| 56 | `_sodium_ge25519_double_scalarmult_vartime` | yes | yes |
| 57 | `_sodium_ge25519_from_hash` | yes | yes |
| 58 | `_sodium_ge25519_from_uniform` | yes | yes |
| 59 | `_sodium_ge25519_frombytes` | yes | yes |
| 60 | `_sodium_ge25519_frombytes_negate_vartime` | yes | yes |
| 61 | `_sodium_ge25519_has_small_order` | yes | yes |
| 62 | `_sodium_ge25519_is_canonical` | yes | yes |
| 63 | `_sodium_ge25519_is_on_curve` | yes | yes |
| 64 | `_sodium_ge25519_is_on_main_subgroup` | yes | yes |
| 65 | `_sodium_ge25519_p1p1_to_p2` | yes | yes |
| 66 | `_sodium_ge25519_p1p1_to_p3` | yes | yes |
| 67 | `_sodium_ge25519_p2_to_p3` | yes | yes |
| 68 | `_sodium_ge25519_p3_add` | yes | yes |
| 69 | `_sodium_ge25519_p3_sub` | yes | yes |
| 70 | `_sodium_ge25519_p3_tobytes` | yes | yes |
| 71 | `_sodium_ge25519_scalarmult` | yes | yes |
| 72 | `_sodium_ge25519_scalarmult_base` | yes | yes |
| 73 | `_sodium_ge25519_tobytes` | yes | yes |
| 74 | `_sodium_keccak1600_ref_extract_bytes` | yes | yes |
| 75 | `_sodium_keccak1600_ref_init` | yes | yes |
| 76 | `_sodium_keccak1600_ref_permute_12` | yes | yes |
| 77 | `_sodium_keccak1600_ref_permute_24` | yes | yes |
| 78 | `_sodium_keccak1600_ref_xor_bytes` | yes | yes |
| 79 | `_sodium_mlkem768_ref_dec` | yes | yes |
| 80 | `_sodium_mlkem768_ref_enc` | yes | yes |
| 81 | `_sodium_mlkem768_ref_enc_deterministic` | yes | yes |
| 82 | `_sodium_mlkem768_ref_keypair` | yes | yes |
| 83 | `_sodium_mlkem768_ref_seed_keypair` | yes | yes |
| 84 | `_sodium_ristretto255_from_hash` | yes | yes |
| 85 | `_sodium_ristretto255_frombytes` | yes | yes |
| 86 | `_sodium_ristretto255_p3_tobytes` | yes | yes |
| 87 | `_sodium_runtime_get_cpu_features` | yes | yes |
| 88 | `_sodium_sc25519_invert` | yes | yes |
| 89 | `_sodium_sc25519_is_canonical` | yes | yes |
| 90 | `_sodium_sc25519_mul` | yes | yes |
| 91 | `_sodium_sc25519_muladd` | yes | yes |
| 92 | `_sodium_sc25519_reduce` | yes | yes |
| 93 | `_sodium_shake128_ref` | yes | yes |
| 94 | `_sodium_shake128_ref_init` | yes | yes |
| 95 | `_sodium_shake128_ref_init_with_domain` | yes | yes |
| 96 | `_sodium_shake128_ref_squeeze` | yes | yes |
| 97 | `_sodium_shake128_ref_update` | yes | yes |
| 98 | `_sodium_shake256_ref` | yes | yes |
| 99 | `_sodium_shake256_ref_init` | yes | yes |
| 100 | `_sodium_shake256_ref_init_with_domain` | yes | yes |
| 101 | `_sodium_shake256_ref_squeeze` | yes | yes |
| 102 | `_sodium_shake256_ref_update` | yes | yes |
| 103 | `_sodium_softaes_block_decrypt` | yes | yes |
| 104 | `_sodium_softaes_block_decryptlast` | yes | yes |
| 105 | `_sodium_softaes_block_encrypt` | yes | yes |
| 106 | `_sodium_softaes_block_encryptlast` | yes | yes |
| 107 | `_sodium_softaes_expand_key128` | yes | yes |
| 108 | `_sodium_softaes_expand_key256` | yes | yes |
| 109 | `_sodium_softaes_inv_mix_columns` | yes | yes |
| 110 | `_sodium_softaes_invert_key_schedule128` | yes | yes |
| 111 | `_sodium_softaes_invert_key_schedule256` | yes | yes |
| 112 | `_sodium_turboshake128_ref` | yes | yes |
| 113 | `_sodium_turboshake128_ref_init` | yes | yes |
| 114 | `_sodium_turboshake128_ref_init_with_domain` | yes | yes |
| 115 | `_sodium_turboshake128_ref_squeeze` | yes | yes |
| 116 | `_sodium_turboshake128_ref_update` | yes | yes |
| 117 | `_sodium_turboshake256_ref` | yes | yes |
| 118 | `_sodium_turboshake256_ref_init` | yes | yes |
| 119 | `_sodium_turboshake256_ref_init_with_domain` | yes | yes |
| 120 | `_sodium_turboshake256_ref_squeeze` | yes | yes |
| 121 | `_sodium_turboshake256_ref_update` | yes | yes |
| 122 | `aegis128l_soft_implementation` | yes | yes |
| 123 | `aegis256_soft_implementation` | yes | yes |
| 124 | `crypto_aead_aegis128l_abytes` | yes | yes |
| 125 | `crypto_aead_aegis128l_decrypt` | yes | yes |
| 126 | `crypto_aead_aegis128l_decrypt_detached` | yes | yes |
| 127 | `crypto_aead_aegis128l_encrypt` | yes | yes |
| 128 | `crypto_aead_aegis128l_encrypt_detached` | yes | yes |
| 129 | `crypto_aead_aegis128l_keybytes` | yes | yes |
| 130 | `crypto_aead_aegis128l_keygen` | yes | yes |
| 131 | `crypto_aead_aegis128l_messagebytes_max` | yes | yes |
| 132 | `crypto_aead_aegis128l_npubbytes` | yes | yes |
| 133 | `crypto_aead_aegis128l_nsecbytes` | yes | yes |
| 134 | `crypto_aead_aegis256_abytes` | yes | yes |
| 135 | `crypto_aead_aegis256_decrypt` | yes | yes |
| 136 | `crypto_aead_aegis256_decrypt_detached` | yes | yes |
| 137 | `crypto_aead_aegis256_encrypt` | yes | yes |
| 138 | `crypto_aead_aegis256_encrypt_detached` | yes | yes |
| 139 | `crypto_aead_aegis256_keybytes` | yes | yes |
| 140 | `crypto_aead_aegis256_keygen` | yes | yes |
| 141 | `crypto_aead_aegis256_messagebytes_max` | yes | yes |
| 142 | `crypto_aead_aegis256_npubbytes` | yes | yes |
| 143 | `crypto_aead_aegis256_nsecbytes` | yes | yes |
| 144 | `crypto_aead_aes256gcm_abytes` | yes | yes |
| 145 | `crypto_aead_aes256gcm_beforenm` | yes | yes |
| 146 | `crypto_aead_aes256gcm_decrypt` | yes | yes |
| 147 | `crypto_aead_aes256gcm_decrypt_afternm` | yes | yes |
| 148 | `crypto_aead_aes256gcm_decrypt_detached` | yes | yes |
| 149 | `crypto_aead_aes256gcm_decrypt_detached_afternm` | yes | yes |
| 150 | `crypto_aead_aes256gcm_encrypt` | yes | yes |
| 151 | `crypto_aead_aes256gcm_encrypt_afternm` | yes | yes |
| 152 | `crypto_aead_aes256gcm_encrypt_detached` | yes | yes |
| 153 | `crypto_aead_aes256gcm_encrypt_detached_afternm` | yes | yes |
| 154 | `crypto_aead_aes256gcm_is_available` | yes | yes |
| 155 | `crypto_aead_aes256gcm_keybytes` | yes | yes |
| 156 | `crypto_aead_aes256gcm_keygen` | yes | yes |
| 157 | `crypto_aead_aes256gcm_messagebytes_max` | yes | yes |
| 158 | `crypto_aead_aes256gcm_npubbytes` | yes | yes |
| 159 | `crypto_aead_aes256gcm_nsecbytes` | yes | yes |
| 160 | `crypto_aead_aes256gcm_statebytes` | yes | yes |
| 161 | `crypto_aead_chacha20poly1305_abytes` | yes | yes |
| 162 | `crypto_aead_chacha20poly1305_decrypt` | yes | yes |
| 163 | `crypto_aead_chacha20poly1305_decrypt_detached` | yes | yes |
| 164 | `crypto_aead_chacha20poly1305_encrypt` | yes | yes |
| 165 | `crypto_aead_chacha20poly1305_encrypt_detached` | yes | yes |
| 166 | `crypto_aead_chacha20poly1305_ietf_abytes` | yes | yes |
| 167 | `crypto_aead_chacha20poly1305_ietf_decrypt` | yes | yes |
| 168 | `crypto_aead_chacha20poly1305_ietf_decrypt_detached` | yes | yes |
| 169 | `crypto_aead_chacha20poly1305_ietf_encrypt` | yes | yes |
| 170 | `crypto_aead_chacha20poly1305_ietf_encrypt_detached` | yes | yes |
| 171 | `crypto_aead_chacha20poly1305_ietf_keybytes` | yes | yes |
| 172 | `crypto_aead_chacha20poly1305_ietf_keygen` | yes | yes |
| 173 | `crypto_aead_chacha20poly1305_ietf_messagebytes_max` | yes | yes |
| 174 | `crypto_aead_chacha20poly1305_ietf_npubbytes` | yes | yes |
| 175 | `crypto_aead_chacha20poly1305_ietf_nsecbytes` | yes | yes |
| 176 | `crypto_aead_chacha20poly1305_keybytes` | yes | yes |
| 177 | `crypto_aead_chacha20poly1305_keygen` | yes | yes |
| 178 | `crypto_aead_chacha20poly1305_messagebytes_max` | yes | yes |
| 179 | `crypto_aead_chacha20poly1305_npubbytes` | yes | yes |
| 180 | `crypto_aead_chacha20poly1305_nsecbytes` | yes | yes |
| 181 | `crypto_aead_xchacha20poly1305_ietf_abytes` | yes | yes |
| 182 | `crypto_aead_xchacha20poly1305_ietf_decrypt` | yes | yes |
| 183 | `crypto_aead_xchacha20poly1305_ietf_decrypt_detached` | yes | yes |
| 184 | `crypto_aead_xchacha20poly1305_ietf_encrypt` | yes | yes |
| 185 | `crypto_aead_xchacha20poly1305_ietf_encrypt_detached` | yes | yes |
| 186 | `crypto_aead_xchacha20poly1305_ietf_keybytes` | yes | yes |
| 187 | `crypto_aead_xchacha20poly1305_ietf_keygen` | yes | yes |
| 188 | `crypto_aead_xchacha20poly1305_ietf_messagebytes_max` | yes | yes |
| 189 | `crypto_aead_xchacha20poly1305_ietf_npubbytes` | yes | yes |
| 190 | `crypto_aead_xchacha20poly1305_ietf_nsecbytes` | yes | yes |
| 191 | `crypto_auth` | yes | yes |
| 192 | `crypto_auth_bytes` | yes | yes |
| 193 | `crypto_auth_hmacsha256` | yes | yes |
| 194 | `crypto_auth_hmacsha256_bytes` | yes | yes |
| 195 | `crypto_auth_hmacsha256_final` | yes | yes |
| 196 | `crypto_auth_hmacsha256_init` | yes | yes |
| 197 | `crypto_auth_hmacsha256_keybytes` | yes | yes |
| 198 | `crypto_auth_hmacsha256_keygen` | yes | yes |
| 199 | `crypto_auth_hmacsha256_statebytes` | yes | yes |
| 200 | `crypto_auth_hmacsha256_update` | yes | yes |
| 201 | `crypto_auth_hmacsha256_verify` | yes | yes |
| 202 | `crypto_auth_hmacsha512` | yes | yes |
| 203 | `crypto_auth_hmacsha512256` | yes | yes |
| 204 | `crypto_auth_hmacsha512256_bytes` | yes | yes |
| 205 | `crypto_auth_hmacsha512256_final` | yes | yes |
| 206 | `crypto_auth_hmacsha512256_init` | yes | yes |
| 207 | `crypto_auth_hmacsha512256_keybytes` | yes | yes |
| 208 | `crypto_auth_hmacsha512256_keygen` | yes | yes |
| 209 | `crypto_auth_hmacsha512256_statebytes` | yes | yes |
| 210 | `crypto_auth_hmacsha512256_update` | yes | yes |
| 211 | `crypto_auth_hmacsha512256_verify` | yes | yes |
| 212 | `crypto_auth_hmacsha512_bytes` | yes | yes |
| 213 | `crypto_auth_hmacsha512_final` | yes | yes |
| 214 | `crypto_auth_hmacsha512_init` | yes | yes |
| 215 | `crypto_auth_hmacsha512_keybytes` | yes | yes |
| 216 | `crypto_auth_hmacsha512_keygen` | yes | yes |
| 217 | `crypto_auth_hmacsha512_statebytes` | yes | yes |
| 218 | `crypto_auth_hmacsha512_update` | yes | yes |
| 219 | `crypto_auth_hmacsha512_verify` | yes | yes |
| 220 | `crypto_auth_keybytes` | yes | yes |
| 221 | `crypto_auth_keygen` | yes | yes |
| 222 | `crypto_auth_primitive` | yes | yes |
| 223 | `crypto_auth_verify` | yes | yes |
| 224 | `crypto_box` | yes | yes |
| 225 | `crypto_box_afternm` | yes | yes |
| 226 | `crypto_box_beforenm` | yes | yes |
| 227 | `crypto_box_beforenmbytes` | yes | yes |
| 228 | `crypto_box_boxzerobytes` | yes | yes |
| 229 | `crypto_box_curve25519xchacha20poly1305_beforenm` | yes | yes |
| 230 | `crypto_box_curve25519xchacha20poly1305_beforenmbytes` | yes | yes |
| 231 | `crypto_box_curve25519xchacha20poly1305_detached` | yes | yes |
| 232 | `crypto_box_curve25519xchacha20poly1305_detached_afternm` | yes | yes |
| 233 | `crypto_box_curve25519xchacha20poly1305_easy` | yes | yes |
| 234 | `crypto_box_curve25519xchacha20poly1305_easy_afternm` | yes | yes |
| 235 | `crypto_box_curve25519xchacha20poly1305_keypair` | yes | yes |
| 236 | `crypto_box_curve25519xchacha20poly1305_macbytes` | yes | yes |
| 237 | `crypto_box_curve25519xchacha20poly1305_messagebytes_max` | yes | yes |
| 238 | `crypto_box_curve25519xchacha20poly1305_noncebytes` | yes | yes |
| 239 | `crypto_box_curve25519xchacha20poly1305_open_detached` | yes | yes |
| 240 | `crypto_box_curve25519xchacha20poly1305_open_detached_afternm` | yes | yes |
| 241 | `crypto_box_curve25519xchacha20poly1305_open_easy` | yes | yes |
| 242 | `crypto_box_curve25519xchacha20poly1305_open_easy_afternm` | yes | yes |
| 243 | `crypto_box_curve25519xchacha20poly1305_publickeybytes` | yes | yes |
| 244 | `crypto_box_curve25519xchacha20poly1305_seal` | yes | yes |
| 245 | `crypto_box_curve25519xchacha20poly1305_seal_open` | yes | yes |
| 246 | `crypto_box_curve25519xchacha20poly1305_sealbytes` | yes | yes |
| 247 | `crypto_box_curve25519xchacha20poly1305_secretkeybytes` | yes | yes |
| 248 | `crypto_box_curve25519xchacha20poly1305_seed_keypair` | yes | yes |
| 249 | `crypto_box_curve25519xchacha20poly1305_seedbytes` | yes | yes |
| 250 | `crypto_box_curve25519xsalsa20poly1305` | yes | yes |
| 251 | `crypto_box_curve25519xsalsa20poly1305_afternm` | yes | yes |
| 252 | `crypto_box_curve25519xsalsa20poly1305_beforenm` | yes | yes |
| 253 | `crypto_box_curve25519xsalsa20poly1305_beforenmbytes` | yes | yes |
| 254 | `crypto_box_curve25519xsalsa20poly1305_boxzerobytes` | yes | yes |
| 255 | `crypto_box_curve25519xsalsa20poly1305_keypair` | yes | yes |
| 256 | `crypto_box_curve25519xsalsa20poly1305_macbytes` | yes | yes |
| 257 | `crypto_box_curve25519xsalsa20poly1305_messagebytes_max` | yes | yes |
| 258 | `crypto_box_curve25519xsalsa20poly1305_noncebytes` | yes | yes |
| 259 | `crypto_box_curve25519xsalsa20poly1305_open` | yes | yes |
| 260 | `crypto_box_curve25519xsalsa20poly1305_open_afternm` | yes | yes |
| 261 | `crypto_box_curve25519xsalsa20poly1305_publickeybytes` | yes | yes |
| 262 | `crypto_box_curve25519xsalsa20poly1305_secretkeybytes` | yes | yes |
| 263 | `crypto_box_curve25519xsalsa20poly1305_seed_keypair` | yes | yes |
| 264 | `crypto_box_curve25519xsalsa20poly1305_seedbytes` | yes | yes |
| 265 | `crypto_box_curve25519xsalsa20poly1305_zerobytes` | yes | yes |
| 266 | `crypto_box_detached` | yes | yes |
| 267 | `crypto_box_detached_afternm` | yes | yes |
| 268 | `crypto_box_easy` | yes | yes |
| 269 | `crypto_box_easy_afternm` | yes | yes |
| 270 | `crypto_box_keypair` | yes | yes |
| 271 | `crypto_box_macbytes` | yes | yes |
| 272 | `crypto_box_messagebytes_max` | yes | yes |
| 273 | `crypto_box_noncebytes` | yes | yes |
| 274 | `crypto_box_open` | yes | yes |
| 275 | `crypto_box_open_afternm` | yes | yes |
| 276 | `crypto_box_open_detached` | yes | yes |
| 277 | `crypto_box_open_detached_afternm` | yes | yes |
| 278 | `crypto_box_open_easy` | yes | yes |
| 279 | `crypto_box_open_easy_afternm` | yes | yes |
| 280 | `crypto_box_primitive` | yes | yes |
| 281 | `crypto_box_publickeybytes` | yes | yes |
| 282 | `crypto_box_seal` | yes | yes |
| 283 | `crypto_box_seal_open` | yes | yes |
| 284 | `crypto_box_sealbytes` | yes | yes |
| 285 | `crypto_box_secretkeybytes` | yes | yes |
| 286 | `crypto_box_seed_keypair` | yes | yes |
| 287 | `crypto_box_seedbytes` | yes | yes |
| 288 | `crypto_box_zerobytes` | yes | yes |
| 289 | `crypto_core_ed25519_add` | yes | yes |
| 290 | `crypto_core_ed25519_bytes` | yes | yes |
| 291 | `crypto_core_ed25519_from_string` | yes | yes |
| 292 | `crypto_core_ed25519_from_string_nu` | yes | yes |
| 293 | `crypto_core_ed25519_hashbytes` | yes | yes |
| 294 | `crypto_core_ed25519_is_valid_point` | yes | yes |
| 295 | `crypto_core_ed25519_nonreducedscalarbytes` | yes | yes |
| 296 | `crypto_core_ed25519_random` | yes | yes |
| 297 | `crypto_core_ed25519_scalar_add` | yes | yes |
| 298 | `crypto_core_ed25519_scalar_complement` | yes | yes |
| 299 | `crypto_core_ed25519_scalar_from_string` | yes | yes |
| 300 | `crypto_core_ed25519_scalar_invert` | yes | yes |
| 301 | `crypto_core_ed25519_scalar_is_canonical` | yes | yes |
| 302 | `crypto_core_ed25519_scalar_mul` | yes | yes |
| 303 | `crypto_core_ed25519_scalar_negate` | yes | yes |
| 304 | `crypto_core_ed25519_scalar_random` | yes | yes |
| 305 | `crypto_core_ed25519_scalar_reduce` | yes | yes |
| 306 | `crypto_core_ed25519_scalar_sub` | yes | yes |
| 307 | `crypto_core_ed25519_scalarbytes` | yes | yes |
| 308 | `crypto_core_ed25519_sub` | yes | yes |
| 309 | `crypto_core_ed25519_uniformbytes` | yes | yes |
| 310 | `crypto_core_hchacha20` | yes | yes |
| 311 | `crypto_core_hchacha20_constbytes` | yes | yes |
| 312 | `crypto_core_hchacha20_inputbytes` | yes | yes |
| 313 | `crypto_core_hchacha20_keybytes` | yes | yes |
| 314 | `crypto_core_hchacha20_outputbytes` | yes | yes |
| 315 | `crypto_core_hsalsa20` | yes | yes |
| 316 | `crypto_core_hsalsa20_constbytes` | yes | yes |
| 317 | `crypto_core_hsalsa20_inputbytes` | yes | yes |
| 318 | `crypto_core_hsalsa20_keybytes` | yes | yes |
| 319 | `crypto_core_hsalsa20_outputbytes` | yes | yes |
| 320 | `crypto_core_keccak1600_extract_bytes` | yes | yes |
| 321 | `crypto_core_keccak1600_init` | yes | yes |
| 322 | `crypto_core_keccak1600_permute_12` | yes | yes |
| 323 | `crypto_core_keccak1600_permute_24` | yes | yes |
| 324 | `crypto_core_keccak1600_statebytes` | yes | yes |
| 325 | `crypto_core_keccak1600_xor_bytes` | yes | yes |
| 326 | `crypto_core_ristretto255_add` | yes | yes |
| 327 | `crypto_core_ristretto255_bytes` | yes | yes |
| 328 | `crypto_core_ristretto255_from_hash` | yes | yes |
| 329 | `crypto_core_ristretto255_from_string` | yes | yes |
| 330 | `crypto_core_ristretto255_hashbytes` | yes | yes |
| 331 | `crypto_core_ristretto255_is_valid_point` | yes | yes |
| 332 | `crypto_core_ristretto255_nonreducedscalarbytes` | yes | yes |
| 333 | `crypto_core_ristretto255_random` | yes | yes |
| 334 | `crypto_core_ristretto255_scalar_add` | yes | yes |
| 335 | `crypto_core_ristretto255_scalar_complement` | yes | yes |
| 336 | `crypto_core_ristretto255_scalar_from_string` | yes | yes |
| 337 | `crypto_core_ristretto255_scalar_invert` | yes | yes |
| 338 | `crypto_core_ristretto255_scalar_is_canonical` | yes | yes |
| 339 | `crypto_core_ristretto255_scalar_mul` | yes | yes |
| 340 | `crypto_core_ristretto255_scalar_negate` | yes | yes |
| 341 | `crypto_core_ristretto255_scalar_random` | yes | yes |
| 342 | `crypto_core_ristretto255_scalar_reduce` | yes | yes |
| 343 | `crypto_core_ristretto255_scalar_sub` | yes | yes |
| 344 | `crypto_core_ristretto255_scalarbytes` | yes | yes |
| 345 | `crypto_core_ristretto255_sub` | yes | yes |
| 346 | `crypto_core_salsa20` | yes | yes |
| 347 | `crypto_core_salsa2012` | yes | yes |
| 348 | `crypto_core_salsa2012_constbytes` | yes | yes |
| 349 | `crypto_core_salsa2012_inputbytes` | yes | yes |
| 350 | `crypto_core_salsa2012_keybytes` | yes | yes |
| 351 | `crypto_core_salsa2012_outputbytes` | yes | yes |
| 352 | `crypto_core_salsa208` | yes | yes |
| 353 | `crypto_core_salsa208_constbytes` | yes | yes |
| 354 | `crypto_core_salsa208_inputbytes` | yes | yes |
| 355 | `crypto_core_salsa208_keybytes` | yes | yes |
| 356 | `crypto_core_salsa208_outputbytes` | yes | yes |
| 357 | `crypto_core_salsa20_constbytes` | yes | yes |
| 358 | `crypto_core_salsa20_inputbytes` | yes | yes |
| 359 | `crypto_core_salsa20_keybytes` | yes | yes |
| 360 | `crypto_core_salsa20_outputbytes` | yes | yes |
| 361 | `crypto_generichash` | yes | yes |
| 362 | `crypto_generichash_blake2b` | yes | yes |
| 363 | `crypto_generichash_blake2b_bytes` | yes | yes |
| 364 | `crypto_generichash_blake2b_bytes_max` | yes | yes |
| 365 | `crypto_generichash_blake2b_bytes_min` | yes | yes |
| 366 | `crypto_generichash_blake2b_final` | yes | yes |
| 367 | `crypto_generichash_blake2b_init` | yes | yes |
| 368 | `crypto_generichash_blake2b_init_salt_personal` | yes | yes |
| 369 | `crypto_generichash_blake2b_keybytes` | yes | yes |
| 370 | `crypto_generichash_blake2b_keybytes_max` | yes | yes |
| 371 | `crypto_generichash_blake2b_keybytes_min` | yes | yes |
| 372 | `crypto_generichash_blake2b_keygen` | yes | yes |
| 373 | `crypto_generichash_blake2b_personalbytes` | yes | yes |
| 374 | `crypto_generichash_blake2b_salt_personal` | yes | yes |
| 375 | `crypto_generichash_blake2b_saltbytes` | yes | yes |
| 376 | `crypto_generichash_blake2b_statebytes` | yes | yes |
| 377 | `crypto_generichash_blake2b_update` | yes | yes |
| 378 | `crypto_generichash_bytes` | yes | yes |
| 379 | `crypto_generichash_bytes_max` | yes | yes |
| 380 | `crypto_generichash_bytes_min` | yes | yes |
| 381 | `crypto_generichash_final` | yes | yes |
| 382 | `crypto_generichash_init` | yes | yes |
| 383 | `crypto_generichash_keybytes` | yes | yes |
| 384 | `crypto_generichash_keybytes_max` | yes | yes |
| 385 | `crypto_generichash_keybytes_min` | yes | yes |
| 386 | `crypto_generichash_keygen` | yes | yes |
| 387 | `crypto_generichash_primitive` | yes | yes |
| 388 | `crypto_generichash_statebytes` | yes | yes |
| 389 | `crypto_generichash_update` | yes | yes |
| 390 | `crypto_hash` | yes | yes |
| 391 | `crypto_hash_bytes` | yes | yes |
| 392 | `crypto_hash_primitive` | yes | yes |
| 393 | `crypto_hash_sha256` | yes | yes |
| 394 | `crypto_hash_sha256_bytes` | yes | yes |
| 395 | `crypto_hash_sha256_final` | yes | yes |
| 396 | `crypto_hash_sha256_init` | yes | yes |
| 397 | `crypto_hash_sha256_statebytes` | yes | yes |
| 398 | `crypto_hash_sha256_update` | yes | yes |
| 399 | `crypto_hash_sha3256` | yes | yes |
| 400 | `crypto_hash_sha3256_bytes` | yes | yes |
| 401 | `crypto_hash_sha3256_final` | yes | yes |
| 402 | `crypto_hash_sha3256_init` | yes | yes |
| 403 | `crypto_hash_sha3256_statebytes` | yes | yes |
| 404 | `crypto_hash_sha3256_update` | yes | yes |
| 405 | `crypto_hash_sha3512` | yes | yes |
| 406 | `crypto_hash_sha3512_bytes` | yes | yes |
| 407 | `crypto_hash_sha3512_final` | yes | yes |
| 408 | `crypto_hash_sha3512_init` | yes | yes |
| 409 | `crypto_hash_sha3512_statebytes` | yes | yes |
| 410 | `crypto_hash_sha3512_update` | yes | yes |
| 411 | `crypto_hash_sha512` | yes | yes |
| 412 | `crypto_hash_sha512_bytes` | yes | yes |
| 413 | `crypto_hash_sha512_final` | yes | yes |
| 414 | `crypto_hash_sha512_init` | yes | yes |
| 415 | `crypto_hash_sha512_statebytes` | yes | yes |
| 416 | `crypto_hash_sha512_update` | yes | yes |
| 417 | `crypto_ipcrypt_bytes` | yes | yes |
| 418 | `crypto_ipcrypt_decrypt` | yes | yes |
| 419 | `crypto_ipcrypt_encrypt` | yes | yes |
| 420 | `crypto_ipcrypt_keybytes` | yes | yes |
| 421 | `crypto_ipcrypt_keygen` | yes | yes |
| 422 | `crypto_ipcrypt_nd_decrypt` | yes | yes |
| 423 | `crypto_ipcrypt_nd_encrypt` | yes | yes |
| 424 | `crypto_ipcrypt_nd_inputbytes` | yes | yes |
| 425 | `crypto_ipcrypt_nd_keybytes` | yes | yes |
| 426 | `crypto_ipcrypt_nd_keygen` | yes | yes |
| 427 | `crypto_ipcrypt_nd_outputbytes` | yes | yes |
| 428 | `crypto_ipcrypt_nd_tweakbytes` | yes | yes |
| 429 | `crypto_ipcrypt_ndx_decrypt` | yes | yes |
| 430 | `crypto_ipcrypt_ndx_encrypt` | yes | yes |
| 431 | `crypto_ipcrypt_ndx_inputbytes` | yes | yes |
| 432 | `crypto_ipcrypt_ndx_keybytes` | yes | yes |
| 433 | `crypto_ipcrypt_ndx_keygen` | yes | yes |
| 434 | `crypto_ipcrypt_ndx_outputbytes` | yes | yes |
| 435 | `crypto_ipcrypt_ndx_tweakbytes` | yes | yes |
| 436 | `crypto_ipcrypt_pfx_bytes` | yes | yes |
| 437 | `crypto_ipcrypt_pfx_decrypt` | yes | yes |
| 438 | `crypto_ipcrypt_pfx_encrypt` | yes | yes |
| 439 | `crypto_ipcrypt_pfx_keybytes` | yes | yes |
| 440 | `crypto_ipcrypt_pfx_keygen` | yes | yes |
| 441 | `crypto_kdf_blake2b_bytes_max` | yes | yes |
| 442 | `crypto_kdf_blake2b_bytes_min` | yes | yes |
| 443 | `crypto_kdf_blake2b_contextbytes` | yes | yes |
| 444 | `crypto_kdf_blake2b_derive_from_key` | yes | yes |
| 445 | `crypto_kdf_blake2b_keybytes` | yes | yes |
| 446 | `crypto_kdf_bytes_max` | yes | yes |
| 447 | `crypto_kdf_bytes_min` | yes | yes |
| 448 | `crypto_kdf_contextbytes` | yes | yes |
| 449 | `crypto_kdf_derive_from_key` | yes | yes |
| 450 | `crypto_kdf_hkdf_sha256_bytes_max` | yes | yes |
| 451 | `crypto_kdf_hkdf_sha256_bytes_min` | yes | yes |
| 452 | `crypto_kdf_hkdf_sha256_expand` | yes | yes |
| 453 | `crypto_kdf_hkdf_sha256_extract` | yes | yes |
| 454 | `crypto_kdf_hkdf_sha256_extract_final` | yes | yes |
| 455 | `crypto_kdf_hkdf_sha256_extract_init` | yes | yes |
| 456 | `crypto_kdf_hkdf_sha256_extract_update` | yes | yes |
| 457 | `crypto_kdf_hkdf_sha256_keybytes` | yes | yes |
| 458 | `crypto_kdf_hkdf_sha256_keygen` | yes | yes |
| 459 | `crypto_kdf_hkdf_sha256_statebytes` | yes | yes |
| 460 | `crypto_kdf_hkdf_sha512_bytes_max` | yes | yes |
| 461 | `crypto_kdf_hkdf_sha512_bytes_min` | yes | yes |
| 462 | `crypto_kdf_hkdf_sha512_expand` | yes | yes |
| 463 | `crypto_kdf_hkdf_sha512_extract` | yes | yes |
| 464 | `crypto_kdf_hkdf_sha512_extract_final` | yes | yes |
| 465 | `crypto_kdf_hkdf_sha512_extract_init` | yes | yes |
| 466 | `crypto_kdf_hkdf_sha512_extract_update` | yes | yes |
| 467 | `crypto_kdf_hkdf_sha512_keybytes` | yes | yes |
| 468 | `crypto_kdf_hkdf_sha512_keygen` | yes | yes |
| 469 | `crypto_kdf_hkdf_sha512_statebytes` | yes | yes |
| 470 | `crypto_kdf_keybytes` | yes | yes |
| 471 | `crypto_kdf_keygen` | yes | yes |
| 472 | `crypto_kdf_primitive` | yes | yes |
| 473 | `crypto_kem_ciphertextbytes` | yes | yes |
| 474 | `crypto_kem_dec` | yes | yes |
| 475 | `crypto_kem_enc` | yes | yes |
| 476 | `crypto_kem_keypair` | yes | yes |
| 477 | `crypto_kem_mlkem768_ciphertextbytes` | yes | yes |
| 478 | `crypto_kem_mlkem768_dec` | yes | yes |
| 479 | `crypto_kem_mlkem768_enc` | yes | yes |
| 480 | `crypto_kem_mlkem768_enc_deterministic` | yes | yes |
| 481 | `crypto_kem_mlkem768_keypair` | yes | yes |
| 482 | `crypto_kem_mlkem768_publickeybytes` | yes | yes |
| 483 | `crypto_kem_mlkem768_secretkeybytes` | yes | yes |
| 484 | `crypto_kem_mlkem768_seed_keypair` | yes | yes |
| 485 | `crypto_kem_mlkem768_seedbytes` | yes | yes |
| 486 | `crypto_kem_mlkem768_sharedsecretbytes` | yes | yes |
| 487 | `crypto_kem_primitive` | yes | yes |
| 488 | `crypto_kem_publickeybytes` | yes | yes |
| 489 | `crypto_kem_secretkeybytes` | yes | yes |
| 490 | `crypto_kem_seed_keypair` | yes | yes |
| 491 | `crypto_kem_seedbytes` | yes | yes |
| 492 | `crypto_kem_sharedsecretbytes` | yes | yes |
| 493 | `crypto_kem_xwing_ciphertextbytes` | yes | yes |
| 494 | `crypto_kem_xwing_dec` | yes | yes |
| 495 | `crypto_kem_xwing_enc` | yes | yes |
| 496 | `crypto_kem_xwing_enc_deterministic` | yes | yes |
| 497 | `crypto_kem_xwing_keypair` | yes | yes |
| 498 | `crypto_kem_xwing_publickeybytes` | yes | yes |
| 499 | `crypto_kem_xwing_secretkeybytes` | yes | yes |
| 500 | `crypto_kem_xwing_seed_keypair` | yes | yes |
| 501 | `crypto_kem_xwing_seedbytes` | yes | yes |
| 502 | `crypto_kem_xwing_sharedsecretbytes` | yes | yes |
| 503 | `crypto_kx_client_session_keys` | yes | yes |
| 504 | `crypto_kx_keypair` | yes | yes |
| 505 | `crypto_kx_primitive` | yes | yes |
| 506 | `crypto_kx_publickeybytes` | yes | yes |
| 507 | `crypto_kx_secretkeybytes` | yes | yes |
| 508 | `crypto_kx_seed_keypair` | yes | yes |
| 509 | `crypto_kx_seedbytes` | yes | yes |
| 510 | `crypto_kx_server_session_keys` | yes | yes |
| 511 | `crypto_kx_sessionkeybytes` | yes | yes |
| 512 | `crypto_onetimeauth` | yes | yes |
| 513 | `crypto_onetimeauth_bytes` | yes | yes |
| 514 | `crypto_onetimeauth_final` | yes | yes |
| 515 | `crypto_onetimeauth_init` | yes | yes |
| 516 | `crypto_onetimeauth_keybytes` | yes | yes |
| 517 | `crypto_onetimeauth_keygen` | yes | yes |
| 518 | `crypto_onetimeauth_poly1305` | yes | yes |
| 519 | `crypto_onetimeauth_poly1305_bytes` | yes | yes |
| 520 | `crypto_onetimeauth_poly1305_donna_implementation` | yes | yes |
| 521 | `crypto_onetimeauth_poly1305_final` | yes | yes |
| 522 | `crypto_onetimeauth_poly1305_init` | yes | yes |
| 523 | `crypto_onetimeauth_poly1305_keybytes` | yes | yes |
| 524 | `crypto_onetimeauth_poly1305_keygen` | yes | yes |
| 525 | `crypto_onetimeauth_poly1305_statebytes` | yes | yes |
| 526 | `crypto_onetimeauth_poly1305_update` | yes | yes |
| 527 | `crypto_onetimeauth_poly1305_verify` | yes | yes |
| 528 | `crypto_onetimeauth_primitive` | yes | yes |
| 529 | `crypto_onetimeauth_statebytes` | yes | yes |
| 530 | `crypto_onetimeauth_update` | yes | yes |
| 531 | `crypto_onetimeauth_verify` | yes | yes |
| 532 | `crypto_pwhash` | yes | yes |
| 533 | `crypto_pwhash_alg_argon2i13` | yes | yes |
| 534 | `crypto_pwhash_alg_argon2id13` | yes | yes |
| 535 | `crypto_pwhash_alg_default` | yes | yes |
| 536 | `crypto_pwhash_argon2i` | yes | yes |
| 537 | `crypto_pwhash_argon2i_alg_argon2i13` | yes | yes |
| 538 | `crypto_pwhash_argon2i_bytes_max` | yes | yes |
| 539 | `crypto_pwhash_argon2i_bytes_min` | yes | yes |
| 540 | `crypto_pwhash_argon2i_memlimit_interactive` | yes | yes |
| 541 | `crypto_pwhash_argon2i_memlimit_max` | yes | yes |
| 542 | `crypto_pwhash_argon2i_memlimit_min` | yes | yes |
| 543 | `crypto_pwhash_argon2i_memlimit_moderate` | yes | yes |
| 544 | `crypto_pwhash_argon2i_memlimit_sensitive` | yes | yes |
| 545 | `crypto_pwhash_argon2i_opslimit_interactive` | yes | yes |
| 546 | `crypto_pwhash_argon2i_opslimit_max` | yes | yes |
| 547 | `crypto_pwhash_argon2i_opslimit_min` | yes | yes |
| 548 | `crypto_pwhash_argon2i_opslimit_moderate` | yes | yes |
| 549 | `crypto_pwhash_argon2i_opslimit_sensitive` | yes | yes |
| 550 | `crypto_pwhash_argon2i_passwd_max` | yes | yes |
| 551 | `crypto_pwhash_argon2i_passwd_min` | yes | yes |
| 552 | `crypto_pwhash_argon2i_saltbytes` | yes | yes |
| 553 | `crypto_pwhash_argon2i_str` | yes | yes |
| 554 | `crypto_pwhash_argon2i_str_needs_rehash` | yes | yes |
| 555 | `crypto_pwhash_argon2i_str_verify` | yes | yes |
| 556 | `crypto_pwhash_argon2i_strbytes` | yes | yes |
| 557 | `crypto_pwhash_argon2i_strprefix` | yes | yes |
| 558 | `crypto_pwhash_argon2id` | yes | yes |
| 559 | `crypto_pwhash_argon2id_alg_argon2id13` | yes | yes |
| 560 | `crypto_pwhash_argon2id_bytes_max` | yes | yes |
| 561 | `crypto_pwhash_argon2id_bytes_min` | yes | yes |
| 562 | `crypto_pwhash_argon2id_memlimit_interactive` | yes | yes |
| 563 | `crypto_pwhash_argon2id_memlimit_max` | yes | yes |
| 564 | `crypto_pwhash_argon2id_memlimit_min` | yes | yes |
| 565 | `crypto_pwhash_argon2id_memlimit_moderate` | yes | yes |
| 566 | `crypto_pwhash_argon2id_memlimit_sensitive` | yes | yes |
| 567 | `crypto_pwhash_argon2id_opslimit_interactive` | yes | yes |
| 568 | `crypto_pwhash_argon2id_opslimit_max` | yes | yes |
| 569 | `crypto_pwhash_argon2id_opslimit_min` | yes | yes |
| 570 | `crypto_pwhash_argon2id_opslimit_moderate` | yes | yes |
| 571 | `crypto_pwhash_argon2id_opslimit_sensitive` | yes | yes |
| 572 | `crypto_pwhash_argon2id_passwd_max` | yes | yes |
| 573 | `crypto_pwhash_argon2id_passwd_min` | yes | yes |
| 574 | `crypto_pwhash_argon2id_saltbytes` | yes | yes |
| 575 | `crypto_pwhash_argon2id_str` | yes | yes |
| 576 | `crypto_pwhash_argon2id_str_needs_rehash` | yes | yes |
| 577 | `crypto_pwhash_argon2id_str_verify` | yes | yes |
| 578 | `crypto_pwhash_argon2id_strbytes` | yes | yes |
| 579 | `crypto_pwhash_argon2id_strprefix` | yes | yes |
| 580 | `crypto_pwhash_bytes_max` | yes | yes |
| 581 | `crypto_pwhash_bytes_min` | yes | yes |
| 582 | `crypto_pwhash_memlimit_interactive` | yes | yes |
| 583 | `crypto_pwhash_memlimit_max` | yes | yes |
| 584 | `crypto_pwhash_memlimit_min` | yes | yes |
| 585 | `crypto_pwhash_memlimit_moderate` | yes | yes |
| 586 | `crypto_pwhash_memlimit_sensitive` | yes | yes |
| 587 | `crypto_pwhash_opslimit_interactive` | yes | yes |
| 588 | `crypto_pwhash_opslimit_max` | yes | yes |
| 589 | `crypto_pwhash_opslimit_min` | yes | yes |
| 590 | `crypto_pwhash_opslimit_moderate` | yes | yes |
| 591 | `crypto_pwhash_opslimit_sensitive` | yes | yes |
| 592 | `crypto_pwhash_passwd_max` | yes | yes |
| 593 | `crypto_pwhash_passwd_min` | yes | yes |
| 594 | `crypto_pwhash_primitive` | yes | yes |
| 595 | `crypto_pwhash_saltbytes` | yes | yes |
| 596 | `crypto_pwhash_scryptsalsa208sha256` | yes | yes |
| 597 | `crypto_pwhash_scryptsalsa208sha256_bytes_max` | yes | yes |
| 598 | `crypto_pwhash_scryptsalsa208sha256_bytes_min` | yes | yes |
| 599 | `crypto_pwhash_scryptsalsa208sha256_ll` | yes | yes |
| 600 | `crypto_pwhash_scryptsalsa208sha256_memlimit_interactive` | yes | yes |
| 601 | `crypto_pwhash_scryptsalsa208sha256_memlimit_max` | yes | yes |
| 602 | `crypto_pwhash_scryptsalsa208sha256_memlimit_min` | yes | yes |
| 603 | `crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive` | yes | yes |
| 604 | `crypto_pwhash_scryptsalsa208sha256_opslimit_interactive` | yes | yes |
| 605 | `crypto_pwhash_scryptsalsa208sha256_opslimit_max` | yes | yes |
| 606 | `crypto_pwhash_scryptsalsa208sha256_opslimit_min` | yes | yes |
| 607 | `crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive` | yes | yes |
| 608 | `crypto_pwhash_scryptsalsa208sha256_passwd_max` | yes | yes |
| 609 | `crypto_pwhash_scryptsalsa208sha256_passwd_min` | yes | yes |
| 610 | `crypto_pwhash_scryptsalsa208sha256_saltbytes` | yes | yes |
| 611 | `crypto_pwhash_scryptsalsa208sha256_str` | yes | yes |
| 612 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | yes | yes |
| 613 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | yes | yes |
| 614 | `crypto_pwhash_scryptsalsa208sha256_strbytes` | yes | yes |
| 615 | `crypto_pwhash_scryptsalsa208sha256_strprefix` | yes | yes |
| 616 | `crypto_pwhash_str` | yes | yes |
| 617 | `crypto_pwhash_str_alg` | yes | yes |
| 618 | `crypto_pwhash_str_needs_rehash` | yes | yes |
| 619 | `crypto_pwhash_str_verify` | yes | yes |
| 620 | `crypto_pwhash_strbytes` | yes | yes |
| 621 | `crypto_pwhash_strprefix` | yes | yes |
| 622 | `crypto_scalarmult` | yes | yes |
| 623 | `crypto_scalarmult_base` | yes | yes |
| 624 | `crypto_scalarmult_bytes` | yes | yes |
| 625 | `crypto_scalarmult_curve25519` | yes | yes |
| 626 | `crypto_scalarmult_curve25519_base` | yes | yes |
| 627 | `crypto_scalarmult_curve25519_bytes` | yes | yes |
| 628 | `crypto_scalarmult_curve25519_ref10_implementation` | yes | yes |
| 629 | `crypto_scalarmult_curve25519_scalarbytes` | yes | yes |
| 630 | `crypto_scalarmult_ed25519` | yes | yes |
| 631 | `crypto_scalarmult_ed25519_base` | yes | yes |
| 632 | `crypto_scalarmult_ed25519_base_noclamp` | yes | yes |
| 633 | `crypto_scalarmult_ed25519_bytes` | yes | yes |
| 634 | `crypto_scalarmult_ed25519_noclamp` | yes | yes |
| 635 | `crypto_scalarmult_ed25519_scalarbytes` | yes | yes |
| 636 | `crypto_scalarmult_primitive` | yes | yes |
| 637 | `crypto_scalarmult_ristretto255` | yes | yes |
| 638 | `crypto_scalarmult_ristretto255_base` | yes | yes |
| 639 | `crypto_scalarmult_ristretto255_bytes` | yes | yes |
| 640 | `crypto_scalarmult_ristretto255_scalarbytes` | yes | yes |
| 641 | `crypto_scalarmult_scalarbytes` | yes | yes |
| 642 | `crypto_secretbox` | yes | yes |
| 643 | `crypto_secretbox_boxzerobytes` | yes | yes |
| 644 | `crypto_secretbox_detached` | yes | yes |
| 645 | `crypto_secretbox_easy` | yes | yes |
| 646 | `crypto_secretbox_keybytes` | yes | yes |
| 647 | `crypto_secretbox_keygen` | yes | yes |
| 648 | `crypto_secretbox_macbytes` | yes | yes |
| 649 | `crypto_secretbox_messagebytes_max` | yes | yes |
| 650 | `crypto_secretbox_noncebytes` | yes | yes |
| 651 | `crypto_secretbox_open` | yes | yes |
| 652 | `crypto_secretbox_open_detached` | yes | yes |
| 653 | `crypto_secretbox_open_easy` | yes | yes |
| 654 | `crypto_secretbox_primitive` | yes | yes |
| 655 | `crypto_secretbox_xchacha20poly1305_detached` | yes | yes |
| 656 | `crypto_secretbox_xchacha20poly1305_easy` | yes | yes |
| 657 | `crypto_secretbox_xchacha20poly1305_keybytes` | yes | yes |
| 658 | `crypto_secretbox_xchacha20poly1305_macbytes` | yes | yes |
| 659 | `crypto_secretbox_xchacha20poly1305_messagebytes_max` | yes | yes |
| 660 | `crypto_secretbox_xchacha20poly1305_noncebytes` | yes | yes |
| 661 | `crypto_secretbox_xchacha20poly1305_open_detached` | yes | yes |
| 662 | `crypto_secretbox_xchacha20poly1305_open_easy` | yes | yes |
| 663 | `crypto_secretbox_xsalsa20poly1305` | yes | yes |
| 664 | `crypto_secretbox_xsalsa20poly1305_boxzerobytes` | yes | yes |
| 665 | `crypto_secretbox_xsalsa20poly1305_keybytes` | yes | yes |
| 666 | `crypto_secretbox_xsalsa20poly1305_keygen` | yes | yes |
| 667 | `crypto_secretbox_xsalsa20poly1305_macbytes` | yes | yes |
| 668 | `crypto_secretbox_xsalsa20poly1305_messagebytes_max` | yes | yes |
| 669 | `crypto_secretbox_xsalsa20poly1305_noncebytes` | yes | yes |
| 670 | `crypto_secretbox_xsalsa20poly1305_open` | yes | yes |
| 671 | `crypto_secretbox_xsalsa20poly1305_zerobytes` | yes | yes |
| 672 | `crypto_secretbox_zerobytes` | yes | yes |
| 673 | `crypto_secretstream_xchacha20poly1305_abytes` | yes | yes |
| 674 | `crypto_secretstream_xchacha20poly1305_headerbytes` | yes | yes |
| 675 | `crypto_secretstream_xchacha20poly1305_init_pull` | yes | yes |
| 676 | `crypto_secretstream_xchacha20poly1305_init_push` | yes | yes |
| 677 | `crypto_secretstream_xchacha20poly1305_keybytes` | yes | yes |
| 678 | `crypto_secretstream_xchacha20poly1305_keygen` | yes | yes |
| 679 | `crypto_secretstream_xchacha20poly1305_messagebytes_max` | yes | yes |
| 680 | `crypto_secretstream_xchacha20poly1305_pull` | yes | yes |
| 681 | `crypto_secretstream_xchacha20poly1305_push` | yes | yes |
| 682 | `crypto_secretstream_xchacha20poly1305_rekey` | yes | yes |
| 683 | `crypto_secretstream_xchacha20poly1305_statebytes` | yes | yes |
| 684 | `crypto_secretstream_xchacha20poly1305_tag_final` | yes | yes |
| 685 | `crypto_secretstream_xchacha20poly1305_tag_message` | yes | yes |
| 686 | `crypto_secretstream_xchacha20poly1305_tag_push` | yes | yes |
| 687 | `crypto_secretstream_xchacha20poly1305_tag_rekey` | yes | yes |
| 688 | `crypto_shorthash` | yes | yes |
| 689 | `crypto_shorthash_bytes` | yes | yes |
| 690 | `crypto_shorthash_keybytes` | yes | yes |
| 691 | `crypto_shorthash_keygen` | yes | yes |
| 692 | `crypto_shorthash_primitive` | yes | yes |
| 693 | `crypto_shorthash_siphash24` | yes | yes |
| 694 | `crypto_shorthash_siphash24_bytes` | yes | yes |
| 695 | `crypto_shorthash_siphash24_keybytes` | yes | yes |
| 696 | `crypto_shorthash_siphashx24` | yes | yes |
| 697 | `crypto_shorthash_siphashx24_bytes` | yes | yes |
| 698 | `crypto_shorthash_siphashx24_keybytes` | yes | yes |
| 699 | `crypto_sign` | yes | yes |
| 700 | `crypto_sign_bytes` | yes | yes |
| 701 | `crypto_sign_detached` | yes | yes |
| 702 | `crypto_sign_ed25519` | yes | yes |
| 703 | `crypto_sign_ed25519_bytes` | yes | yes |
| 704 | `crypto_sign_ed25519_detached` | yes | yes |
| 705 | `crypto_sign_ed25519_keypair` | yes | yes |
| 706 | `crypto_sign_ed25519_messagebytes_max` | yes | yes |
| 707 | `crypto_sign_ed25519_open` | yes | yes |
| 708 | `crypto_sign_ed25519_pk_to_curve25519` | yes | yes |
| 709 | `crypto_sign_ed25519_publickeybytes` | yes | yes |
| 710 | `crypto_sign_ed25519_secretkeybytes` | yes | yes |
| 711 | `crypto_sign_ed25519_seed_keypair` | yes | yes |
| 712 | `crypto_sign_ed25519_seedbytes` | yes | yes |
| 713 | `crypto_sign_ed25519_sk_to_curve25519` | yes | yes |
| 714 | `crypto_sign_ed25519_sk_to_pk` | yes | yes |
| 715 | `crypto_sign_ed25519_sk_to_seed` | yes | yes |
| 716 | `crypto_sign_ed25519_verify_detached` | yes | yes |
| 717 | `crypto_sign_ed25519ph_final_create` | yes | yes |
| 718 | `crypto_sign_ed25519ph_final_verify` | yes | yes |
| 719 | `crypto_sign_ed25519ph_init` | yes | yes |
| 720 | `crypto_sign_ed25519ph_statebytes` | yes | yes |
| 721 | `crypto_sign_ed25519ph_update` | yes | yes |
| 722 | `crypto_sign_final_create` | yes | yes |
| 723 | `crypto_sign_final_verify` | yes | yes |
| 724 | `crypto_sign_init` | yes | yes |
| 725 | `crypto_sign_keypair` | yes | yes |
| 726 | `crypto_sign_messagebytes_max` | yes | yes |
| 727 | `crypto_sign_open` | yes | yes |
| 728 | `crypto_sign_primitive` | yes | yes |
| 729 | `crypto_sign_publickeybytes` | yes | yes |
| 730 | `crypto_sign_secretkeybytes` | yes | yes |
| 731 | `crypto_sign_seed_keypair` | yes | yes |
| 732 | `crypto_sign_seedbytes` | yes | yes |
| 733 | `crypto_sign_statebytes` | yes | yes |
| 734 | `crypto_sign_update` | yes | yes |
| 735 | `crypto_sign_verify_detached` | yes | yes |
| 736 | `crypto_stream` | yes | yes |
| 737 | `crypto_stream_chacha20` | yes | yes |
| 738 | `crypto_stream_chacha20_ietf` | yes | yes |
| 739 | `crypto_stream_chacha20_ietf_ext` | yes | yes |
| 740 | `crypto_stream_chacha20_ietf_ext_xor_ic` | yes | yes |
| 741 | `crypto_stream_chacha20_ietf_keybytes` | yes | yes |
| 742 | `crypto_stream_chacha20_ietf_keygen` | yes | yes |
| 743 | `crypto_stream_chacha20_ietf_messagebytes_max` | yes | yes |
| 744 | `crypto_stream_chacha20_ietf_noncebytes` | yes | yes |
| 745 | `crypto_stream_chacha20_ietf_xor` | yes | yes |
| 746 | `crypto_stream_chacha20_ietf_xor_ic` | yes | yes |
| 747 | `crypto_stream_chacha20_keybytes` | yes | yes |
| 748 | `crypto_stream_chacha20_keygen` | yes | yes |
| 749 | `crypto_stream_chacha20_messagebytes_max` | yes | yes |
| 750 | `crypto_stream_chacha20_noncebytes` | yes | yes |
| 751 | `crypto_stream_chacha20_ref_implementation` | yes | yes |
| 752 | `crypto_stream_chacha20_xor` | yes | yes |
| 753 | `crypto_stream_chacha20_xor_ic` | yes | yes |
| 754 | `crypto_stream_keybytes` | yes | yes |
| 755 | `crypto_stream_keygen` | yes | yes |
| 756 | `crypto_stream_messagebytes_max` | yes | yes |
| 757 | `crypto_stream_noncebytes` | yes | yes |
| 758 | `crypto_stream_primitive` | yes | yes |
| 759 | `crypto_stream_salsa20` | yes | yes |
| 760 | `crypto_stream_salsa2012` | yes | yes |
| 761 | `crypto_stream_salsa2012_keybytes` | yes | yes |
| 762 | `crypto_stream_salsa2012_keygen` | yes | yes |
| 763 | `crypto_stream_salsa2012_messagebytes_max` | yes | yes |
| 764 | `crypto_stream_salsa2012_noncebytes` | yes | yes |
| 765 | `crypto_stream_salsa2012_xor` | yes | yes |
| 766 | `crypto_stream_salsa208` | yes | yes |
| 767 | `crypto_stream_salsa208_keybytes` | yes | yes |
| 768 | `crypto_stream_salsa208_keygen` | yes | yes |
| 769 | `crypto_stream_salsa208_messagebytes_max` | yes | yes |
| 770 | `crypto_stream_salsa208_noncebytes` | yes | yes |
| 771 | `crypto_stream_salsa208_xor` | yes | yes |
| 772 | `crypto_stream_salsa20_keybytes` | yes | yes |
| 773 | `crypto_stream_salsa20_keygen` | yes | yes |
| 774 | `crypto_stream_salsa20_messagebytes_max` | yes | yes |
| 775 | `crypto_stream_salsa20_noncebytes` | yes | yes |
| 776 | `crypto_stream_salsa20_ref_implementation` | yes | yes |
| 777 | `crypto_stream_salsa20_xor` | yes | yes |
| 778 | `crypto_stream_salsa20_xor_ic` | yes | yes |
| 779 | `crypto_stream_xchacha20` | yes | yes |
| 780 | `crypto_stream_xchacha20_keybytes` | yes | yes |
| 781 | `crypto_stream_xchacha20_keygen` | yes | yes |
| 782 | `crypto_stream_xchacha20_messagebytes_max` | yes | yes |
| 783 | `crypto_stream_xchacha20_noncebytes` | yes | yes |
| 784 | `crypto_stream_xchacha20_xor` | yes | yes |
| 785 | `crypto_stream_xchacha20_xor_ic` | yes | yes |
| 786 | `crypto_stream_xor` | yes | yes |
| 787 | `crypto_stream_xsalsa20` | yes | yes |
| 788 | `crypto_stream_xsalsa20_keybytes` | yes | yes |
| 789 | `crypto_stream_xsalsa20_keygen` | yes | yes |
| 790 | `crypto_stream_xsalsa20_messagebytes_max` | yes | yes |
| 791 | `crypto_stream_xsalsa20_noncebytes` | yes | yes |
| 792 | `crypto_stream_xsalsa20_xor` | yes | yes |
| 793 | `crypto_stream_xsalsa20_xor_ic` | yes | yes |
| 794 | `crypto_verify_16` | yes | yes |
| 795 | `crypto_verify_16_bytes` | yes | yes |
| 796 | `crypto_verify_32` | yes | yes |
| 797 | `crypto_verify_32_bytes` | yes | yes |
| 798 | `crypto_verify_64` | yes | yes |
| 799 | `crypto_verify_64_bytes` | yes | yes |
| 800 | `crypto_xof_shake128` | yes | yes |
| 801 | `crypto_xof_shake128_blockbytes` | yes | yes |
| 802 | `crypto_xof_shake128_domain_standard` | yes | yes |
| 803 | `crypto_xof_shake128_init` | yes | yes |
| 804 | `crypto_xof_shake128_init_with_domain` | yes | yes |
| 805 | `crypto_xof_shake128_squeeze` | yes | yes |
| 806 | `crypto_xof_shake128_statebytes` | yes | yes |
| 807 | `crypto_xof_shake128_update` | yes | yes |
| 808 | `crypto_xof_shake256` | yes | yes |
| 809 | `crypto_xof_shake256_blockbytes` | yes | yes |
| 810 | `crypto_xof_shake256_domain_standard` | yes | yes |
| 811 | `crypto_xof_shake256_init` | yes | yes |
| 812 | `crypto_xof_shake256_init_with_domain` | yes | yes |
| 813 | `crypto_xof_shake256_squeeze` | yes | yes |
| 814 | `crypto_xof_shake256_statebytes` | yes | yes |
| 815 | `crypto_xof_shake256_update` | yes | yes |
| 816 | `crypto_xof_turboshake128` | yes | yes |
| 817 | `crypto_xof_turboshake128_blockbytes` | yes | yes |
| 818 | `crypto_xof_turboshake128_domain_standard` | yes | yes |
| 819 | `crypto_xof_turboshake128_init` | yes | yes |
| 820 | `crypto_xof_turboshake128_init_with_domain` | yes | yes |
| 821 | `crypto_xof_turboshake128_squeeze` | yes | yes |
| 822 | `crypto_xof_turboshake128_statebytes` | yes | yes |
| 823 | `crypto_xof_turboshake128_update` | yes | yes |
| 824 | `crypto_xof_turboshake256` | yes | yes |
| 825 | `crypto_xof_turboshake256_blockbytes` | yes | yes |
| 826 | `crypto_xof_turboshake256_domain_standard` | yes | yes |
| 827 | `crypto_xof_turboshake256_init` | yes | yes |
| 828 | `crypto_xof_turboshake256_init_with_domain` | yes | yes |
| 829 | `crypto_xof_turboshake256_squeeze` | yes | yes |
| 830 | `crypto_xof_turboshake256_statebytes` | yes | yes |
| 831 | `crypto_xof_turboshake256_update` | yes | yes |
| 832 | `ipcrypt_soft_implementation` | yes | yes |
| 833 | `randombytes` | yes | yes |
| 834 | `randombytes_buf` | yes | yes |
| 835 | `randombytes_buf_deterministic` | yes | yes |
| 836 | `randombytes_close` | yes | yes |
| 837 | `randombytes_implementation_name` | yes | yes |
| 838 | `randombytes_internal_implementation` | yes | yes |
| 839 | `randombytes_random` | yes | yes |
| 840 | `randombytes_seedbytes` | yes | yes |
| 841 | `randombytes_set_implementation` | yes | yes |
| 842 | `randombytes_stir` | yes | yes |
| 843 | `randombytes_sysrandom_implementation` | yes | yes |
| 844 | `randombytes_uniform` | yes | yes |
| 845 | `sodium_add` | yes | yes |
| 846 | `sodium_allocarray` | yes | yes |
| 847 | `sodium_base642bin` | yes | yes |
| 848 | `sodium_base64_encoded_len` | yes | yes |
| 849 | `sodium_bin2base64` | yes | yes |
| 850 | `sodium_bin2hex` | yes | yes |
| 851 | `sodium_bin2ip` | yes | yes |
| 852 | `sodium_compare` | yes | yes |
| 853 | `sodium_crit_enter` | yes | yes |
| 854 | `sodium_crit_leave` | yes | yes |
| 855 | `sodium_free` | yes | yes |
| 856 | `sodium_hex2bin` | yes | yes |
| 857 | `sodium_increment` | yes | yes |
| 858 | `sodium_init` | yes | yes |
| 859 | `sodium_ip2bin` | yes | yes |
| 860 | `sodium_is_zero` | yes | yes |
| 861 | `sodium_library_minimal` | yes | yes |
| 862 | `sodium_library_version_major` | yes | yes |
| 863 | `sodium_library_version_minor` | yes | yes |
| 864 | `sodium_malloc` | yes | yes |
| 865 | `sodium_memcmp` | yes | yes |
| 866 | `sodium_memzero` | yes | yes |
| 867 | `sodium_misuse` | yes | yes |
| 868 | `sodium_mlock` | yes | yes |
| 869 | `sodium_mprotect_noaccess` | yes | yes |
| 870 | `sodium_mprotect_readonly` | yes | yes |
| 871 | `sodium_mprotect_readwrite` | yes | yes |
| 872 | `sodium_munlock` | yes | yes |
| 873 | `sodium_pad` | yes | yes |
| 874 | `sodium_runtime_has_aesni` | yes | yes |
| 875 | `sodium_runtime_has_armcrypto` | yes | yes |
| 876 | `sodium_runtime_has_avx` | yes | yes |
| 877 | `sodium_runtime_has_avx2` | yes | yes |
| 878 | `sodium_runtime_has_avx512f` | yes | yes |
| 879 | `sodium_runtime_has_neon` | yes | yes |
| 880 | `sodium_runtime_has_pclmul` | yes | yes |
| 881 | `sodium_runtime_has_rdrand` | yes | yes |
| 882 | `sodium_runtime_has_sse2` | yes | yes |
| 883 | `sodium_runtime_has_sse3` | yes | yes |
| 884 | `sodium_runtime_has_sse41` | yes | yes |
| 885 | `sodium_runtime_has_ssse3` | yes | yes |
| 886 | `sodium_set_misuse_handler` | yes | yes |
| 887 | `sodium_stackzero` | yes | yes |
| 888 | `sodium_sub` | yes | yes |
| 889 | `sodium_unpad` | yes | yes |
| 890 | `sodium_version_string` | yes | yes |
