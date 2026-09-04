| pwhash-E1 | crypto_pwhash | alg not ALG_ARGON2I13/ALG_ARGON2ID13 (switch default; any out-of-range int) | errno=EINVAL, returns -1 | [x] |
| pwhash-E2 | crypto_pwhash_str_alg | alg not ALG_ARGON2I13/ALG_ARGON2ID13 | sodium_misuse() | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` |
| pwhash-E3 | crypto_pwhash_str_verify | str matches neither `$argon2id$` nor `$argon2i$` (strncmp) | errno=EINVAL, returns -1 | [x] |
| pwhash-E4 | crypto_pwhash_str_needs_rehash | str matches neither `$argon2id$` nor `$argon2i$` | errno=EINVAL, returns -1 | [x] |
| pwhash-E5 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen > BYTES_MAX (4294967295) | errno=EFBIG, returns -1 | [not testable — `memset(out, 0, outlen)` runs before the check, so any test would write >4 GiB into the caller's buffer; verified by inspection that the Rust has the identical order] |
| pwhash-E6 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen < BYTES_MIN (16) | errno=EINVAL, returns -1 | [x] |
| pwhash-E7 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen > PASSWD_MAX (4294967295) | errno=EFBIG, returns -1 | [x] |
| pwhash-E8 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit > OPSLIMIT_MAX (4294967295) | errno=EFBIG, returns -1 | [x] |
| pwhash-E9 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit > MEMLIMIT_MAX (4398046510080) | errno=EFBIG, returns -1 | [x] |
| pwhash-E10 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen < PASSWD_MIN | errno=EINVAL, returns -1 | [unreachable — PASSWD_MIN == 0 and passwdlen is unsigned] |
| pwhash-E11 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit < OPSLIMIT_MIN (3 / 1) | errno=EINVAL, returns -1 | [x] |
| pwhash-E12 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit < MEMLIMIT_MIN (8192) | errno=EINVAL, returns -1 | [x] |
| pwhash-E13 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | `(const void *) out == (const void *) passwd` | errno=EINVAL, returns -1 | [x] |
| pwhash-E14 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | alg not the family's own ALG constant (switch default) | errno=EINVAL, returns -1 | [x] |
| pwhash-E15 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | argon2{i,id}_hash_raw() != ARGON2_OK | returns -1 (errno untouched) | [not testable — with the accepted limit ranges the only way to fail is an allocation failure of several TiB; verified by inspection] |
| pwhash-E16 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen > PASSWD_MAX \|\| opslimit > OPSLIMIT_MAX \|\| memlimit > MEMLIMIT_MAX | errno=EFBIG, returns -1, out zeroed | [x] |
| pwhash-E17 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen < PASSWD_MIN \|\| opslimit < OPSLIMIT_MIN \|\| memlimit < MEMLIMIT_MIN | errno=EINVAL, returns -1, out zeroed | [x] |
| pwhash-E18 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | argon2{i,id}_hash_encoded() != ARGON2_OK | returns -1 | [not testable — STRBYTES (128) is always large enough for a 16-byte salt + 32-byte hash; only an allocation failure could trigger it] |
| pwhash-E19 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | passwdlen > PASSWD_MAX | errno=EFBIG, returns -1 | [x] |
| pwhash-E20 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | passwdlen < PASSWD_MIN | errno=EINVAL, returns -1 | [unreachable — PASSWD_MIN == 0] |
| pwhash-E21 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | argon2*_verify() == ARGON2_VERIFY_MISMATCH | errno=EINVAL, returns -1 | [x] |
| pwhash-E22 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify | argon2*_verify() fails for any other reason (decoding, validation) | returns -1, errno untouched | [x] |
| pwhash-E23 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | opslimit > UINT32_MAX \|\| memlimit/1024 > UINT32_MAX \|\| strlen(str) >= crypto_pwhash_STRBYTES (128) | errno=EINVAL, returns -1 | [x] |
| pwhash-E24 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | calloc(fodder_len, 1) == NULL | returns -1 | [not testable — requires calloc failure for a <128-byte request] |
| pwhash-E25 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | argon2_decode_string() != 0 | errno=EINVAL, returns -1 | [x] |
| pwhash-E26 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash | ctx.t_cost != opslimit \|\| ctx.m_cost != memlimit/1024 | returns 1 | [x] |
| pwhash-E27 | _sodium_argon2_ctx | argon2_validate_inputs() != ARGON2_OK | returns the validation code unchanged | [x] |
| pwhash-E28 | _sodium_argon2_ctx | type != Argon2_id && type != Argon2_i (any other int) | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E29 | _sodium_argon2_ctx | argon2_initialize() != ARGON2_OK | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires a multi-TiB allocation to fail; verified by inspection] |
| pwhash-E30 | _sodium_argon2_hash, _sodium_argon2i_hash_raw/encoded, _sodium_argon2id_hash_raw/encoded | pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF) | returns ARGON2_PWD_TOO_LONG (-5) | [x] |
| pwhash-E31 | _sodium_argon2_hash (same wrappers) | hashlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | returns ARGON2_OUTPUT_TOO_LONG (-3) | [x] |
| pwhash-E32 | _sodium_argon2_hash (same wrappers) | saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) | returns ARGON2_SALT_TOO_LONG (-7) | [x] |
| pwhash-E33 | _sodium_argon2_hash | malloc(hashlen) == NULL | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E34 | _sodium_argon2_hash | argon2_ctx() != ARGON2_OK | frees/zeroes `out` and returns that code | [x] |
| pwhash-E35 | _sodium_argon2_hash | argon2_encode_string() != ARGON2_OK (encoded buffer too small) | zeroes out+encoded, returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E36 | _sodium_argon2_verify, _sodium_argon2i_verify, _sodium_argon2id_verify | strlen(encoded) > UINT32_MAX | returns ARGON2_DECODING_LENGTH_FAIL (-34) | [not testable — needs a >4 GiB NUL-terminated string; verified by inspection] |
| pwhash-E37 | _sodium_argon2_verify | any of malloc(ctx.ad/ctx.salt/ctx.out) == NULL, or malloc(out) == NULL | frees the rest, returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E38 | _sodium_argon2_verify | argon2_decode_string() != ARGON2_OK | returns the decode code unchanged (-32 / -26 / validation codes) | [x] |
| pwhash-E39 | _sodium_argon2_verify | sodium_memcmp(out, ctx.out, ctx.outlen) != 0 | returns ARGON2_VERIFY_MISMATCH (-35) | [x] |
| pwhash-E40 | _sodium_argon2_validate_inputs | context == NULL | returns ARGON2_INCORRECT_PARAMETER (-25) | [x] |
| pwhash-E41 | _sodium_argon2_validate_inputs | context->out == NULL | returns ARGON2_OUTPUT_PTR_NULL (-1) | [x] |
| pwhash-E42 | _sodium_argon2_validate_inputs | outlen < ARGON2_MIN_OUTLEN (16) | returns ARGON2_OUTPUT_TOO_SHORT (-2) | [x] |
| pwhash-E43 | _sodium_argon2_validate_inputs | outlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | returns ARGON2_OUTPUT_TOO_LONG (-3) | [unreachable — outlen is uint32_t; outlen == UINT32_MAX tested and accepted] |
| pwhash-E44 | _sodium_argon2_validate_inputs | pwd == NULL && pwdlen != 0 | returns ARGON2_PWD_PTR_MISMATCH (-18) | [x] |
| pwhash-E45 | _sodium_argon2_validate_inputs | pwdlen < ARGON2_MIN_PWD_LENGTH (0) | returns ARGON2_PWD_TOO_SHORT (-4) | [unreachable — MIN is 0, field is unsigned] |
| pwhash-E46 | _sodium_argon2_validate_inputs | pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF) | returns ARGON2_PWD_TOO_LONG (-5) | [unreachable — field is uint32_t] |
| pwhash-E47 | _sodium_argon2_validate_inputs | salt == NULL && saltlen != 0 | returns ARGON2_SALT_PTR_MISMATCH (-19) | [x] |
| pwhash-E48 | _sodium_argon2_validate_inputs | saltlen < ARGON2_MIN_SALT_LENGTH (8) | returns ARGON2_SALT_TOO_SHORT (-6) | [x] |
| pwhash-E49 | _sodium_argon2_validate_inputs | saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) | returns ARGON2_SALT_TOO_LONG (-7) | [unreachable — field is uint32_t] |
| pwhash-E50 | _sodium_argon2_validate_inputs | secret == NULL && secretlen != 0 | returns ARGON2_SECRET_PTR_MISMATCH (-20) | [x] |
| pwhash-E51 | _sodium_argon2_validate_inputs | secret != NULL && secretlen < ARGON2_MIN_SECRET (0) | returns ARGON2_SECRET_TOO_SHORT (-10) | [unreachable — MIN is 0] |
| pwhash-E52 | _sodium_argon2_validate_inputs | secret != NULL && secretlen > ARGON2_MAX_SECRET (0xFFFFFFFF) | returns ARGON2_SECRET_TOO_LONG (-11) | [unreachable — field is uint32_t] |
| pwhash-E53 | _sodium_argon2_validate_inputs | ad == NULL && adlen != 0 | returns ARGON2_AD_PTR_MISMATCH (-21) | [x] |
| pwhash-E54 | _sodium_argon2_validate_inputs | ad != NULL && adlen < ARGON2_MIN_AD_LENGTH (0) | returns ARGON2_AD_TOO_SHORT (-8) | [unreachable — MIN is 0] |
| pwhash-E55 | _sodium_argon2_validate_inputs | ad != NULL && adlen > ARGON2_MAX_AD_LENGTH (0xFFFFFFFF) | returns ARGON2_AD_TOO_LONG (-9) | [unreachable — field is uint32_t] |
| pwhash-E56 | _sodium_argon2_validate_inputs | lanes < ARGON2_MIN_LANES (1) | returns ARGON2_LANES_TOO_FEW (-16) | [x] |
| pwhash-E57 | _sodium_argon2_validate_inputs | lanes > ARGON2_MAX_LANES (0xFFFFFF) | returns ARGON2_LANES_TOO_MANY (-17) | [x] |
| pwhash-E58 | _sodium_argon2_validate_inputs | m_cost < ARGON2_MIN_MEMORY (8) | returns ARGON2_MEMORY_TOO_LITTLE (-14) | [x] |
| pwhash-E59 | _sodium_argon2_validate_inputs | m_cost > ARGON2_MAX_MEMORY (0xFFFFFFFF) | returns ARGON2_MEMORY_TOO_MUCH (-15) | [unreachable — field is uint32_t; m_cost == UINT32_MAX tested and accepted] |
| pwhash-E60 | _sodium_argon2_validate_inputs | m_cost < 8 * lanes | returns ARGON2_MEMORY_TOO_LITTLE (-14) | [x] |
| pwhash-E61 | _sodium_argon2_validate_inputs | t_cost < ARGON2_MIN_TIME (1) | returns ARGON2_TIME_TOO_SMALL (-12) | [x] |
| pwhash-E62 | _sodium_argon2_validate_inputs | t_cost > ARGON2_MAX_TIME (0xFFFFFFFF) | returns ARGON2_TIME_TOO_LARGE (-13) | [unreachable — field is uint32_t; t_cost == UINT32_MAX tested and accepted] |
| pwhash-E63 | _sodium_argon2_validate_inputs | threads < ARGON2_MIN_THREADS (1) | returns ARGON2_THREADS_TOO_FEW (-28) | [x] |
| pwhash-E64 | _sodium_argon2_validate_inputs | threads > ARGON2_MAX_THREADS (0xFFFFFF) | returns ARGON2_THREADS_TOO_MANY (-29) | [x] |
| pwhash-E65 | _sodium_argon2_initialize | instance == NULL \|\| context == NULL | returns ARGON2_INCORRECT_PARAMETER (-25) | [x] |
| pwhash-E66 | _sodium_argon2_initialize | malloc(8 * segment_length) == NULL | returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — requires malloc failure] |
| pwhash-E67 | _sodium_argon2_initialize | allocate_memory() != ARGON2_OK (region==NULL, m_cost==0, size overflow, malloc failure) | frees the instance, returns ARGON2_MEMORY_ALLOCATION_ERROR (-22) | [not testable — `region` is always non-NULL, m_cost >= 8 after argon2_ctx's rounding, `1024 * (uint32_t)` cannot overflow size_t; only malloc failure remains] |
| pwhash-E68 | _sodium_argon2_fill_memory_blocks | instance == NULL \|\| instance->lanes == 0 | returns without touching memory | [x] |
| pwhash-E69 | _sodium_argon2_finalize | context == NULL \|\| instance == NULL | returns without touching memory | [x] |
| pwhash-E70 | _sodium_argon2_fill_segment_ref | instance == NULL | returns immediately | [x] |
| pwhash-E71 | _sodium_argon2_decode_string | type is neither Argon2_id nor Argon2_i | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E72 | _sodium_argon2_decode_string | prefix mismatch (`$argon2id` / `$argon2i`, `$v=`, `$m=`, `,t=`, `,p=`, `$`) — 7 distinct `CC()` sites | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E73 | _sodium_argon2_decode_string | decode_decimal(): no digit at all | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E74 | _sodium_argon2_decode_string | decode_decimal(): non-minimal encoding (leading zero, e.g. `m=08`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E75 | _sodium_argon2_decode_string | decode_decimal(): value does not fit in unsigned long (`acc > ULONG_MAX/10` or `c > ULONG_MAX-acc`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E76 | _sodium_argon2_decode_string | DECIMAL_U32(): decoded value > UINT32_MAX (v=, m=, t=, p=) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E77 | _sodium_argon2_decode_string | version != ARGON2_VERSION_NUMBER (0x13) | returns ARGON2_INCORRECT_TYPE (-26) | [x] |
| pwhash-E78 | _sodium_argon2_decode_string | m_cost/t_cost/lanes > UINT32_MAX after DECIMAL_U32 | returns ARGON2_INCORRECT_TYPE (-26) | [unreachable — DECIMAL_U32 already rejects > UINT32_MAX] |
| pwhash-E79 | _sodium_argon2_decode_string | BIN(): sodium_base642bin() != 0 (bad char, padding, output too long for the caller's buffer) or bin_len > UINT32_MAX — 2 sites (salt, out) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E80 | _sodium_argon2_decode_string | argon2_validate_inputs() != ARGON2_OK after decoding (e.g. `p=0`, short salt, short out) | returns the validation code | [x] |
| pwhash-E81 | _sodium_argon2_decode_string | trailing characters after the final base64 field (`*str != 0`) | returns ARGON2_DECODING_FAIL (-32) | [x] |
| pwhash-E82 | _sodium_argon2_encode_string | type is neither Argon2_id nor Argon2_i (switch default) | returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E83 | _sodium_argon2_encode_string | SS()/SX(): `strlen(str) >= dst_len` — 9 distinct sites (prefix, version, `$m=`, m, `,t=`, t, `,p=`, p, `$`) | returns ARGON2_ENCODING_FAIL (-31) | [x] |
| pwhash-E84 | _sodium_argon2_encode_string | argon2_validate_inputs() != ARGON2_OK (checked after the prefix is written) | returns the validation code | [x] |
| pwhash-E85 | _sodium_argon2_encode_string | SB(): sodium_bin2base64() returns NULL — 2 sites (salt, out) | returns ARGON2_ENCODING_FAIL (-31) | [abort — sodium_bin2base64() calls sodium_misuse() when `b64_maxlen <= b64_len`, so this `return` is dead code in practice; the Rust reproduces the same call and therefore aborts identically. Only dst_len values that fail an earlier SS() check are exercised] |
| pwhash-E86 | _sodium_blake2b_long | outlen > UINT32_MAX | `goto fail`, returns -1 (out untouched) | [x] |
| pwhash-E87 | _sodium_blake2b_long | any crypto_generichash_blake2b_{init,update,final,} call returns < 0 (e.g. outlen == 0) | `goto fail`, returns that negative value | [x] |
| pwhash-E88 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | buflen > (2^32 - 1) * 32 | errno=EFBIG, returns -1 | [x] |
| pwhash-E89 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | (uint64_t)r * (uint64_t)p >= 2^30 | errno=EFBIG, returns -1 | [x] |
| pwhash-E90 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N > UINT32_MAX | errno=EFBIG, returns -1 | [x] |
| pwhash-E91 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N not a power of two, or N < 2 (incl. N=0, N=1) | errno=EINVAL, returns -1 | [x] |
| pwhash-E92 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | r == 0 \|\| p == 0 | errno=EINVAL, returns -1 | [x] |
| pwhash-E93 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | r > SIZE_MAX / 128 / p | errno=ENOMEM, returns -1 | [unreachable on 64-bit — `r*p < 2^30` already bounds `128*r*p < 2^37 << SIZE_MAX`] |
| pwhash-E94 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | N > SIZE_MAX / 128 / r | errno=ENOMEM, returns -1 | [x] |
| pwhash-E95 | _sodium_escrypt_kdf_nosse | B_size + V_size wraps (`need < V_size`) | errno=ENOMEM, returns -1 | [unreachable on 64-bit — the preceding checks bound V_size <= SIZE_MAX and B_size < 2^37, so the sum cannot wrap] |
| pwhash-E96 | _sodium_escrypt_kdf_nosse | need + XY_size wraps (`need < XY_size`) | errno=ENOMEM, returns -1 | [unreachable on 64-bit — same bound] |
| pwhash-E97 | _sodium_escrypt_kdf_nosse | escrypt_free_region() != 0 | returns -1 | [unreachable — without HAVE_MMAP escrypt_free_region() always returns 0] |
| pwhash-E98 | _sodium_escrypt_kdf_nosse, crypto_pwhash_scryptsalsa208sha256_ll | escrypt_alloc_region() == NULL | returns -1 | [x] |
| pwhash-E99 | _sodium_escrypt_PBKDF2_SHA256 | dkLen > 0x1fffffffe0 | sodium_misuse() | [x] SIGABRT in both, verified out-of-process by `tests/gaps.rs::abort_parity` (dkLen check precedes any write to buf) |
| pwhash-E100 | _sodium_escrypt_alloc_region | size + 63 < size (overflow) | errno=ENOMEM, base/aligned NULL, size 0, returns NULL | [x] |
| pwhash-E101 | _sodium_escrypt_alloc_region | malloc(size + 63) == NULL | base/aligned NULL, size 0, returns NULL | [x] |
| pwhash-E102 | _sodium_escrypt_free_region | munmap() failure | returns -1 | [unreachable — HAVE_MMAP is not defined in this build, the `free(base)` branch is compiled] |
| pwhash-E103 | _sodium_escrypt_parse_setting | setting does not start with `$7$` | returns NULL | [x] |
| pwhash-E104 | _sodium_escrypt_parse_setting | decode64_one() on the N_log2 character fails (not in itoa64) | returns NULL, *N_log2_p = 0 | [x] |
| pwhash-E105 | _sodium_escrypt_parse_setting | decode64_uint32() fails on the 5-char r field | returns NULL, *r_p = 0 | [x] |
| pwhash-E106 | _sodium_escrypt_parse_setting | decode64_uint32() fails on the 5-char p field | returns NULL, *p_p = 0 | [x] |
| pwhash-E107 | _sodium_escrypt_r | escrypt_parse_setting() == NULL | returns NULL | [x] |
| pwhash-E108 | _sodium_escrypt_r | buf == NULL | returns NULL | [x] |
| pwhash-E109 | _sodium_escrypt_r | need > buflen | returns NULL | [x] |
| pwhash-E110 | _sodium_escrypt_r | need < saltlen (wrap) | returns NULL | [unreachable — need = prefixlen + saltlen + 88 cannot wrap for a NUL-terminated setting] |
| pwhash-E111 | _sodium_escrypt_r | escrypt_kdf() != 0 (setting encodes N_log2 = 0 -> N = 1, or r = 0, or p = 0) | returns NULL | [x] |
| pwhash-E112 | _sodium_escrypt_r | encode64() == NULL \|\| dst >= buf + buflen | returns NULL | [unreachable — `need <= buflen` was already checked ("Can't happen" in the C)] |
| pwhash-E113 | _sodium_escrypt_gensalt_r | need > buflen | returns NULL | [x] |
| pwhash-E114 | _sodium_escrypt_gensalt_r | need < saltlen (wrap) \|\| saltlen < srclen | returns NULL | [unreachable — BYTES2CHARS(n) = (8n+5)/6 >= n for all n] |
| pwhash-E115 | _sodium_escrypt_gensalt_r | N_log2 > 63 | returns NULL | [x] |
| pwhash-E116 | _sodium_escrypt_gensalt_r | (uint64_t)r * (uint64_t)p >= 2^30 | returns NULL | [x] |
| pwhash-E117 | _sodium_escrypt_gensalt_r | encode64_uint32()/encode64() == NULL, or dst >= buf + buflen | returns NULL | [unreachable — the `need > buflen` check already guarantees room ("Can't happen" in the C)] |
| pwhash-E118 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_init_local() != 0, escrypt_free_local() != 0 | returns -1 | [unreachable — both always return 0 in this build] |
| pwhash-E119 | crypto_pwhash_scryptsalsa208sha256 | passwdlen > PASSWD_MAX (SODIUM_SIZE_MAX) | errno=EFBIG, returns -1 | [unreachable on 64-bit — PASSWD_MAX == SIZE_MAX] |
| pwhash-E120 | crypto_pwhash_scryptsalsa208sha256 | outlen > BYTES_MAX (0x1fffffffe0) | errno=EFBIG, returns -1 | [not testable — `memset(out, 0, outlen)` runs before the check; verified by inspection that the Rust order is identical] |
| pwhash-E121 | crypto_pwhash_scryptsalsa208sha256 | outlen < BYTES_MIN (16) | errno=EINVAL, returns -1 | [x] |
| pwhash-E122 | crypto_pwhash_scryptsalsa208sha256, _str, _str_needs_rehash | pickparams() != 0 | errno=EINVAL, returns -1 | [unreachable — pickparams() always returns 0] |
| pwhash-E123 | crypto_pwhash_scryptsalsa208sha256 | `(const void *) out == (const void *) passwd` | errno=EINVAL, returns -1 | [x] |
| pwhash-E124 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen > PASSWD_MAX / passwdlen < PASSWD_MIN | errno=EFBIG / EINVAL, returns -1 | [unreachable on 64-bit — PASSWD_MAX == SIZE_MAX, PASSWD_MIN == 0] |
| pwhash-E125 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_gensalt_r() == NULL | errno=EINVAL, returns -1 | [unreachable — pickparams() yields N_log2 <= 62 and r*p = 8*p < 2^30] |
| pwhash-E126 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_init_local() != 0 | returns -1 | [unreachable — always returns 0] |
| pwhash-E127 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_r() == NULL | errno=EINVAL, returns -1 | [unreachable — pickparams() always yields N >= 2, r = 8, p >= 1, and STRBYTES == the exact `need`] |
| pwhash-E128 | crypto_pwhash_scryptsalsa208sha256_str_verify | sodium_strnlen(str, STRBYTES) != STRBYTES - 1 (too short, or no NUL within 102 bytes) | returns -1 | [x] |
| pwhash-E129 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_init_local() != 0 | returns -1 | [unreachable — always returns 0] |
| pwhash-E130 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_r() == NULL (bad setting, or kdf rejects the encoded N/r/p) | returns -1 | [x] |
| pwhash-E131 | crypto_pwhash_scryptsalsa208sha256_str_verify | sodium_memcmp(wanted, str, STRBYTES) != 0 (wrong password) | returns -1 | [x] |
| pwhash-E132 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | sodium_strnlen(str, STRBYTES) != STRBYTES - 1 | errno=EINVAL, returns -1 | [x] |
| pwhash-E133 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | escrypt_parse_setting() == NULL | errno=EINVAL, returns -1 | [x] |
| pwhash-E134 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | N_log2 != N_log2_ \|\| r != r_ \|\| p != p_ | returns 1 | [x] |
