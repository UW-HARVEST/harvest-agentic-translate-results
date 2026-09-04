| pwhash-1 | crypto_pwhash_alg_argon2i13, crypto_pwhash_alg_argon2id13, crypto_pwhash_alg_default, crypto_pwhash_bytes_min/max, crypto_pwhash_passwd_min/max, crypto_pwhash_saltbytes, crypto_pwhash_strbytes, crypto_pwhash_strprefix, crypto_pwhash_primitive, crypto_pwhash_opslimit_min/max/interactive/moderate/sensitive, crypto_pwhash_memlimit_min/max/interactive/moderate/sensitive | all 21 generic getters, value-checked against the header macros | [x] |
| pwhash-2 | crypto_pwhash_argon2i_alg_argon2i13, _bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/moderate/sensitive, _memlimit_min/max/interactive/moderate/sensitive | all 18 argon2i getters | [x] |
| pwhash-3 | crypto_pwhash_argon2id_alg_argon2id13, _bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/moderate/sensitive, _memlimit_min/max/interactive/moderate/sensitive | all 18 argon2id getters | [x] |
| pwhash-4 | crypto_pwhash_scryptsalsa208sha256_bytes_min/max, _passwd_min/max, _saltbytes, _strbytes, _strprefix, _opslimit_min/max/interactive/sensitive, _memlimit_min/max/interactive/sensitive | all 15 scrypt getters (BYTES_MAX = min(SIZE_MAX,0x1fffffffe0), PASSWD_MAX = SIZE_MAX) | [x] |
| pwhash-5 | _crypto_pwhash_argon2_pick_best_implementation | no SIMD compiled -> always selects argon2_fill_segment_ref, returns 0 | [x] |
| pwhash-6 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2I13 (1), opslimit=3, memlimit=8192, outlen=16 | [x] |
| pwhash-7 | crypto_pwhash | alg=crypto_pwhash_ALG_ARGON2ID13 (2), opslimit=1, memlimit=8192, outlen=16 | [x] |
| pwhash-8 | crypto_pwhash | out-of-range enum alg = 0, 3, -1, 999, INT_MIN, INT_MAX -> -1/EINVAL, full out buffer compared | [x] |
| pwhash-9 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | passwdlen = 0, 1, 63, 64, 65, 200 x 3 random cases each, random salt per case | [x] |
| pwhash-10 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | outlen = 15 (MIN-1, rejected), 16 (MIN), 17, 31, 32, 64, 200; canary-filled out buffer, full-buffer compare | [x] |
| pwhash-11 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | opslimit = 0, MIN-1, MIN, MIN+1, MIN+2, 2^32, UINT64_MAX | [x] |
| pwhash-12 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | memlimit = 0, 1, 8191, 8192, 8193 (not 1024-aligned), 9215, 16384, 65536, MEMLIMIT_MAX+1, SIZE_MAX | [x] |
| pwhash-13 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | alg argument = own alg (ok) / other family's alg / 0 / 3 / -1 / 999 (all rejected) | [x] |
| pwhash-14 | crypto_pwhash_argon2i, crypto_pwhash_argon2id | out == passwd (same pointer) -> EINVAL | [x] |
| pwhash-15 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | passwdlen = 0, 1, 32, 200 x 3 cases, opslimit=MIN, memlimit=8192, deterministic randombytes -> byte-exact 128-byte out buffer | [x] |
| pwhash-16 | crypto_pwhash_argon2i_str, crypto_pwhash_argon2id_str | out-of-range opslimit/memlimit/passwdlen (7 rejection combinations), full out buffer compared | [x] |
| pwhash-17 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify, crypto_pwhash_str_verify | correct password (0), wrong password (-1/EINVAL), passwdlen=2^32 (EFBIG) | [x] |
| pwhash-18 | crypto_pwhash_argon2i_str_verify, crypto_pwhash_argon2id_str_verify, crypto_pwhash_str_verify | 21 malformed hash strings x 3 verifiers: valid-other-alg, corrupted b64 hash char, corrupted salt char, corrupted `m=` digit, empty, missing leading `$`, prefix only, truncated (half), missing hash field, over-long b64, over-short b64, trailing garbage, wrong version, leading-zero decimal, decimal > 2^32, `p=0`, `$7$` (scrypt) prefix | [x] |
| pwhash-19 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash, crypto_pwhash_str_needs_rehash | matching params -> 0, different opslimit -> 1, different memlimit -> 1 | [x] |
| pwhash-20 | crypto_pwhash_argon2i_str_needs_rehash, crypto_pwhash_argon2id_str_needs_rehash, crypto_pwhash_str_needs_rehash | 21 malformed strings x 4 (opslimit, memlimit) combos incl. opslimit>UINT32_MAX and memlimit=SIZE_MAX; plus a 128-char string (>= crypto_pwhash_STRBYTES) | [x] |
| pwhash-21 | crypto_pwhash_str | == crypto_pwhash_argon2id_str; byte-exact under deterministic RNG, cross-verified with crypto_pwhash_str_verify | [x] |
| pwhash-22 | crypto_pwhash_str_alg | alg = 1 and 2 (byte-exact out), plus opslimit/memlimit rejection paths for both algs | [x] |
| pwhash-23 | _sodium_argon2i_hash_raw, _sodium_argon2id_hash_raw | 9 (t_cost, m_cost, parallelism) combos x hashlen 16/17/31/32/64/65/100, random pwd (1..40) and salt (8..32) | [x] |
| pwhash-24 | _sodium_argon2i_hash_encoded, _sodium_argon2id_hash_encoded | same 9 x 7 grid, 256-byte encoded buffer, canary-compared in full | [x] |
| pwhash-25 | _sodium_argon2i_hash_raw, _sodium_argon2id_hash_raw | rejected parameter combos with exact ARGON2_* codes: t_cost=0, m_cost=7, m_cost=0, lanes=0, m_cost<8*lanes (2 shapes), lanes=0x1000000 | [x] |
| pwhash-26 | _sodium_argon2i_hash_encoded | hashlen = 0/15 (OUTPUT_TOO_SHORT), 16 (ok), 2^32 and SIZE_MAX (OUTPUT_TOO_LONG) | [x] |
| pwhash-27 | _sodium_argon2i_hash_encoded | pwdlen = 2^32 / SIZE_MAX (PWD_TOO_LONG); saltlen = 0/1/7 (SALT_TOO_SHORT), 2^32 / SIZE_MAX (SALT_TOO_LONG) | [x] |
| pwhash-28 | _sodium_argon2i_hash_encoded | pwd = NULL with pwdlen = 0 (ok) and 1 (PWD_PTR_MISMATCH); salt = NULL with saltlen 0 / 8 | [x] |
| pwhash-29 | _sodium_argon2id_hash_encoded | encodedlen too small (1, 5, 12, 13, 20, 26, 27) -> ARGON2_ENCODING_FAIL | [x] |
| pwhash-30 | _sodium_argon2_hash | type = 1, 2 (ok) and 0, 3, -1, 999 (out-of-range enum -> ARGON2_INCORRECT_TYPE); hash AND encoded requested simultaneously | [x] |
| pwhash-31 | _sodium_argon2_hash | hash = NULL && encoded = NULL (no output requested); encoded != NULL with encodedlen = 0 (encoding skipped) | [x] |
| pwhash-32 | _sodium_argon2_verify, _sodium_argon2i_verify, _sodium_argon2id_verify | 2 types x 3 random (t_cost, m_cost, pwd, salt) cases: correct password, wrong password (VERIFY_MISMATCH), wrong type, out-of-range type (0/3/-1/999), empty encoded string | [x] |
| pwhash-33 | _sodium_argon2_validate_inputs | 30-row matrix: NULL context, out=NULL, outlen 0/1/15/16/UINT32_MAX, pwd NULL x len, salt NULL x len, saltlen 0/1/7/8, secret NULL/set x len, ad NULL/set x len, lanes 0/0xFFFFFF/0x1000000, m_cost 0/1/7/8/15/16/UINT32_MAX, m_cost<8*lanes, t_cost 0/UINT32_MAX, threads 0/0xFFFFFF/0x1000000; context must not be mutated | [x] |
| pwhash-34 | _sodium_argon2_ctx | 6 (t_cost, m_cost, lanes) combos x 2 random cases x outlen 16/32/64/80 x type 1/2, secret and ad both non-NULL | [x] |
| pwhash-35 | _sodium_argon2_ctx | out-of-range type = 0, 3, -1, 999, INT_MIN -> ARGON2_INCORRECT_TYPE; validation failures propagated unchanged; NULL context | [x] |
| pwhash-36 | _sodium_argon2_initialize, _sodium_argon2_fill_memory_blocks, _sodium_argon2_finalize | 5 (t_cost, m_cost, lanes, type) instances driven directly; the 2 first blocks per lane compared after initialize, the WHOLE memory region (memory_blocks x 1024 B) compared after every pass, out compared after finalize, region/pseudo_rands freed | [x] |
| pwhash-37 | _sodium_argon2_fill_segment_ref | same 5 instances, every (pass, slice, lane) segment driven by hand; region compared after each complete pass (exercises starting_index=2, prev_offset wraparound, data-independent vs data-dependent addressing for Argon2i/Argon2id) | [x] |
| pwhash-38 | _sodium_argon2_initialize, _sodium_argon2_fill_memory_blocks, _sodium_argon2_finalize, _sodium_argon2_fill_segment_ref | NULL-pointer / lanes==0 early-return paths | [x] |
| pwhash-39 | _sodium_argon2_encode_string | 8 random (saltlen 8..32, outlen 16..65) x m_cost/t_cost/lanes from {8,9,100,65536,UINT32_MAX} x {1,2,7,UINT32_MAX,3} x {1..5}, type 1/2/0/3/-1, dst_len = 0/1/5/11/12/13/header_len-1/header_len/need/need+1/512 | [x] |
| pwhash-40 | _sodium_argon2_encode_string | invalid ctx (outlen=0, lanes=0, out=NULL) -> validation code returned after the prefix has been written | [x] |
| pwhash-41 | _sodium_argon2_decode_string | 41 input strings x type 1/2/0/3/-1/999 x 4 (max saltlen, max outlen) shapes incl. (0,0); all out-params and both scratch buffers compared | [x] |
| pwhash-42 | _sodium_blake2b_long | outlen = 0,1,2,15,16,31,32,63,64 (= BYTES_MAX, single-shot path), 65,66,95,96,97,127,128,129,200,1024,1025 (multi-block extension path) x inlen = 0,1,4,63,64,72,128,1024; canary buffer compared in full; in=NULL with inlen=0 | [x] |
| pwhash-43 | crypto_pwhash_scryptsalsa208sha256_ll | 11 (N, r, p) combos {N=2..1024, r=1..8, p=1..3} x buflen = 0,1,16,31,32,33,64,100 x 2 random (passwd, salt) cases | [x] |
| pwhash-44 | _sodium_escrypt_kdf_nosse | same grid via a caller-managed escrypt_local_t, called twice per region so the "region already large enough" reuse path is taken; output cross-checked against crypto_pwhash_scryptsalsa208sha256_ll | [x] |
| pwhash-45 | crypto_pwhash_scryptsalsa208sha256_ll | rejected params with exact errno: N=0/1/3/5/6/1000/0xFFFFFFFF (EINVAL), N=2^32/UINT64_MAX (EFBIG), r=0, p=0, r=0&&p=0 (EINVAL), r*p=2^30 and r=p=0xFFFFFFFF (EFBIG), buflen=2^37 (EFBIG), N=2^31 & r=2^27 (ENOMEM) | [x] |
| pwhash-46 | crypto_pwhash_scryptsalsa208sha256_ll | escrypt_alloc_region failure (N=2^30, r=2^26 -> ~2^63 bytes) -> -1 | [x] |
| pwhash-47 | _sodium_escrypt_PBKDF2_SHA256 | c = 0, 1, 2, 3, 10 x dkLen = 0,1,2,31,32,33,63,64,65,100,128 x 4 random (passwd, salt) cases; canary buffer compared in full | [x] |
| pwhash-48 | _sodium_escrypt_alloc_region, _sodium_escrypt_free_region | size = 0,1,63,64,65,4096,2^20: returns 64-byte-aligned `aligned` within [base, base+63], records size, frees and re-zeroes the region | [x] |
| pwhash-49 | _sodium_escrypt_alloc_region | size = SIZE_MAX and SIZE_MAX-62 (size+63 overflows) -> NULL / size 0 / ENOMEM; size = 2^62 (malloc failure) | [x] |
| pwhash-50 | _sodium_escrypt_init_local, _sodium_escrypt_free_local | init_local zeroes a dirty region and returns 0; free_local/free_region on a zeroed region are no-ops returning 0 | [x] |
| pwhash-51 | _sodium_escrypt_gensalt_r | srclen = 0,1,2,3,4,31,32,33,48 x 10 (N_log2, r, p) combos incl. N_log2 = 0/63/64/255 and r*p >= 2^30 and r=p=0 x buflen = 0/1/need-1/need/need+1/128; full 256-byte buffer compared | [x] |
| pwhash-52 | _sodium_escrypt_parse_setting | 5 gensalt_r-produced settings (N_log2 = 0,1,10,14,63) + 11 malformed: empty, `$`, `$7`, `$7$`, `$8$...`, missing `$`, invalid N_log2/r/p characters, salt with `$hash` suffix, truncated r field; returned offset and all three out-params compared | [x] |
| pwhash-53 | _sodium_escrypt_r | 5 (N_log2, r, p) settings x passwdlen 0/1/32 x buflen 102 (exact `need`)/103/200, deterministic RNG -> full 256-byte buffer byte-exact | [x] |
| pwhash-54 | _sodium_escrypt_r | buflen = 0/1/50/101 (< need) -> NULL; invalid settings -> NULL; buf = NULL -> NULL with no randombytes consumed | [x] |
| pwhash-55 | crypto_pwhash_scryptsalsa208sha256 | 7 (opslimit, memlimit) combos incl. (0,0) and (1,0) which exercise both pickparams branches x outlen 16/17/32/64/100 x 3 random passwd cases | [x] |
| pwhash-56 | crypto_pwhash_scryptsalsa208sha256 | outlen = 0/1/15 (< BYTES_MIN) -> EINVAL; out == passwd -> EINVAL | [x] |
| pwhash-57 | crypto_pwhash_scryptsalsa208sha256_str | 4 (opslimit, memlimit) combos x passwdlen 0/1/32 x 3 cases, deterministic RNG -> byte-exact 102-byte string, `$7$` prefix, NUL at index 101 | [x] |
| pwhash-58 | crypto_pwhash_scryptsalsa208sha256_str_verify | correct password (0), wrong password (-1) | [x] |
| pwhash-59 | crypto_pwhash_scryptsalsa208sha256_str_needs_rehash | matching params -> 0; 3 other (opslimit, memlimit) combos | [x] |
| pwhash-60 | crypto_pwhash_scryptsalsa208sha256_str_verify, _str_needs_rehash | 6 malformed strings: empty, short, 101 garbage chars, right length + `$7$` + invalid base64, 102 bytes with NO NUL terminator (sodium_strnlen boundary), 102 chars + NUL | [x] |
