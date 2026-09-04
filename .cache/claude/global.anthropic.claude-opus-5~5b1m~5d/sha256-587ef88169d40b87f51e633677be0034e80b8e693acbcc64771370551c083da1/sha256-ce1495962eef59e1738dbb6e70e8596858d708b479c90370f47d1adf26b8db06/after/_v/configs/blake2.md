| blake2-1 | crypto_generichash_bytes_min/max/bytes, crypto_generichash_keybytes_min/max/keybytes, crypto_generichash_statebytes, crypto_generichash_primitive | all compile-time getters (16/64/32, 16/64/32, 384, "blake2b") | [x] |
| blake2-2 | crypto_generichash_blake2b_bytes_min/max/bytes, _keybytes_min/max/keybytes, _saltbytes, _personalbytes, _statebytes | all compile-time getters (16/64/32, 16/64/32, 16, 16, 384) | [x] |
| blake2-3 | crypto_generichash, crypto_generichash_blake2b | outlen ∈ {1,15,16,17,31,32,33,63,64} × keylen ∈ {0,1,15,16,31,32,33,63,64} × inlen ∈ {0,1,2,7,8,63,64,127,128,129,191,192,255,256,257,383,384,385,1000,4096}; canary-guarded output | [x] |
| blake2-4 | crypto_generichash_blake2b | key != NULL with keylen == 0 → unkeyed path, must equal key == NULL | [x] |
| blake2-5 | crypto_generichash_blake2b | in == NULL with inlen == 0 (misuse check does not fire) | [x] |
| blake2-6 | crypto_generichash_blake2b_salt_personal | salt/personal ∈ {NULL, random, all-zero}² (5 combinations incl. NULL==zeros equivalence) × keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,64,128,129,256,257,1000} | [x] |
| blake2-7 | crypto_generichash_init, crypto_generichash_update, crypto_generichash_final | keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,63,64,127,128,129,191,192,255,256,257,300,512,1000,4096}, randomized chunk splits including leading/interior/trailing 0-length updates; whole 384-byte opaque state compared after init, after every update and after final; digest cross-checked against the one-shot API | [x] |
| blake2-8 | crypto_generichash_blake2b_init, _update, _final | same matrix as blake2-7 through the blake2b-specific entry points, randomized chunk splits, full state compare | [x] |
| blake2-9 | crypto_generichash_blake2b_init_salt_personal, _update, _final | salt × personal ∈ {NULL, set}² (4 combinations) × keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,128,256,257,700}, randomized chunk splits, full state compare, digest == crypto_generichash_blake2b_salt_personal | [x] |
| blake2-10 | crypto_generichash_blake2b_init, _init_salt_personal | key == NULL with keylen ∈ {1,16,32,64} selects the *unkeyed* branch; state must equal keylen == 0 | [x] |
| blake2-11 | crypto_generichash_blake2b_update, _final | update after final (state has f[0] != 0); update with in == NULL, inlen == 0 | [x] |
| blake2-12 | _sodium_blake2b_init | every valid outlen 1..=64, full state compare | [x] |
| blake2-13 | _sodium_blake2b_init_salt_personal | salt × personal ∈ {NULL, set}² × outlen ∈ {1,16,32,64}, full state compare | [x] |
| blake2-14 | _sodium_blake2b_init_key | keylen 1..=64 × outlen ∈ {1,32,64}, full state compare (includes the 128-byte key-block update) | [x] |
| blake2-15 | _sodium_blake2b_init_key_salt_personal | keylen 1..=64 × outlen ∈ {1,32,64} × salt/personal set and NULL, full state compare | [x] |
| blake2-16 | _sodium_blake2b_init_param, _sodium_blake2b_update, _sodium_blake2b_final | 40 fully random 64-byte parameter blocks (arbitrary digest_length/key_length/fanout/depth/leaf_length/node_offset/node_depth/inner_length/reserved/salt/personal), random inlen 0..600, random outlen 1..64 | [x] |
| blake2-17 | _sodium_blake2b_update, _sodium_blake2b_final (blake2b_set_lastnode) | state with last_node = 1 poked directly at offset 360 (unreachable via public API) × inlen ∈ {0,1,128,257,600} | [x] |
| blake2-18 | _sodium_blake2b | keylen ∈ {0,1,16,32,64} × outlen ∈ {1,16,32,64} × inlen ∈ {0,1,127,128,129,256,257,1024}; in = NULL when inlen = 0; result == crypto_generichash_blake2b | [x] |
| blake2-19 | _sodium_blake2b_salt_personal | same matrix as blake2-18 × salt/personal ∈ {NULL, set}² | [x] |
| blake2-20 | _sodium_blake2b_compress_ref | 64 fully random 384-byte states (arbitrary h/t/f including carry and both finalisation flags) × random 128-byte blocks, plus all-zero and all-0xff states | [x] |
| blake2-21 | _sodium_blake2b_pick_best_implementation, _crypto_generichash_blake2b_pick_best_implementation | no HAVE_*INTRIN_H macros ⇒ always selects blake2b_compress_ref; return value + hashing still correct afterwards | [x] |
| blake2-22 | _sodium_blake2b_long | outlen ∈ {0,1,16,32,63,64} (single-pass branch), {65,66,95,96} (chained, no loop iteration), {97,127,128,129,160,192,200,1000} (chained with 1..n loop iterations) × inlen ∈ {0,1,64,128,257,1000} | [x] |
| blake2-23 | crypto_generichash_keygen, crypto_generichash_blake2b_keygen, crypto_kdf_keygen, crypto_shorthash_keygen, crypto_kdf_hkdf_sha256_keygen, crypto_kdf_hkdf_sha512_keygen | writes exactly KEYBYTES (32/32/32/16/32/64) bytes, canary beyond untouched (bytes are randombytes_buf output, so not comparable) | [x] |
| blake2-24 | crypto_kdf_bytes_min/max, crypto_kdf_contextbytes, crypto_kdf_keybytes, crypto_kdf_primitive, crypto_kdf_blake2b_bytes_min/max, _contextbytes, _keybytes | all compile-time getters (16/64/8/32/"blake2b") | [x] |
| blake2-25 | crypto_kdf_blake2b_derive_from_key, crypto_kdf_derive_from_key | subkey_len ∈ {16,17,31,32,33,63,64} × subkey_id ∈ {0,1,2,0xff,0x100,0xffffffff,0x100000000,u64::MAX,0x0123456789abcdef} × random 32-byte key and 8-byte ctx; result independently reproduced through crypto_generichash_blake2b_salt_personal(salt=LE64(id)‖0, personal=ctx‖0) | [x] |
| blake2-26 | crypto_kdf_blake2b_derive_from_key | ctx containing embedded NUL bytes (ctx is a fixed 8-byte array, not a C string) | [x] |
| blake2-27 | crypto_kdf_hkdf_sha256_keybytes/bytes_min/bytes_max/statebytes | getters (32 / 0 / 0xff*32 = 8160 / 208) | [x] |
| blake2-28 | crypto_kdf_hkdf_sha256_extract | salt_len ∈ {0,1,16,32,55,63,64,65,100,128,200} × ikm_len ∈ {0,1,16,32,64,100,127,128,129,500} (crosses the 64-byte HMAC key-compression boundary) | [x] |
| blake2-29 | crypto_kdf_hkdf_sha256_extract_init, _extract_update, _extract_final | same salt/ikm matrix, randomized chunk splits including 0-length updates; whole 208-byte state compared after init and after every update; extract_final must zero the state; streamed prk == one-shot prk | [x] |
| blake2-30 | crypto_kdf_hkdf_sha256_extract | salt == NULL / salt_len == 0; ikm == NULL / ikm_len == 0 | [x] |
| blake2-31 | crypto_kdf_hkdf_sha256_expand | out_len ∈ {0,1,31,32,33,63,64,65,96,100,255,1000,8159,8160} (0, exactly-one-block, one-past-block, multi-block, `left != 0` tail, and BYTES_MAX) × ctx_len ∈ {0,1,8,32,64,200}, canary-guarded output | [x] |
| blake2-32 | crypto_kdf_hkdf_sha256_expand | ctx == NULL with ctx_len == 0 | [x] |
| blake2-33 | crypto_kdf_hkdf_sha512_keybytes/bytes_min/bytes_max/statebytes | getters (64 / 0 / 0xff*64 = 16320 / 416) | [x] |
| blake2-34 | crypto_kdf_hkdf_sha512_extract | salt_len ∈ {0,1,16,32,55,63,64,65,100,128,200} × ikm_len ∈ {0,1,16,32,64,100,127,128,129,500} (crosses the 128-byte HMAC key-compression boundary) | [x] |
| blake2-35 | crypto_kdf_hkdf_sha512_extract_init, _extract_update, _extract_final | same matrix, randomized chunk splits including 0-length updates, whole 416-byte state compared after init/each update, state zeroed by final, streamed == one-shot | [x] |
| blake2-36 | crypto_kdf_hkdf_sha512_extract | salt == NULL / salt_len == 0; ikm == NULL / ikm_len == 0 | [x] |
| blake2-37 | crypto_kdf_hkdf_sha512_expand | out_len ∈ {0,1,63,64,65,127,128,129,192,100,255,1000,16319,16320} × ctx_len ∈ {0,1,8,32,64,200}, canary-guarded output | [x] |
| blake2-38 | crypto_kdf_hkdf_sha512_expand | ctx == NULL with ctx_len == 0 | [x] |
| blake2-39 | crypto_shorthash_bytes, _keybytes, _primitive, crypto_shorthash_siphash24_bytes/_keybytes, crypto_shorthash_siphashx24_bytes/_keybytes | getters (8/16/"siphash24", 8/16, 16/16) | [x] |
| blake2-40 | crypto_shorthash_siphash24, crypto_shorthash | every inlen 0..=80 (all 8 `left` residues at every 8n boundary) plus {100,127,128,129,255,256,257,1000,4096}, 3 random keys each; crypto_shorthash == crypto_shorthash_siphash24 | [x] |
| blake2-41 | crypto_shorthash_siphash24 | all-zero and all-0xff keys × inlen ∈ {0,7,8,9,64} | [x] |
| blake2-42 | crypto_shorthash_siphashx24 | every inlen 0..=80 plus {100,127,128,129,255,256,257,1000,4096}, 3 random keys each (16-byte output, second finalisation round) | [x] |
| blake2-43 | crypto_shorthash_siphashx24 | all-zero and all-0xff keys × inlen ∈ {0,7,8,9,64} | [x] |
| blake2-44 | crypto_verify_16_bytes, crypto_verify_32_bytes, crypto_verify_64_bytes | getters (16/32/64) | [x] |
| blake2-45 | crypto_verify_16 | 20 random equal pairs, aliased pointers, every one of 16×8 single-bit differences in both argument orders, 50 random pairs, all-zero/all-0xff degenerate pairs | [x] |
| blake2-46 | crypto_verify_32 | 20 random equal pairs, aliased pointers, every one of 32×8 single-bit differences in both argument orders, 50 random pairs, degenerate pairs | [x] |
| blake2-47 | crypto_verify_64 | 20 random equal pairs, aliased pointers, every one of 64×8 single-bit differences in both argument orders, 50 random pairs, degenerate pairs | [x] |
