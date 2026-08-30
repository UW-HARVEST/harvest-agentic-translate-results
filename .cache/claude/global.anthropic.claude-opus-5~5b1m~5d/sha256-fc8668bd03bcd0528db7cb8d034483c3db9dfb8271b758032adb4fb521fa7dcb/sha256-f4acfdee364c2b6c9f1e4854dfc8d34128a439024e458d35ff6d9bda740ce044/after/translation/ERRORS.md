# ERRORS.md — error / rejection surface table (Phase A -> Phase C)

Derived mechanically from the C sources by grepping for every `return -1`,
every `RNG_BAD_*` / `RNG_SUCCESS` return, every `#error`, every `assert`,
every explicit range / NULL check, and every min/max constant:

```
grep -rn -e "return -1" -e "return NULL" -e "assert" -e "RNG_BAD" \
        -e "RNG_SUCCESS" -e "#error" -e "abort()" -e "== NULL" -e "!= NULL" c_src
```

Every row has a differential test in `tests/phase_c_errors.rs`
(`SPX_BYTES` = `crypto_sign_bytes()`, `RNG_SUCCESS`=0, `RNG_BAD_MAXLEN`=-1,
`RNG_BAD_OUTBUF`=-2, `RNG_BAD_REQ_LEN`=-3).

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `crypto_sign_verify` | `siglen != SPX_BYTES` — `sign.c:179` | returns `-1`, no output written | `e01_verify_siglen_mismatch` | [x] |
| 2  | `crypto_sign_verify` | `siglen == SPX_BYTES` but recomputed root `!=` `pk+SPX_N` (corrupted sig / wrong pk / wrong msg) — `sign.c:235` | returns `-1` | `e02_verify_root_mismatch` | [x] |
| 3  | `crypto_sign_verify` | valid signature | returns `0` | `e03_verify_ok` | [x] |
| 4  | `crypto_sign_open` | `smlen < SPX_BYTES` — `sign.c:269` | `memset(m,0,smlen)`, `*mlen = 0`, returns `-1` | `e04_open_smlen_too_small` | [x] |
| 5  | `crypto_sign_open` | `smlen == 0` (boundary of row 4) | `memset(m,0,0)` (no write), `*mlen=0`, returns `-1` | `e05_open_smlen_zero` | [x] |
| 6  | `crypto_sign_open` | `smlen == SPX_BYTES - 1` (one step past valid range) | `memset(m,0,smlen)`, `*mlen=0`, returns `-1` | `e06_open_smlen_off_by_one` | [x] |
| 7  | `crypto_sign_open` | `smlen >= SPX_BYTES` but inner `crypto_sign_verify` fails — `sign.c:277` | `memset(m,0,smlen)` (the FULL smlen, not mlen!), `*mlen=0`, returns `-1` | `e07_open_verify_fails` | [x] |
| 8  | `crypto_sign_open` | `smlen == SPX_BYTES` exactly, valid sig, empty message | `*mlen=0`, returns `0` | `e08_open_empty_message` | [x] |
| 9  | `seedexpander_init` | `maxlen >= 0x100000000` — `rng.c:32` | returns `RNG_BAD_MAXLEN` (-1), ctx untouched | `e09_seedexpander_init_maxlen` | [x] |
| 10 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted) | returns `RNG_SUCCESS` (0) | `e10_seedexpander_init_maxlen_boundary` | [x] |
| 11 | `seedexpander` | `x == NULL` — `rng.c:66` (checked BEFORE the length check) | returns `RNG_BAD_OUTBUF` (-2), ctx untouched | `e11_seedexpander_null_out` | [x] |
| 12 | `seedexpander` | `xlen >= ctx->length_remaining` — `rng.c:68` | returns `RNG_BAD_REQ_LEN` (-3), ctx untouched | `e12_seedexpander_req_len` | [x] |
| 13 | `seedexpander` | `xlen == ctx->length_remaining - 1` (largest accepted) | returns `RNG_SUCCESS` (0) | `e13_seedexpander_req_len_boundary` | [x] |
| 14 | `seedexpander` | `xlen == 0` with `length_remaining == 0` (init'd with maxlen 0) | returns `RNG_BAD_REQ_LEN` (0 >= 0) | `e14_seedexpander_zero_len_zero_remaining` | [x] |
| 15 | `seedexpander` | `xlen == 0`, `length_remaining > 0` | returns `RNG_SUCCESS`, no bytes written, ctx unchanged | `e15_seedexpander_zero_len` | [x] |
| 16 | `seedexpander` | `xlen <= 16 - buffer_pos` (early `return RNG_SUCCESS` inside the loop — `rng.c:79`) | returns `RNG_SUCCESS`, served from the buffer only | `e16_seedexpander_buffered_path` | [x] |
| 17 | `randombytes` | any `xlen` (there is no failure path) — `rng.c:182` | always returns `RNG_SUCCESS` (0) | `e17_randombytes_always_success` | [x] |
| 18 | `randombytes` | `xlen == 0` | returns `0`, writes nothing, but STILL runs `AES256_CTR_DRBG_Update` and bumps `reseed_counter` | `e18_randombytes_zero_len` | [x] |
| 19 | `randombytes_init` | `personalization_string == NULL` — `rng.c:145` | no XOR applied to the seed material | `e19_randombytes_init_null_pers` | [x] |
| 20 | `randombytes_init` | `personalization_string != NULL` | 48-byte XOR applied | `e20_randombytes_init_with_pers` | [x] |
| 21 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` — `rng.c:203` | no XOR applied to `temp` | `e21_drbg_update_null_provided` | [x] |
| 22 | `set_type` | out-of-range "enum": `type` is `uint32_t`, C truncates to `unsigned char`; values `7`, `255`, `256`, `0x100+3`, `0xFFFFFFFF` have no valid `SPX_ADDR_TYPE_*` variant | accepted; byte at `SPX_OFFSET_TYPE` = `type & 0xff` | `e22_set_type_out_of_range` | [x] |
| 23 | `set_layer_addr` | `layer >= 256` (e.g. `SPX_D`, `256`, `0xFFFFFFFF`) | accepted; truncated to `layer & 0xff` | `e23_set_layer_out_of_range` | [x] |
| 24 | `set_chain_addr` | `chain >= 256` (valid range is `0..SPX_WOTS_LEN`) | accepted; truncated to `chain & 0xff` | `e24_set_chain_out_of_range` | [x] |
| 25 | `set_hash_addr` | `hash >= 256` (valid range is `0..SPX_WOTS_W`) | accepted; truncated to `hash & 0xff` | `e25_set_hash_out_of_range` | [x] |
| 26 | `set_tree_height` | `tree_height >= 256` (valid range is `0..SPX_TREE_HEIGHT`) | accepted; truncated to `& 0xff` | `e26_set_tree_height_out_of_range` | [x] |
| 27 | `set_keypair_addr` / `set_tree_index` | full `uint32_t` range incl. `0xFFFFFFFF` | accepted; big-endian 4-byte store, no masking | `e27_u32_fields_full_range` | [x] |
| 28 | `set_tree_addr` | full `uint64_t` range incl. `u64::MAX` | accepted; big-endian 8-byte store, no masking | `e28_set_tree_addr_full_range` | [x] |
| 29 | `ull_to_bytes` | `outlen == 0` (loop body never runs) | writes nothing | `e29_ull_to_bytes_zero_len` | [x] |
| 30 | `ull_to_bytes` | `outlen > 8` (more bytes than the input has) | high bytes are zero-filled | `e30_ull_to_bytes_oversized` | [x] |
| 31 | `bytes_to_ull` | `inlen == 0` | returns `0` | `e31_bytes_to_ull_zero_len` | [x] |
| 32 | `bytes_to_ull` | `inlen > 8` (shift count `8*(inlen-1-i)` >= 64 -> C UB, but the compiled behaviour is the ground truth) | matches the C `.so` bit-for-bit | `e32_bytes_to_ull_oversized` | [x] |
| 33 | `crypto_sign_signature` | `mlen == 0` | returns `0`, `*siglen = SPX_BYTES` | `e33_sign_empty_message` | [x] |
| 34 | `crypto_sign` / `crypto_sign_open` | `mlen == 0` round-trip | `*smlen = SPX_BYTES`; open returns `0`, `*mlen = 0` | `e34_sign_open_empty_roundtrip` | [x] |
| 35 | `crypto_sign_seed_keypair` | any seed (no validation at all — `sign.c:52`) | always returns `0` | `e35_seed_keypair_always_zero` | [x] |
| 36 | `crypto_sign_keypair` | no validation | always returns `0` | `e36_keypair_always_zero` | [x] |
| 37 | `crypto_sign_bytes` / `..._secretkeybytes` / `..._publickeybytes` / `..._seedbytes` | no inputs | fixed constants (`SPX_BYTES`, `4*SPX_N`, `2*SPX_N`, `3*SPX_N`) | `e37_size_getters` | [x] |
| 38 | `thash` | `inblocks == 0` (degenerate but accepted; hashes only pub_seed+addr) | no rejection, deterministic digest | `e38_thash_zero_inblocks` | [x] |
| 39 | `thash` | `inblocks == 1` vs `> 1` — the `if (inblocks > 1)`/`if (inblocks == 1)` branch in `thash_{sha2,blake}_*` (SHA-512/BLAKE-512 path) and `thash_haraka_*` (haraka512 vs haraka_S path) | different primitive selected | `e39_thash_branch_boundary` | [x] |
| 40 | `compute_root` | `tree_height == 0` -> `for (i = 0; i < tree_height - 1; i++)` with unsigned `tree_height` wraps to `0xFFFFFFFF` (C UB-free but pathological loop) | NOT tested at runtime (would loop ~2^32 times / read out of bounds); documented as unreachable from every public caller (`SPX_FORS_HEIGHT >= 6`, `SPX_TREE_HEIGHT >= 3`) | n/a | [x] |
| 41 | `handleErrors` (`rng.c:107`) | OpenSSL `EVP_*` failure | `ERR_print_errors_fp(stderr); abort()` | unreachable — Rust uses a self-contained AES-256 that cannot fail; documented, no test | [x] |
| 42 | compile-time `#error`s: `SPX_TREE_HEIGHT*SPX_D != SPX_FULL_HEIGHT` (`params-*.h`), `SPX_TREE_HEIGHT*(SPX_D-1) > 64` (`address.c:22`, `hash_*.c`), `SPX_BLAKE256_OUTPUT_BYTES < SPX_N` (`blake.h:10`), `SPX_SHA256_OUTPUT_BYTES < SPX_N` (`sha2.h:14`), `SPX_N > SPX_SHAX_BLOCK_BYTES` (`hash_sha2.c:79`), `SPX_SHAX_BLOCK_BYTES` not a power of two (`hash_sha2.c:139`), `SPX_WOTS_W` not in {16,256} | build fails | these constrain the *configuration* space, not runtime inputs; all 48 valid feature combinations satisfy them (verified: all 48 C builds and `cargo check`s succeed) | [x] |
