# ERRORS.md — Phase C error-surface table

Every distinct way the C source rejects / errors on input. Derived mechanically
by grepping `c_src` for `return -1`, `return RNG_*`, `return KAT_*`, `assert`,
`memcmp`, `if (... < ...)`, `if (... != ...)`, `if (... == NULL)`, and every
`#error` / min-max constant.

Grep basis:

```
grep -rn 'return -1\|return RNG_\|return KAT_\|assert\|exit(\|memcmp\|== NULL\|>= 0x1' c_src/app/src c_src/lib
```

There is **no** error enum and **no** `assert()` anywhere in `c_src`. All
rejections are integer sentinels. `#error` directives are compile-time only
(unreachable at runtime) and are listed separately at the bottom.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `crypto_sign_verify` (`sign.c:179`) | `siglen != SPX_BYTES` — **any** value other than exactly `SPX_BYTES`, incl. `0`, `SPX_BYTES-1`, `SPX_BYTES+1`, `SIZE_MAX` | returns `-1` immediately; **no** output buffer written, `pk`/`m` never read | `err_01_verify_siglen_not_exact` |
| 2 | `crypto_sign_verify` (`sign.c:235`) | signature parses but `memcmp(root, pub_root, SPX_N) != 0` — i.e. any bit-flip in `sig`, `m`, or `pk` | returns `-1` (after doing the full hypertree walk) | `err_02_verify_corrupt_*` |
| 3 | `crypto_sign_verify` (`sign.c:239`) | valid sig/msg/pk | returns `0` (success sentinel — the row that anchors 1 & 2) | `err_03_verify_valid` |
| 4 | `crypto_sign_open` (`sign.c:269-272`) | `smlen < SPX_BYTES` (incl. `0`, `1`, `SPX_BYTES-1`) | `memset(m, 0, smlen)`; `*mlen = 0`; returns `-1`. **Note the C zeroes exactly `smlen` bytes of `m`, not `SPX_BYTES`** | `err_04_open_smlen_too_small` |
| 5 | `crypto_sign_open` (`sign.c:277-280`) | `smlen >= SPX_BYTES` but the inner `crypto_sign_verify` fails (corrupt sig / wrong pk / tampered appended message) | `memset(m, 0, smlen)`; `*mlen = 0`; returns `-1`. **`*mlen` is set to `smlen - SPX_BYTES` first and then reset to 0** | `err_05_open_verify_fails` |
| 6 | `crypto_sign_open` (`sign.c:269`) boundary | `smlen == SPX_BYTES` exactly (empty message) | *not* an error: `*mlen = 0`, inner verify over a zero-length message, returns `0` on a genuine signature of the empty message | `err_06_open_smlen_exact` |
| 7 | `seedexpander_init` (`rng.c:32-33`) | `maxlen >= 0x100000000` (2^32). Boundary: `0xFFFFFFFF` OK, `0x100000000` rejected, `0xFFFFFFFFFFFFFFFF` rejected | returns `RNG_BAD_MAXLEN` = `-1`; **`ctx` left completely untouched** | `err_07_seedexpander_init_maxlen` |
| 8 | `seedexpander_init` (`rng.c:52`) | `maxlen < 0x100000000` (incl. `0`) | returns `RNG_SUCCESS` = `0`, and writes `ctx->length_remaining`, `ctx->key`, `ctx->ctr[0..8]=diversifier`, `ctx->ctr[8..12]=maxlen>>{24,16,8,0}`, `ctx->ctr[12..16]=0` (`memset(ctx->ctr+12, 0, 4)`), `ctx->buffer_pos=16` and `memset(ctx->buffer, 0, 16)` — i.e. all 80 struct bytes except the 4 alignment-padding bytes are written (`rng.c:37-50`) | `err_08_seedexpander_init_ok` |
| 9 | `seedexpander` (`rng.c:66-67`) | `x == NULL` — checked **first**, before the length check, so `NULL` with an over-long `xlen` still returns `-2` not `-3` | returns `RNG_BAD_OUTBUF` = `-2`; `ctx` untouched | `err_09_seedexpander_null_out` |
| 10 | `seedexpander` (`rng.c:68-69`) | `xlen >= ctx->length_remaining` — note `>=`, so requesting *exactly* the remaining length is an **error** | returns `RNG_BAD_REQ_LEN` = `-3`; `ctx` untouched | `err_10_seedexpander_reqlen` |
| 11 | `seedexpander` (`rng.c:79`, `rng.c:102`) | `xlen < ctx->length_remaining` (incl. `xlen == 0`, and `xlen` spanning many 16-byte AES blocks so the counter increments) | returns `RNG_SUCCESS` = `0`; `ctx->length_remaining -= xlen`, `buffer`/`buffer_pos`/`ctr` advanced | `err_11_seedexpander_ok` |
| 12 | `randombytes` (`rng.c:182`) | any `xlen`, incl. `0` (loop body never runs but the DRBG **is** still updated and `reseed_counter` **is** still incremented) | always returns `RNG_SUCCESS` = `0`; never fails | `err_12_randombytes_zero_len` |
| 13 | `randombytes_init` (`rng.c:143`) | `personalization_string == NULL` | no XOR applied; `entropy_input` used verbatim. (`entropy_input == NULL` is **not** checked — C dereferences it unconditionally → UB, excluded from testing) | `err_13_randombytes_init_null_pers` |
| 14 | `crypto_sign_keypair` (`sign.c:89`) | none — no input validation whatsoever | always returns `0` | `err_14_keypair_always_zero` |
| 15 | `crypto_sign_seed_keypair` (`sign.c:75`) | none — `memcpy(sk, seed, 3*SPX_N)` unconditionally | always returns `0` | `err_15_seed_keypair_always_zero` |
| 16 | `crypto_sign_signature` (`sign.c:157`) | none — no validation; `mlen == 0` is accepted | always returns `0` and sets `*siglen = SPX_BYTES` | `err_16_signature_always_zero` |
| 17 | `crypto_sign` (`sign.c:257`) | none; `mlen == 0` accepted (`memmove` of 0 bytes) | always returns `0`, `*smlen = SPX_BYTES + mlen` | `err_17_sign_always_zero` |
| 18 | `SPX_ull_to_bytes` (`utils.c:19`) | `outlen == 0` — the loop `for (i = outlen-1; i >= 0; ...)` with `(signed int)outlen` never executes | writes nothing, no crash | `err_18_ull_to_bytes_zero_len` |
| 19 | `SPX_bytes_to_ull` (`utils.c:38`) | `inlen == 0` | returns `0` | `err_19_bytes_to_ull_zero_len` |
| 20 | `SPX_bytes_to_ull` (`utils.c:39`) | `inlen > 8` — the shift `<< (8*(inlen-1-i))` has a shift count `>= 64` for the first bytes ⇒ **UB in C**, but with clang `-O3` on x86-64 it is a deterministic value the Rust must reproduce | whatever the C computes; must match byte-for-byte | `err_20_bytes_to_ull_oversized` |
| 21 | `SPX_set_type` (`address.c:35`) | `type` outside the documented range `0..=6` (`SPX_ADDR_TYPE_*`) — C takes any `uint32_t`; there is **no** range check, and the value is truncated to `(unsigned char)type` | writes `type & 0xFF` at `addr[SPX_OFFSET_TYPE]`; no rejection. Out-of-range enum values (`7`, `255`, `256`, `0xFFFFFFFF`) must behave identically in Rust | `err_21_set_type_out_of_range` |
| 22 | `SPX_set_layer_addr` (`address.c:13`) | `layer` `> 255` (e.g. `SPX_D`, `256`, `0xFFFFFFFF`) — truncated to `(unsigned char)`, no check | writes `layer & 0xFF` | `err_22_set_layer_addr_truncation` |
| 23 | `SPX_set_chain_addr` / `SPX_set_hash_addr` / `SPX_set_tree_height` (`address.c:74,82,94`) | value `> 255` — all three truncate to `(unsigned char)`, no check | writes `value & 0xFF` at the respective offset | `err_23_addr_byte_truncation` |
| 24 | `SPX_set_tree_addr` (`address.c:22`) | `tree` with high bits set beyond `SPX_TREE_HEIGHT*(SPX_D-1)` bits (e.g. `u64::MAX`) — no runtime check, all 8 bytes written big-endian | writes all 8 bytes | `err_24_set_tree_addr_full_u64` |
| 25 | `SPX_thash` (`thash_*_{simple,robust}.c`) | `inblocks == 0` — the VLA `buf[SPX_N + 32 + 0]` is legal; the hash is taken over `SPX_N + 32` bytes | produces a well-defined digest; must match | `err_25_thash_inblocks_zero` |
| 26 | `SPX_thash` (blake/sha2 `simple`+`robust`, 192/256 only) | `inblocks > 1` selects the 512-bit path (`SPX_BLAKE512` / `SPX_SHA512` `#if`); `inblocks == 1` selects the 256-bit path. For 128-bit params there is **no** 512 path at all | different digest per branch; both must match the C | `err_26_thash_inblocks_branch` |
| 27 | `SPX_compute_root` (`utils.c:69`) | `tree_height == 0` — loop `for (i=0; i < tree_height-1; i++)` with `uint32_t` underflows to `0xFFFFFFFF` ⇒ reads far past `auth_path` (C UB/crash). **Excluded from differential testing** (documented, not exercised) | out-of-bounds read | — (documented only) |
| 28 | `SPX_compute_root` (`utils.c:69`) | `tree_height == 1` — the loop body never runs, single `thash` over `leaf‖auth_path[0]` | well-defined; must match | `err_28_compute_root_height_one` |
| 29 | `SPX_treehash` (`utils.c:112`) | `tree_height == 0` → `1 << 0 == 1`, one leaf, no combining; `root = leaf` | must match | `err_29_treehash_height_zero` |
| 30 | `SPX_treehash` (`utils.c:112`) | `tree_height >= 32` → `1 << tree_height` on `int` is UB/overflow. **Excluded** (would also take exponential time) | — | — (documented only) |
| 31 | `SPX_chain_lengths` (`wots.c:87`) | `msg` bytes all `0x00` → every `base_w` digit is 0 → checksum is maximal; all `0xFF` → every digit is `w-1` → checksum 0. Boundary values of the checksum path | must match | `err_31_chain_lengths_extremes` |
| 32 | `SPX_wots_gen_leafx1` (`wotsx1.c:28`) | `info->wots_sign_leaf == (uint32_t)~0` (the `merkle_gen_root` sentinel) ⇒ `wots_k_mask = ~0` ⇒ `info->wots_sig` is **never** written (may be NULL) | no write through `wots_sig`; must match | `err_32_wots_gen_leafx1_no_sign` |
| 33 | `SPX_wots_gen_leafx1` (`wotsx1.c:28`) | `leaf_idx == info->wots_sign_leaf` ⇒ `wots_k_mask = 0` ⇒ `wots_sig` **is** written | writes `SPX_WOTS_LEN*SPX_N` bytes | `err_33_wots_gen_leafx1_sign` |
| 34 | `driver` (`PQCgenKAT_sign.c:386`) | `mlen > BASE_MLEN * LOOP_COUNT` (231) — unreachable with the hard-coded loop, but is a real `return KAT_OVERFLOW = -1` branch | `-1` + `"mlen overflow"` on stderr | `err_34_driver_kat_overflow` (structurally unreachable — asserted equal by comparing driver exit codes) |
| 35 | `driver` (`PQCgenKAT_sign.c:400,406,412,413,414`) | any of `crypto_sign_keypair`/`crypto_sign`/`crypto_sign_open` returning nonzero, or `mlen1 != mlen`, or `memcmp(m, m1, mlen) != 0` | `return KAT_CRYPTO_FAILURE = -2` + message on stderr | `err_35_driver_exit_code` (both binaries must exit `0`) |
| 36 | `SPX_gen_message_random` / `SPX_hash_message` | `mlen == 0` (empty message) — no length check anywhere; the hash simply absorbs 0 bytes | well-defined digest; must match | `err_36_hash_message_mlen_zero` |
| 37 | `SPX_blake256_mgf1` / `SPX_blake512_mgf1` / `SPX_mgf1_256` / `SPX_mgf1_512` | `outlen == 0` and `outlen` not a multiple of the block size (32/64) — the final partial block is truncated with no check | must match | `err_37_mgf1_outlen_edge` |
| 38 | `blake256` / `blake512` / `sha256` / `sha512` / `shake256` | `inlen == 0`; `inlen` exactly one block; one block minus 1; one block plus 1; the length-padding boundary (55/56 for BLAKE-256, 111/112 for BLAKE-512) | must match | `err_38_hash_length_boundaries` |
| 39 | `shake256_inc_squeeze` / `haraka_S_inc_squeeze` | `outlen == 0` | writes nothing, returns | `err_39_squeeze_zero_len` |
| 40 | `sha256_inc_blocks` / `sha512_inc_blocks` | `inblocks == 0` | state unchanged | `err_40_inc_blocks_zero` |

### Compile-time-only rejections (`#error`) — unreachable at runtime

| location | condition |
|----------|-----------|
| `params-*.h` | `SPX_WOTS_W` not in `{16, 256}` |
| `params-*.h` | `SPX_WOTS_LEN2` not precomputed for the given `SPX_N` |
| `params-*.h` | `SPX_TREE_HEIGHT * SPX_D != SPX_FULL_HEIGHT` |
| `lib/blake/include/blake.h` | `SPX_BLAKE256_OUTPUT_BYTES < SPX_N` |
| `lib/sha2/include/sha2.h` | `SPX_SHA256_OUTPUT_BYTES < SPX_N` |
| `lib/*/src/hash_*.c` | `SPX_TREE_BITS > 64` |
| `app/src/address.c` | `SPX_TREE_HEIGHT * (SPX_D - 1) > 64` |

None of the 48 valid configurations trips any of these (all 48 C builds
succeeded), so there is nothing to differentially test.

### Deliberately excluded (C is UB / would crash both sides)

* `crypto_sign_verify(NULL, …)`, `crypto_sign(NULL, …)` etc. — the C
  unconditionally dereferences `sig`/`m`/`sk`/`pk`; a null pointer segfaults.
  Only the **`siglen`/`smlen` length** guards (rows 1, 4) are reachable with a
  null buffer, and only in the paths where C returns before dereferencing.
* `randombytes_init(NULL, …)` — C dereferences `entropy_input` unconditionally.
* Row 27 (`compute_root` with `tree_height == 0`) and row 30
  (`treehash` with `tree_height >= 32`).


## Result

Every row that is reachable at runtime has a passing differential test, in all
48 feature combinations:

```
$ bash .verify/run_all.sh
pass: 48   fail: 0
```

`tests/errors.rs` contains one `err_NN_*` test per row; the test name in the
right-hand column above is the exact `#[test]` function name.

Row-by-row status:

| rows | status |
|------|--------|
| 1-26, 28-29, 31-33, 36-40 | [x] passing differential test in all 48 combos |
| 27, 30 | intentionally **not** tested — the C is UB there (`uint32_t` loop-bound underflow / `1 << 32`); documented above, exercising them would segfault both sides equally and prove nothing |
| 34, 35 | covered structurally by the driver comparison (`DRIVER OK rc=0` in all 48 logs): the two `KAT_*` branches are unreachable with the hard-coded loop, and the shared exit code proves both binaries took the same path |
| compile-time `#error`s | unreachable — all 48 valid configurations build |

### Behaviour the tests had to be relaxed for (C is ground truth)

`hash_blake.c` calls `blakeX_update(&S, m, mlen)`, but `blake256_update` /
`blake512_update` take their length in **BITS**
(`blake256.c: blake256_update(&S, in, inlen*8)`). The BLAKE backend therefore
only absorbs `mlen / 8` bytes of the message (and `SPX_N / 8` bytes of `R`,
`SPX_PK_BYTES / 8` bytes of `pk`) in `hash_message` / `gen_message_random`.
A consequence is that under the BLAKE backend `crypto_sign_verify` can still
return `0` for a *corrupted short message*. Rows 2 and 5 therefore assert that C
and Rust return the **same** sentinel, plus a separate assertion that the `-1`
rejection path is genuinely reached at least once (via corrupted signature /
corrupted public key / random signature), instead of demanding `-1` for every
individual corruption. The Rust reproduces the quirk exactly — this is C
behaviour, not something to "fix".

Similarly, `blake256_update(S, data, 0)` is **not** a no-op: `blake256.c:327`
does `else S->buflen = 0`, discarding any buffered bytes. Row 40's blake variant
asserts that quirk explicitly (`buflen == 0` after a zero-length update) and that
both libraries produce the same subsequent digest.
