| G2 | crypto_pwhash | alg not in {crypto_pwhash_ALG_ARGON2I13=1, crypto_pwhash_ALG_ARGON2ID13=2} (e.g. alg=0, 3, -1, 100) | errno=EINVAL, returns -1 (crypto_pwhash.c:142) |
| G2 | crypto_pwhash_str_alg | alg not in {1,2} (e.g. alg=0, 3, -1) | sodium_misuse() -> abort()/SIGABRT (no return; crypto_pwhash.c:169) |
| G2 | crypto_pwhash_str_verify | str does not begin with "$argon2id$" nor "$argon2i$" (e.g. "$7$...", "$argon2d$...", "", "xyz") | errno=EINVAL, returns -1 (crypto_pwhash.c:187) |
| G2 | crypto_pwhash_str_needs_rehash | str does not begin with "$argon2id$" nor "$argon2i$" | errno=EINVAL, returns -1 (crypto_pwhash.c:204) |
| G2 | crypto_pwhash_argon2i | outlen > crypto_pwhash_argon2i_BYTES_MAX (=min(SIZE_MAX,4294967295)=4294967295 on 64-bit) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i | outlen < crypto_pwhash_argon2i_BYTES_MIN (16); outlen in 0..15 | errno=EINVAL, returns -1; out[0..outlen) already zeroed by memset |
| G2 | crypto_pwhash_argon2i | passwdlen > crypto_pwhash_argon2i_PASSWD_MAX (4294967295) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i | opslimit > crypto_pwhash_argon2i_OPSLIMIT_MAX (4294967295), e.g. 4294967296 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i | memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX (4398046510080 when SIZE_MAX>=4398046510080; else 2147483648; else 32768) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i | opslimit < crypto_pwhash_argon2i_OPSLIMIT_MIN (3), i.e. opslimit in {0,1,2} | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i | memlimit < crypto_pwhash_argon2i_MEMLIMIT_MIN (8192), i.e. 0..8191 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i | passwdlen < crypto_pwhash_argon2i_PASSWD_MIN (0) -- statically unreachable (MIN==0) | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_argon2i | out and passwd are the same pointer ((const void*)out == (const void*)passwd) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i | alg != crypto_pwhash_argon2i_ALG_ARGON2I13 (1) -- e.g. alg=2 (argon2id) passed to the argon2i entry point | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i | argon2i_hash_raw() returns != ARGON2_OK (internal: ARGON2_MEMORY_ALLOCATION_ERROR=-22, ARGON2_PWD_TOO_LONG=-5, ARGON2_OUTPUT_TOO_LONG=-3, ARGON2_SALT_TOO_LONG=-7 ...) | public: returns -1, errno unchanged; internal enum discarded |
| G2 | crypto_pwhash_argon2id | outlen > crypto_pwhash_argon2id_BYTES_MAX (4294967295) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id | outlen < crypto_pwhash_argon2id_BYTES_MIN (16), i.e. 0..15 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id | passwdlen > crypto_pwhash_argon2id_PASSWD_MAX (4294967295) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id | opslimit > crypto_pwhash_argon2id_OPSLIMIT_MAX (4294967295) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id | memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX (4398046510080 on 64-bit) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id | opslimit < crypto_pwhash_argon2id_OPSLIMIT_MIN (1), i.e. opslimit==0 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id | memlimit < crypto_pwhash_argon2id_MEMLIMIT_MIN (8192), i.e. 0..8191 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id | out == passwd (aliasing) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id | alg != crypto_pwhash_argon2id_ALG_ARGON2ID13 (2) -- e.g. alg=1 passed to the argon2id entry point | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id | argon2id_hash_raw() returns != ARGON2_OK | public: returns -1, errno unchanged |
| G2 | crypto_pwhash_argon2i_str | passwdlen > crypto_pwhash_argon2i_PASSWD_MAX (4294967295) | errno=EFBIG, returns -1; out fully zeroed (128 bytes) |
| G2 | crypto_pwhash_argon2i_str | opslimit > 4294967295 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i_str | memlimit > crypto_pwhash_argon2i_MEMLIMIT_MAX | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i_str | opslimit < 3 (crypto_pwhash_argon2i_OPSLIMIT_MIN) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i_str | memlimit < 8192 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i_str | argon2i_hash_encoded() != ARGON2_OK (internally ARGON2_ENCODING_FAIL=-31 if 128-byte buffer too small, ARGON2_MEMORY_ALLOCATION_ERROR=-22) | public: returns -1 |
| G2 | crypto_pwhash_argon2id_str | passwdlen > 4294967295 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id_str | opslimit > 4294967295 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id_str | memlimit > crypto_pwhash_argon2id_MEMLIMIT_MAX | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id_str | opslimit < 1 (crypto_pwhash_argon2id_OPSLIMIT_MIN), i.e. opslimit==0 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id_str | memlimit < 8192 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id_str | argon2id_hash_encoded() != ARGON2_OK | public: returns -1 |
| G2 | crypto_pwhash_argon2i_str_verify | passwdlen > crypto_pwhash_argon2i_PASSWD_MAX (4294967295) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2i_str_verify | passwdlen < PASSWD_MIN (0) -- statically unreachable | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_argon2i_str_verify | password does not match the hash in str: argon2i_verify() == ARGON2_VERIFY_MISMATCH (-35) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2i_str_verify | str is malformed: argon2i_verify() returns ARGON2_DECODING_FAIL (-32) / ARGON2_INCORRECT_TYPE (-26) / ARGON2_DECODING_LENGTH_FAIL (-34) / ARGON2_OUTPUT_TOO_SHORT (-2) / ARGON2_SALT_TOO_SHORT (-6) / ARGON2_MEMORY_TOO_LITTLE (-14) / ARGON2_TIME_TOO_SMALL (-12) / ARGON2_LANES_TOO_FEW (-16) | public: returns -1, errno NOT set (left unchanged) |
| G2 | crypto_pwhash_argon2i_str_verify | str is an "$argon2id$..." string (wrong variant for this entry point) | internal ARGON2_DECODING_FAIL (-32) -> returns -1, errno unchanged |
| G2 | crypto_pwhash_argon2id_str_verify | passwdlen > 4294967295 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_argon2id_str_verify | passwdlen < 0 -- statically unreachable (PASSWD_MIN=0) | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_argon2id_str_verify | password mismatch: argon2id_verify() == ARGON2_VERIFY_MISMATCH (-35) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_argon2id_str_verify | str malformed / wrong variant ("$argon2i$..." given to argon2id verifier) | internal ARGON2_DECODING_FAIL (-32) -> returns -1, errno unchanged |
| G2 | _needs_rehash (crypto_pwhash_argon2i_str_needs_rehash / crypto_pwhash_argon2id_str_needs_rehash) | opslimit > UINT32_MAX (4294967295), e.g. 4294967296 | errno=EINVAL, returns -1 |
| G2 | _needs_rehash (both str_needs_rehash) | (memlimit / 1024U) > UINT32_MAX, i.e. memlimit > 4398046511104 | errno=EINVAL, returns -1 |
| G2 | _needs_rehash (both str_needs_rehash) | strlen(str) >= crypto_pwhash_STRBYTES (128) | errno=EINVAL, returns -1 |
| G2 | _needs_rehash (both str_needs_rehash) | calloc(strlen(str),1) returns NULL (OOM; also note strlen(str)==0 -> calloc(0,1)) | returns -1, errno unchanged (from calloc) |
| G2 | _needs_rehash (both str_needs_rehash) | argon2_decode_string(&ctx, str, type) != ARGON2_OK: malformed string, or wrong variant for the requested argon2_type (argon2i string given to argon2id_str_needs_rehash and vice versa) | errno=EINVAL, returns -1 |
| G2 | _needs_rehash (both str_needs_rehash) | decoded ctx.t_cost != (uint32_t)opslimit OR ctx.m_cost != (uint32_t)(memlimit/1024) | returns 1 (rehash needed) -- NOT an error; note memlimit is truncated by /1024 first |
| G2 | argon2_ctx | argon2_validate_inputs(context) != ARGON2_OK | propagates that ARGON2_* enum verbatim |
| G2 | argon2_ctx | type != Argon2_id (2) and type != Argon2_i (1) | ARGON2_INCORRECT_TYPE (-26) |
| G2 | argon2_ctx | argon2_initialize() != ARGON2_OK (allocation failure) | propagates ARGON2_MEMORY_ALLOCATION_ERROR (-22) / ARGON2_INCORRECT_PARAMETER (-25) |
| G2 | argon2_hash | pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF = 4294967295) | ARGON2_PWD_TOO_LONG (-5) |
| G2 | argon2_hash | hashlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | ARGON2_OUTPUT_TOO_LONG (-3) |
| G2 | argon2_hash | saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) | ARGON2_SALT_TOO_LONG (-7) |
| G2 | argon2_hash | malloc(hashlen) == NULL | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | argon2_hash | argon2_ctx() != ARGON2_OK | zeroes+frees out, returns that ARGON2_* enum |
| G2 | argon2_hash | encoded != NULL && encodedlen != 0 && argon2_encode_string() != ARGON2_OK | zeroes out and encoded, returns ARGON2_ENCODING_FAIL (-31) |
| G2 | argon2_verify | strlen(encoded) > UINT32_MAX | ARGON2_DECODING_LENGTH_FAIL (-34) |
| G2 | argon2_verify | any of malloc(ctx.adlen)/malloc(ctx.saltlen)/malloc(ctx.outlen) == NULL, or second malloc(ctx.outlen) == NULL | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | argon2_verify | argon2_decode_string() != ARGON2_OK | propagates decode_result (typically ARGON2_DECODING_FAIL=-32) |
| G2 | argon2_verify | recomputed hash != decoded hash (sodium_memcmp(out, ctx.out, ctx.outlen) != 0) | ARGON2_VERIFY_MISMATCH (-35) |
| G2 | argon2_verify | argon2_hash() on the decoded params fails (e.g. m_cost too small after decode) | propagates that enum (public boundary: -1) |
| G2 | argon2_validate_inputs | context == NULL | ARGON2_INCORRECT_PARAMETER (-25) |
| G2 | argon2_validate_inputs | context->out == NULL | ARGON2_OUTPUT_PTR_NULL (-1) |
| G2 | argon2_validate_inputs | context->outlen < ARGON2_MIN_OUTLEN (16) -- reachable from argon2_decode_string when the encoded hash decodes to < 16 bytes | ARGON2_OUTPUT_TOO_SHORT (-2); public -1 |
| G2 | argon2_validate_inputs | context->outlen > ARGON2_MAX_OUTLEN (0xFFFFFFFF) | ARGON2_OUTPUT_TOO_LONG (-3) |
| G2 | argon2_validate_inputs | context->pwd == NULL && context->pwdlen != 0 | ARGON2_PWD_PTR_MISMATCH (-18) |
| G2 | argon2_validate_inputs | context->pwdlen < ARGON2_MIN_PWD_LENGTH (0) -- statically unreachable | ARGON2_PWD_TOO_SHORT (-4) (dead branch) |
| G2 | argon2_validate_inputs | context->pwdlen > ARGON2_MAX_PWD_LENGTH (0xFFFFFFFF) -- unreachable, field is uint32_t | ARGON2_PWD_TOO_LONG (-5) (dead branch) |
| G2 | argon2_validate_inputs | context->salt == NULL && context->saltlen != 0 | ARGON2_SALT_PTR_MISMATCH (-19) |
| G2 | argon2_validate_inputs | context->saltlen < ARGON2_MIN_SALT_LENGTH (8) -- reachable via argon2_decode_string with a salt shorter than 8 bytes | ARGON2_SALT_TOO_SHORT (-6); public -1 |
| G2 | argon2_validate_inputs | context->saltlen > ARGON2_MAX_SALT_LENGTH (0xFFFFFFFF) -- unreachable (uint32_t) | ARGON2_SALT_TOO_LONG (-7) (dead branch) |
| G2 | argon2_validate_inputs | context->secret == NULL && context->secretlen != 0 | ARGON2_SECRET_PTR_MISMATCH (-20) |
| G2 | argon2_validate_inputs | context->secret != NULL && secretlen < ARGON2_MIN_SECRET (0) -- unreachable | ARGON2_SECRET_TOO_SHORT (-10) (dead branch) |
| G2 | argon2_validate_inputs | context->secret != NULL && secretlen > ARGON2_MAX_SECRET (0xFFFFFFFF) -- unreachable | ARGON2_SECRET_TOO_LONG (-11) (dead branch) |
| G2 | argon2_validate_inputs | context->ad == NULL && context->adlen != 0 | ARGON2_AD_PTR_MISMATCH (-21) |
| G2 | argon2_validate_inputs | context->ad != NULL && adlen < ARGON2_MIN_AD_LENGTH (0) -- unreachable | ARGON2_AD_TOO_SHORT (-8) (dead branch) |
| G2 | argon2_validate_inputs | context->ad != NULL && adlen > ARGON2_MAX_AD_LENGTH (0xFFFFFFFF) -- unreachable | ARGON2_AD_TOO_LONG (-9) (dead branch) |
| G2 | argon2_validate_inputs | context->lanes < ARGON2_MIN_LANES (1), i.e. p=0 in an encoded string | ARGON2_LANES_TOO_FEW (-16); public -1 |
| G2 | argon2_validate_inputs | context->lanes > ARGON2_MAX_LANES (0xFFFFFF = 16777215), e.g. p=16777216 in an encoded string | ARGON2_LANES_TOO_MANY (-17); public -1 |
| G2 | argon2_validate_inputs | context->m_cost < ARGON2_MIN_MEMORY (2*ARGON2_SYNC_POINTS = 8), e.g. m=0..7 in an encoded string | ARGON2_MEMORY_TOO_LITTLE (-14); public -1 |
| G2 | argon2_validate_inputs | context->m_cost > ARGON2_MAX_MEMORY (min(0xFFFFFFFF, 1<<min(32, sizeof(void*)*8-11)) = 0xFFFFFFFF on 64-bit, 0x200000=2097152 on 32-bit) | ARGON2_MEMORY_TOO_MUCH (-15) |
| G2 | argon2_validate_inputs | context->m_cost < 8 * context->lanes (e.g. m=8, p=2 in an encoded string) | ARGON2_MEMORY_TOO_LITTLE (-14); public -1 |
| G2 | argon2_validate_inputs | context->t_cost < ARGON2_MIN_TIME (1), i.e. t=0 in an encoded string | ARGON2_TIME_TOO_SMALL (-12); public -1 |
| G2 | argon2_validate_inputs | context->t_cost > ARGON2_MAX_TIME (0xFFFFFFFF) -- unreachable (uint32_t) | ARGON2_TIME_TOO_LARGE (-13) (dead branch) |
| G2 | argon2_validate_inputs | context->threads < ARGON2_MIN_THREADS (1) -- threads is set = lanes, so shadowed by LANES_TOO_FEW | ARGON2_THREADS_TOO_FEW (-28) |
| G2 | argon2_validate_inputs | context->threads > ARGON2_MAX_THREADS (0xFFFFFF) -- shadowed by LANES_TOO_MANY | ARGON2_THREADS_TOO_MANY (-29) |
| G2 | allocate_memory (argon2-core.c) | region == NULL | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | allocate_memory (argon2-core.c) | m_cost == 0, or sizeof(block)*m_cost overflows (memory_size/m_cost != sizeof(block), block=1024 bytes) | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | allocate_memory (argon2-core.c) | malloc(sizeof(block_region)) == NULL | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | allocate_memory (argon2-core.c) | mmap == MAP_FAILED, or posix_memalign(...,64,memory_size) != 0 (errno is assigned the posix_memalign return value), or malloc(memory_size+63) == NULL / memory_size+63 overflows (errno = ENOMEM) | frees region, *region=NULL, ARGON2_MEMORY_ALLOCATION_ERROR (-22); errno = posix_memalign rc or ENOMEM |
| G2 | argon2_initialize | instance == NULL or context == NULL | ARGON2_INCORRECT_PARAMETER (-25) |
| G2 | argon2_initialize | malloc(sizeof(uint64_t)*instance->segment_length) == NULL | ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | argon2_initialize | allocate_memory() != ARGON2_OK | frees instance, propagates ARGON2_MEMORY_ALLOCATION_ERROR (-22) |
| G2 | decode_decimal (argon2-encoding.c) | no digit at the current position (e.g. "$m=x", "$m=$") | returns NULL -> caller returns ARGON2_DECODING_FAIL (-32) |
| G2 | decode_decimal (argon2-encoding.c) | non-minimal encoding: leading zero with more than one digit (e.g. "m=08", "v=019") | returns NULL -> ARGON2_DECODING_FAIL (-32) |
| G2 | decode_decimal (argon2-encoding.c) | accumulator overflow: acc > ULONG_MAX/10 or digit > ULONG_MAX-acc | returns NULL -> ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | type == Argon2_id but str does not start with "$argon2id"; or type == Argon2_i but str does not start with "$argon2i" | ARGON2_DECODING_FAIL (-32); public -1 |
| G2 | argon2_decode_string | type is neither Argon2_i nor Argon2_id | ARGON2_INCORRECT_TYPE (-26) |
| G2 | argon2_decode_string | missing mandatory "$v=" segment (e.g. old-style "$argon2i$m=...", or "$argon2id$..." fed with type=Argon2_i where the leftover is "d$v=") | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | version decodes but > UINT32_MAX, or is not a minimal decimal | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | version != ARGON2_VERSION_NUMBER (0x13 = 19), e.g. "$v=16" (Argon2 1.0) or "$v=20" | ARGON2_INCORRECT_TYPE (-26); public -1 |
| G2 | argon2_decode_string | missing "$m=" after the version | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | m value not a minimal decimal or > UINT32_MAX (e.g. "m=4294967296") | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | m > UINT32_MAX post-check | ARGON2_INCORRECT_TYPE (-26) (dead branch; DECIMAL_U32 already rejects) |
| G2 | argon2_decode_string | missing ",t=" separator | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | t value not a minimal decimal or > UINT32_MAX | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | t > UINT32_MAX post-check | ARGON2_INCORRECT_TYPE (-26) (dead branch) |
| G2 | argon2_decode_string | missing ",p=" separator | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | p value not a minimal decimal or > UINT32_MAX | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | p > UINT32_MAX post-check | ARGON2_INCORRECT_TYPE (-26) (dead branch) |
| G2 | argon2_decode_string | missing "$" before the salt field | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | salt field: sodium_base642bin(..., sodium_base64_VARIANT_ORIGINAL_NO_PADDING) != 0 -- invalid base64 char set / decoded salt longer than ctx->saltlen buffer | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | salt field decodes to bin_len > UINT32_MAX | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | missing "$" between salt and hash | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | hash field: sodium_base642bin fails (invalid chars, or decoded length > ctx->outlen buffer), or bin_len > UINT32_MAX | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_decode_string | base64 fields use '=' padding (VARIANT_ORIGINAL_NO_PADDING stops at '=', so the following CC("$")/end-of-string check fails) | ARGON2_DECODING_FAIL (-32); public -1 |
| G2 | argon2_decode_string | argon2_validate_inputs(ctx) fails on the decoded parameters (m<8, m<8*p, t=0, p=0, salt<8 bytes, hash<16 bytes) | propagates ARGON2_MEMORY_TOO_LITTLE(-14)/ARGON2_TIME_TOO_SMALL(-12)/ARGON2_LANES_TOO_FEW(-16)/ARGON2_SALT_TOO_SHORT(-6)/ARGON2_OUTPUT_TOO_SHORT(-2); public -1 |
| G2 | argon2_decode_string | trailing characters after the final base64 field (*str != 0), e.g. "...$hash$extra" or "...$hash\n" | ARGON2_DECODING_FAIL (-32) |
| G2 | argon2_encode_string | type is neither Argon2_i nor Argon2_id | ARGON2_ENCODING_FAIL (-31) |
| G2 | argon2_encode_string | dst_len too small for any literal/decimal segment (strlen(seg) >= dst_len) | ARGON2_ENCODING_FAIL (-31) |
| G2 | argon2_encode_string | sodium_bin2base64(dst, dst_len, salt/out, ...) returns NULL (destination buffer too small) | ARGON2_ENCODING_FAIL (-31) |
| G2 | argon2_encode_string | argon2_validate_inputs(ctx) != ARGON2_OK | propagates the validate enum (not ARGON2_ENCODING_FAIL) |
| G2 | blake2b_long | outlen > UINT32_MAX | returns -1 (ret initialized to -1, goto fail) |
| G2 | blake2b_long | any crypto_generichash_blake2b_init/update/final returns < 0 | returns that negative value (-1) |
| G2 | crypto_pwhash_scryptsalsa208sha256 | passwdlen > crypto_pwhash_scryptsalsa208sha256_PASSWD_MAX (SODIUM_SIZE_MAX = min(UINT64_MAX,SIZE_MAX)) -- unreachable on 64-bit | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256 | outlen > crypto_pwhash_scryptsalsa208sha256_BYTES_MAX (0x1fffffffe0 = 137438953440) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256 | outlen < crypto_pwhash_scryptsalsa208sha256_BYTES_MIN (16), i.e. 0..15 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256 | pickparams() != 0 -- statically unreachable (pickparams always returns 0); NOTE opslimit/memlimit are NOT range-checked here, so opslimit<OPSLIMIT_MIN(32768) and memlimit<MEMLIMIT_MIN(16777216) are silently clamped/accepted | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256 | out == passwd (aliasing) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256 | crypto_pwhash_scryptsalsa208sha256_ll() fails on the derived (N,r,p) -- e.g. pickparams yields p==0 | returns -1 with errno from the _ll layer (EINVAL/ENOMEM/EFBIG) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen > PASSWD_MAX (SODIUM_SIZE_MAX) | errno=EFBIG, returns -1; out fully zeroed (102 bytes) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | passwdlen < PASSWD_MIN (0) -- unreachable; or pickparams()!=0 -- unreachable | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_gensalt_r(N_log2, r, p, salt, 32, setting, 58) == NULL (N_log2>63 or (uint64_t)r*p >= 2^30) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_init_local() != 0 -- statically always 0 | returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str | escrypt_r(...) == NULL (KDF failure / allocation failure) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | sodium_strnlen(str,102) != 101 -- i.e. str is not exactly crypto_pwhash_scryptsalsa208sha256_STRBYTES-1 = 101 chars (too short, empty, or >=102) | returns -1, errno NOT set |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_init_local() != 0 | returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | escrypt_r() == NULL: str is 101 chars but not a valid "$7$" setting (bad prefix, bad itoa64 chars), or the salt field length makes need > 102, or the KDF rejects N/r/p | returns -1, errno may be set by the KDF |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_verify | password does not match: sodium_memcmp(wanted, str, 102) != 0 | returns -1 (the sodium_memcmp result), errno unchanged |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | pickparams() != 0 -- unreachable | errno=EINVAL, returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | sodium_strnlen(str,102) != 101 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | escrypt_parse_setting(str,...) == NULL (str does not start with "$7$", or N_log2/r/p chars are outside "./0-9A-Za-z") | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | parsed N_log2/r/p differ from the ones pickparams derives from (opslimit, memlimit) | returns 1 (rehash needed) -- not an error |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_init_local() != 0 | returns -1 (dead branch) |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_free_local() != 0 (munmap failure) | returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | buflen > ((2^32)-1)*32 = 137438953440 (only compiled when SIZE_MAX > UINT32_MAX) | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | (uint64_t)r * (uint64_t)p >= 2^30 (1073741824), e.g. r=1,p=1073741824 or r=32768,p=32768 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | N > UINT32_MAX (4294967295), e.g. N=2^33 | errno=EFBIG, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | N is not a power of two ((N & (N-1)) != 0), e.g. N=3, N=1000 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | N < 2 (N=0 or N=1) | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | r == 0 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | p == 0 | errno=EINVAL, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | r > SIZE_MAX/128/p, or (32-bit only) r > SIZE_MAX/256, or N > SIZE_MAX/128/r | errno=ENOMEM, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | size_t overflow in need = B_size + V_size (need < V_size) or need += XY_size (need < XY_size) | errno=ENOMEM, returns -1 |
| G2 | crypto_pwhash_scryptsalsa208sha256_ll -> escrypt_kdf_nosse/sse | escrypt_free_region() fails, or escrypt_alloc_region(local, need) returns NULL (mmap/posix_memalign/malloc failure) | returns -1, errno from posix_memalign or ENOMEM |
| G2 | escrypt_PBKDF2_SHA256 (pbkdf2-sha256.c) | dkLen > 0x1fffffffe0ULL (137438953440) -- only compiled when SIZE_MAX > 0x1fffffffe0 | sodium_misuse() -> abort()/SIGABRT (no return value) |
| G2 | escrypt_parse_setting | setting[0..2] != "$7$" | returns NULL (callers map to -1 / EINVAL) |
| G2 | escrypt_parse_setting | setting[3] (N_log2 char) not in "./0123456789A-Za-z" (decode64_one fails) | returns NULL, *N_log2_p = 0 |
| G2 | escrypt_parse_setting | any of the 5 r characters not in the itoa64 alphabet (decode64_uint32 30 bits) | returns NULL, *r_p = 0 |
| G2 | escrypt_parse_setting | any of the 5 p characters not in the itoa64 alphabet | returns NULL, *p_p = 0 |
| G2 | decode64_one | src byte not present in "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz" (e.g. '$', '-', '+', 0x00, 0xFF) | *dst=0, returns -1 |
| G2 | decode64_uint32 | decode64_one fails on any of the ceil(dstbits/6) characters | *dst=0, returns NULL |
| G2 | encode64_uint32 | dstlen < 1 while bits remain | returns NULL |
| G2 | encode64 | encode64_uint32 returns NULL (destination exhausted) | returns NULL |
| G2 | escrypt_r | escrypt_parse_setting(setting) == NULL | returns NULL |
| G2 | escrypt_r | buf == NULL | returns NULL |
| G2 | escrypt_r | need = prefixlen + saltlen + 1 + 43 + 1 > buflen (with buflen=102 this means the salt field is longer than 43 chars) | returns NULL |
| G2 | escrypt_r | need < saltlen (size_t overflow) | returns NULL |
| G2 | escrypt_r | escrypt_kdf(local, ..., N=1<<N_log2, r, p, hash, 32) != 0 -- e.g. N_log2=0 gives N=1 (< 2) -> EINVAL, or r=0/p=0 from the setting | returns NULL, errno set by the KDF |
| G2 | escrypt_r | encode64() returns NULL, or dst >= buf + buflen after encoding | returns NULL ("can't happen") |
| G2 | escrypt_gensalt_r | need = 14 + BYTES2CHARS(srclen) + 1 > buflen | returns NULL |
| G2 | escrypt_gensalt_r | need < saltlen (overflow) or saltlen < srclen | returns NULL |
| G2 | escrypt_gensalt_r | N_log2 > 63 | returns NULL |
| G2 | escrypt_gensalt_r | (uint64_t)r * (uint64_t)p >= (1U << 30) | returns NULL |
| G2 | escrypt_gensalt_r | encode64_uint32/encode64 returns NULL, or dst >= buf + buflen | returns NULL |
| G2 | escrypt_alloc_region (scrypt_platform.c) | mmap fails (MAP_FAILED), posix_memalign(...,64,size) != 0 (errno = its return value), size+63 overflows (errno = ENOMEM), or malloc(size+63) == NULL | returns NULL; region->base=NULL, region->size=0 |
| G2 | escrypt_free_region (scrypt_platform.c) | munmap(region->base, region->size) fails | returns -1 |
