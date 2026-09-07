# ERRORS.md — error / rejection surface of the C implementation

Derived mechanically from `c_src` by grepping every `return -N`, every error
enum, every explicit range check, every null check and every min/max constant:

```
$ grep -rn "return -\|return NULL\|RETURN_ERROR\|assert\|abort()\|exit(" c_src/app/src c_src/lib
c_src/app/src/rng.c:109:    abort();          <- handleErrors(), OpenSSL failure only
c_src/app/src/sign.c:180:        return -1;    <- crypto_sign_verify, siglen
c_src/app/src/sign.c:236:        return -1;    <- crypto_sign_verify, root memcmp
c_src/app/src/sign.c:272:        return -1;    <- crypto_sign_open, smlen too small
c_src/app/src/sign.c:280:        return -1;    <- crypto_sign_open, inner verify
$ grep -rn "RNG_" c_src/app/src/rng.c c_src/app/include/rng.h
rng.h:  RNG_SUCCESS 0 / RNG_BAD_MAXLEN -1 / RNG_BAD_OUTBUF -2 / RNG_BAD_REQ_LEN -3
rng.c:33  return RNG_BAD_MAXLEN;   <- seedexpander_init, maxlen
rng.c:67  return RNG_BAD_OUTBUF;   <- seedexpander, x == NULL
rng.c:69  return RNG_BAD_REQ_LEN;  <- seedexpander, xlen vs length_remaining
```

There is **no** input validation anywhere else: no function in `address.c`,
`utils.c`, `utilsx1.c`, `wots.c`, `wotsx1.c`, `fors.c`, `merkle.c` or any
`hash_*.c` / `thash_*.c` returns a status at all — they are all `void`.  The
rows below therefore also record the *silent* acceptance behaviour (truncation,
clamping, wrap-around), because "the C accepts this and does X" is just as much
part of the contract as "the C rejects this".

`SPX_BYTES`, `SPX_N`, … are the parameter-set constants; the tests compute them
the same way the C headers do.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| E1 | `crypto_sign_verify` | `siglen == 0` | returns `-1` before touching `sig` (`sign.c:179`) | `e01_verify_siglen_zero` | [x] |
| E2 | `crypto_sign_verify` | `siglen == SPX_BYTES - 1` | returns `-1` | `e02_verify_siglen_minus_one` | [x] |
| E3 | `crypto_sign_verify` | `siglen == SPX_BYTES + 1` | returns `-1` | `e03_verify_siglen_plus_one` | [x] |
| E4 | `crypto_sign_verify` | `siglen == SIZE_MAX`, `SIZE_MAX/2`, `2^40` | returns `-1` | `e04_verify_siglen_huge` | [x] |
| E5 | `crypto_sign_verify` | correct `siglen`, one byte of `sig` flipped, at 8 offsets spanning the R field, the FORS sk / auth-path region, the layer-0 WOTS region and the layer-0 auth path | reconstructed root ≠ `pk+SPX_N` ⇒ `-1` (`sign.c:235`) | `e05_verify_corrupt_sig_regions` | [x] |
| E6 | `crypto_sign_verify` | correct `sig`, each of 1000 message bytes flipped in turn | C and Rust must return the SAME code for every one; **not every flip is rejected** — see the byte/bit note below | `e06_verify_wrong_message` | [x] |
| E7 | `crypto_sign_verify` | `pk` from a different key pair, and each `pk` byte flipped in turn | `-1` | `e07_verify_wrong_pk` | [x] |
| E8 | `crypto_sign_verify` | correct `sig`+`m`+`pk`, `mlen` changed (shorter and longer) | same code on both sides; at least one is `-1` | `e08_verify_wrong_mlen` | [x] |
| E9 | `crypto_sign_verify` | all-zero `sig`/`pk`/`m`, and all-`0xff` | `-1` (root will not match) | `e09_verify_all_zero` | [x] |
| E10 | `crypto_sign_verify` | genuinely valid signature, `mlen ∈ {0,1,33,231}` | returns `0` (the only success path) | `e10_verify_valid` | [x] |
| E11 | `crypto_sign_open` | `smlen == 0` | `memset(m,0,0)`, `*mlen = 0`, returns `-1` (`sign.c:269`) | `e11_open_smlen_zero` | [x] |
| E12 | `crypto_sign_open` | `smlen == 1` and `smlen == SPX_BYTES - 1` | zeroes `smlen` bytes of `m`, `*mlen = 0`, `-1` | `e12_open_smlen_minus_one` | [x] |
| E13 | `crypto_sign_open` | `smlen == SPX_BYTES` (empty message — boundary, *accepted*) | `*mlen = 0`, returns `0` | `e13_open_smlen_exactly_sigbytes` | [x] |
| E14 | `crypto_sign_open` | `smlen >= SPX_BYTES`, signature region corrupted (5 offsets) | zeroes `smlen` bytes of `m`, `*mlen = 0`, `-1` (`sign.c:277`); message-region corruption is compared for equality only | `e14_open_bad_signature` | [x] |
| E15 | `crypto_sign_open` | `smlen` 64 bytes longer than what was signed | verify sees a longer message ⇒ `-1`, `m` zeroed for `smlen` bytes | `e15_open_smlen_too_long` | [x] |
| E16 | `crypto_sign_seed_keypair` | all-zero, all-`0xff` and counting seeds | returns `0` unconditionally — no validation | `e16_seed_keypair_always_zero` | [x] |
| E17 | `crypto_sign_signature` | all-zero and all-`0xff` `sk`; `mlen == 0` | returns `0`, `*siglen == SPX_BYTES` | `e17_signature_always_zero` | [x] |
| E18 | `crypto_sign` | `mlen == 0` | returns `0`, `*smlen == SPX_BYTES` | `e18_sign_mlen_zero` | [x] |
| E19 | `seedexpander_init` | `maxlen == 0x100000000` (first rejected value) | `RNG_BAD_MAXLEN` = `-1` (`rng.c:32`), `ctx` untouched (verified against a pre-filled pattern) | `e19_seedexpander_init_maxlen_2p32` | [x] |
| E20 | `seedexpander_init` | `maxlen ∈ {UINT64_MAX, 0x100000001, 2^63}` | `RNG_BAD_MAXLEN` = `-1` | `e20_seedexpander_init_maxlen_max` | [x] |
| E21 | `seedexpander_init` | `maxlen == 0xFFFFFFFF` (largest accepted value) | `RNG_SUCCESS` = `0`, `ctr[8..12] = FF FF FF FF`, `ctr[12..16] = 0`, `buffer_pos = 16`, `buffer` zeroed | `e21_seedexpander_init_maxlen_2p32_minus_1` | [x] |
| E22 | `seedexpander_init` | `maxlen == 0` | `RNG_SUCCESS` = `0`, `length_remaining = 0` | `e22_seedexpander_init_maxlen_zero` | [x] |
| E23 | `seedexpander` | `x == NULL` | `RNG_BAD_OUTBUF` = `-2` (`rng.c:66`), `ctx` untouched | `e23_seedexpander_null_out` | [x] |
| E24 | `seedexpander` | `x == NULL` **and** `xlen` out of range | still `RNG_BAD_OUTBUF` = `-2` (null check comes first) | `e24_seedexpander_null_out_precedence` | [x] |
| E25 | `seedexpander` | `xlen == ctx->length_remaining` (note: `>=`, so *equal* is rejected — off-by-one in the C, replicated verbatim) | `RNG_BAD_REQ_LEN` = `-3` (`rng.c:68`), `ctx` untouched | `e25_seedexpander_xlen_equals_remaining` | [x] |
| E26 | `seedexpander` | `xlen > ctx->length_remaining`, incl. `UINT64_MAX` | `RNG_BAD_REQ_LEN` = `-3` | `e26_seedexpander_xlen_gt_remaining` | [x] |
| E27 | `seedexpander` | `xlen == ctx->length_remaining - 1` (largest accepted) | `RNG_SUCCESS` = `0`, exactly `xlen` bytes written | `e27_seedexpander_xlen_max_ok` | [x] |
| E28 | `seedexpander` | `xlen == 0` with `length_remaining > 0` | `RNG_SUCCESS` = `0`, nothing written, state unchanged | `e28_seedexpander_xlen_zero` | [x] |
| E29 | `seedexpander` | `length_remaining == 0` (after `maxlen = 0`), `xlen ∈ {0,1,16}` | `xlen >= 0` ⇒ `RNG_BAD_REQ_LEN` = `-3` even for `xlen == 0` | `e29_seedexpander_zero_budget` | [x] |
| E30 | `randombytes` (`rng.c`) | `xlen == 0` | returns `RNG_SUCCESS` = `0`, nothing written (loop not entered), but the trailing `AES256_CTR_DRBG_Update(NULL, …)` still runs and `reseed_counter` still increments | `e30_randombytes_xlen_zero` | [x] |
| E31 | `randombytes` (`rng.c`) | `xlen ∈ {0,1,15,16,17,100,4096}` — there is no failure path | always `RNG_SUCCESS` = `0` | `e31_randombytes_always_success` | [x] |
| E32 | `randombytes_init` | `personalization_string == NULL` | skips the 48-byte XOR (`rng.c:143`); equivalent to an all-zero string, and different from a non-zero one | `e32_randombytes_init_null_pers` | [x] |
| E33 | `AES256_CTR_DRBG_Update` | `provided_data == NULL` | skips the 48-byte XOR (`rng.c:205`); equivalent to all-zero data | `e33_drbg_update_null_data` | [x] |
| E34 | `SPX_set_type` | out-of-range "enum" value `∈ {7, 8, 127, 128, 255, 256, 0xFF00, 0x12345678, UINT32_MAX}` — a C enum parameter accepts any `int` | no rejection; `(unsigned char)type` stores the **low byte** at `SPX_OFFSET_TYPE`, so `256 → 0x00`, `UINT32_MAX → 0xFF` | `e34_set_type_out_of_range` | [x] |
| E35 | `SPX_set_layer_addr` | `layer ∈ {SPX_D, SPX_D+1, 255, 256, 0x10000, UINT32_MAX}` | no rejection; low byte stored | `e35_set_layer_out_of_range` | [x] |
| E36 | `SPX_set_chain_addr` / `SPX_set_hash_addr` | `≥ SPX_WOTS_W`, `≥ SPX_WOTS_LEN`, `256`, `UINT32_MAX` | no rejection; low byte stored | `e36_set_chain_hash_out_of_range` | [x] |
| E37 | `SPX_set_tree_height` | `> SPX_TREE_HEIGHT`, `SPX_FULL_HEIGHT`, `256`, `UINT32_MAX` | no rejection; low byte stored | `e37_set_tree_height_out_of_range` | [x] |
| E38 | `SPX_set_tree_addr` | `tree` beyond `2^(TREE_HEIGHT*(D-1))`, incl. `UINT64_MAX` and `2^63` | no rejection; all 8 bytes written big-endian, unmasked | `e38_set_tree_addr_out_of_range` | [x] |
| E39 | `SPX_set_keypair_addr` / `SPX_set_tree_index` | `≥ 2^SPX_TREE_HEIGHT`, incl. `UINT32_MAX` | no rejection; all 4 bytes written big-endian | `e39_set_kp_treeindex_out_of_range` | [x] |
| E40 | `SPX_ull_to_bytes` | `outlen == 0` | loop starts at `-1` ⇒ writes nothing | `e40_ull_to_bytes_outlen_zero` | [x] |
| E41 | `SPX_ull_to_bytes` | `outlen ∈ {9,10,12,16}` | writes `outlen` bytes: the low 8 are the big-endian value, the excess high bytes become `0` | `e41_ull_to_bytes_outlen_gt8` | [x] |
| E42 | `SPX_bytes_to_ull` | `inlen == 0` | returns `0` | `e42_bytes_to_ull_inlen_zero` | [x] |
| E43 | `SPX_bytes_to_ull` | `inlen ∈ 9..=16` — the shift count `8*(inlen-1-i)` reaches `≥ 64`, which is UB in C and an over-shift in Rust | must produce the *same* value the compiled C does (it does: both mask the shift count mod 64 on x86-64) | `e43_bytes_to_ull_inlen_gt8` | [x] |
| E44 | `SPX_thash` | `inblocks == 0` | no rejection; blake/shake/sha2 hash only `pub_seed‖addr`, haraka takes the `inblocks != 1` (`haraka_S`) branch with an empty payload; the ADRS is left unmodified | `e44_thash_inblocks_zero` | [x] |
| E45 | `SPX_thash` | `inblocks == 1` vs `2` — the branch boundary for haraka (F vs H) and for sha2/blake with `SPX_*512` (`if (inblocks > 1)`) | different primitive on each side; the two outputs must also differ from each other | `e45_thash_inblocks_boundary` | [x] |
| E46 | `SPX_thash` | `inblocks == SPX_FORS_TREES` and `SPX_WOTS_LEN` (the largest values the library itself uses) | no rejection | `e46_thash_inblocks_large` | [x] |
| E47 | `SPX_compute_root` | `tree_height == 1` (minimum sane value), `leaf_idx ∈ {0,1,2,3,UINT32_MAX}` | one `thash`, loop body never runs. `tree_height == 0` makes `tree_height - 1` underflow to `0xFFFFFFFF` and loop ~2^32 times reading past `auth_path` — pathological, **deliberately not executed** | `e47_compute_root_height_one` | [x] |
| E48 | `SPX_treehash` | `tree_height == 0`, `leaf_idx ∈ {0,1,2,UINT32_MAX}`, `idx_offset ∈ {0,1,0x1000}` | `1 << 0 == 1` leaf, root = that leaf; `auth_path` written iff `leaf_idx ^ 1 == 0` | `e48_treehash_height_zero` | [x] |
| E49 | `SPX_wots_pk_from_sig` | `msg` all-`0x00` and all-`0xff`, i.e. `chain_lengths` at both extremes, so `gen_chain` runs with `start = 0, steps = W-1` and with `start = W-1, steps = 0` — the `i < SPX_WOTS_W` clamp from both ends | no rejection; chain clamped at `SPX_WOTS_W` | `e49_wots_chain_clamp` | [x] |
| E50 | `SPX_wots_gen_leafx1` | `info->wots_sign_leaf != leaf_idx` (`wots_k_mask = ~0`, "generate pk only") with `info->wots_sig == NULL` and arbitrary `wots_steps` | must not dereference the null pointer — `k == wots_k` can never hold | `e50_wots_gen_leafx1_null_sig` | [x] |
| E51 | `SPX_prf_addr`, `SPX_thash` | an out-of-range ADRS **type** byte (`7 … UINT32_MAX`) carried through the composed pipeline rather than set in isolation | no rejection; identical output on both sides | `e51_out_of_range_type_through_pipeline` | [x] |
| E52 | the four size accessors | no input; cannot fail | exact parameter-set values | `e52_size_accessors_exact` | [x] |
| — | `handleErrors` (`rng.c:106`) | an OpenSSL `EVP_*` call fails | `ERR_print_errors_fp(stderr)` then `abort()` | **not triggerable** — the Rust port uses the pure-Rust `aes` crate, which has no failure path, so the function is intentionally absent | n/a |
| — | `randombytes` (`randombytes.c`) | `open("/dev/urandom")` fails, or `read()` returns `< 1` | `sleep(1)` and retry forever; never reports an error | **not differentially testable** (would require making `/dev/urandom` fail); the Rust `randombytes_urandom` mirrors the retry loop | n/a |
| — | `#error` directives in the parameter headers | `SPX_WOTS_W ∉ {16,256}`, `SPX_N ∉ {2..256}`, `SPX_D ∤ SPX_FULL_HEIGHT`, `SPX_TREE_BITS > 64`, `SPX_N > SPX_SHAX_BLOCK_BYTES`, `SPX_SHA256_OUTPUT_BYTES < SPX_N` | compile-time failure | **build-time only** — none of the 24 shipped parameter sets trips them, and `params.rs` encodes the same precomputed values | n/a |

## Two C behaviours worth calling out

Both were found by these tests and are faithfully reproduced by the Rust; they
are *not* bugs in the translation.

**1. `hash_blake.c` passes byte counts to a bit-count API.**
`blake256_update` / `blake512_update` take `datalen` in BITS — `blake256()`
itself calls `blake256_update(&S, in, inlen*8)`.  But `gen_message_random` and
`hash_message` in `hash_blake.c` drive the streaming API directly and pass
`SPX_N`, `SPX_PK_BYTES` and `mlen` as-is, i.e. as *bytes*.  Consequences, all
matched byte-for-byte by the Rust:

* only `mlen/8` bytes of the message are ever copied into the block buffer;
* for short inputs the accumulated bit count never reaches a compression, so
  `blake*_final` returns the unchanged IV and the message does not influence the
  digest at all.  This is why "flip any message byte ⇒ signature rejected" is
  not a property of the blake backend (rows E6, E8, E14).

**2. `gen_message_random` overruns `R` on the blake backend.**
It ends in `blakeX_final(&S, R)`, which writes the full 32- or 64-byte digest,
not `SPX_N` bytes.  In `crypto_sign_signature` the destination is the signature
buffer, so the extra bytes are simply overwritten later.  The tests therefore
compare a 64-byte window past `R` rather than asserting it untouched.

Rows marked `n/a` are not runtime-reachable through the FFI boundary.  Every
other row has a differential test in `tests/errors.rs` that calls **both** the C
and the Rust `.so` and asserts the *same* code / sentinel — not merely "both
failed".
