# ERRORS.md — error-surface table

Every distinct way the C library rejects or errors on input, derived by grepping
`c_src` for `return -`, `return NULL`, `return RNG_*`, `assert`, `abort`,
`#error`, and every explicit range / null / length check.

Grep used:

```
grep -rn "return -\|return NULL\|assert\|RETURN_ERROR\|abort()\|exit(\|#error\|return RNG" \
  --include=*.c --include=*.h c_src
```

Error constants (`app/include/rng.h`):
`RNG_SUCCESS 0`, `RNG_BAD_MAXLEN -1`, `RNG_BAD_OUTBUF -2`, `RNG_BAD_REQ_LEN -3`.

## Runtime rejections (differentially tested)

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `crypto_sign_verify` | `siglen != SPX_BYTES` — `siglen = 0` | returns `-1`; no output buffer touched | `err_01_verify_siglen_zero` |
| 2 | `crypto_sign_verify` | `siglen != SPX_BYTES` — `siglen = SPX_BYTES - 1` (one below) | returns `-1` | `err_02_verify_siglen_minus_one` |
| 3 | `crypto_sign_verify` | `siglen != SPX_BYTES` — `siglen = SPX_BYTES + 1` (one above) | returns `-1` | `err_03_verify_siglen_plus_one` |
| 4 | `crypto_sign_verify` | `siglen != SPX_BYTES` — huge `siglen` (`SIZE_MAX`) | returns `-1` (length check happens before any read) | `err_04_verify_siglen_huge` |
| 5 | `crypto_sign_verify` (sign.c:235) | correct `siglen`, but signature bytes corrupted → `memcmp(root, pub_root, SPX_N) != 0` | returns `-1` | `err_05_verify_corrupt_sig` |
| 6 | `crypto_sign_verify` (sign.c:235) | correct `siglen` and signature, but message mutated | returns `-1` **when the mutated byte is actually absorbed**. C quirk: the BLAKE backend's `hash_message` calls `blakeX_update(&S, m, mlen)` while `blake256_update`/`blake512_update` take their length in *bits*, so only `mlen / 8` bytes of the message reach the digest and mutating a later byte is accepted (`0`). Ground truth — the Rust must match, including the acceptance. | `err_06_verify_wrong_message` |
| 7 | `crypto_sign_verify` (sign.c:235) | correct `siglen` and signature, but `pk` from a different keypair | returns `-1` | `err_07_verify_wrong_pk` |
| 8 | `crypto_sign_verify` (sign.c:235) | valid signature verified with `mlen` altered (truncated message) | returns `-1` | `err_08_verify_wrong_mlen` |
| 9 | `crypto_sign_open` (sign.c:269) | `smlen < SPX_BYTES` — `smlen = 0` | returns `-1`, `*mlen = 0`, `memset(m, 0, smlen)` (i.e. nothing) | `err_09_open_smlen_zero` |
| 10 | `crypto_sign_open` (sign.c:269) | `smlen < SPX_BYTES` — `smlen = SPX_BYTES - 1` | returns `-1`, `*mlen = 0`, first `SPX_BYTES-1` bytes of `m` zeroed | `err_10_open_smlen_minus_one` |
| 11 | `crypto_sign_open` (sign.c:277) | `smlen >= SPX_BYTES` but inner `crypto_sign_verify` fails (corrupted signature) | returns `-1`, `*mlen = 0`, **`smlen` bytes** of `m` zeroed (note: `smlen`, not `smlen - SPX_BYTES`) | `err_11_open_verify_fail_zeroizes` |
| 12 | `crypto_sign_open` (sign.c:277) | `smlen = SPX_BYTES` exactly, tail message empty, signature invalid | returns `-1`, `*mlen = 0`, `SPX_BYTES` bytes of `m` zeroed | `err_12_open_exact_len_invalid` |
| 13 | `seedexpander_init` (rng.c:32) | `maxlen >= 0x100000000` (`maxlen = 0x100000000`) | returns `RNG_BAD_MAXLEN` (`-1`); `ctx` **not** modified | `err_13_seedexpander_init_maxlen_boundary` |
| 14 | `seedexpander_init` (rng.c:32) | `maxlen = ULONG_MAX` | returns `-1` | `err_14_seedexpander_init_maxlen_max` |
| 15 | `seedexpander_init` | `maxlen = 0xFFFFFFFF` (one below the limit → accepted) | returns `RNG_SUCCESS` (`0`) and writes the documented `ctx` layout | `err_15_seedexpander_init_maxlen_ok` |
| 16 | `seedexpander` (rng.c:66) | `x == NULL` | returns `RNG_BAD_OUTBUF` (`-2`); checked **before** the length check, so NULL wins even with a bad length | `err_16_seedexpander_null_out` |
| 17 | `seedexpander` (rng.c:68) | `xlen >= ctx->length_remaining` — `xlen == length_remaining` (note `>=`, so requesting exactly the remaining amount is rejected) | returns `RNG_BAD_REQ_LEN` (`-3`); `ctx` unchanged | `err_17_seedexpander_xlen_eq_remaining` |
| 18 | `seedexpander` (rng.c:68) | `xlen > ctx->length_remaining` | returns `-3` | `err_18_seedexpander_xlen_gt_remaining` |
| 19 | `seedexpander` (rng.c:66+68) | `x == NULL` **and** `xlen > length_remaining` together | returns `-2` (not `-3`) — precedence check | `err_19_seedexpander_null_beats_len` |
| 20 | `seedexpander` | `xlen == 0` with `length_remaining > 0` (valid, degenerate) | returns `RNG_SUCCESS` (`0`); `ctx` unchanged (`while(xlen>0)` never entered) | `err_20_seedexpander_zero_len` |
| 21 | `randombytes` (rng.c, `sphincs_core_det`) | `xlen == 0` | returns `RNG_SUCCESS` (`0`), still performs the trailing `AES256_CTR_DRBG_Update` and `reseed_counter++` — i.e. **state advances even for a zero-length request** | `err_21_randombytes_zero_len` |
| 22 | `randombytes_init` (rng.c:143) | `personalization_string == NULL` | no error return (void); `seed_material` = `entropy_input` unmodified | `err_22_randombytes_init_null_pers` |
| 23 | `crypto_sign_seed_keypair` | any seed — always returns `0` (no validation whatsoever) | returns `0` | `err_23_seed_keypair_always_zero` |
| 24 | `crypto_sign_signature` | any message incl. `mlen = 0` — always returns `0`, `*siglen = SPX_BYTES` | returns `0` | `err_24_signature_always_zero` |
| 25 | `crypto_sign` | always returns `0`, `*smlen = SPX_BYTES + mlen` | returns `0` | `err_25_sign_always_zero` |
| 26 | `crypto_sign_keypair` | always returns `0` | returns `0` | `err_26_keypair_always_zero` |

## Out-of-range values crossing the FFI boundary

The C API takes plain `uint32_t` where a bounded quantity is meant; C accepts
any `int`/`uint32_t` and silently truncates or wraps. These are real inputs the
C handles and the Rust must handle identically.

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 27 | `SPX_set_type` | `type` outside `SPX_ADDR_TYPE_WOTS..SPX_ADDR_TYPE_FORSPRF` (0..6): `7`, `255`, `256`, `0xFFFFFFFF` | no rejection — stores `(unsigned char)type` at `SPX_OFFSET_TYPE`, i.e. `type & 0xFF` | `err_27_set_type_out_of_range` |
| 28 | `SPX_set_layer_addr` | `layer >= 256` (e.g. `0x1234`, `0xFFFFFFFF`) | stores `(unsigned char)layer` at `SPX_OFFSET_LAYER` | `err_28_set_layer_truncates` |
| 29 | `SPX_set_chain_addr` | `chain >= 256` | stores `(unsigned char)chain` at `SPX_OFFSET_CHAIN_ADDR` | `err_29_set_chain_truncates` |
| 30 | `SPX_set_hash_addr` | `hash >= 256` | stores `(unsigned char)hash` at `SPX_OFFSET_HASH_ADDR` | `err_30_set_hash_truncates` |
| 31 | `SPX_set_tree_height` | `tree_height >= 256` | stores `(unsigned char)tree_height` at `SPX_OFFSET_TREE_HGT` | `err_31_set_tree_height_truncates` |
| 32 | `SPX_set_tree_addr` | `tree = UINT64_MAX` (far beyond `2^SPX_TREE_BITS`) | writes all 8 bytes big-endian, no masking | `err_32_set_tree_addr_full_range` |
| 33 | `SPX_set_keypair_addr` / `SPX_set_tree_index` | `0xFFFFFFFF` | writes 4 bytes big-endian, no masking | `err_33_set_u32_fields_full_range` |
| 34 | `SPX_ull_to_bytes` | `outlen = 0` | writes nothing (loop from `-1`) — must not touch the buffer | `err_34_ull_to_bytes_outlen_zero` |
| 35 | `SPX_ull_to_bytes` | `outlen > 8` (e.g. `16`) with a large `in` | writes `outlen` bytes; the high `outlen-8` bytes end up `0` because `in >> 8` exhausts the value | `err_35_ull_to_bytes_outlen_gt_8` |
| 36 | `SPX_bytes_to_ull` | `inlen = 0` | returns `0`; reads nothing | `err_36_bytes_to_ull_inlen_zero` |
| 37 | `SPX_bytes_to_ull` | `inlen = 8` (maximum meaningful) | returns the 8 bytes big-endian | `err_37_bytes_to_ull_inlen_8` |
| 38 | `SPX_thash` | `inblocks = 0` | degenerate but accepted: hashes only `pub_seed‖addr` (backend dependent); Haraka takes the `inblocks != 1` branch | `err_38_thash_inblocks_zero` |
| 39 | `SPX_chain_lengths` | message byte values `0x00` and `0xFF` everywhere (checksum extremes) | no rejection; must match exactly, including the `csum` shift | `err_39_chain_lengths_extremes` |
| 40 | `SPX_wots_gen_leafx1` | `info->wots_sign_leaf = ~0u` with `info->wots_sig = NULL` (the `INITIALIZE_LEAF_INFO_X1` "benign" setup used by `merkle_gen_root`) | `k == wots_k` never true, so the `NULL` `wots_sig` is never dereferenced; produces a pk-only leaf | `err_40_wots_gen_leafx1_null_sig` |

## Non-runtime / unreachable rejections (documented, not testable)

| # | site | trigger | why not differentially tested |
|---|------|---------|-------------------------------|
| 41 | `rng.c:109` `handleErrors()` → `abort()` | OpenSSL `EVP_*` failure inside `AES256_ECB` | Only reachable on an OpenSSL allocation/init failure. Cannot be induced from the public API; both implementations would terminate the process. |
| 42 | `randombytes.c` (`sphincs_core`) | `/dev/urandom` cannot be opened / short read | Infinite `sleep(1)` retry loop, never returns an error. Also not the `randombytes` exported by the Rust `.so` (see `SYMBOLS.md`). |
| 43 | `params-sphincs-*.h:35` | `SPX_WOTS_W` not 16 or 256 | `#error`, compile time. No shipped parameter set violates it (`W = 16` in all 24). |
| 44 | `params-sphincs-*.h:47,57` | `SPX_N` outside `{2..256}` | `#error`, compile time. |
| 45 | `params-sphincs-*.h:69` | `SPX_D` does not divide `SPX_FULL_HEIGHT` | `#error`, compile time. Holds for all 6 SECPARs. |
| 46 | `address.c:21` | `SPX_TREE_HEIGHT * (SPX_D - 1) > 64` | `#error`, compile time. |
| 47 | `blake.h:9` / `sha2.h:13` | `SPX_N > 32` | `#error`, compile time. |
| 48 | `hash_*.c` | `SPX_TREE_BITS > 64` | `#error`, compile time. |
| 49 | `hash_sha2.c:78` | `SPX_N > SPX_SHAX_BLOCK_BYTES` | `#error`, compile time. |
| 50 | `hash_sha2.c:138` | `SPX_SHAX_BLOCK_BYTES` not a power of two | `#error`, compile time. |
| 51 | `utils.c:70` `compute_root` | `tree_height == 0` | `for (i = 0; i < tree_height - 1; i++)` with `uint32_t` wraps to `0xFFFFFFFF` iterations and walks `auth_path` off the end. Undefined behaviour / effectively non-terminating in both implementations; not a rejection the C performs. |
| 52 | `hash_*.c` `hash_message` | `SPX_D == 1` → `*tree = 0` | Dead branch: `SPX_D` is 7..22 in every shipped parameter set, so the `else` arm is always taken. |
| 53 | `utils.c` `bytes_to_ull` | `inlen > 8` → shift count `>= 64` | Undefined behaviour in C (`<<` by more than the width). Not a rejection; excluded because the C has no defined result to match. Row 37 pins the largest defined input. |

## Coverage

Rows 1–40 each have a dedicated differential test in
`translation/tests/errors.rs` that calls both the C `.so` and the Rust `.so`
and asserts the **same** return value / sentinel and the same output-buffer
side effects — not merely "both failed". Where the C returns an error *and*
zeroises a buffer (rows 9–12) the whole output buffer is compared; where the C
returns an error and leaves state untouched (rows 13–20) that is asserted too.

Result: **40/40 rows pass in all 48 configurations.**

```
$ TESTS=errors ./scripts/test_all.sh
OK   haraka-robust-128s (ok. 40 passed )
...
OK   blake-simple-256f  (ok. 40 passed )
```

Negative control (to show the tests are not vacuous): changing
`siglen != SPX_BYTES` to `siglen < SPX_BYTES` in `src/sign.rs` made rows 3 and 4
fail; flipping the leaf parity test in `compute_root` made row 6 fail. Both were
reverted.
