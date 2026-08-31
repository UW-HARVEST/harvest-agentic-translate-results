## Area 8 — crypto_pwhash + crypto_ipcrypt

Files covered: `crypto_pwhash/crypto_pwhash.c`; `crypto_pwhash/argon2/{argon2.c, argon2-core.c,
argon2-encoding.c, argon2-fill-block-ref.c, blake2b-long.c, pwhash_argon2i.c, pwhash_argon2id.c}`;
`crypto_pwhash/scryptsalsa208sha256/{crypto_scrypt-common.c, pbkdf2-sha256.c,
pwhash_scryptsalsa208sha256.c, scrypt_platform.c, nosse/pwhash_scryptsalsa208sha256_nosse.c}`;
`crypto_ipcrypt/{crypto_ipcrypt.c, ipcrypt_soft.c}`; headers `crypto_pwhash.h`,
`crypto_pwhash_argon2i.h`, `crypto_pwhash_argon2id.h`, `crypto_pwhash_scryptsalsa208sha256.h`,
`crypto_ipcrypt.h`, `argon2.h`, `argon2-core.h`, `argon2-encoding.h`, `crypto_scrypt.h`.

Numeric constants assumed below are those of a 64-bit Linux build (`SIZE_MAX = 2^64-1`,
`HAVE_MMAP`, no `HAVE_*INTRIN_H`/`HAVE_ARMCRYPTO`, so `argon2_fill_segment_ref`,
`escrypt_kdf_nosse` and `ipcrypt_soft_implementation` are the selected implementations):

| symbol | value |
|---|---|
| `crypto_pwhash_BYTES_MIN` / `_MAX` | 16 / 4294967295 |
| `crypto_pwhash_PASSWD_MIN` / `_MAX` | 0 / 4294967295 |
| `crypto_pwhash_SALTBYTES` / `_STRBYTES` | 16 / 128 |
| `crypto_pwhash_argon2i_OPSLIMIT_MIN` / `argon2id_OPSLIMIT_MIN` | 3 / 1 |
| `crypto_pwhash_argon2*_OPSLIMIT_MAX` | 4294967295 |
| `crypto_pwhash_argon2*_MEMLIMIT_MIN` / `_MAX` | 8192 / 4398046510080 |
| `crypto_pwhash_argon2i_STRPREFIX` / `argon2id_STRPREFIX` | `"$argon2i$"` / `"$argon2id$"` |
| `ARGON2_MIN_OUTLEN` / `MAX_OUTLEN` | 16 / 0xFFFFFFFF |
| `ARGON2_MIN_SALT_LENGTH` / `MAX_SALT_LENGTH` | 8 / 0xFFFFFFFF |
| `ARGON2_MIN_MEMORY` / `MAX_MEMORY` | 8 / 0xFFFFFFFF |
| `ARGON2_MIN_LANES` / `MAX_LANES` / `MIN_THREADS` / `MAX_THREADS` | 1 / 0xFFFFFF / 1 / 0xFFFFFF |
| `ARGON2_MIN_TIME` / `MAX_TIME` | 1 / 0xFFFFFFFF |
| `ARGON2_VERSION_NUMBER` | 0x13 (decimal 19) |
| `crypto_pwhash_scryptsalsa208sha256_BYTES_MIN` / `_MAX` | 16 / 0x1fffffffe0 (137438953440) |
| `..._scryptsalsa208sha256_SALTBYTES` / `_STRBYTES` | 32 / 102 (string body is exactly 101 chars) |
| `..._scryptsalsa208sha256_OPSLIMIT_MIN` / `MEMLIMIT_MIN` | 32768 / 16777216 (**not enforced**, see 8.118) |
| `crypto_ipcrypt_{BYTES,KEYBYTES}` | 16 / 16 |
| `crypto_ipcrypt_ND_{KEYBYTES,TWEAKBYTES,INPUTBYTES,OUTPUTBYTES}` | 16 / 8 / 16 / 24 |
| `crypto_ipcrypt_NDX_{KEYBYTES,TWEAKBYTES,INPUTBYTES,OUTPUTBYTES}` | 32 / 16 / 16 / 32 |
| `crypto_ipcrypt_PFX_{KEYBYTES,BYTES}` | 32 / 16 |

`argon2_error_codes` values used below: `ARGON2_OK`=0, `OUTPUT_PTR_NULL`=-1, `OUTPUT_TOO_SHORT`=-2,
`OUTPUT_TOO_LONG`=-3, `PWD_TOO_SHORT`=-4, `PWD_TOO_LONG`=-5, `SALT_TOO_SHORT`=-6, `SALT_TOO_LONG`=-7,
`AD_TOO_SHORT`=-8, `AD_TOO_LONG`=-9, `SECRET_TOO_SHORT`=-10, `SECRET_TOO_LONG`=-11,
`TIME_TOO_SMALL`=-12, `TIME_TOO_LARGE`=-13, `MEMORY_TOO_LITTLE`=-14, `MEMORY_TOO_MUCH`=-15,
`LANES_TOO_FEW`=-16, `LANES_TOO_MANY`=-17, `PWD_PTR_MISMATCH`=-18, `SALT_PTR_MISMATCH`=-19,
`SECRET_PTR_MISMATCH`=-20, `AD_PTR_MISMATCH`=-21, `MEMORY_ALLOCATION_ERROR`=-22,
`FREE_MEMORY_CBK_NULL`=-23, `ALLOCATE_MEMORY_CBK_NULL`=-24, `INCORRECT_PARAMETER`=-25,
`INCORRECT_TYPE`=-26, `OUT_PTR_MISMATCH`=-27, `THREADS_TOO_FEW`=-28, `THREADS_TOO_MANY`=-29,
`MISSING_ARGS`=-30, `ENCODING_FAIL`=-31, `DECODING_FAIL`=-32, `THREAD_FAIL`=-33,
`DECODING_LENGTH_FAIL`=-34, `VERIFY_MISMATCH`=-35.

### ERROR SURFACE

| # | function | trigger (exact invalid input/condition) | expected C result | status |
|---|----------|------------------------------------------|-------------------|--------|
| 8.1 | `crypto_pwhash` | `alg` is not `crypto_pwhash_ALG_ARGON2I13`(1) nor `ALG_ARGON2ID13`(2): e.g. `alg = 0` | `-1`, `errno = EINVAL`; `out` untouched (dispatch happens before any memset) | verified |
| 8.2 | `crypto_pwhash` | `alg = 3` (above the last valid id) | `-1`, `errno = EINVAL` | verified |
| 8.3 | `crypto_pwhash` | `alg = -1` | `-1`, `errno = EINVAL` | verified |
| 8.4 | `crypto_pwhash_str_alg` | `alg` not in {1,2}, e.g. `alg = 0` | `sodium_misuse()` → prints a message and `abort()`s; the function never returns (the trailing `return -1` is unreachable) | verified |
| 8.5 | `crypto_pwhash_str_verify` | `str` starts with neither `"$argon2id$"` nor `"$argon2i$"`, e.g. `"$7$..."` or `"$argon2d$v=19$..."` or `""` | `-1`, `errno = EINVAL` (no argon2 work done) | verified |
| 8.6 | `crypto_pwhash_str_needs_rehash` | same prefix condition as 8.5 | `-1`, `errno = EINVAL` | verified |
| 8.7 | `crypto_pwhash_argon2i` | `outlen > crypto_pwhash_argon2i_BYTES_MAX` (4294967295), e.g. `outlen = 4294967296` | `-1`, `errno = EFBIG` (note: `memset(out, 0, outlen)` already ran, i.e. the caller's buffer is zeroed / UB if shorter) | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >4 GiB caller buffer) |
| 8.8 | `crypto_pwhash_argon2i` | `outlen < crypto_pwhash_argon2i_BYTES_MIN` (16): `outlen = 0`, `1`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.9 | `crypto_pwhash_argon2i` | `passwdlen > crypto_pwhash_argon2i_PASSWD_MAX` (4294967295) | `-1`, `errno = EFBIG` | verified |
| 8.10 | `crypto_pwhash_argon2i` | `opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX` (4294967295), e.g. `4294967296` | `-1`, `errno = EFBIG` | verified |
| 8.11 | `crypto_pwhash_argon2i` | `memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX` (4398046510080) | `-1`, `errno = EFBIG` | verified |
| 8.12 | `crypto_pwhash_argon2i` | `opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN` (3): `opslimit = 0`, `1`, `2` | `-1`, `errno = EINVAL` | verified |
| 8.13 | `crypto_pwhash_argon2i` | `memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN` (8192): `memlimit = 0`, `1024`, `8191` | `-1`, `errno = EINVAL` | verified |
| 8.14 | `crypto_pwhash_argon2i` | `passwdlen < crypto_pwhash_argon2i_PASSWD_MIN` (0) | unreachable (`PASSWD_MIN == 0`); documented dead branch, would be `-1`/`EINVAL` | unreachable-from-public-API (PASSWD_MIN == 0) |
| 8.15 | `crypto_pwhash_argon2i` | `(const void *) out == (const void *) passwd` (output aliases password) | `-1`, `errno = EINVAL` | verified |
| 8.16 | `crypto_pwhash_argon2i` | `alg != crypto_pwhash_argon2i_ALG_ARGON2I13` (1) — e.g. `alg = 2` (`ARGON2ID13`) passed to the argon2i entry point | `-1`, `errno = EINVAL` (switch `default`) | verified |
| 8.17 | `crypto_pwhash_argon2i` | inner `argon2i_hash_raw() != ARGON2_OK` (only reachable via memory-allocation failure) | `-1`, `errno` left as set by the allocator | unreachable-from-public-API (inner allocation failure) |
| 8.18 | `crypto_pwhash_argon2id` | `outlen > crypto_pwhash_argon2id_BYTES_MAX` (4294967295) | `-1`, `errno = EFBIG` | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >4 GiB caller buffer) |
| 8.19 | `crypto_pwhash_argon2id` | `outlen < 16`: `0`, `1`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.20 | `crypto_pwhash_argon2id` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.21 | `crypto_pwhash_argon2id` | `opslimit > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.22 | `crypto_pwhash_argon2id` | `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.23 | `crypto_pwhash_argon2id` | `opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN` (1), i.e. `opslimit = 0` | `-1`, `errno = EINVAL` | verified |
| 8.24 | `crypto_pwhash_argon2id` | `memlimit < 8192`: `0`, `8191` | `-1`, `errno = EINVAL` | verified |
| 8.25 | `crypto_pwhash_argon2id` | `out == passwd` | `-1`, `errno = EINVAL` | verified |
| 8.26 | `crypto_pwhash_argon2id` | `alg != 2` — e.g. `alg = 1` (`ARGON2I13`) passed to the argon2id entry point, or `alg = 0` | `-1`, `errno = EINVAL` | verified |
| 8.27 | `crypto_pwhash_argon2id` | inner `argon2id_hash_raw() != ARGON2_OK` | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.28 | `crypto_pwhash_argon2i_str` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` (`out` already fully zeroed) | verified |
| 8.29 | `crypto_pwhash_argon2i_str` | `opslimit > 4294967295` | `-1`, `errno = EFBIG` | verified |
| 8.30 | `crypto_pwhash_argon2i_str` | `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.31 | `crypto_pwhash_argon2i_str` | `opslimit < 3` (`0`,`1`,`2`) | `-1`, `errno = EINVAL` | verified |
| 8.32 | `crypto_pwhash_argon2i_str` | `memlimit < 8192` | `-1`, `errno = EINVAL` | verified |
| 8.33 | `crypto_pwhash_argon2i_str` | `argon2i_hash_encoded() != ARGON2_OK` (encoding buffer too small / allocation failure) | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.34 | `crypto_pwhash_argon2id_str` | `passwdlen > 4294967295` / `opslimit > 4294967295` / `memlimit > 4398046510080` | `-1`, `errno = EFBIG` | verified |
| 8.35 | `crypto_pwhash_argon2id_str` | `opslimit < 1` (i.e. `0`) | `-1`, `errno = EINVAL` | verified |
| 8.36 | `crypto_pwhash_argon2id_str` | `memlimit < 8192` | `-1`, `errno = EINVAL` | verified |
| 8.37 | `crypto_pwhash_argon2id_str` | `argon2id_hash_encoded() != ARGON2_OK` | `-1` | unreachable-from-public-API (inner allocation failure) |
| 8.38 | `crypto_pwhash_argon2i_str_verify` | `passwdlen > crypto_pwhash_argon2i_PASSWD_MAX` (4294967295) | `-1`, `errno = EFBIG` | verified |
| 8.39 | `crypto_pwhash_argon2i_str_verify` | `passwdlen < PASSWD_MIN` (0) | unreachable dead branch (would be `-1`/`EINVAL`) | unreachable-from-public-API (PASSWD_MIN == 0) |
| 8.40 | `crypto_pwhash_argon2i_str_verify` | correct string, wrong password → `argon2i_verify` returns `ARGON2_VERIFY_MISMATCH` (-35) | `-1`, `errno = EINVAL` | verified |
| 8.41 | `crypto_pwhash_argon2i_str_verify` | malformed `str` (any `argon2_decode_string` failure, see 8.79–8.100) | `-1`, `errno` **not** set by this function (only `VERIFY_MISMATCH` sets `EINVAL`) | verified |
| 8.42 | `crypto_pwhash_argon2i_str_verify` | `str` is an argon2**id** string, e.g. `"$argon2id$v=19$m=8,t=1,p=1$<salt>$<hash>"` — `CC("$argon2i")` matches but the next `CC("$v=")` sees `"d$v="` | `-1` (inner `ARGON2_DECODING_FAIL` = -32) | verified |
| 8.43 | `crypto_pwhash_argon2i_str_verify` | `str = ""` (empty) | `-1` (inner `ARGON2_DECODING_FAIL`) | verified |
| 8.44 | `crypto_pwhash_argon2id_str_verify` | `passwdlen > 4294967295` | `-1`, `errno = EFBIG` (LCOV-excluded branch) | verified |
| 8.45 | `crypto_pwhash_argon2id_str_verify` | wrong password (`ARGON2_VERIFY_MISMATCH`) | `-1`, `errno = EINVAL` | verified |
| 8.46 | `crypto_pwhash_argon2id_str_verify` | malformed `str` / wrong prefix (`"$argon2i$v=19$..."` fails `CC("$argon2id")`) | `-1` (inner `ARGON2_DECODING_FAIL`) | verified |
| 8.47 | `_needs_rehash` (via `crypto_pwhash_argon2i_str_needs_rehash` / `crypto_pwhash_argon2id_str_needs_rehash`) | `opslimit > UINT32_MAX`, e.g. `4294967296` | `-1`, `errno = EINVAL` | verified |
| 8.48 | `_needs_rehash` | `memlimit / 1024U > UINT32_MAX`, i.e. `memlimit > 4398046511104` | `-1`, `errno = EINVAL` | verified |
| 8.49 | `_needs_rehash` | `strlen(str) >= crypto_pwhash_STRBYTES` (128) | `-1`, `errno = EINVAL` | verified |
| 8.50 | `_needs_rehash` | `calloc(strlen(str), 1)` returns NULL (OOM) | `-1` (errno from `calloc`) | unreachable-from-public-API (calloc() failure) |
| 8.51 | `_needs_rehash` | `argon2_decode_string()` fails (malformed string, wrong type, bad version, bad base64, trailing garbage, salt < 8 bytes, hash < 16 bytes, …) | `-1`, `errno = EINVAL` | verified |
| 8.52 | `_needs_rehash` | valid string but `ctx.t_cost != (uint32_t) opslimit` **or** `ctx.m_cost != (uint32_t) (memlimit/1024)` | `1` (non-zero, non-error “needs rehash”); note `p`/lanes and the argon2 *type* are **not** compared | verified |
| 8.53 | `argon2_ctx` | `argon2_validate_inputs(context) != ARGON2_OK` | that validation code is returned verbatim (see 8.58–8.78) | verified |
| 8.54 | `argon2_ctx` | `type` is neither `Argon2_id`(2) nor `Argon2_i`(1), e.g. `type = 0` or `3` (Argon2_d is not compiled in) | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.55 | `argon2_ctx` | `argon2_initialize()` fails | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` (or `ARGON2_INCORRECT_PARAMETER` = `-25` if instance/context NULL) | unreachable-from-public-API (allocation failure) |
| 8.56 | `argon2_ctx` | `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` (from `argon2_validate_inputs`) | verified |
| 8.57 | `argon2_validate_inputs` | `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` | verified |
| 8.58 | `argon2_validate_inputs` | `context->out == NULL` | `ARGON2_OUTPUT_PTR_NULL` = `-1` | verified |
| 8.59 | `argon2_validate_inputs` | `context->outlen < ARGON2_MIN_OUTLEN` (16): `0`, `1`, `15` | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.60 | `argon2_validate_inputs` | `context->outlen > ARGON2_MAX_OUTLEN` (0xFFFFFFFF) | `ARGON2_OUTPUT_TOO_LONG` = `-3`; **unreachable through `argon2_context`** because `outlen` is `uint32_t` (reachable only via `argon2_hash`, row 8.72) | unreachable-from-public-API (outlen is uint32_t (reachable via argon2_hash, row 8.84)) |
| 8.61 | `argon2_validate_inputs` | `context->pwd == NULL && context->pwdlen != 0` | `ARGON2_PWD_PTR_MISMATCH` = `-18` | verified |
| 8.62 | `argon2_validate_inputs` | `context->pwdlen < ARGON2_MIN_PWD_LENGTH` (0) | `ARGON2_PWD_TOO_SHORT` = `-4`; unreachable (min is 0 and the field is unsigned) | unreachable-from-public-API (ARGON2_MIN_PWD_LENGTH == 0) |
| 8.63 | `argon2_validate_inputs` | `context->pwdlen > ARGON2_MAX_PWD_LENGTH` (0xFFFFFFFF) | `ARGON2_PWD_TOO_LONG` = `-5`; unreachable via the `uint32_t` field (reachable via `argon2_hash`, row 8.71) | unreachable-from-public-API (pwdlen is uint32_t (reachable via argon2_hash, row 8.83)) |
| 8.64 | `argon2_validate_inputs` | `context->salt == NULL && context->saltlen != 0` | `ARGON2_SALT_PTR_MISMATCH` = `-19` | verified |
| 8.65 | `argon2_validate_inputs` | `context->saltlen < ARGON2_MIN_SALT_LENGTH` (8): `0` (with `salt == NULL`), `1`, `7` | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.66 | `argon2_validate_inputs` | `context->saltlen > ARGON2_MAX_SALT_LENGTH` (0xFFFFFFFF) | `ARGON2_SALT_TOO_LONG` = `-7`; unreachable via the `uint32_t` field (reachable via `argon2_hash`, row 8.73) | unreachable-from-public-API (saltlen is uint32_t (reachable via argon2_hash, row 8.85)) |
| 8.67 | `argon2_validate_inputs` | `context->secret == NULL && context->secretlen != 0` | `ARGON2_SECRET_PTR_MISMATCH` = `-20` | verified |
| 8.68 | `argon2_validate_inputs` | `secret != NULL && secretlen < ARGON2_MIN_SECRET` (0) | `ARGON2_SECRET_TOO_SHORT` = `-10`; unreachable (min is 0) | unreachable-from-public-API (ARGON2_MIN_SECRET == 0) |
| 8.69 | `argon2_validate_inputs` | `secret != NULL && secretlen > ARGON2_MAX_SECRET` (0xFFFFFFFF) | `ARGON2_SECRET_TOO_LONG` = `-11`; unreachable (`uint32_t` field) | unreachable-from-public-API (secretlen is uint32_t) |
| 8.70 | `argon2_validate_inputs` | `context->ad == NULL && context->adlen != 0` | `ARGON2_AD_PTR_MISMATCH` = `-21` | verified |
| 8.71 | `argon2_validate_inputs` | `ad != NULL && adlen < ARGON2_MIN_AD_LENGTH` (0) | `ARGON2_AD_TOO_SHORT` = `-8`; unreachable (min is 0) | unreachable-from-public-API (ARGON2_MIN_AD_LENGTH == 0) |
| 8.72 | `argon2_validate_inputs` | `ad != NULL && adlen > ARGON2_MAX_AD_LENGTH` (0xFFFFFFFF) | `ARGON2_AD_TOO_LONG` = `-9`; unreachable (`uint32_t` field) | unreachable-from-public-API (adlen is uint32_t) |
| 8.73 | `argon2_validate_inputs` | `context->lanes < ARGON2_MIN_LANES` (1), i.e. `lanes = 0` | `ARGON2_LANES_TOO_FEW` = `-16` | verified |
| 8.74 | `argon2_validate_inputs` | `context->lanes > ARGON2_MAX_LANES` (0xFFFFFF), e.g. `lanes = 0x1000000` | `ARGON2_LANES_TOO_MANY` = `-17` | verified |
| 8.75 | `argon2_validate_inputs` | `context->m_cost < ARGON2_MIN_MEMORY` (8): `m_cost = 0..7` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.76 | `argon2_validate_inputs` | `context->m_cost > ARGON2_MAX_MEMORY` (0xFFFFFFFF on this build) | `ARGON2_MEMORY_TOO_MUCH` = `-15`; unreachable because `m_cost` is `uint32_t` and `ARGON2_MAX_MEMORY == UINT32_MAX` here (reachable on platforms where `ARGON2_MAX_MEMORY_BITS < 32`, e.g. 32-bit `void *` → max 2^21) | unreachable-from-public-API (ARGON2_MAX_MEMORY == UINT32_MAX on this build) |
| 8.77 | `argon2_validate_inputs` | second memory check: `m_cost < 8 * lanes` with `m_cost >= 8`, e.g. `lanes = 4, m_cost = 31` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` (distinct branch from 8.75) | verified |
| 8.78 | `argon2_validate_inputs` | `context->t_cost < ARGON2_MIN_TIME` (1), i.e. `t_cost = 0` | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.79 | `argon2_validate_inputs` | `context->t_cost > ARGON2_MAX_TIME` (0xFFFFFFFF) | `ARGON2_TIME_TOO_LARGE` = `-13`; unreachable (`uint32_t` field) | unreachable-from-public-API (t_cost is uint32_t) |
| 8.80 | `argon2_validate_inputs` | `context->threads < ARGON2_MIN_THREADS` (1), i.e. `threads = 0` (with `lanes >= 1`) | `ARGON2_THREADS_TOO_FEW` = `-28` | verified |
| 8.81 | `argon2_validate_inputs` | `context->threads > ARGON2_MAX_THREADS` (0xFFFFFF) | `ARGON2_THREADS_TOO_MANY` = `-29` | verified |
| 8.82 | `argon2_validate_inputs` | (never produced) `FREE_MEMORY_CBK_NULL` -23, `ALLOCATE_MEMORY_CBK_NULL` -24, `OUT_PTR_MISMATCH` -27, `MISSING_ARGS` -30, `THREAD_FAIL` -33 | dead enum values in this libsodium fork: no code path returns them | unreachable-from-public-API (dead enum values; no code path returns them) |
| 8.83 | `argon2_hash` | `pwdlen > ARGON2_MAX_PWD_LENGTH` (0xFFFFFFFF) — reachable because `pwdlen` is `size_t` | `ARGON2_PWD_TOO_LONG` = `-5` (checked *after* `randombytes_buf(hash, hashlen)` has already overwritten the caller's `hash` buffer) | verified |
| 8.84 | `argon2_hash` | `hashlen > ARGON2_MAX_OUTLEN` (0xFFFFFFFF) | `ARGON2_OUTPUT_TOO_LONG` = `-3` | verified |
| 8.85 | `argon2_hash` | `saltlen > ARGON2_MAX_SALT_LENGTH` (0xFFFFFFFF) | `ARGON2_SALT_TOO_LONG` = `-7` | verified |
| 8.86 | `argon2_hash` | `malloc(hashlen)` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.87 | `argon2_hash` | any `argon2_ctx` failure (e.g. `saltlen = 4` → `-6`; `m_cost = 4` → `-14`; `t_cost = 0` → `-12`; `parallelism = 0` → `-16`; `hashlen = 8` → `-2`) | that code is returned verbatim; `out` scratch buffer is zeroed and freed | verified |
| 8.88 | `argon2_hash` | `encoded != NULL && encodedlen != 0` and `argon2_encode_string()` fails (buffer too small) | `ARGON2_ENCODING_FAIL` = `-31`; both `out` and `encoded` are zeroed | verified |
| 8.89 | `argon2i_hash_encoded` / `argon2id_hash_encoded` | `encodedlen` smaller than the required encoded length, e.g. `encodedlen = 10` with `hashlen = 32` | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.90 | `argon2i_hash_raw` / `argon2id_hash_raw` | `hashlen < 16` (e.g. 8) | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.91 | `argon2i_hash_raw` / `argon2id_hash_raw` | `saltlen < 8` (e.g. 4) | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.92 | `argon2i_hash_raw` / `argon2id_hash_raw` | `parallelism = 0` | `ARGON2_LANES_TOO_FEW` = `-16` | verified |
| 8.93 | `argon2i_hash_raw` / `argon2id_hash_raw` | `t_cost = 0` | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.94 | `argon2i_hash_raw` / `argon2id_hash_raw` | `m_cost < 8` or `m_cost < 8 * parallelism` (e.g. `m_cost = 8, parallelism = 2`) | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.95 | `argon2_verify` (`argon2i_verify` / `argon2id_verify`) | `strlen(encoded) > UINT32_MAX` | `ARGON2_DECODING_LENGTH_FAIL` = `-34` | unreachable-from-public-API (strlen() cannot exceed UINT32_MAX here) |
| 8.96 | `argon2_verify` | any of the three `malloc(strlen(encoded))` (ad/salt/out) or the fourth `malloc(ctx.outlen)` returns NULL — including `encoded = ""` on implementations where `malloc(0)` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.97 | `argon2_verify` | `argon2_decode_string()` != OK | that decode code is returned verbatim (`-32`, `-26`, or a validation code) | verified |
| 8.98 | `argon2_verify` | decode OK, re-hash OK, `sodium_memcmp(out, ctx.out, ctx.outlen) != 0` (wrong password) | `ARGON2_VERIFY_MISMATCH` = `-35` | verified |
| 8.99 | `argon2_verify` | decode OK but the recomputation `argon2_hash(...)` fails (allocation failure) | that code is returned; **no** mismatch conversion (the `ret == ARGON2_OK` guard) | unreachable-from-public-API (allocation failure) |
| 8.100 | `argon2_verify` | `type` not `Argon2_i`/`Argon2_id` (reaches `argon2_decode_string`’s `else`) | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.101 | `argon2_initialize` | `instance == NULL` or `context == NULL` | `ARGON2_INCORRECT_PARAMETER` = `-25` | verified |
| 8.102 | `argon2_initialize` | `malloc(sizeof(uint64_t) * segment_length)` for `pseudo_rands` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.103 | `allocate_memory` (static, via `argon2_initialize`) | `region == NULL` | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (region is never NULL at the call site) |
| 8.104 | `allocate_memory` | `m_cost == 0`, or `sizeof(block) * m_cost` overflows (`memory_size / m_cost != sizeof(block)`) | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (m_cost >= 8 is already validated) |
| 8.105 | `allocate_memory` | `malloc(sizeof(block_region))` returns NULL | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22` | unreachable-from-public-API (malloc() failure) |
| 8.106 | `allocate_memory` | `mmap()` fails (`MAP_FAILED`) — e.g. m_cost near `ARGON2_MAX_MEMORY` (4 TiB) | `ARGON2_MEMORY_ALLOCATION_ERROR` = `-22`; `*region` freed and set to NULL | unreachable-from-public-API (would mmap(MAP_POPULATE) 4 TiB - not safely testable) |
| 8.107 | `blake2b_long` | `outlen > UINT32_MAX` | `-1` (goto fail with `ret` still `-1`); unreachable from argon2 | verified |
| 8.108 | `blake2b_long` | `outlen == 0` (or any `outlen` rejected by `crypto_generichash_blake2b_init`, which requires `1 <= outlen <= 64` for the short path) | `-1` (the value returned by the failing `crypto_generichash_blake2b_*` call) | verified |
| 8.109 | `argon2_finalize` | `blake2b_long()` fails | **return value is ignored**: `argon2_finalize` is `void`, so `context->out` is left unmodified and `argon2_ctx` still returns `ARGON2_OK`. Silent-failure path (unreachable in practice since `outlen >= 16`) | unreachable-from-public-API (outlen >= 16 is already validated) |
| 8.110 | `argon2_fill_memory_blocks` | `instance == NULL` or `instance->lanes == 0` | returns early (`void`); no error signalled | verified |
| 8.111 | `argon2_fill_segment_ref` | `instance == NULL` | returns early (`void`) | verified |
| 8.112 | `argon2_decode_string` | `type` neither `Argon2_i` nor `Argon2_id` | `ARGON2_INCORRECT_TYPE` = `-26` | verified |
| 8.113 | `argon2_decode_string` | wrong type prefix: `"$argon2id..."` decoded as `Argon2_i` succeeds at `CC("$argon2i")` but then fails `CC("$v=")`; `"$argon2i..."` decoded as `Argon2_id` fails `CC("$argon2id")` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.114 | `argon2_decode_string` | prefix garbage: `""`, `"argon2i$v=19$..."` (no leading `$`), `"$argon2d$v=19$..."`, `"$ARGON2I$..."` (case-sensitive), `"$argon"` (truncated) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.115 | `argon2_decode_string` | missing `$v=`: `"$argon2id$m=8,t=1,p=1$..."` (libsodium requires the version field; the `CC_opt` optional-prefix macro is defined but unused) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.116 | `argon2_decode_string` | `v=` value is not a minimal decimal: `"$argon2id$v=$..."` (no digit), `"$argon2id$v=019$..."` (leading zero), `"$argon2id$v=+19$..."`, `"$argon2id$v=1a9$..."` (stops at `a`, later `CC("$m=")` fails) | `ARGON2_DECODING_FAIL` = `-32` (via `decode_decimal`/`DECIMAL_U32` returning NULL, or the following `CC`) | verified |
| 8.117 | `argon2_decode_string` | `v=` value `> UINT32_MAX`, e.g. `"v=4294967296"`; or so long it overflows `unsigned long` | `ARGON2_DECODING_FAIL` = `-32` (`DECIMAL_U32` rejects `dec_x > UINT32_MAX`; `decode_decimal` rejects `acc > ULONG_MAX/10`) | verified |
| 8.118 | `argon2_decode_string` | `version != ARGON2_VERSION_NUMBER` (0x13 = 19): `"v=16"`, `"v=0"`, `"v=20"` | `ARGON2_INCORRECT_TYPE` = `-26` (note: *not* `DECODING_FAIL`) | verified |
| 8.119 | `argon2_decode_string` | missing `$m=` after the version, e.g. `"$argon2id$v=19$t=1,p=1$..."` or `"$argon2id$v=19"` (truncated) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.120 | `argon2_decode_string` | bad `m=` value: `"m="` (empty), `"m=08"` (leading zero), `"m=4294967296"` (> UINT32_MAX), `"m=-8"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.121 | `argon2_decode_string` | missing `,t=`: `"$argon2id$v=19$m=8$..."` or `"$argon2id$v=19$m=8;t=1..."` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.122 | `argon2_decode_string` | bad `t=` value: `"t="`, `"t=01"`, `"t=4294967296"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.123 | `argon2_decode_string` | missing `,p=`: `"$argon2id$v=19$m=8,t=1$..."` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.124 | `argon2_decode_string` | bad `p=` value: `"p="`, `"p=01"`, `"p=4294967296"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.125 | `argon2_decode_string` | the three `if (ctx->m_cost / t_cost / lanes > UINT32_MAX)` guards after each `DECIMAL_U32` | `ARGON2_INCORRECT_TYPE` = `-26`; dead code (the values are already `uint32_t`) | verified |
| 8.126 | `argon2_decode_string` | missing `$` before the salt: `"$argon2id$v=19$m=8,t=1,p=1<salt>$<hash>"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.127 | `argon2_decode_string` | salt Base64 decodes to more than `maxsaltlen` bytes (`sodium_base642bin` → `ERANGE`) — i.e. salt longer than the caller's buffer (`ctx->saltlen` on entry) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.128 | `argon2_decode_string` | salt Base64 has invalid trailing bits (`acc_len > 4` or non-zero low bits), e.g. `"...$c29tZQ=="` (padding is rejected: `ORIGINAL_NO_PADDING` variant) or a 1-char group | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.129 | `argon2_decode_string` | salt field empty (`"$argon2id$v=19$m=8,t=1,p=1$$<hash>"`) → `saltlen = 0` and `salt != NULL` | `ARGON2_SALT_TOO_SHORT` = `-6` from the `argon2_validate_inputs` call at the end | verified |
| 8.130 | `argon2_decode_string` | salt shorter than 8 bytes after decoding, e.g. base64 of 4 bytes | `ARGON2_SALT_TOO_SHORT` = `-6` | verified |
| 8.131 | `argon2_decode_string` | missing `$` between salt and hash, e.g. `"...$<salt><hash>"` (the salt Base64 consumes both, then `CC("$")` sees NUL) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.132 | `argon2_decode_string` | hash Base64 exceeds `maxoutlen` (`ctx->outlen` on entry) | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.133 | `argon2_decode_string` | hash Base64 invalid / truncated group, e.g. `"...$Zg"` decodes to 1 byte | `ARGON2_OUTPUT_TOO_SHORT` = `-2` (validation) — or `-32` if the bit-padding check fails first | verified |
| 8.134 | `argon2_decode_string` | hash field empty (`"...$<salt>$"`) → `outlen = 0` | `ARGON2_OUTPUT_TOO_SHORT` = `-2` | verified |
| 8.135 | `argon2_decode_string` | `p=0` in the string (lanes 0) | `ARGON2_LANES_TOO_FEW` = `-16` (from the final `argon2_validate_inputs`; note `threads` is set from `lanes`, so `-28` is not reached first) | verified |
| 8.136 | `argon2_decode_string` | `t=0` in the string | `ARGON2_TIME_TOO_SMALL` = `-12` | verified |
| 8.137 | `argon2_decode_string` | `m=0`..`m=7` in the string | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.138 | `argon2_decode_string` | `m` valid but `m < 8 * p`, e.g. `"m=8,t=1,p=2"` | `ARGON2_MEMORY_TOO_LITTLE` = `-14` | verified |
| 8.139 | `argon2_decode_string` | trailing garbage after the hash: `*str != 0`, e.g. `"...$<hash>$"`, `"...$<hash>x"`, `"...$<hash>\n"` | `ARGON2_DECODING_FAIL` = `-32` | verified |
| 8.140 | `argon2_decode_string` | via `argon2_verify` with `ctx.pwd == NULL, pwdlen == 0` — decoding leaves `ctx->pwd` NULL; if a caller sets `pwd = NULL, pwdlen != 0` | `ARGON2_PWD_PTR_MISMATCH` = `-18` from the final validation | verified |
| 8.141 | `decode_decimal` (static) | no digit at all at the current position | `NULL` → caller yields `ARGON2_DECODING_FAIL` | verified |
| 8.142 | `decode_decimal` | non-minimal encoding: first char `'0'` and more than one digit consumed (`"00"`, `"007"`, `"019"`) — note bare `"0"` **is** accepted | `NULL` → `ARGON2_DECODING_FAIL` | verified |
| 8.143 | `decode_decimal` | value overflows `unsigned long`: `acc > ULONG_MAX/10` before the multiply, or `c > ULONG_MAX - acc` after | `NULL` → `ARGON2_DECODING_FAIL` | verified |
| 8.144 | `argon2_encode_string` | `type` not `Argon2_i`/`Argon2_id` | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.145 | `argon2_encode_string` | `dst_len` too small for the `"$argon2id$v="` / `"$argon2i$v="` prefix (`pp_len >= dst_len` in `SS`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.146 | `argon2_encode_string` | `argon2_validate_inputs(ctx) != ARGON2_OK` (checked **after** the prefix has already been written into `dst`) | that validation code (e.g. `-6` for a short salt, `-2` for a short out); `dst` already contains a partial `"$argon2id$v="` string | verified |
| 8.147 | `argon2_encode_string` | `dst_len` runs out at any later `SS`/`SX` (`"$m="`, m_cost digits, `",t="`, `",p="`, `"$"`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.148 | `argon2_encode_string` | `sodium_bin2base64` returns NULL because `dst_len` is too small for the salt or the output (`SB`) | `ARGON2_ENCODING_FAIL` = `-31` | verified |
| 8.149 | `crypto_pwhash_scryptsalsa208sha256` | `outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX` (0x1fffffffe0) | `-1`, `errno = EFBIG` (LCOV-excluded); `memset(out, 0, outlen)` already ran | unreachable-from-public-API (memset(out,0,outlen) precedes the check; needs a >137 GB caller buffer) |
| 8.150 | `crypto_pwhash_scryptsalsa208sha256` | `passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX` (`SODIUM_SIZE_MAX`) | `-1`, `errno = EFBIG`; unreachable on 64-bit (`PASSWD_MAX == SIZE_MAX`) | unreachable-from-public-API (PASSWD_MAX == SIZE_MAX on 64-bit) |
| 8.151 | `crypto_pwhash_scryptsalsa208sha256` | `outlen < 16`: `0`, `15` | `-1`, `errno = EINVAL` | verified |
| 8.152 | `crypto_pwhash_scryptsalsa208sha256` | `pickparams() != 0` | unreachable — `pickparams` always returns `0`; documented dead branch (`-1`/`EINVAL`) | unreachable-from-public-API (pickparams() always returns 0) |
| 8.153 | `crypto_pwhash_scryptsalsa208sha256` | `(const void *) out == (const void *) passwd` | `-1`, `errno = EINVAL` | verified |
| 8.154 | `crypto_pwhash_scryptsalsa208sha256` | **no** validation of `opslimit`/`memlimit` against `OPSLIMIT_MIN`(32768)/`MEMLIMIT_MIN`(16777216): `opslimit = 0` is silently clamped to 32768 by `pickparams`, `memlimit = 0` yields `N=2, r=8, p=512` | returns `0` (success) — asymmetric with the argon2 entry points; **not** a rejection | verified |
| 8.155 | `crypto_pwhash_scryptsalsa208sha256` | inner `crypto_pwhash_scryptsalsa208sha256_ll` failure (see 8.169–8.181), e.g. giant `memlimit` making `r*p >= 2^30` | `-1` with `errno` from `escrypt_kdf_nosse` | unreachable-from-public-API (pickparams output can never make r*p >= 2^30; any other failure needs a >=64 GiB region) |
| 8.156 | `crypto_pwhash_scryptsalsa208sha256_str` | `passwdlen > PASSWD_MAX` (`SIZE_MAX`) | `-1`, `errno = EFBIG`; unreachable on 64-bit | unreachable-from-public-API (PASSWD_MAX == SIZE_MAX on 64-bit) |
| 8.157 | `crypto_pwhash_scryptsalsa208sha256_str` | `passwdlen < PASSWD_MIN` (0) or `pickparams() != 0` | `-1`, `errno = EINVAL`; both unreachable | unreachable-from-public-API (PASSWD_MIN == 0 and pickparams() always returns 0) |
| 8.158 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_gensalt_r(...) == NULL` | `-1`, `errno = EINVAL`; unreachable from `pickparams` output (`N_log2 <= 63`, `r*p <= 0x3FFFFFF8 < 2^30`) | unreachable-from-public-API (pickparams output always satisfies gensalt) |
| 8.159 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_init_local() != 0` | `-1`; unreachable (`escrypt_init_local` always returns 0) | unreachable-from-public-API (escrypt_init_local() always returns 0) |
| 8.160 | `crypto_pwhash_scryptsalsa208sha256_str` | `escrypt_r(...) == NULL` (KDF failure / allocation failure) | `-1`, `errno = EINVAL` | unreachable-from-public-API (pickparams output always yields a working KDF setting) |
| 8.161 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `sodium_strnlen(str, 102) != 101`: `str` shorter than 101 chars (`""`, a truncated `$7$…`), or 102+ chars / not NUL-terminated within 102 | `-1` (errno untouched) | verified |
| 8.162 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | 101-char `str` whose setting is malformed → `escrypt_r` returns NULL: prefix not `"$7$"`, `N_log2` char outside itoa64 (`"./0-9A-Za-z"`), a non-itoa64 char in the 5-char `r` or `p` fields, or `need > buflen` | `-1` | verified |
| 8.163 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | wrong password (well-formed string, `sodium_memcmp(wanted, str, 102) != 0`) | `-1` (the value of `sodium_memcmp`) | verified |
| 8.164 | `crypto_pwhash_scryptsalsa208sha256_str_verify` | `escrypt_init_local() != 0` | `-1`; unreachable | unreachable-from-public-API (escrypt_init_local() always returns 0) |
| 8.165 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `pickparams() != 0` | `-1`, `errno = EINVAL`; unreachable | unreachable-from-public-API (pickparams() always returns 0) |
| 8.166 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `sodium_strnlen(str, 102) != 101` (too short / too long) | `-1`, `errno = EINVAL` | verified |
| 8.167 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | `escrypt_parse_setting(str, …) == NULL` (bad `$7$` prefix or bad itoa64 chars) | `-1`, `errno = EINVAL` | verified |
| 8.168 | `crypto_pwhash_scryptsalsa208sha256_str_needs_rehash` | valid string but `N_log2`, `r` or `p` differ from `pickparams(opslimit, memlimit)` | `1` (non-zero, non-error) | verified |
| 8.169 | `crypto_pwhash_scryptsalsa208sha256_ll` / `escrypt_kdf_nosse` | `buflen > ((2^32)-1)*32` = 137438953440 (only compiled when `SIZE_MAX > UINT32_MAX`) | `-1`, `errno = EFBIG` | verified |
| 8.170 | `crypto_pwhash_scryptsalsa208sha256_ll` | `(uint64_t) r * p >= 2^30`, e.g. `r = 1, p = 1073741824` or `r = 32768, p = 32768` | `-1`, `errno = EFBIG` | verified |
| 8.171 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > UINT32_MAX`, e.g. `N = 2^32` | `-1`, `errno = EFBIG` | verified |
| 8.172 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N` not a power of two: `3`, `1000`, `1023` | `-1`, `errno = EINVAL` | verified |
| 8.173 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N < 2`: `N = 0` or `N = 1` (note `N = 0` also passes the power-of-two test, so the `N < 2` clause is the one that fires) | `-1`, `errno = EINVAL` | verified |
| 8.174 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r == 0` | `-1`, `errno = EINVAL` | verified |
| 8.175 | `crypto_pwhash_scryptsalsa208sha256_ll` | `p == 0` | `-1`, `errno = EINVAL` | verified |
| 8.176 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r > SIZE_MAX / 128 / p` | `-1`, `errno = ENOMEM` | unreachable-from-public-API (r*p < 2^30 is checked first, so r > SIZE_MAX/128/p is impossible on 64-bit) |
| 8.177 | `crypto_pwhash_scryptsalsa208sha256_ll` | `r > SIZE_MAX / 256` (only compiled when `SIZE_MAX/256 <= UINT32_MAX`, i.e. 32-bit) | `-1`, `errno = ENOMEM` | unreachable-from-public-API (only compiled when SIZE_MAX/256 <= UINT32_MAX (32-bit)) |
| 8.178 | `crypto_pwhash_scryptsalsa208sha256_ll` | `N > SIZE_MAX / 128 / r` | `-1`, `errno = ENOMEM` | verified |
| 8.179 | `crypto_pwhash_scryptsalsa208sha256_ll` | `need = B_size + V_size` wraps (`need < V_size`) | `-1`, `errno = ENOMEM` | verified |
| 8.180 | `crypto_pwhash_scryptsalsa208sha256_ll` | `need += XY_size` wraps (`need < XY_size`) | `-1`, `errno = ENOMEM` | verified |
| 8.181 | `crypto_pwhash_scryptsalsa208sha256_ll` | `escrypt_free_region()` fails (munmap error) or `escrypt_alloc_region()` returns NULL (OOM for `128*r*(N+p) + 256*r + 64` bytes) | `-1` | unreachable-from-public-API (would need a >=256 GiB mmap(MAP_POPULATE) - not safely testable) |
| 8.182 | `escrypt_parse_setting` | `setting[0] != '$' \|\| setting[1] != '7' \|\| setting[2] != '$'` — e.g. `"$6$…"`, `"7$…"`, `""` | `NULL` | verified |
| 8.183 | `escrypt_parse_setting` | `setting[3]` (the `N_log2` char) is not in `"./0123456789A-Za-z"`, e.g. `'$'`, `'-'`, `'*'`, NUL | `NULL` (and `*N_log2_p` set to 0) | verified |
| 8.184 | `escrypt_parse_setting` | any of the 5 chars of the 30-bit `r` field is not in itoa64 (includes a string that ends early, since NUL is not in itoa64) | `NULL` (`*r_p = 0`) | verified |
| 8.185 | `escrypt_parse_setting` | any of the 5 chars of the 30-bit `p` field is not in itoa64 | `NULL` (`*p_p = 0`) | verified |
| 8.186 | `escrypt_gensalt_r` | `need = 14 + BYTES2CHARS(srclen) + 1 > buflen`, e.g. `srclen = 32` (`saltlen = 43`, `need = 58`) with `buflen = 57` | `NULL` | verified |
| 8.187 | `escrypt_gensalt_r` | `need < saltlen` (size wrap) | `NULL`; unreachable | unreachable-from-public-API (size wrap is impossible) |
| 8.188 | `escrypt_gensalt_r` | `saltlen < srclen`, i.e. `BYTES2CHARS(srclen) < srclen` | `NULL`; unreachable (`(8b+5)/6 >= b`) | unreachable-from-public-API ((8b+5)/6 >= b always holds) |
| 8.189 | `escrypt_gensalt_r` | `N_log2 > 63` (would index past `itoa64`) | `NULL` | verified |
| 8.190 | `escrypt_gensalt_r` | `(uint64_t) r * p >= 2^30` | `NULL` | verified |
| 8.191 | `escrypt_gensalt_r` | `encode64_uint32`/`encode64` runs out of `dstlen`, or `dst >= buf + buflen` | `NULL` (“can't happen” after the `need > buflen` check) | unreachable-from-public-API ("can't happen" after the need > buflen check) |
| 8.192 | `escrypt_r` | `escrypt_parse_setting(setting, …) == NULL` (see 8.182–8.185) | `NULL` (note: `randombytes_buf(buf, buflen)` has already scrambled the caller's output buffer) | verified |
| 8.193 | `escrypt_r` | `buf == NULL` | `NULL` | verified |
| 8.194 | `escrypt_r` | `need = prefixlen + saltlen + 1 + 43 + 1 > buflen` — e.g. a `$7$` setting with an over-long salt field, or `buflen` < 102 | `NULL` | verified |
| 8.195 | `escrypt_r` | `need < saltlen` (size wrap) | `NULL` | unreachable-from-public-API (size wrap is impossible) |
| 8.196 | `escrypt_r` | `escrypt_kdf(...) != 0` — any of 8.169–8.181, e.g. a setting encoding `r = 0`/`p = 0` or `N_log2 = 0` (→ `N = 1 < 2`) or `N_log2 = 63` (→ `N > UINT32_MAX`) | `NULL` | verified |
| 8.197 | `escrypt_r` | final `encode64` returns NULL or `dst >= buf + buflen` | `NULL` (“can't happen”) | unreachable-from-public-API ("can't happen" after the need > buflen check) |
| 8.198 | `escrypt_alloc_region` | `mmap()` fails (`MAP_FAILED`) | `NULL` returned, `region->base = NULL`, `region->size = 0` | verified |
| 8.199 | `escrypt_alloc_region` | non-mmap fallback: `size + 63 < size` (wrap) or `malloc(size + 63) == NULL` | `NULL`, `errno = ENOMEM` | unreachable-from-public-API (the non-mmap fallback is not compiled (HAVE_MMAP)) |
| 8.200 | `escrypt_free_region` | `munmap(region->base, region->size)` fails | `-1` (propagates to `-1` from `_ll` / `escrypt_free_local`) | unreachable-from-public-API (munmap() failure) |
| 8.201 | `escrypt_PBKDF2_SHA256` | `dkLen > 0x1fffffffe0` (only compiled when `SIZE_MAX > 0x1fffffffe0`) | `sodium_misuse()` → `abort()`; the function is `void` and cannot report an error otherwise | verified |
| 8.202 | `crypto_ipcrypt_encrypt` / `_decrypt` | — | **no rejection branch exists**: the function is `void`, performs a fixed 16-byte AES-128 ECB block operation, and validates nothing. Buffers shorter than `crypto_ipcrypt_BYTES` (16) / keys shorter than `crypto_ipcrypt_KEYBYTES` (16) are out-of-bounds reads/writes (UB), not errors | verified |
| 8.203 | `crypto_ipcrypt_nd_encrypt` | — | `void`, no validation. Reads exactly 16 bytes of `in`, 8 of `t`, 16 of `k`; writes exactly 24 bytes of `out` (`t` copied to `out[0..8)`, ciphertext to `out[8..24)`). Short `out` (e.g. 16 bytes) is a buffer overflow, not a reported error | verified |
| 8.204 | `crypto_ipcrypt_nd_decrypt` | — | `void`, no validation. Reads 24 bytes of `in` (tweak `in[0..8)`, ct `in[8..24)`), writes 16 bytes of `out`. A corrupted/forged input is decrypted to garbage: **there is no authentication tag and therefore no failure indication** | verified |
| 8.205 | `crypto_ipcrypt_ndx_encrypt` | — | `void`, no validation. 32-byte key (`k[0..16)` = data key, `k[16..32)` = tweak key), 16-byte tweak, 16-byte input, 32-byte output | verified |
| 8.206 | `crypto_ipcrypt_ndx_decrypt` | — | `void`, no validation; no integrity check, so a forged 32-byte input silently produces garbage | verified |
| 8.207 | `crypto_ipcrypt_ndx_encrypt` / `_ndx_decrypt` | degenerate key where the two halves are identical (`k[0..16) == k[16..32)`, detected as `tkeys[5] XOR rkeys[5] == 0`) | **not an error**: the data key is silently replaced by `k[i] ^ 0x5a` and the operation proceeds. Must be modelled as a normal (not rejecting) path | verified |
| 8.208 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | — | `void`, no validation; 32-byte key, 16-byte in/out | verified |
| 8.209 | `crypto_ipcrypt_pfx_encrypt` / `_pfx_decrypt` | degenerate key `k[0..16) == k[16..32)` (`k1keys[5] XOR k2keys[5] == 0`) | **not an error**: `k2` is re-derived as `k[i] ^ 0x5a`; operation proceeds | verified |
| 8.210 | `crypto_ipcrypt_keygen` / `_nd_keygen` / `_ndx_keygen` / `_pfx_keygen` | — | `void`, cannot fail (delegates to `randombytes_buf`) | verified |
| 8.211 | `_crypto_ipcrypt_pick_best_implementation` | — | always returns `0`; with no `HAVE_ARMCRYPTO` / `HAVE_AVXINTRIN_H`+`HAVE_WMMINTRIN_H` it always selects `ipcrypt_soft_implementation` | verified |
| 8.212 | `_crypto_pwhash_argon2_pick_best_implementation` | — | always returns `0`; with no SIMD macros it always selects `argon2_fill_segment_ref` | verified |
| 8.213 | (adjacent, `sodium/codecs.c` — **not** in area 8's files) `sodium_ip2bin` | libsodium 1.0.23's `crypto_ipcrypt` has **no** IP-string entry points; string↔16-byte conversion is done by `sodium_ip2bin` / `sodium_bin2ip`. Rejections: zone (`%…`) on a non-IPv6 address, empty zone, zone char outside `[0-9a-zA-Z._-]`, malformed IPv6 (bad `::`, >4 hex digits per group, wrong group count, embedded IPv4 not at the end), malformed IPv4 (>3 digits, octet > 255, missing/extra dots, trailing junk) | `-1` (0 on success). Listed here only so the “bad IP string” cases are accounted for; they belong to the utils/codecs area | verified |
| 8.214 | (adjacent, `sodium/codecs.c`) `sodium_bin2ip` | `ip_maxlen <= 2`, or the rendered address needs `>= ip_maxlen` bytes | `NULL` | verified |

**Row count: 214.**  162 rows are `verified` by
`tests/a8_argon2.rs` (8.1 – 8.52), `tests/a8_argon2_core.rs` (8.53 – 8.111 and 8.148),
`tests/a8_argon2_encoding.rs` (8.112 – 8.147), `tests/a8_scrypt.rs` (8.149 – 8.201) and
`tests/a8_ipcrypt.rs` (8.202 – 8.214).  The other 52 rows, marked
`unreachable-from-public-API` are real C branches that cannot fire on this platform (a
`uint32_t` field, a minimum of 0, `pickparams` never failing) or that would require an
allocation failure / a multi-terabyte buffer.

Corrections found while writing those tests (the C is authoritative):

* **8.48** — the truncating division makes the first rejected `memlimit`
  `4294967296 * 1024 = 4398046511104`, not "`> 4398046511104`"; `4398046511103` is accepted.
* **8.116** — `"v=1a9"` is not a `DECODING_FAIL`: `decode_decimal` stops at `'a'` with the
  value 1 and the `version != ARGON2_VERSION_NUMBER` check fires first, giving
  `ARGON2_INCORRECT_TYPE` (-26).
* **8.133** — `"...$YWJjZA=="` gives `ARGON2_OUTPUT_TOO_SHORT` (-2), not -32: `'='` is
  outside the `ORIGINAL_NO_PADDING` alphabet, so 4 bytes decode and the
  `argon2_validate_inputs` call runs *before* the trailing-NUL check.
* **8.148** — `sodium_bin2base64` never returns NULL for a short buffer; it calls
  `sodium_misuse()` and **aborts**.  So the `SB` NULL check is dead code and
  `argon2_encode_string` / `argon2_hash` abort instead of returning
  `ARGON2_ENCODING_FAIL` once `dst_len` is large enough to get past the last `SS`
  (`dst_len >= 27` for `$argon2i$v=19$m=8,t=1,p=1$`).  Verified with `eq_abort`.
* **8.139** — appending `'x'` to the hash is *not* trailing garbage (it is a valid Base64
  character and simply lengthens the digest); a character outside the alphabet is needed.
* **8.183/8.184/8.185** — `decode64_one` is `strchr(itoa64, c)`, which also matches the
  terminating NUL of `itoa64` and yields the out-of-range value 64.  A `$7$` setting that
  ends early therefore keeps reading past its own NUL, and a truncation landing exactly on
  the last character of the `r` or `p` field still parses.

### Cross-cutting notes for the Rust port

1. `errno` is part of the observable contract for every `crypto_pwhash*` and scrypt entry point:
   `EINVAL` for “below minimum / malformed / aliasing / bad alg”, `EFBIG` for “above maximum”,
   `ENOMEM` for scrypt sizing overflows. `argon2_*` and `escrypt_*` do not touch `errno`
   themselves except through `posix_memalign`/`malloc`.
2. Every `crypto_pwhash_argon2*` and scrypt entry point zeroes (`out`) or randomizes
   (`argon2_hash`'s `hash`, `escrypt_r`'s `buf`) the caller's output buffer **before** validating,
   so a rejected call still mutates the output.
3. Rows marked *unreachable* correspond to real C branches that cannot fire on this platform
   (field is `uint32_t`, min is 0, or `pickparams` never fails). They still need to exist in the
   port if the port exposes the same internal functions with wider integer types.
4. `crypto_pwhash_str_alg` with an unknown `alg` **aborts** (`sodium_misuse`) rather than returning
   `-1`; this is the only abort in the argon2 path. `escrypt_PBKDF2_SHA256` with `dkLen` above
   `0x1fffffffe0` is the only abort in the scrypt path.
5. All of `crypto_ipcrypt_*` is total (`void`, never fails). The only data-dependent branches are
   the degenerate-key fixups (8.207, 8.209) and `is_ipv4_mapped` in the PFX variants.
