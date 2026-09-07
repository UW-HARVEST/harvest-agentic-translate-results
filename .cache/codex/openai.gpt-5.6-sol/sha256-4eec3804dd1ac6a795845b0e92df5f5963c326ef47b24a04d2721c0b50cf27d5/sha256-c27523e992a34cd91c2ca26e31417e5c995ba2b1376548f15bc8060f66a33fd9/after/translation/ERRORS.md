# ERRORS.md — the error-surface table

Derived mechanically from the C sources. Every distinct rejection was found with

```sh
grep -rn --include='*.c' -E 'return -|return NULL|assert|abort\(\)|exit\(|RNG_BAD|== NULL|!= NULL' \
    c_src/app/src c_src/lib
grep -rn --include='*.c' -E 'if *\(' c_src/app/src/sign.c
```

which yields exactly these sites (excluding `PQCgenKAT_sign.c`, the driver
program, which is not part of the library):

```
app/src/sign.c:180  return -1;                app/src/rng.c:33   return RNG_BAD_MAXLEN;
app/src/sign.c:236  return -1;                app/src/rng.c:66-67 if (x == NULL) return RNG_BAD_OUTBUF;
app/src/sign.c:272  return -1;                app/src/rng.c:69   return RNG_BAD_REQ_LEN;
app/src/sign.c:280  return -1;                app/src/rng.c:109  abort();     (OpenSSL failure only)
                                              app/src/rng.c:205  if (provided_data != NULL)
```

There are **no** `assert`s anywhere in the library, and none of the hash
backends (`lib/*/src/*.c`) has an error return: `blake256`/`blake512` always
`return 0`, everything else returns `void`.

Sentinel values (`app/include/rng.h`): `RNG_SUCCESS 0`, `RNG_BAD_MAXLEN -1`,
`RNG_BAD_OUTBUF -2`, `RNG_BAD_REQ_LEN -3`.

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `crypto_sign_verify` (`sign.c:179`) | `siglen != SPX_BYTES` — checked before anything else. Tested with `0`, `1`, `SPX_BYTES-1`, `SPX_BYTES+1`, `2*SPX_BYTES`, `SIZE_MAX` | `-1` | `diff_errors.rs::err_verify_wrong_siglen` | [x] |
| 2 | `crypto_sign_verify` (`sign.c:235`) | `memcmp(root, pk + SPX_N, SPX_N) != 0` — the recomputed hypertree root differs. Reached via a 1-bit flip in the signature (R, FORS part, WOTS part, auth path, last byte), a fully random signature, a corrupted `pk` (root half *and* seed half), and a modified long message | `-1` | `diff_errors.rs::err_verify_root_mismatch` | [x] |
| 3 | `crypto_sign_open` (`sign.c:269`) | `smlen < SPX_BYTES`. Side effects are part of the contract: `memset(m, 0, smlen)` and `*mlen = 0`. Tested with `0`, `1`, `2`, `SPX_BYTES/2`, `SPX_BYTES-1` (and `SPX_BYTES` as the accepted boundary) | `-1`, `*mlen == 0`, first `smlen` bytes of `m` zeroed, bytes beyond `smlen` untouched | `diff_errors.rs::err_sign_open_short_smlen`, `::sign_open_smlen_exactly_spx_bytes` | [x] |
| 4 | `crypto_sign_open` (`sign.c:277`) | the inner `crypto_sign_verify` fails. Same side effects (`memset(m, 0, smlen)`, `*mlen = 0`). Reached by flipping a bit inside the signature region and by using the wrong `pk` | `-1`, `*mlen == 0`, `m[0..smlen] == 0` | `diff_errors.rs::err_sign_open_bad_signature` | [x] |
| 5 | `seedexpander_init` (`rng.c:32`) | `maxlen >= 0x100000000`. Tested at `0x1_0000_0000`, `0x1_0000_0001`, `0x2_0000_0000`, `0xffff_ffff_ffff_0000`, `UINT64_MAX`; `0xffff_ffff` is the largest accepted value. The context must be left untouched | `RNG_BAD_MAXLEN` (`-1`) | `diff_rng.rs::err_seedexpander_init_bad_maxlen` | [x] |
| 6 | `seedexpander` (`rng.c:66`) | `x == NULL`. Checked *before* the length check, so it wins even for an otherwise-invalid `xlen` (tested with `xlen` = 0, 1, 16, 1024, 100000) | `RNG_BAD_OUTBUF` (`-2`) | `diff_rng.rs::err_seedexpander_null_outbuf` | [x] |
| 7 | `seedexpander` (`rng.c:68`) | `xlen >= ctx->length_remaining` — note the `>=`, so requesting exactly `length_remaining` is *rejected* and `length_remaining - 1` is the largest accepted request. Tested for `maxlen` ∈ {0,1,2,16,100} × `xlen` ∈ {`maxlen`, `maxlen+1`, `maxlen+1000`, `UINT64_MAX`}, plus `maxlen-1` as the accepted boundary | `RNG_BAD_REQ_LEN` (`-3`) | `diff_rng.rs::err_seedexpander_bad_req_len` | [x] |
| 8 | `AES256_CTR_DRBG_Update` (`rng.c:205`) | `provided_data == NULL` — not an error, but a distinct NULL-pointer branch that skips the 48-byte XOR | no XOR; `Key`/`V` derived from the three AES blocks only | `diff_rng.rs::drbg_update_matches_with_and_without_provided_data` | [x] |
| 9 | `randombytes_init` (`rng.c:141`) | `personalization_string == NULL` — distinct NULL-pointer branch that skips the seed-material XOR | `DRBG_ctx` seeded from `entropy_input` alone | `diff_rng.rs::randombytes_stream_and_drbg_ctx_state` (alternates NULL / non-NULL) | [x] |
| 10 | `randombytes` (`rng.c:154`) | `xlen == 0` — the `while` body never runs, but the trailing `AES256_CTR_DRBG_Update(NULL, …)` and `reseed_counter++` still happen | `RNG_SUCCESS`, `DRBG_ctx` still advanced | `diff_rng.rs::randombytes_zero_length` | [x] |
| 11 | `handleErrors` (`rng.c:107`) | an OpenSSL `EVP_*` call fails → `ERR_print_errors_fp(stderr); abort()` | process abort | **not reachable**: it can only fire if `EVP_CIPHER_CTX_new` / `EVP_EncryptInit_ex` / `EVP_EncryptUpdate` fail for AES-256-ECB, which cannot be provoked from the API. The Rust translation uses a self-contained AES-256 with no failure mode; equivalence of the *successful* path is what `diff_rng.rs::aes256_ecb_matches` establishes over 2000 random (key, block) pairs plus the all-zero/all-ff extremes. | [x] |

## Generic FFI boundaries (not in the table, covered anyway)

| condition | expected | test | ✔ |
|---|---|---|---|
| `thash(out, in, 0, …)` — `inblocks == 0`, a zero-length input region | both hash `pub_seed ‖ addr` only | `diff_errors.rs::boundary_thash_inblocks` | [x] |
| `thash` with `inblocks` one past the largest value the library ever uses (`max(SPX_WOTS_LEN, SPX_FORS_TREES) + 1`) | identical output | `diff_errors.rs::boundary_thash_inblocks` | [x] |
| `ull_to_bytes(out, 0, v)` — zero length: the `for (i = outlen-1; i >= 0; i--)` loop must not run | nothing written | `diff_errors.rs::boundary_zero_and_max_lengths` | [x] |
| `bytes_to_ull(in, 0)` | `0` | `diff_errors.rs::boundary_zero_and_max_lengths` | [x] |
| `bytes_to_ull(in, inlen)` for the whole documented range `0..=8` (`inlen > 8` would shift a `u64` by ≥ 64, i.e. UB in C, so it is out of scope) | identical | `diff_utils_address.rs::bytes_to_ull_all_inlens` | [x] |
| **out-of-range enum across the FFI**: `set_type(addr, t)` takes the `SPX_ADDR_TYPE_*` constants `0..=6` but is a plain `uint32_t`; the C truncates it to one byte. Tested with `7, 8, 9, 100, 255, 256, 257, 0x10006, 0x7fffffff, 0x80000000, 0xffffffff` and 64 random `u32`s, then the resulting address is fed to `prf_addr` and `thash` | identical address bytes and identical hashes | `diff_errors.rs::boundary_out_of_range_addr_type` | [x] |
| the other one-byte address fields (`set_layer_addr`, `set_chain_addr`, `set_hash_addr`, `set_tree_height`) with values past `0xff` | identical truncation | `diff_errors.rs::boundary_out_of_range_addr_fields` | [x] |
| `treehash(..., tree_height = 0, ...)` — `1 << 0 == 1`, so one leaf and no `thash` | identical root/auth path | `diff_errors.rs::boundary_treehash_zero_height` | [x] |
| `crypto_sign_signature` / `crypto_sign_verify` with `mlen == 0` and a **NULL** message pointer (the C never dereferences it) | identical signature; the signature verifies | `diff_errors.rs::boundary_zero_length_message` | [x] |
| `randombytes(NULL, 0)` — NULL output with zero length | `RNG_SUCCESS`, state still advanced | `diff_rng.rs::randombytes_zero_length` | [x] |
| `seedexpander_init` at `maxlen = 0` followed by any request | `RNG_BAD_REQ_LEN` | `diff_rng.rs::err_seedexpander_bad_req_len` | [x] |

## Conditions deliberately **not** tested (undefined behaviour in the C)

These are documented rather than exercised, because the C reference itself has
no defined behaviour for them, so there is no ground truth to compare against:

* `compute_root(..., tree_height = 0, ...)` — `for (i = 0; i < tree_height - 1; i++)`
  with `uint32_t tree_height == 0` gives `0xFFFFFFFF`, so the C reads ~4 GiB
  past the end of `auth_path` and crashes.
* `bytes_to_ull(in, inlen > 8)` and `ull_to_bytes(out, outlen > 8, …)` —
  shifting a `unsigned long long` by ≥ 64 bits.
* `treehash(..., tree_height >= 31, ...)` — `1 << tree_height` on a signed
  `int`.
* Passing `NULL` for `pk` / `sk` / `sig` / `addr` / `ctx`, or a buffer shorter
  than the size the header mandates: the C dereferences these unconditionally
  and segfaults. (The two places where the C *does* have a NULL check —
  `seedexpander`'s `x` and `AES256_CTR_DRBG_Update`/`randombytes_init`'s
  optional inputs — are rows 6, 8 and 9 above.)
