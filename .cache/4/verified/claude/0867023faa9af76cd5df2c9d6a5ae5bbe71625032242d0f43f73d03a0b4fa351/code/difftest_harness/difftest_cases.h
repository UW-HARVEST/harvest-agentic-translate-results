/* Included in the middle of main() in difftest.c. */

#define CASE(NAME, OUTLEN, ...)                                            \
    do {                                                                   \
        result ra, rb;                                                     \
        int    missing = 0;                                                \
        memset(&ra, 0, sizeof ra);                                         \
        memset(&rb, 0, sizeof rb);                                         \
        if (verbose) fprintf(stderr, "RUN  %s\n", NAME);                    \
        for (int _i = 0; _i < 2; _i++) {                                   \
            void   *h = _i ? hR : hC;                                      \
            result *R = _i ? &rb : &ra;                                    \
            (void) h; (void) R;                                            \
            det_reset();                                                   \
            __VA_ARGS__;                                                   \
        }                                                                  \
        if (missing) { n_skip++; fprintf(stderr, "SKIP %s\n", NAME); }      \
        else report(NAME, &ra, &rb, OUTLEN);                               \
    } while (0)

#define GETF(type, var, symname)                                           \
    type var = (type) dlsym(h, symname);                                   \
    if (!var) { missing = 1; break; }

typedef unsigned char       uc;
typedef unsigned long long  ull;

/* generated function-pointer typedefs */
typedef unsigned long long (*fp0)(void);
typedef unsigned char (*fpuc_void)(void);
typedef int (*fp_fromstr)(uc *, const uc *, size_t, const uc *, size_t, int);
typedef int (*fp_h2c)(uc *, size_t, const uc *, size_t, const uc *, size_t, int);
typedef void (*fpv_ip)(uc *, const uc *, const uc *);
typedef void (*fpv_ipt)(uc *, const uc *, const uc *, const uc *);
typedef void (*fpv_2p)(uc *, const uc *);
typedef const char * (*fp1)(void);
typedef int (*fp2)(const uc *, const uc *);
typedef char * (*fp3)(char *, size_t, const uc *, size_t);
typedef int (*fp4)(uc *, size_t, const char *, size_t, const char *, size_t *, const char **);
typedef char * (*fp5)(char *, size_t, const uc *, size_t, int);
typedef size_t (*fp6)(size_t, int);
typedef int (*fp7)(uc *, size_t, const char *, size_t, const char *, size_t *, const char **, int);
typedef int (*fp8)(uc *, const char *, size_t);
typedef char * (*fp9)(char *, size_t, const uc *);
typedef int (*fp10)(size_t *, uc *, size_t, size_t, size_t);
typedef int (*fp11)(size_t *, const uc *, size_t, size_t);
typedef int (*fp12)(const uc *, const uc *, size_t);
typedef int (*fp13)(const void *, const void *, size_t);
typedef int (*fp14)(const uc *, size_t);
typedef void (*fp15)(uc *, size_t);
typedef void (*fp16)(uc *, const uc *, size_t);
typedef int (*fp17)(uc *, const uc *, const uc *, const uc *);
typedef int (*fp18)(uc *, ull, const uc *, const uc *);
typedef int (*fp19)(uc *, const uc *, ull, const uc *, const uc *);
typedef int (*fp20)(uc *, const uc *, ull, const uc *, ull, const uc *);
typedef int (*fp21)(uc *, const uc *, ull, const uc *, uint32_t, const uc *);
typedef void (*fp22)(uc *);
typedef int (*fp23)(uc *, const uc *, ull);
typedef size_t (*fp24)(void);
typedef int (*fp25)(void *);
typedef int (*fp26)(void *, const uc *, ull);
typedef int (*fp27)(void *, uc *);
typedef int (*fp28)(uc *, size_t, const uc *, size_t);
typedef int (*fp29)(void *, const uc *, size_t);
typedef int (*fp30)(void *, uc *, size_t);
typedef int (*fp31)(void *, uc);
typedef int (*fp32)(uc *, size_t, const uc *, ull, const uc *, size_t);
typedef int (*fp33)(uc *, size_t, const uc *, ull, const uc *, size_t, const uc *, const uc *);
typedef int (*fp34)(void *, const uc *, size_t, size_t);
typedef int (*fp35)(void *, const uc *, size_t, size_t, const uc *, const uc *);
typedef int (*fp36)(uc *, const uc *, ull, const uc *);
typedef int (*fp37)(const uc *, const uc *, ull, const uc *);
typedef int (*fp38)(void *, const uc *);
typedef int (*fp39)(uc *, ull *, const uc *, ull, const uc *, ull, const uc *, const uc *, const uc *);
typedef int (*fp40)(uc *, ull *, uc *, const uc *, ull, const uc *, ull, const uc *, const uc *);
typedef int (*fp41)(uc *, uc *, ull *, const uc *, ull, const uc *, ull, const uc *, const uc *, const uc *);
typedef int (*fp42)(void);
typedef int (*fp43)(uc *, uc *, const uc *, ull, const uc *, const uc *);
typedef int (*fp44)(void *, uc *, const uc *);
typedef int (*fp45)(void *, uc *, ull *, const uc *, ull, const uc *, ull, uc);
typedef int (*fp46)(void *, const uc *, const uc *);
typedef int (*fp47)(void *, uc *, ull *, uc *, const uc *, ull, const uc *, ull);
typedef void (*fp48)(void *);
typedef int (*fp49)(uc *, const uc *);
typedef int (*fp50)(uc *, const uc *, const uc *);
typedef int (*fp51)(const uc *);
typedef void (*fp52)(uc *, const uc *);
typedef void (*fp53)(uc *, const uc *, const uc *);
typedef int (*fp54)(uc *, const char *, const uc *, size_t, const uc *, size_t);
typedef int (*fp55)(uc *, uc *, const uc *);
typedef int (*fp56)(uc *, uc *);
typedef int (*fp57)(uc *, const uc *, ull, const uc *, const uc *, const uc *);
typedef int (*fp58)(uc *, uc *, const uc *, ull, const uc *, const uc *, const uc *);
typedef int (*fp59)(uc *, ull *, const uc *, ull, const uc *);
typedef int (*fp60)(void *, uc *, ull *, const uc *);
typedef int (*fp61)(uc *, size_t, uint64_t, const char *, const uc *);
typedef int (*fp62)(uc *, const uc *, size_t, const uc *, size_t);
typedef int (*fp63)(uc *, size_t, const char *, size_t, const uc *);
typedef int (*fp64)(uc *, uc *, const uc *, const uc *);
typedef int (*fp65)(uc *, uc *, const uc *, const uc *, const uc *);
typedef int (*fp66)(uc *, ull, const char *, ull, const uc *, ull, size_t, int);
typedef int (*fp67)(char *, const char *, ull, ull, size_t);
typedef int (*fp68)(const char *, const char *, ull);
typedef int (*fp69)(const char *, ull, size_t);
typedef int (*fp70)(char *, const char *, ull, ull, size_t, int);
typedef int (*fp71)(uc *, ull, const char *, ull, const uc *, ull, size_t);
typedef int (*fp72)(const uc *, size_t, const uc *, size_t, uint64_t, uint32_t, uint32_t, uc *, size_t);
typedef void (*fp73)(void *, size_t, const uc *);
typedef uint32_t (*fp74)(uint32_t);
typedef void (*fp75)(uc *, const uc *, const uc *, const uc *);
typedef void (*fp76)(int32_t *, const uc *);
typedef void (*fp77)(uc *, const int32_t *);
typedef void (*fp78)(int32_t *, const int32_t *);
typedef void (*fp79)(void *, const uc *);
typedef void (*fp80)(uc *, const void *);
typedef int (*fp81)(uc *, size_t, const char *, const uc *, size_t, int);
typedef int (*fp82)(void *, size_t, const void *, size_t);
typedef void (*fp83)(void *, const uc *, size_t, size_t);
typedef void (*fp84)(void *, uc *, size_t, size_t);

/* =================== constants / accessors =================== */
{
    static const char *const acc[] = {
        "crypto_verify_16_bytes","crypto_verify_32_bytes","crypto_verify_64_bytes",
        "crypto_core_salsa20_outputbytes","crypto_core_salsa20_inputbytes",
        "crypto_core_salsa20_keybytes","crypto_core_salsa20_constbytes",
        "crypto_core_salsa2012_outputbytes","crypto_core_salsa2012_inputbytes",
        "crypto_core_salsa2012_keybytes","crypto_core_salsa2012_constbytes",
        "crypto_core_salsa208_outputbytes","crypto_core_salsa208_inputbytes",
        "crypto_core_salsa208_keybytes","crypto_core_salsa208_constbytes",
        "crypto_core_hsalsa20_outputbytes","crypto_core_hsalsa20_inputbytes",
        "crypto_core_hsalsa20_keybytes","crypto_core_hsalsa20_constbytes",
        "crypto_core_hchacha20_outputbytes","crypto_core_hchacha20_inputbytes",
        "crypto_core_hchacha20_keybytes","crypto_core_hchacha20_constbytes",
        "crypto_stream_keybytes","crypto_stream_noncebytes","crypto_stream_messagebytes_max",
        "crypto_stream_salsa20_keybytes","crypto_stream_salsa20_noncebytes",
        "crypto_stream_salsa20_messagebytes_max",
        "crypto_stream_salsa2012_keybytes","crypto_stream_salsa2012_noncebytes",
        "crypto_stream_salsa2012_messagebytes_max",
        "crypto_stream_salsa208_keybytes","crypto_stream_salsa208_noncebytes",
        "crypto_stream_salsa208_messagebytes_max",
        "crypto_stream_xsalsa20_keybytes","crypto_stream_xsalsa20_noncebytes",
        "crypto_stream_xsalsa20_messagebytes_max",
        "crypto_stream_chacha20_keybytes","crypto_stream_chacha20_noncebytes",
        "crypto_stream_chacha20_messagebytes_max",
        "crypto_stream_chacha20_ietf_keybytes","crypto_stream_chacha20_ietf_noncebytes",
        "crypto_stream_chacha20_ietf_messagebytes_max",
        "crypto_stream_xchacha20_keybytes","crypto_stream_xchacha20_noncebytes",
        "crypto_stream_xchacha20_messagebytes_max",
        "crypto_hash_bytes","crypto_hash_sha256_bytes","crypto_hash_sha256_statebytes",
        "crypto_hash_sha512_bytes","crypto_hash_sha512_statebytes",
        "crypto_hash_sha3256_bytes","crypto_hash_sha3256_statebytes",
        "crypto_hash_sha3512_bytes","crypto_hash_sha3512_statebytes",
        "crypto_core_keccak1600_statebytes",
        "crypto_xof_shake128_blockbytes","crypto_xof_shake128_statebytes",
                "crypto_xof_shake256_blockbytes","crypto_xof_shake256_statebytes",
                "crypto_xof_turboshake128_blockbytes","crypto_xof_turboshake128_statebytes",
                "crypto_xof_turboshake256_blockbytes","crypto_xof_turboshake256_statebytes",
                "crypto_generichash_bytes","crypto_generichash_bytes_min","crypto_generichash_bytes_max",
        "crypto_generichash_keybytes","crypto_generichash_keybytes_min",
        "crypto_generichash_keybytes_max","crypto_generichash_statebytes",
        "crypto_generichash_blake2b_bytes","crypto_generichash_blake2b_bytes_min",
        "crypto_generichash_blake2b_bytes_max","crypto_generichash_blake2b_keybytes",
        "crypto_generichash_blake2b_keybytes_min","crypto_generichash_blake2b_keybytes_max",
        "crypto_generichash_blake2b_saltbytes","crypto_generichash_blake2b_personalbytes",
        "crypto_generichash_blake2b_statebytes",
        "crypto_onetimeauth_bytes","crypto_onetimeauth_keybytes","crypto_onetimeauth_statebytes",
        "crypto_onetimeauth_poly1305_bytes","crypto_onetimeauth_poly1305_keybytes",
        "crypto_onetimeauth_poly1305_statebytes",
        "crypto_shorthash_bytes","crypto_shorthash_keybytes",
        "crypto_shorthash_siphash24_bytes","crypto_shorthash_siphash24_keybytes",
        "crypto_shorthash_siphashx24_bytes","crypto_shorthash_siphashx24_keybytes",
        "crypto_auth_bytes","crypto_auth_keybytes",
        "crypto_auth_hmacsha256_bytes","crypto_auth_hmacsha256_keybytes",
        "crypto_auth_hmacsha256_statebytes",
        "crypto_auth_hmacsha512_bytes","crypto_auth_hmacsha512_keybytes",
        "crypto_auth_hmacsha512_statebytes",
        "crypto_auth_hmacsha512256_bytes","crypto_auth_hmacsha512256_keybytes",
        "crypto_auth_hmacsha512256_statebytes",
        "crypto_aead_chacha20poly1305_keybytes","crypto_aead_chacha20poly1305_nsecbytes",
        "crypto_aead_chacha20poly1305_npubbytes","crypto_aead_chacha20poly1305_abytes",
        "crypto_aead_chacha20poly1305_messagebytes_max",
        "crypto_aead_chacha20poly1305_ietf_keybytes",
        "crypto_aead_chacha20poly1305_ietf_nsecbytes",
        "crypto_aead_chacha20poly1305_ietf_npubbytes",
        "crypto_aead_chacha20poly1305_ietf_abytes",
        "crypto_aead_chacha20poly1305_ietf_messagebytes_max",
        "crypto_aead_xchacha20poly1305_ietf_keybytes",
        "crypto_aead_xchacha20poly1305_ietf_nsecbytes",
        "crypto_aead_xchacha20poly1305_ietf_npubbytes",
        "crypto_aead_xchacha20poly1305_ietf_abytes",
        "crypto_aead_xchacha20poly1305_ietf_messagebytes_max",
        "crypto_aead_aegis128l_keybytes","crypto_aead_aegis128l_nsecbytes",
        "crypto_aead_aegis128l_npubbytes","crypto_aead_aegis128l_abytes",
        "crypto_aead_aegis128l_messagebytes_max",
        "crypto_aead_aegis256_keybytes","crypto_aead_aegis256_nsecbytes",
        "crypto_aead_aegis256_npubbytes","crypto_aead_aegis256_abytes",
        "crypto_aead_aegis256_messagebytes_max",
        "crypto_aead_aes256gcm_keybytes","crypto_aead_aes256gcm_nsecbytes",
        "crypto_aead_aes256gcm_npubbytes","crypto_aead_aes256gcm_abytes",
        "crypto_aead_aes256gcm_messagebytes_max","crypto_aead_aes256gcm_statebytes",
        "crypto_secretbox_keybytes","crypto_secretbox_noncebytes","crypto_secretbox_macbytes",
        "crypto_secretbox_zerobytes","crypto_secretbox_boxzerobytes",
        "crypto_secretbox_messagebytes_max",
        "crypto_secretbox_xsalsa20poly1305_keybytes",
        "crypto_secretbox_xsalsa20poly1305_noncebytes",
        "crypto_secretbox_xsalsa20poly1305_macbytes",
        "crypto_secretbox_xsalsa20poly1305_zerobytes",
        "crypto_secretbox_xsalsa20poly1305_boxzerobytes",
        "crypto_secretbox_xsalsa20poly1305_messagebytes_max",
        "crypto_secretbox_xchacha20poly1305_keybytes",
        "crypto_secretbox_xchacha20poly1305_noncebytes",
        "crypto_secretbox_xchacha20poly1305_macbytes",
        "crypto_secretbox_xchacha20poly1305_messagebytes_max",
        "crypto_secretstream_xchacha20poly1305_statebytes",
        "crypto_secretstream_xchacha20poly1305_abytes",
        "crypto_secretstream_xchacha20poly1305_headerbytes",
        "crypto_secretstream_xchacha20poly1305_keybytes",
        "crypto_secretstream_xchacha20poly1305_messagebytes_max",
        "crypto_box_seedbytes","crypto_box_publickeybytes","crypto_box_secretkeybytes",
        "crypto_box_noncebytes","crypto_box_macbytes","crypto_box_messagebytes_max",
        "crypto_box_beforenmbytes","crypto_box_sealbytes","crypto_box_zerobytes",
        "crypto_box_boxzerobytes",
        "crypto_box_curve25519xsalsa20poly1305_seedbytes",
        "crypto_box_curve25519xsalsa20poly1305_publickeybytes",
        "crypto_box_curve25519xsalsa20poly1305_secretkeybytes",
        "crypto_box_curve25519xsalsa20poly1305_beforenmbytes",
        "crypto_box_curve25519xsalsa20poly1305_noncebytes",
        "crypto_box_curve25519xsalsa20poly1305_zerobytes",
        "crypto_box_curve25519xsalsa20poly1305_boxzerobytes",
        "crypto_box_curve25519xsalsa20poly1305_macbytes",
        "crypto_box_curve25519xsalsa20poly1305_messagebytes_max",
        "crypto_box_curve25519xchacha20poly1305_seedbytes",
        "crypto_box_curve25519xchacha20poly1305_publickeybytes",
        "crypto_box_curve25519xchacha20poly1305_secretkeybytes",
        "crypto_box_curve25519xchacha20poly1305_beforenmbytes",
        "crypto_box_curve25519xchacha20poly1305_noncebytes",
        "crypto_box_curve25519xchacha20poly1305_macbytes",
        "crypto_box_curve25519xchacha20poly1305_messagebytes_max",
        "crypto_box_curve25519xchacha20poly1305_sealbytes",
        "crypto_sign_bytes","crypto_sign_seedbytes","crypto_sign_publickeybytes",
        "crypto_sign_secretkeybytes","crypto_sign_messagebytes_max","crypto_sign_statebytes",
        "crypto_sign_ed25519_bytes","crypto_sign_ed25519_seedbytes",
        "crypto_sign_ed25519_publickeybytes","crypto_sign_ed25519_secretkeybytes",
        "crypto_sign_ed25519_messagebytes_max","crypto_sign_ed25519ph_statebytes",
        "crypto_scalarmult_bytes","crypto_scalarmult_scalarbytes",
        "crypto_scalarmult_curve25519_bytes","crypto_scalarmult_curve25519_scalarbytes",
        "crypto_scalarmult_ed25519_bytes","crypto_scalarmult_ed25519_scalarbytes",
        "crypto_scalarmult_ristretto255_bytes","crypto_scalarmult_ristretto255_scalarbytes",
        "crypto_core_ed25519_bytes","crypto_core_ed25519_uniformbytes",
        "crypto_core_ed25519_hashbytes","crypto_core_ed25519_scalarbytes",
        "crypto_core_ed25519_nonreducedscalarbytes",
        "crypto_core_ristretto255_bytes","crypto_core_ristretto255_hashbytes",
        "crypto_core_ristretto255_scalarbytes",
        "crypto_core_ristretto255_nonreducedscalarbytes",
        "crypto_kdf_bytes_min","crypto_kdf_bytes_max","crypto_kdf_contextbytes",
        "crypto_kdf_keybytes",
        "crypto_kdf_blake2b_bytes_min","crypto_kdf_blake2b_bytes_max",
        "crypto_kdf_blake2b_contextbytes","crypto_kdf_blake2b_keybytes",
        "crypto_kdf_hkdf_sha256_bytes_min","crypto_kdf_hkdf_sha256_bytes_max",
        "crypto_kdf_hkdf_sha256_keybytes","crypto_kdf_hkdf_sha256_statebytes",
        "crypto_kdf_hkdf_sha512_bytes_min","crypto_kdf_hkdf_sha512_bytes_max",
        "crypto_kdf_hkdf_sha512_keybytes","crypto_kdf_hkdf_sha512_statebytes",
        "crypto_kem_seedbytes","crypto_kem_publickeybytes","crypto_kem_secretkeybytes",
        "crypto_kem_ciphertextbytes","crypto_kem_sharedsecretbytes",
        "crypto_kem_mlkem768_seedbytes","crypto_kem_mlkem768_publickeybytes",
        "crypto_kem_mlkem768_secretkeybytes","crypto_kem_mlkem768_ciphertextbytes",
        "crypto_kem_mlkem768_sharedsecretbytes",
        "crypto_kem_xwing_seedbytes","crypto_kem_xwing_publickeybytes",
        "crypto_kem_xwing_secretkeybytes","crypto_kem_xwing_ciphertextbytes",
        "crypto_kem_xwing_sharedsecretbytes",
        "crypto_kx_publickeybytes","crypto_kx_secretkeybytes","crypto_kx_seedbytes",
        "crypto_kx_sessionkeybytes",
        "crypto_ipcrypt_keybytes","crypto_ipcrypt_bytes",
        "crypto_ipcrypt_nd_keybytes","crypto_ipcrypt_nd_tweakbytes",
        "crypto_ipcrypt_nd_inputbytes","crypto_ipcrypt_nd_outputbytes",
        "crypto_ipcrypt_ndx_keybytes","crypto_ipcrypt_ndx_tweakbytes",
        "crypto_ipcrypt_ndx_inputbytes","crypto_ipcrypt_ndx_outputbytes",
        "crypto_ipcrypt_pfx_keybytes","crypto_ipcrypt_pfx_bytes",
        "crypto_pwhash_alg_argon2i13","crypto_pwhash_alg_argon2id13",
        "crypto_pwhash_alg_default","crypto_pwhash_bytes_min","crypto_pwhash_bytes_max",
        "crypto_pwhash_passwd_min","crypto_pwhash_passwd_max","crypto_pwhash_saltbytes",
        "crypto_pwhash_strbytes","crypto_pwhash_opslimit_min","crypto_pwhash_opslimit_max",
        "crypto_pwhash_memlimit_min","crypto_pwhash_memlimit_max",
        "crypto_pwhash_opslimit_interactive","crypto_pwhash_memlimit_interactive",
        "crypto_pwhash_opslimit_moderate","crypto_pwhash_memlimit_moderate",
        "crypto_pwhash_opslimit_sensitive","crypto_pwhash_memlimit_sensitive",
        "crypto_pwhash_argon2i_alg_argon2i13","crypto_pwhash_argon2i_bytes_min",
        "crypto_pwhash_argon2i_bytes_max","crypto_pwhash_argon2i_passwd_min",
        "crypto_pwhash_argon2i_passwd_max","crypto_pwhash_argon2i_saltbytes",
        "crypto_pwhash_argon2i_strbytes","crypto_pwhash_argon2i_opslimit_min",
        "crypto_pwhash_argon2i_opslimit_max","crypto_pwhash_argon2i_memlimit_min",
        "crypto_pwhash_argon2i_memlimit_max","crypto_pwhash_argon2i_opslimit_interactive",
        "crypto_pwhash_argon2i_memlimit_interactive",
        "crypto_pwhash_argon2i_opslimit_moderate","crypto_pwhash_argon2i_memlimit_moderate",
        "crypto_pwhash_argon2i_opslimit_sensitive","crypto_pwhash_argon2i_memlimit_sensitive",
        "crypto_pwhash_argon2id_alg_argon2id13","crypto_pwhash_argon2id_bytes_min",
        "crypto_pwhash_argon2id_bytes_max","crypto_pwhash_argon2id_passwd_min",
        "crypto_pwhash_argon2id_passwd_max","crypto_pwhash_argon2id_saltbytes",
        "crypto_pwhash_argon2id_strbytes","crypto_pwhash_argon2id_opslimit_min",
        "crypto_pwhash_argon2id_opslimit_max","crypto_pwhash_argon2id_memlimit_min",
        "crypto_pwhash_argon2id_memlimit_max",
        "crypto_pwhash_argon2id_opslimit_interactive",
        "crypto_pwhash_argon2id_memlimit_interactive",
        "crypto_pwhash_argon2id_opslimit_moderate",
        "crypto_pwhash_argon2id_memlimit_moderate",
        "crypto_pwhash_argon2id_opslimit_sensitive",
        "crypto_pwhash_argon2id_memlimit_sensitive",
        "crypto_pwhash_scryptsalsa208sha256_bytes_min",
        "crypto_pwhash_scryptsalsa208sha256_bytes_max",
        "crypto_pwhash_scryptsalsa208sha256_passwd_min",
        "crypto_pwhash_scryptsalsa208sha256_passwd_max",
        "crypto_pwhash_scryptsalsa208sha256_saltbytes",
        "crypto_pwhash_scryptsalsa208sha256_strbytes",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_min",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_max",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_min",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_max",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_interactive",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_interactive",
        "crypto_pwhash_scryptsalsa208sha256_opslimit_sensitive",
        "crypto_pwhash_scryptsalsa208sha256_memlimit_sensitive",
        "randombytes_seedbytes",
        "sodium_library_version_major","sodium_library_version_minor",
        "sodium_library_minimal",
        "sodium_runtime_has_neon","sodium_runtime_has_armcrypto","sodium_runtime_has_sse2",
        "sodium_runtime_has_sse3","sodium_runtime_has_ssse3","sodium_runtime_has_sse41",
        "sodium_runtime_has_avx","sodium_runtime_has_avx2","sodium_runtime_has_avx512f",
        "sodium_runtime_has_pclmul","sodium_runtime_has_aesni","sodium_runtime_has_rdrand",
        "crypto_aead_aes256gcm_is_available",
        NULL
    };
    static const char *const acc8[] = {
        "crypto_xof_shake128_domain_standard","crypto_xof_shake256_domain_standard",
        "crypto_xof_turboshake128_domain_standard","crypto_xof_turboshake256_domain_standard",
        NULL
    };
    for (int k = 0; acc8[k]; k++) {
        CASE(acc8[k], 0, {
            GETF(fpuc_void, f, acc8[k]);
            R->ret = (long long) f();
        });
    }
    for (int k = 0; acc[k]; k++) {
        CASE(acc[k], 0, {
            GETF(fp0, f, acc[k]);
            R->ret = (long long) f();
        });
    }
    /* string accessors */
    static const char *const strs[] = {
        "sodium_version_string","crypto_stream_primitive","crypto_hash_primitive",
        "crypto_generichash_primitive","crypto_onetimeauth_primitive",
        "crypto_shorthash_primitive","crypto_auth_primitive","crypto_secretbox_primitive",
        "crypto_sign_primitive","crypto_scalarmult_primitive","crypto_kdf_primitive",
        "crypto_kem_primitive","crypto_kx_primitive","crypto_pwhash_primitive",
        "crypto_pwhash_strprefix","crypto_pwhash_argon2i_strprefix",
        "crypto_pwhash_argon2id_strprefix",
        "crypto_pwhash_scryptsalsa208sha256_strprefix",
        NULL
    };
    for (int k = 0; strs[k]; k++) {
        CASE(strs[k], 64, {
            GETF(fp1, f, strs[k]);
            const char *s = f();
            if (s) { size_t n = strlen(s); if (n > 63) n = 63; memcpy(R->out, s, n + 1); }
            R->ret = s ? 1 : 0;
        });
    }
}

/* =================== crypto_verify =================== */
{
    uc a[64], b[64];
    fillr(a, 64); memcpy(b, a, 64);
    CASE("crypto_verify_16 eq", 0, { GETF(fp2, f, "crypto_verify_16"); R->ret = f(a, b); });
    CASE("crypto_verify_32 eq", 0, { GETF(fp2, f, "crypto_verify_32"); R->ret = f(a, b); });
    CASE("crypto_verify_64 eq", 0, { GETF(fp2, f, "crypto_verify_64"); R->ret = f(a, b); });
    b[7] ^= 0x40;
    CASE("crypto_verify_16 ne", 0, { GETF(fp2, f, "crypto_verify_16"); R->ret = f(a, b); });
    CASE("crypto_verify_32 ne", 0, { GETF(fp2, f, "crypto_verify_32"); R->ret = f(a, b); });
    CASE("crypto_verify_64 ne", 0, { GETF(fp2, f, "crypto_verify_64"); R->ret = f(a, b); });
}

/* =================== sodium utils =================== */
{
    uc bin[37];
    fillr(bin, sizeof bin);
    CASE("sodium_bin2hex", 80, {
        GETF(fp3, f, "sodium_bin2hex");
        f((char *) R->out, 80, bin, sizeof bin);
        R->ret = 0;
    });
    CASE("sodium_hex2bin", 40, {
        GETF(fp4, f, "sodium_hex2bin");
        size_t bl = 0; const char *he = NULL;
        R->ret = f(R->out, 40, "0a1B2c:3d4e", 11, ":", &bl, &he);
        R->extra = bl * 1000 + (unsigned) (he - (const char *) "0a1B2c:3d4e");
    });
    CASE("sodium_hex2bin strict", 40, {
        GETF(fp4, f, "sodium_hex2bin");
        size_t bl = 0;
        R->ret = f(R->out, 40, "0a1b2", 5, NULL, &bl, NULL);
        R->extra = bl;
    });
    for (int variant = 1; variant <= 7; variant += 2) {
        char nm[64];
        snprintf(nm, sizeof nm, "sodium_bin2base64 v%d", variant);
        CASE(nm, 128, {
            GETF(fp5, f, "sodium_bin2base64");
            GETF(fp6, g, "sodium_base64_encoded_len");
            size_t need = g(sizeof bin, variant);
            f((char *) R->out, need, bin, sizeof bin, variant);
            R->extra = need;
            R->ret = 0;
        });
        snprintf(nm, sizeof nm, "sodium_base642bin v%d", variant);
        CASE(nm, 64, {
            GETF(fp5, e, "sodium_bin2base64");
            GETF(fp6, g, "sodium_base64_encoded_len");
            GETF(fp7, f, "sodium_base642bin");
            char b64[256]; size_t need = g(sizeof bin, variant);
            e(b64, need, bin, sizeof bin, variant);
            size_t bl = 0;
            R->ret = f(R->out, 64, b64, strlen(b64), NULL, &bl, NULL, variant);
            R->extra = bl;
        });
    }
    static const char *const ips[] = {
        "127.0.0.1","255.254.253.252","::1","::","2001:db8::1",
        "2001:0db8:85a3:0000:0000:8a2e:0370:7334","::ffff:1.2.3.4",
        "fe80::1%eth0","1.2.3","1.2.3.4.5","::g","", "0:0:0:0:0:0:0:0",
        "1:2:3:4:5:6:7:8","1::8","1:2:3:4:5:6:1.2.3.4", NULL
    };
    for (int k = 0; ips[k]; k++) {
        char nm[96];
        snprintf(nm, sizeof nm, "sodium_ip2bin %s", ips[k]);
        CASE(nm, 16, {
            GETF(fp8, f, "sodium_ip2bin");
            R->ret = f(R->out, ips[k], strlen(ips[k]));
        });
    }
    for (int k = 0; k < 24; k++) {
        uc  b16[16];
        char nm[64];
        if (k < 8) { memset(b16, 0, 16); if (k) b16[k] = 1; }
        else if (k == 8) { memset(b16, 0, 10); b16[10] = 0xff; b16[11] = 0xff; b16[12] = 192; b16[13] = 168; b16[14] = 1; b16[15] = 1; }
        else fillr(b16, 16);
        snprintf(nm, sizeof nm, "sodium_bin2ip %d", k);
        CASE(nm, 48, {
            GETF(fp9, f, "sodium_bin2ip");
            char *p = f((char *) R->out, 48, b16);
            R->ret = p ? 1 : 0;
        });
    }
    for (int bs = 1; bs <= 17; bs += 4) {
        for (int ul = 0; ul <= 20; ul += 7) {
            char nm[64];
            snprintf(nm, sizeof nm, "sodium_pad bs%d ul%d", bs, ul);
            CASE(nm, 64, {
                GETF(fp10, f, "sodium_pad");
                size_t pl = 0;
                memset(R->out, 0xAA, 64);
                memcpy(R->out, bin, (size_t) ul < sizeof bin ? (size_t) ul : sizeof bin);
                R->ret = f(&pl, R->out, ul, bs, 64);
                R->extra = pl;
            });
            snprintf(nm, sizeof nm, "sodium_unpad bs%d ul%d", bs, ul);
            CASE(nm, 64, {
                GETF(fp10, p, "sodium_pad");
                GETF(fp11, f, "sodium_unpad");
                size_t pl = 0;
                memset(R->out, 0xAA, 64);
                memcpy(R->out, bin, (size_t) ul < sizeof bin ? (size_t) ul : sizeof bin);
                if (p(&pl, R->out, ul, bs, 64) != 0) { R->ret = -99; break; }
                size_t upl = 0;
                R->ret  = f(&upl, R->out, pl, bs);
                R->extra = upl;
            });
        }
    }
    {
        uc x[32], y[32];
        fillr(x, 32); memcpy(y, x, 32);
        CASE("sodium_compare eq", 0, { GETF(fp12, f, "sodium_compare"); R->ret = f(x, y, 32); });
        y[0] = (uc) (x[0] + 1);
        CASE("sodium_compare lt", 0, { GETF(fp12, f, "sodium_compare"); R->ret = f(x, y, 32); });
        CASE("sodium_compare gt", 0, { GETF(fp12, f, "sodium_compare"); R->ret = f(y, x, 32); });
        CASE("sodium_memcmp", 0, { GETF(fp13, f, "sodium_memcmp"); R->ret = f(x, y, 32); });
        CASE("sodium_is_zero nz", 0, { GETF(fp14, f, "sodium_is_zero"); R->ret = f(x, 32); });
        uc z[32] = { 0 };
        CASE("sodium_is_zero z", 0, { GETF(fp14, f, "sodium_is_zero"); R->ret = f(z, 32); });
    }
    for (int nlen = 1; nlen <= 32; nlen += 7) {
        char nm[64];
        snprintf(nm, sizeof nm, "sodium_increment %d", nlen);
        CASE(nm, 32, {
            GETF(fp15, f, "sodium_increment");
            memset(R->out, 0xff, 32);
            memcpy(R->out, bin, (size_t) nlen);
            f(R->out, nlen);
        });
        snprintf(nm, sizeof nm, "sodium_add %d", nlen);
        CASE(nm, 32, {
            GETF(fp16, f, "sodium_add");
            memset(R->out, 0, 32);
            memcpy(R->out, bin, (size_t) nlen);
            f(R->out, bin + 1, nlen);
        });
        snprintf(nm, sizeof nm, "sodium_sub %d", nlen);
        CASE(nm, 32, {
            GETF(fp16, f, "sodium_sub");
            memset(R->out, 0, 32);
            memcpy(R->out, bin, (size_t) nlen);
            f(R->out, bin + 1, nlen);
        });
    }
    CASE("sodium_increment carry", 32, {
        GETF(fp15, f, "sodium_increment");
        memset(R->out, 0xff, 32);
        f(R->out, 24);
    });
}

/* =================== crypto_core =================== */
{
    uc in[16], k[32], c[16];
    fillr(in, 16); fillr(k, 32); fillr(c, 16);
    static const char *const cores[] = { "crypto_core_salsa20","crypto_core_salsa2012","crypto_core_salsa208", NULL };
    for (int j = 0; cores[j]; j++) {
        char nm[64];
        snprintf(nm, sizeof nm, "%s", cores[j]);
        CASE(nm, 64, { GETF(fp17, f, cores[j]); R->ret = f(R->out, in, k, c); });
        snprintf(nm, sizeof nm, "%s null-c", cores[j]);
        CASE(nm, 64, { GETF(fp17, f, cores[j]); R->ret = f(R->out, in, k, NULL); });
    }
    CASE("crypto_core_hsalsa20", 32, { GETF(fp17, f, "crypto_core_hsalsa20"); R->ret = f(R->out, in, k, c); });
    CASE("crypto_core_hsalsa20 null-c", 32, { GETF(fp17, f, "crypto_core_hsalsa20"); R->ret = f(R->out, in, k, NULL); });
    CASE("crypto_core_hchacha20", 32, { GETF(fp17, f, "crypto_core_hchacha20"); R->ret = f(R->out, in, k, c); });
    CASE("crypto_core_hchacha20 null-c", 32, { GETF(fp17, f, "crypto_core_hchacha20"); R->ret = f(R->out, in, k, NULL); });
}

/* =================== crypto_stream =================== */
{
    uc key[32], n8[8], n24[24], n12[12], msg[600];
    fillr(key, 32); fillr(n8, 8); fillr(n24, 24); fillr(n12, 12); fillr(msg, sizeof msg);
    struct { const char *nm; int nlen; } s2[] = {
        { "crypto_stream_salsa20", 8 }, { "crypto_stream_salsa2012", 8 },
        { "crypto_stream_salsa208", 8 }, { "crypto_stream_xsalsa20", 24 },
        { "crypto_stream_chacha20", 8 }, { "crypto_stream_chacha20_ietf", 12 },
        { "crypto_stream_xchacha20", 24 }, { "crypto_stream", 24 },
    };
    for (unsigned j = 0; j < sizeof s2 / sizeof s2[0]; j++) {
        const uc *nn = s2[j].nlen == 8 ? n8 : (s2[j].nlen == 12 ? n12 : n24);
        for (int len = 0; len <= 600; len = len ? len * 3 + 1 : 1) {
            char nm[96];
            snprintf(nm, sizeof nm, "%s len%d", s2[j].nm, len);
            CASE(nm, (size_t) len, {
                GETF(fp18, f, s2[j].nm);
                R->ret = f(R->out, (ull) len, nn, key);
            });
            char nm2[96];
            snprintf(nm2, sizeof nm2, "%s_xor len%d", s2[j].nm, len);
            char sym[96];
            snprintf(sym, sizeof sym, "%s_xor", s2[j].nm);
            CASE(nm2, (size_t) len, {
                GETF(fp19, f, sym);
                R->ret = f(R->out, msg, (ull) len, nn, key);
            });
        }
    }
    struct { const char *sym; int nlen; ull ic; } xic[] = {
        { "crypto_stream_salsa20_xor_ic", 8, 3 },
        { "crypto_stream_xsalsa20_xor_ic", 24, 5 },
        { "crypto_stream_chacha20_xor_ic", 8, 7 },
        { "crypto_stream_xchacha20_xor_ic", 24, 9 },
    };
    for (unsigned j = 0; j < sizeof xic / sizeof xic[0]; j++) {
        const uc *nn = xic[j].nlen == 8 ? n8 : n24;
        char nm[96];
        snprintf(nm, sizeof nm, "%s", xic[j].sym);
        CASE(nm, 300, {
            GETF(fp20, f, xic[j].sym);
            R->ret = f(R->out, msg, 300, nn, xic[j].ic, key);
        });
    }
    CASE("crypto_stream_chacha20_ietf_xor_ic", 300, {
        GETF(fp21, f, "crypto_stream_chacha20_ietf_xor_ic");
        R->ret = f(R->out, msg, 300, n12, 11, key);
    });
    CASE("crypto_stream_chacha20_ietf_ext", 300, {
        GETF(fp18, f, "crypto_stream_chacha20_ietf_ext");
        R->ret = f(R->out, 300, n12, key);
    });
    CASE("crypto_stream_chacha20_ietf_ext_xor_ic", 300, {
        GETF(fp21, f, "crypto_stream_chacha20_ietf_ext_xor_ic");
        R->ret = f(R->out, msg, 300, n12, 2, key);
    });
    static const char *const kg[] = {
        "crypto_stream_keygen","crypto_stream_salsa20_keygen","crypto_stream_salsa2012_keygen",
        "crypto_stream_salsa208_keygen","crypto_stream_xsalsa20_keygen",
        "crypto_stream_chacha20_keygen","crypto_stream_chacha20_ietf_keygen",
        "crypto_stream_xchacha20_keygen","crypto_auth_keygen","crypto_auth_hmacsha256_keygen",
        "crypto_auth_hmacsha512_keygen","crypto_auth_hmacsha512256_keygen",
        "crypto_generichash_keygen","crypto_generichash_blake2b_keygen",
        "crypto_onetimeauth_keygen","crypto_onetimeauth_poly1305_keygen",
        "crypto_shorthash_keygen","crypto_secretbox_keygen",
        "crypto_secretbox_xsalsa20poly1305_keygen",
        "crypto_secretstream_xchacha20poly1305_keygen",
        "crypto_aead_chacha20poly1305_keygen","crypto_aead_chacha20poly1305_ietf_keygen",
        "crypto_aead_xchacha20poly1305_ietf_keygen","crypto_aead_aegis128l_keygen",
        "crypto_aead_aegis256_keygen","crypto_aead_aes256gcm_keygen",
        "crypto_kdf_keygen","crypto_ipcrypt_keygen","crypto_ipcrypt_nd_keygen",
        "crypto_ipcrypt_ndx_keygen","crypto_ipcrypt_pfx_keygen",
        "crypto_kdf_hkdf_sha256_keygen","crypto_kdf_hkdf_sha512_keygen",
        NULL
    };
    for (int j = 0; kg[j]; j++) {
        CASE(kg[j], 64, {
            GETF(fp22, f, kg[j]);
            memset(R->out, 0, 64);
            f(R->out);
        });
    }
}

/* =================== hashes =================== */
{
    uc msg[600];
    fillr(msg, sizeof msg);
    struct { const char *nm; size_t out; } hs[] = {
        { "crypto_hash", 64 }, { "crypto_hash_sha256", 32 }, { "crypto_hash_sha512", 64 },
        { "crypto_hash_sha3256", 32 }, { "crypto_hash_sha3512", 64 },
    };
    for (unsigned j = 0; j < sizeof hs / sizeof hs[0]; j++) {
        for (int len = 0; len <= 600; len = len ? len * 5 + 3 : 1) {
            char nm[96];
            snprintf(nm, sizeof nm, "%s len%d", hs[j].nm, len);
            CASE(nm, hs[j].out, {
                GETF(fp23, f, hs[j].nm);
                R->ret = f(R->out, msg, (ull) len);
            });
        }
    }
    /* streaming */
    struct { const char *pfx; size_t out; } st[] = {
        { "crypto_hash_sha256", 32 }, { "crypto_hash_sha512", 64 },
        { "crypto_hash_sha3256", 32 }, { "crypto_hash_sha3512", 64 },
    };
    for (unsigned j = 0; j < sizeof st / sizeof st[0]; j++) {
        char nm[96];
        snprintf(nm, sizeof nm, "%s streaming", st[j].pfx);
        CASE(nm, st[j].out, {
            char s1[96], s2[96], s3[96], s4[96];
            snprintf(s1, sizeof s1, "%s_init", st[j].pfx);
            snprintf(s2, sizeof s2, "%s_update", st[j].pfx);
            snprintf(s3, sizeof s3, "%s_final", st[j].pfx);
            snprintf(s4, sizeof s4, "%s_statebytes", st[j].pfx);
            GETF(fp24, sb, s4);
            GETF(fp25, fi, s1);
            GETF(fp26, fu, s2);
            GETF(fp27, ff, s3);
            void *state = calloc(1, sb() + 64);
            R->ret = fi(state);
            size_t off = 0, chunk = 1;
            while (off < sizeof msg) {
                size_t c = chunk;
                if (off + c > sizeof msg) c = sizeof msg - off;
                R->ret |= fu(state, msg + off, c);
                off += c;
                chunk = chunk * 2 + 1;
            }
            R->ret |= ff(state, R->out);
            R->extra = sb();
            free(state);
        });
    }
}

/* =================== XOFs =================== */
{
    uc msg[300], key[32];
    fillr(msg, sizeof msg); fillr(key, 32);
    static const char *const xf[] = {
        "crypto_xof_shake128","crypto_xof_shake256",
        "crypto_xof_turboshake128","crypto_xof_turboshake256", NULL
    };
    for (int j = 0; xf[j]; j++) {
        char nm[128], sym[128];
        for (int ol = 1; ol <= 400; ol = ol * 7 + 1) {
            snprintf(nm, sizeof nm, "%s out%d", xf[j], ol);
            CASE(nm, (size_t) ol, {
                GETF(fp28, f, xf[j]);
                R->ret = f(R->out, (size_t) ol, msg, sizeof msg);
            });
        }
        snprintf(nm, sizeof nm, "%s streaming", xf[j]);
        CASE(nm, 200, {
            char s1[160], s2[160], s3[160], s4[160];
            snprintf(s1, sizeof s1, "%s_init", xf[j]);
            snprintf(s2, sizeof s2, "%s_update", xf[j]);
            snprintf(s3, sizeof s3, "%s_squeeze", xf[j]);
            snprintf(s4, sizeof s4, "%s_statebytes", xf[j]);
            GETF(fp24, sb, s4);
            GETF(fp25, fi, s1);
            GETF(fp29, fu, s2);
            GETF(fp30, fq, s3);
            void *state = calloc(1, sb() + 64);
            R->ret = fi(state);
            size_t off = 0, chunk = 1;
            while (off < sizeof msg) {
                size_t c = chunk;
                if (off + c > sizeof msg) c = sizeof msg - off;
                R->ret |= fu(state, msg + off, c);
                off += c;
                chunk = chunk * 3 + 1;
            }
            size_t so = 0; chunk = 1;
            while (so < 200) {
                size_t c = chunk;
                if (so + c > 200) c = 200 - so;
                R->ret |= fq(state, R->out + so, c);
                so += c;
                chunk = chunk * 5 + 1;
            }
            R->extra = sb();
            free(state);
        });
        snprintf(nm, sizeof nm, "%s init_with_domain", xf[j]);
        snprintf(sym, sizeof sym, "%s_init_with_domain", xf[j]);
        CASE(nm, 100, {
            char s2[160], s3[160], s4[160];
            snprintf(s2, sizeof s2, "%s_update", xf[j]);
            snprintf(s3, sizeof s3, "%s_squeeze", xf[j]);
            snprintf(s4, sizeof s4, "%s_statebytes", xf[j]);
            GETF(fp24, sb, s4);
            GETF(fp31, fi, sym);
            GETF(fp29, fu, s2);
            GETF(fp30, fq, s3);
            void *state = calloc(1, sb() + 64);
            R->ret = fi(state, 0x0b);
            R->ret |= fu(state, msg, sizeof msg);
            R->ret |= fq(state, R->out, 100);
            free(state);
        });
    }
}

/* =================== generichash =================== */
{
    uc msg[600], key[64], salt[16], pers[16];
    fillr(msg, sizeof msg); fillr(key, 64); fillr(salt, 16); fillr(pers, 16);
    for (int ol = 16; ol <= 64; ol += 16) {
        for (int kl = 0; kl <= 64; kl += 32) {
            char nm[96];
            snprintf(nm, sizeof nm, "crypto_generichash out%d key%d", ol, kl);
            CASE(nm, (size_t) ol, {
                GETF(fp32, f, "crypto_generichash");
                R->ret = f(R->out, (size_t) ol, msg, sizeof msg, kl ? key : NULL, (size_t) kl);
            });
            snprintf(nm, sizeof nm, "crypto_generichash_blake2b out%d key%d", ol, kl);
            CASE(nm, (size_t) ol, {
                GETF(fp32, f, "crypto_generichash_blake2b");
                R->ret = f(R->out, (size_t) ol, msg, sizeof msg, kl ? key : NULL, (size_t) kl);
            });
            snprintf(nm, sizeof nm, "crypto_generichash_blake2b_salt_personal out%d key%d", ol, kl);
            CASE(nm, (size_t) ol, {
                GETF(fp33, f, "crypto_generichash_blake2b_salt_personal");
                R->ret = f(R->out, (size_t) ol, msg, sizeof msg, kl ? key : NULL, (size_t) kl, salt, pers);
            });
        }
    }
    CASE("crypto_generichash streaming", 32, {
        GETF(fp24, sb, "crypto_generichash_statebytes");
        GETF(fp34, fi, "crypto_generichash_init");
        GETF(fp26, fu, "crypto_generichash_update");
        GETF(fp30, ff, "crypto_generichash_final");
        void *st = calloc(1, sb() + 64);
        R->ret = fi(st, key, 32, 32);
        size_t off = 0, chunk = 1;
        while (off < sizeof msg) {
            size_t c = chunk;
            if (off + c > sizeof msg) c = sizeof msg - off;
            R->ret |= fu(st, msg + off, c);
            off += c;
            chunk = chunk * 2 + 1;
        }
        R->ret |= ff(st, R->out, 32);
        R->extra = sb();
        free(st);
    });
    CASE("crypto_generichash_blake2b_init_salt_personal", 32, {
        GETF(fp24, sb, "crypto_generichash_blake2b_statebytes");
        GETF(fp35, fi, "crypto_generichash_blake2b_init_salt_personal");
        GETF(fp26, fu, "crypto_generichash_blake2b_update");
        GETF(fp30, ff, "crypto_generichash_blake2b_final");
        void *st = calloc(1, sb() + 64);
        R->ret = fi(st, key, 32, 32, salt, pers);
        R->ret |= fu(st, msg, sizeof msg);
        R->ret |= ff(st, R->out, 32);
        free(st);
    });
}

/* =================== onetimeauth / shorthash / auth =================== */
{
    uc msg[600], key[64];
    fillr(msg, sizeof msg); fillr(key, 64);
    for (int len = 0; len <= 600; len = len ? len * 5 + 3 : 1) {
        char nm[96];
        snprintf(nm, sizeof nm, "crypto_onetimeauth len%d", len);
        CASE(nm, 16, {
            GETF(fp36, f, "crypto_onetimeauth");
            R->ret = f(R->out, msg, (ull) len, key);
        });
        snprintf(nm, sizeof nm, "crypto_onetimeauth_poly1305 len%d", len);
        CASE(nm, 16, {
            GETF(fp36, f, "crypto_onetimeauth_poly1305");
            R->ret = f(R->out, msg, (ull) len, key);
        });
    }
    CASE("crypto_onetimeauth verify", 16, {
        GETF(fp36, f, "crypto_onetimeauth");
        GETF(fp37, v, "crypto_onetimeauth_verify");
        f(R->out, msg, 137, key);
        R->ret = v(R->out, msg, 137, key);
        uc bad[16]; memcpy(bad, R->out, 16); bad[0] ^= 1;
        R->extra = (unsigned long long) (v(bad, msg, 137, key) + 100);
    });
    CASE("crypto_onetimeauth streaming", 16, {
        GETF(fp24, sb, "crypto_onetimeauth_statebytes");
        GETF(fp38, fi, "crypto_onetimeauth_init");
        GETF(fp26, fu, "crypto_onetimeauth_update");
        GETF(fp27, ff, "crypto_onetimeauth_final");
        void *st = calloc(1, sb() + 64);
        R->ret = fi(st, key);
        size_t off = 0, chunk = 1;
        while (off < sizeof msg) {
            size_t c = chunk;
            if (off + c > sizeof msg) c = sizeof msg - off;
            R->ret |= fu(st, msg + off, c);
            off += c;
            chunk = chunk * 2 + 1;
        }
        R->ret |= ff(st, R->out);
        R->extra = sb();
        free(st);
    });
    static const char *const sh[] = { "crypto_shorthash","crypto_shorthash_siphash24", NULL };
    for (int j = 0; sh[j]; j++) {
        for (int len = 0; len <= 300; len = len ? len * 5 + 3 : 1) {
            char nm[96];
            snprintf(nm, sizeof nm, "%s len%d", sh[j], len);
            CASE(nm, 8, {
                GETF(fp36, f, sh[j]);
                R->ret = f(R->out, msg, (ull) len, key);
            });
        }
    }
    for (int len = 0; len <= 300; len = len ? len * 5 + 3 : 1) {
        char nm[96];
        snprintf(nm, sizeof nm, "crypto_shorthash_siphashx24 len%d", len);
        CASE(nm, 16, {
            GETF(fp36, f, "crypto_shorthash_siphashx24");
            R->ret = f(R->out, msg, (ull) len, key);
        });
    }
    struct { const char *nm; size_t out; size_t klen; } au[] = {
        { "crypto_auth", 32, 32 }, { "crypto_auth_hmacsha256", 32, 32 },
        { "crypto_auth_hmacsha512", 64, 32 }, { "crypto_auth_hmacsha512256", 32, 32 },
    };
    for (unsigned j = 0; j < sizeof au / sizeof au[0]; j++) {
        for (int len = 0; len <= 600; len = len ? len * 7 + 3 : 1) {
            char nm[96];
            snprintf(nm, sizeof nm, "%s len%d", au[j].nm, len);
            CASE(nm, au[j].out, {
                GETF(fp36, f, au[j].nm);
                R->ret = f(R->out, msg, (ull) len, key);
            });
        }
        char nm[96], s1[128], s2[128], s3[128], s4[128], sv[128];
        snprintf(nm, sizeof nm, "%s verify", au[j].nm);
        snprintf(sv, sizeof sv, "%s_verify", au[j].nm);
        CASE(nm, au[j].out, {
            GETF(fp36, f, au[j].nm);
            GETF(fp37, v, sv);
            f(R->out, msg, 137, key);
            R->ret = v(R->out, msg, 137, key);
            uc bad[64]; memcpy(bad, R->out, au[j].out); bad[0] ^= 1;
            R->extra = (unsigned long long) (v(bad, msg, 137, key) + 100);
        });
        if (j == 0) continue;
        snprintf(nm, sizeof nm, "%s streaming", au[j].nm);
        snprintf(s1, sizeof s1, "%s_init", au[j].nm);
        snprintf(s2, sizeof s2, "%s_update", au[j].nm);
        snprintf(s3, sizeof s3, "%s_final", au[j].nm);
        snprintf(s4, sizeof s4, "%s_statebytes", au[j].nm);
        CASE(nm, au[j].out, {
            GETF(fp24, sb, s4);
            GETF(fp29, fi, s1);
            GETF(fp26, fu, s2);
            GETF(fp27, ff, s3);
            void *st = calloc(1, sb() + 64);
            R->ret = fi(st, key, 41);
            size_t off = 0, chunk = 1;
            while (off < sizeof msg) {
                size_t c = chunk;
                if (off + c > sizeof msg) c = sizeof msg - off;
                R->ret |= fu(st, msg + off, c);
                off += c;
                chunk = chunk * 2 + 1;
            }
            R->ret |= ff(st, R->out);
            R->extra = sb();
            free(st);
        });
    }
}

/* =================== AEAD =================== */
{
    uc key[32], npub[32], msg[600], ad[37];
    fillr(key, 32); fillr(npub, 32); fillr(msg, sizeof msg); fillr(ad, sizeof ad);
    struct { const char *pfx; int npub; int abytes; } ae[] = {
        { "crypto_aead_chacha20poly1305", 8, 16 },
        { "crypto_aead_chacha20poly1305_ietf", 12, 16 },
        { "crypto_aead_xchacha20poly1305_ietf", 24, 16 },
        { "crypto_aead_aegis128l", 16, 32 },
        { "crypto_aead_aegis256", 32, 32 },
    };
    for (unsigned j = 0; j < sizeof ae / sizeof ae[0]; j++) {
        int kb = 32;
        if (strstr(ae[j].pfx, "aegis128l")) kb = 16;
        for (int len = 0; len <= 600; len = len ? len * 7 + 5 : 1) {
            char nm[160], sym[160];
            snprintf(nm, sizeof nm, "%s_encrypt len%d", ae[j].pfx, len);
            snprintf(sym, sizeof sym, "%s_encrypt", ae[j].pfx);
            CASE(nm, (size_t) len + ae[j].abytes, {
                GETF(fp39, f, sym);
                ull clen = 0;
                R->ret = f(R->out, &clen, msg, (ull) len, ad, sizeof ad, NULL, npub, key);
                R->extra = clen;
            });
            snprintf(nm, sizeof nm, "%s_decrypt len%d", ae[j].pfx, len);
            CASE(nm, (size_t) len, {
                char se[160], sd[160];
                snprintf(se, sizeof se, "%s_encrypt", ae[j].pfx);
                snprintf(sd, sizeof sd, "%s_decrypt", ae[j].pfx);
                GETF(fp39, e, se);
                GETF(fp40, f, sd);
                uc ct[700]; ull clen = 0;
                e(ct, &clen, msg, (ull) len, ad, sizeof ad, NULL, npub, key);
                ull mlen = 0;
                R->ret = f(R->out, &mlen, NULL, ct, clen, ad, sizeof ad, npub, key);
                R->extra = mlen;
            });
            snprintf(nm, sizeof nm, "%s_decrypt tamper len%d", ae[j].pfx, len);
            CASE(nm, (size_t) len, {
                char se[160], sd[160];
                snprintf(se, sizeof se, "%s_encrypt", ae[j].pfx);
                snprintf(sd, sizeof sd, "%s_decrypt", ae[j].pfx);
                GETF(fp39, e, se);
                GETF(fp40, f, sd);
                uc ct[700]; ull clen = 0;
                e(ct, &clen, msg, (ull) len, ad, sizeof ad, NULL, npub, key);
                ct[0] ^= 0x80;
                ull mlen = 0;
                R->ret = f(R->out, &mlen, NULL, ct, clen, ad, sizeof ad, npub, key);
                R->extra = mlen;
            });
            snprintf(nm, sizeof nm, "%s_encrypt_detached len%d", ae[j].pfx, len);
            snprintf(sym, sizeof sym, "%s_encrypt_detached", ae[j].pfx);
            CASE(nm, (size_t) len + ae[j].abytes, {
                GETF(fp41, f, sym);
                ull maclen = 0;
                R->ret = f(R->out, R->out + len, &maclen, msg, (ull) len, ad, sizeof ad, NULL, npub, key);
                R->extra = maclen;
            });
        }
        (void) kb;
    }
    /* aes256gcm is unavailable in a portable build: check the error paths */
    CASE("crypto_aead_aes256gcm_is_available", 0, {
        GETF(fp42, f, "crypto_aead_aes256gcm_is_available");
        R->ret = f();
    });
}

/* =================== secretbox =================== */
{
    uc key[32], n24[24], msg[600], padded[700];
    fillr(key, 32); fillr(n24, 24); fillr(msg, sizeof msg);
    for (int len = 0; len <= 600; len = len ? len * 7 + 5 : 1) {
        char nm[128];
        snprintf(nm, sizeof nm, "crypto_secretbox_easy len%d", len);
        CASE(nm, (size_t) len + 16, {
            GETF(fp19, f, "crypto_secretbox_easy");
            R->ret = f(R->out, msg, (ull) len, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_open_easy len%d", len);
        CASE(nm, (size_t) len, {
            GETF(fp19, e, "crypto_secretbox_easy");
            GETF(fp19, f, "crypto_secretbox_open_easy");
            uc ct[700];
            e(ct, msg, (ull) len, n24, key);
            R->ret = f(R->out, ct, (ull) len + 16, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_detached len%d", len);
        CASE(nm, (size_t) len + 16, {
            GETF(fp43, f, "crypto_secretbox_detached");
            R->ret = f(R->out, R->out + len, msg, (ull) len, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_xchacha20poly1305_easy len%d", len);
        CASE(nm, (size_t) len + 16, {
            GETF(fp19, f, "crypto_secretbox_xchacha20poly1305_easy");
            R->ret = f(R->out, msg, (ull) len, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_xchacha20poly1305_detached len%d", len);
        CASE(nm, (size_t) len + 16, {
            GETF(fp43, f, "crypto_secretbox_xchacha20poly1305_detached");
            R->ret = f(R->out, R->out + len, msg, (ull) len, n24, key);
        });
        /* NaCl-style API needs 32 zero bytes of padding */
        memset(padded, 0, 32);
        memcpy(padded + 32, msg, (size_t) len);
        snprintf(nm, sizeof nm, "crypto_secretbox len%d", len);
        CASE(nm, (size_t) len + 32, {
            GETF(fp19, f, "crypto_secretbox");
            R->ret = f(R->out, padded, (ull) len + 32, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_xsalsa20poly1305 len%d", len);
        CASE(nm, (size_t) len + 32, {
            GETF(fp19, f, "crypto_secretbox_xsalsa20poly1305");
            R->ret = f(R->out, padded, (ull) len + 32, n24, key);
        });
        snprintf(nm, sizeof nm, "crypto_secretbox_open len%d", len);
        CASE(nm, (size_t) len + 32, {
            GETF(fp19, e, "crypto_secretbox");
            GETF(fp19, f, "crypto_secretbox_open");
            uc ct[740];
            e(ct, padded, (ull) len + 32, n24, key);
            R->ret = f(R->out, ct, (ull) len + 32, n24, key);
        });
    }
}

/* =================== secretstream =================== */
CASE("crypto_secretstream_xchacha20poly1305 roundtrip", 2048, {
    uc key[32], m1[100], m2[200], m3[50];
    GETF(fp24, sb, "crypto_secretstream_xchacha20poly1305_statebytes");
    GETF(fp22, kgen, "crypto_secretstream_xchacha20poly1305_keygen");
    GETF(fp44, ipush, "crypto_secretstream_xchacha20poly1305_init_push");
    GETF(fp45, push, "crypto_secretstream_xchacha20poly1305_push");
    GETF(fp46, ipull, "crypto_secretstream_xchacha20poly1305_init_pull");
    GETF(fp47, pull, "crypto_secretstream_xchacha20poly1305_pull");
    GETF(fp48, rekey, "crypto_secretstream_xchacha20poly1305_rekey");
    memset(key, 0x42, 32);
    kgen(key);
    void *st = calloc(1, sb() + 64);
    uc header[64] = { 0 };
    size_t o = 0;
    R->ret = ipush(st, header, key);
    memcpy(R->out + o, header, 24); o += 24;
    memset(m1, 1, sizeof m1); memset(m2, 2, sizeof m2); memset(m3, 3, sizeof m3);
    ull clen = 0;
    R->ret |= push(st, R->out + o, &clen, m1, sizeof m1, NULL, 0, 0);
    o += clen;
    rekey(st);
    R->ret |= push(st, R->out + o, &clen, m2, sizeof m2, m1, 10, 1);
    o += clen;
    R->ret |= push(st, R->out + o, &clen, m3, sizeof m3, NULL, 0, 3);
    o += clen;
    /* now pull it back */
    void *st2 = calloc(1, sb() + 64);
    R->ret |= ipull(st2, header, key);
    uc tag = 0; ull mlen = 0;
    size_t p = 24;
    R->ret |= pull(st2, R->out + o, &mlen, &tag, R->out + p, sizeof m1 + 17, NULL, 0);
    o += mlen; R->out[o++] = tag; p += sizeof m1 + 17;
    rekey(st2);
    R->ret |= pull(st2, R->out + o, &mlen, &tag, R->out + p, sizeof m2 + 17, m1, 10);
    o += mlen; R->out[o++] = tag; p += sizeof m2 + 17;
    R->ret |= pull(st2, R->out + o, &mlen, &tag, R->out + p, sizeof m3 + 17, NULL, 0);
    o += mlen; R->out[o++] = tag;
    R->extra = o * 1000 + sb();
    free(st); free(st2);
});

/* =================== scalarmult / core_ed25519 / ristretto =================== */
{
    uc n32[32], p32[32], h64[64], n64[64];
    fillr(n32, 32); fillr(p32, 32); fillr(h64, 64); fillr(n64, 64);
    n32[0] &= 248; n32[31] &= 127; n32[31] |= 64;
    CASE("crypto_scalarmult_base", 32, {
        GETF(fp49, f, "crypto_scalarmult_base");
        R->ret = f(R->out, n32);
    });
    CASE("crypto_scalarmult_curve25519_base", 32, {
        GETF(fp49, f, "crypto_scalarmult_curve25519_base");
        R->ret = f(R->out, n32);
    });
    CASE("crypto_scalarmult", 32, {
        GETF(fp49, b, "crypto_scalarmult_base");
        GETF(fp50, f, "crypto_scalarmult");
        uc pk[32];
        b(pk, p32);
        R->ret = f(R->out, n32, pk);
    });
    CASE("crypto_scalarmult_curve25519 zero", 32, {
        GETF(fp50, f, "crypto_scalarmult_curve25519");
        uc z[32] = { 0 };
        R->ret = f(R->out, n32, z);
    });
    CASE("crypto_scalarmult_ed25519_base", 32, {
        GETF(fp49, f, "crypto_scalarmult_ed25519_base");
        R->ret = f(R->out, n32);
    });
    CASE("crypto_scalarmult_ed25519_base_noclamp", 32, {
        GETF(fp49, f, "crypto_scalarmult_ed25519_base_noclamp");
        R->ret = f(R->out, n32);
    });
    CASE("crypto_scalarmult_ed25519", 32, {
        GETF(fp49, b, "crypto_scalarmult_ed25519_base");
        GETF(fp50, f, "crypto_scalarmult_ed25519");
        uc pk[32];
        b(pk, p32);
        R->ret = f(R->out, n32, pk);
    });
    CASE("crypto_scalarmult_ed25519_noclamp", 32, {
        GETF(fp49, b, "crypto_scalarmult_ed25519_base");
        GETF(fp50, f, "crypto_scalarmult_ed25519_noclamp");
        uc pk[32];
        b(pk, p32);
        R->ret = f(R->out, n32, pk);
    });
    for (int alg = 1; alg <= 2; alg++) {
        char nm[80];
        snprintf(nm, sizeof nm, "crypto_core_ed25519_from_string_nu alg%d", alg);
        CASE(nm, 32, {
            GETF(fp_fromstr, f, "crypto_core_ed25519_from_string_nu");
            R->ret = f(R->out, (const uc *) "ctx", 3, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "crypto_core_ed25519_from_string alg%d", alg);
        CASE(nm, 32, {
            GETF(fp_fromstr, f, "crypto_core_ed25519_from_string");
            R->ret = f(R->out, (const uc *) "ctx", 3, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "crypto_core_ed25519_scalar_from_string alg%d", alg);
        CASE(nm, 32, {
            GETF(fp_fromstr, f, "crypto_core_ed25519_scalar_from_string");
            R->ret = f(R->out, (const uc *) "ctx", 3, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "crypto_core_ristretto255_from_string alg%d", alg);
        CASE(nm, 32, {
            GETF(fp_fromstr, f, "crypto_core_ristretto255_from_string");
            R->ret = f(R->out, (const uc *) "ctx", 3, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "crypto_core_ristretto255_scalar_from_string alg%d", alg);
        CASE(nm, 32, {
            GETF(fp_fromstr, f, "crypto_core_ristretto255_scalar_from_string");
            R->ret = f(R->out, (const uc *) "ctx", 3, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "_sodium_core_h2c_string_to_hash alg%d", alg);
        CASE(nm, 96, {
            GETF(fp_h2c, f, "_sodium_core_h2c_string_to_hash");
            R->ret = f(R->out, 96, (const uc *) "ctxctx", 6, h64, 64, alg);
        });
        snprintf(nm, sizeof nm, "_sodium_core_h2c_string_to_hash big-ctx alg%d", alg);
        CASE(nm, 64, {
            GETF(fp_h2c, f, "_sodium_core_h2c_string_to_hash");
            uc bigctx[300];
            for (int q = 0; q < 300; q++) bigctx[q] = (uc) (q * 7 + alg);
            R->ret = f(R->out, 64, bigctx, 300, h64, 64, alg);
        });
    }
    CASE("crypto_core_ed25519_add/sub", 64, {
        GETF(fp_fromstr, u, "crypto_core_ed25519_from_string");
        GETF(fp50, a, "crypto_core_ed25519_add");
        GETF(fp50, s, "crypto_core_ed25519_sub");
        uc P[32], Q[32];
        u(P, (const uc *) "c1", 2, h64, 64, 2);
        u(Q, (const uc *) "c2", 2, h64, 64, 2);
        R->ret = a(R->out, P, Q);
        R->ret |= s(R->out + 32, P, Q);
    });
    CASE("crypto_core_ed25519_is_valid_point", 0, {
        GETF(fp_fromstr, u, "crypto_core_ed25519_from_string");
        GETF(fp51, f, "crypto_core_ed25519_is_valid_point");
        uc P[32];
        u(P, (const uc *) "c1", 2, h64, 64, 2);
        R->ret = f(P) * 10 + f(n32);
    });
    CASE("crypto_core_ed25519_scalar_random", 32, {
        GETF(fp22, f, "crypto_core_ed25519_scalar_random");
        f(R->out);
    });
    CASE("crypto_core_ed25519_scalar_ops", 160, {
        GETF(fp52, red, "crypto_core_ed25519_scalar_reduce");
        GETF(fp53, add, "crypto_core_ed25519_scalar_add");
        GETF(fp53, sub, "crypto_core_ed25519_scalar_sub");
        GETF(fp53, mul, "crypto_core_ed25519_scalar_mul");
        GETF(fp52, neg, "crypto_core_ed25519_scalar_negate");
        GETF(fp52, cpl, "crypto_core_ed25519_scalar_complement");
        GETF(fp49, inv, "crypto_core_ed25519_scalar_invert");
        GETF(fp51, isc, "crypto_core_ed25519_scalar_is_canonical");
        red(R->out, n64);
        add(R->out + 32, R->out, n32);
        sub(R->out + 64, R->out, n32);
        mul(R->out + 96, R->out, n32);
        neg(R->out + 128, R->out);
        uc t[32];
        cpl(t, R->out);
        R->ret = inv(t, R->out) * 10 + isc(R->out);
        R->extra = (unsigned long long) isc(n32);
    });
    CASE("crypto_core_ristretto255_from_hash", 32, {
        GETF(fp49, f, "crypto_core_ristretto255_from_hash");
        R->ret = f(R->out, h64);
    });
    CASE("crypto_core_ristretto255_add/sub", 64, {
        GETF(fp49, u, "crypto_core_ristretto255_from_hash");
        GETF(fp50, a, "crypto_core_ristretto255_add");
        GETF(fp50, s, "crypto_core_ristretto255_sub");
        uc P[32], Q[32];
        u(P, h64); u(Q, n64);
        R->ret = a(R->out, P, Q);
        R->ret |= s(R->out + 32, P, Q);
    });
    CASE("crypto_core_ristretto255_is_valid_point", 0, {
        GETF(fp49, u, "crypto_core_ristretto255_from_hash");
        GETF(fp51, f, "crypto_core_ristretto255_is_valid_point");
        uc P[32];
        u(P, h64);
        R->ret = f(P) * 10 + f(n32);
    });
    CASE("crypto_core_ristretto255_scalar_ops", 160, {
        GETF(fp52, red, "crypto_core_ristretto255_scalar_reduce");
        GETF(fp53, add, "crypto_core_ristretto255_scalar_add");
        GETF(fp53, sub, "crypto_core_ristretto255_scalar_sub");
        GETF(fp53, mul, "crypto_core_ristretto255_scalar_mul");
        GETF(fp52, neg, "crypto_core_ristretto255_scalar_negate");
        GETF(fp52, cpl, "crypto_core_ristretto255_scalar_complement");
        GETF(fp49, inv, "crypto_core_ristretto255_scalar_invert");
        GETF(fp51, isc, "crypto_core_ristretto255_scalar_is_canonical");
        red(R->out, n64);
        add(R->out + 32, R->out, n32);
        sub(R->out + 64, R->out, n32);
        mul(R->out + 96, R->out, n32);
        neg(R->out + 128, R->out);
        uc t[32];
        cpl(t, R->out);
        R->ret = inv(t, R->out) * 10 + isc(R->out);
    });
    CASE("crypto_scalarmult_ristretto255", 64, {
        GETF(fp49, b, "crypto_scalarmult_ristretto255_base");
        GETF(fp50, f, "crypto_scalarmult_ristretto255");
        R->ret = b(R->out, n32);
        R->ret |= f(R->out + 32, p32, R->out);
    });
    CASE("crypto_core_ed25519_random", 32, {
        GETF(fp22, f, "crypto_core_ed25519_random");
        f(R->out);
    });
    CASE("crypto_core_ristretto255_random", 32, {
        GETF(fp22, f, "crypto_core_ristretto255_random");
        f(R->out);
    });
    CASE("crypto_core_ristretto255_scalar_random", 32, {
        GETF(fp22, f, "crypto_core_ristretto255_scalar_random");
        f(R->out);
    });
}

/* =================== box =================== */
{
    uc seed[32], msg[300], n24[24];
    fillr(seed, 32); fillr(msg, sizeof msg); fillr(n24, 24);
    static const char *const pfx[] = {
        "crypto_box", "crypto_box_curve25519xchacha20poly1305", NULL
    };
    for (int j = 0; pfx[j]; j++) {
        char nm[160], s[160];
        snprintf(nm, sizeof nm, "%s_seed_keypair", pfx[j]);
        snprintf(s, sizeof s, "%s_seed_keypair", pfx[j]);
        CASE(nm, 64, {
            GETF(fp55, f, s);
            R->ret = f(R->out, R->out + 32, seed);
        });
        snprintf(nm, sizeof nm, "%s_keypair", pfx[j]);
        snprintf(s, sizeof s, "%s_keypair", pfx[j]);
        CASE(nm, 64, {
            GETF(fp56, f, s);
            R->ret = f(R->out, R->out + 32);
        });
        snprintf(nm, sizeof nm, "%s_beforenm", pfx[j]);
        CASE(nm, 32, {
            char sk[160], sb2[160];
            snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
            snprintf(sb2, sizeof sb2, "%s_beforenm", pfx[j]);
            GETF(fp55, kp, sk);
            GETF(fp50, f, sb2);
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            R->ret = f(R->out, pk2, sk1);
        });
        for (int len = 0; len <= 300; len = len ? len * 9 + 5 : 1) {
            snprintf(nm, sizeof nm, "%s_easy len%d", pfx[j], len);
            CASE(nm, (size_t) len + 16, {
                char sk[160], se[160];
                snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
                snprintf(se, sizeof se, "%s_easy", pfx[j]);
                GETF(fp55, kp, sk);
                GETF(fp57, f, se);
                uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32];
                memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
                kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
                R->ret = f(R->out, msg, (ull) len, n24, pk2, sk1);
            });
            snprintf(nm, sizeof nm, "%s_open_easy len%d", pfx[j], len);
            CASE(nm, (size_t) len, {
                char sk[160], se[160], so[160];
                snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
                snprintf(se, sizeof se, "%s_easy", pfx[j]);
                snprintf(so, sizeof so, "%s_open_easy", pfx[j]);
                GETF(fp55, kp, sk);
                GETF(fp57, e, se);
                GETF(fp57, f, so);
                uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], ct[400];
                memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
                kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
                e(ct, msg, (ull) len, n24, pk2, sk1);
                R->ret = f(R->out, ct, (ull) len + 16, n24, pk1, sk2);
            });
            snprintf(nm, sizeof nm, "%s_detached len%d", pfx[j], len);
            CASE(nm, (size_t) len + 16, {
                char sk[160], se[160];
                snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
                snprintf(se, sizeof se, "%s_detached", pfx[j]);
                GETF(fp55, kp, sk);
                GETF(fp58, f, se);
                uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32];
                memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
                kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
                R->ret = f(R->out, R->out + len, msg, (ull) len, n24, pk2, sk1);
            });
            snprintf(nm, sizeof nm, "%s_seal len%d", pfx[j], len);
            CASE(nm, (size_t) len + 48, {
                char sk[160], se[160];
                snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
                snprintf(se, sizeof se, "%s_seal", pfx[j]);
                GETF(fp55, kp, sk);
                GETF(fp36, f, se);
                uc pk1[32], sk1[32];
                kp(pk1, sk1, seed);
                R->ret = f(R->out, msg, (ull) len, pk1);
            });
            snprintf(nm, sizeof nm, "%s_seal_open len%d", pfx[j], len);
            CASE(nm, (size_t) len, {
                char sk[160], se[160], so[160];
                snprintf(sk, sizeof sk, "%s_seed_keypair", pfx[j]);
                snprintf(se, sizeof se, "%s_seal", pfx[j]);
                snprintf(so, sizeof so, "%s_seal_open", pfx[j]);
                GETF(fp55, kp, sk);
                GETF(fp36, e, se);
                GETF(fp19, f, so);
                uc pk1[32], sk1[32], ct[400];
                kp(pk1, sk1, seed);
                e(ct, msg, (ull) len, pk1);
                R->ret = f(R->out, ct, (ull) len + 48, pk1, sk1);
            });
        }
    }
    /* NaCl-style crypto_box / crypto_box_open (32-byte zero padding) */
    {
        uc padded[400];
        memset(padded, 0, 32);
        memcpy(padded + 32, msg, 100);
        CASE("crypto_box nacl", 132, {
            GETF(fp55, kp, "crypto_box_seed_keypair");
            GETF(fp57, f, "crypto_box");
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            R->ret = f(R->out, padded, 132, n24, pk2, sk1);
        });
        CASE("crypto_box_open nacl", 132, {
            GETF(fp55, kp, "crypto_box_seed_keypair");
            GETF(fp57, e, "crypto_box");
            GETF(fp57, f, "crypto_box_open");
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], ct[400];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            e(ct, padded, 132, n24, pk2, sk1);
            R->ret = f(R->out, ct, 132, n24, pk1, sk2);
        });
        CASE("crypto_box_afternm", 132, {
            GETF(fp55, kp, "crypto_box_seed_keypair");
            GETF(fp50, bn, "crypto_box_beforenm");
            GETF(fp19, f, "crypto_box_afternm");
            GETF(fp19, o, "crypto_box_open_afternm");
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], k[32], ct[400];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            bn(k, pk2, sk1);
            R->ret = f(ct, padded, 132, n24, k);
            R->ret |= o(R->out, ct, 132, n24, k);
        });
        CASE("crypto_box_easy_afternm", 116, {
            GETF(fp55, kp, "crypto_box_seed_keypair");
            GETF(fp50, bn, "crypto_box_beforenm");
            GETF(fp19, f, "crypto_box_easy_afternm");
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32], k[32];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            bn(k, pk2, sk1);
            R->ret = f(R->out, msg, 100, n24, k);
        });
        CASE("crypto_box_curve25519xsalsa20poly1305", 132, {
            GETF(fp55, kp, "crypto_box_curve25519xsalsa20poly1305_seed_keypair");
            GETF(fp57, f, "crypto_box_curve25519xsalsa20poly1305");
            uc pk1[32], sk1[32], pk2[32], sk2[32], seed2[32];
            memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
            kp(pk1, sk1, seed); kp(pk2, sk2, seed2);
            R->ret = f(R->out, padded, 132, n24, pk2, sk1);
        });
    }
}

/* =================== sign =================== */
{
    uc seed[32], msg[600];
    fillr(seed, 32); fillr(msg, sizeof msg);
    CASE("crypto_sign_seed_keypair", 96, {
        GETF(fp55, f, "crypto_sign_seed_keypair");
        R->ret = f(R->out, R->out + 32, seed);
    });
    CASE("crypto_sign_keypair", 96, {
        GETF(fp56, f, "crypto_sign_keypair");
        R->ret = f(R->out, R->out + 32);
    });
    CASE("crypto_sign_ed25519_seed_keypair", 96, {
        GETF(fp55, f, "crypto_sign_ed25519_seed_keypair");
        R->ret = f(R->out, R->out + 32, seed);
    });
    for (int len = 0; len <= 600; len = len ? len * 9 + 7 : 1) {
        char nm[96];
        snprintf(nm, sizeof nm, "crypto_sign_detached len%d", len);
        CASE(nm, 64, {
            GETF(fp55, kp, "crypto_sign_seed_keypair");
            GETF(fp59, f, "crypto_sign_detached");
            uc pk[32], sk[64];
            kp(pk, sk, seed);
            ull sl = 0;
            R->ret = f(R->out, &sl, msg, (ull) len, sk);
            R->extra = sl;
        });
        snprintf(nm, sizeof nm, "crypto_sign_verify_detached len%d", len);
        CASE(nm, 64, {
            GETF(fp55, kp, "crypto_sign_seed_keypair");
            GETF(fp59, s, "crypto_sign_detached");
            GETF(fp37, f, "crypto_sign_verify_detached");
            uc pk[32], sk[64];
            kp(pk, sk, seed);
            ull sl = 0;
            s(R->out, &sl, msg, (ull) len, sk);
            R->ret = f(R->out, msg, (ull) len, pk);
            uc bad[64];
            memcpy(bad, R->out, 64); bad[0] ^= 1;
            R->extra = (unsigned long long) (f(bad, msg, (ull) len, pk) + 100);
        });
        snprintf(nm, sizeof nm, "crypto_sign len%d", len);
        CASE(nm, (size_t) len + 64, {
            GETF(fp55, kp, "crypto_sign_seed_keypair");
            GETF(fp59, f, "crypto_sign");
            uc pk[32], sk[64];
            kp(pk, sk, seed);
            ull sl = 0;
            R->ret = f(R->out, &sl, msg, (ull) len, sk);
            R->extra = sl;
        });
        snprintf(nm, sizeof nm, "crypto_sign_open len%d", len);
        CASE(nm, (size_t) len, {
            GETF(fp55, kp, "crypto_sign_seed_keypair");
            GETF(fp59, s, "crypto_sign");
            GETF(fp59, f, "crypto_sign_open");
            uc pk[32], sk[64], sm[700];
            kp(pk, sk, seed);
            ull sl = 0;
            s(sm, &sl, msg, (ull) len, sk);
            ull ml = 0;
            R->ret = f(R->out, &ml, sm, sl, pk);
            R->extra = ml;
        });
    }
    CASE("crypto_sign_ed25519_sk_to_pk/seed/curve25519", 96, {
        GETF(fp55, kp, "crypto_sign_ed25519_seed_keypair");
        GETF(fp49, tp, "crypto_sign_ed25519_sk_to_pk");
        GETF(fp49, ts, "crypto_sign_ed25519_sk_to_seed");
        GETF(fp49, tc, "crypto_sign_ed25519_sk_to_curve25519");
        GETF(fp49, pc, "crypto_sign_ed25519_pk_to_curve25519");
        uc pk[32], sk[64];
        kp(pk, sk, seed);
        R->ret  = tp(R->out, sk);
        R->ret |= ts(R->out + 32, sk);
        R->ret |= tc(R->out + 64, sk);
        uc c[32];
        R->ret |= pc(c, pk);
        R->extra = c[0] + 256ULL * c[31];
    });
    CASE("crypto_sign_ph", 64, {
        GETF(fp24, sb, "crypto_sign_statebytes");
        GETF(fp55, kp, "crypto_sign_seed_keypair");
        GETF(fp25, fi, "crypto_sign_init");
        GETF(fp26, fu, "crypto_sign_update");
        GETF(fp60, fc, "crypto_sign_final_create");
        GETF(fp46, fv, "crypto_sign_final_verify");
        uc pk[32], sk[64];
        kp(pk, sk, seed);
        void *st = calloc(1, sb() + 64);
        R->ret = fi(st);
        R->ret |= fu(st, msg, 137);
        R->ret |= fu(st, msg + 137, 200);
        ull sl = 0;
        R->ret |= fc(st, R->out, &sl, sk);
        void *st2 = calloc(1, sb() + 64);
        fi(st2);
        fu(st2, msg, 337);
        R->extra = (unsigned long long) (fv(st2, R->out, pk) + 100) * 1000 + sb();
        free(st); free(st2);
    });
}

/* =================== kdf =================== */
{
    uc key[64], ctx[8];
    fillr(key, 64); fillr(ctx, 8);
    for (int ol = 16; ol <= 64; ol += 16) {
        char nm[96];
        snprintf(nm, sizeof nm, "crypto_kdf_derive_from_key out%d", ol);
        CASE(nm, (size_t) ol, {
            GETF(fp61, f, "crypto_kdf_derive_from_key");
            R->ret = f(R->out, (size_t) ol, 42, (const char *) ctx, key);
        });
        snprintf(nm, sizeof nm, "crypto_kdf_blake2b_derive_from_key out%d", ol);
        CASE(nm, (size_t) ol, {
            GETF(fp61, f, "crypto_kdf_blake2b_derive_from_key");
            R->ret = f(R->out, (size_t) ol, 42, (const char *) ctx, key);
        });
    }
    static const char *const hk[] = { "crypto_kdf_hkdf_sha256", "crypto_kdf_hkdf_sha512", NULL };
    for (int j = 0; hk[j]; j++) {
        char nm[128], s[128];
        snprintf(nm, sizeof nm, "%s_extract+expand", hk[j]);
        CASE(nm, 200, {
            char se[128], sx[128];
            snprintf(se, sizeof se, "%s_extract", hk[j]);
            snprintf(sx, sizeof sx, "%s_expand", hk[j]);
            GETF(fp62, e, se);
            GETF(fp63, x, sx);
            uc prk[64];
            R->ret = e(prk, key, 16, key + 16, 32);
            memcpy(R->out, prk, 64);
            R->ret |= x(R->out + 64, 100, "info", 4, prk);
        });
        snprintf(nm, sizeof nm, "%s_extract streaming", hk[j]);
        snprintf(s, sizeof s, "%s_extract_init", hk[j]);
        CASE(nm, 128, {
            char s2[128], s3[128], s4[128];
            snprintf(s2, sizeof s2, "%s_extract_update", hk[j]);
            snprintf(s3, sizeof s3, "%s_extract_final", hk[j]);
            snprintf(s4, sizeof s4, "%s_statebytes", hk[j]);
            GETF(fp24, sb, s4);
            GETF(fp29, fi, s);
            GETF(fp29, fu, s2);
            GETF(fp27, ff, s3);
            void *st = calloc(1, sb() + 64);
            R->ret = fi(st, key, 16);
            R->ret |= fu(st, key + 16, 20);
            R->ret |= fu(st, key + 36, 28);
            R->ret |= ff(st, R->out);
            R->extra = sb();
            free(st);
        });
    }
}

/* =================== kem =================== */
{
    uc seed[64];
    fillr(seed, 64);
    struct { const char *pfx; size_t seedb, pkb, skb, ctb, ssb; } km[] = {
        { "crypto_kem_mlkem768", 64, 1184, 2400, 1088, 32 },
        { "crypto_kem_xwing", 32, 1216, 32, 1120, 32 },
        { "crypto_kem", 32, 1216, 32, 1120, 32 },
    };
    for (unsigned j = 0; j < sizeof km / sizeof km[0]; j++) {
        char nm[160], s[160];
        snprintf(nm, sizeof nm, "%s_seed_keypair", km[j].pfx);
        snprintf(s, sizeof s, "%s_seed_keypair", km[j].pfx);
        CASE(nm, 0, {
            char sq[160];
            snprintf(sq, sizeof sq, "%s_publickeybytes", km[j].pfx);
            GETF(fp24, pkb, sq);
            snprintf(sq, sizeof sq, "%s_secretkeybytes", km[j].pfx);
            GETF(fp24, skb, sq);
            snprintf(sq, sizeof sq, "%s_seedbytes", km[j].pfx);
            GETF(fp24, sdb, sq);
            GETF(fp55, f, s);
            uc *pk = malloc(pkb() + 16), *sk = malloc(skb() + 16);
            R->ret = f(pk, sk, seed);
            /* hash the keys into out via a simple checksum so sizes fit */
            unsigned long long acc = 1469598103934665603ULL;
            for (size_t i = 0; i < pkb(); i++) { acc ^= pk[i]; acc *= 1099511628211ULL; }
            for (size_t i = 0; i < skb(); i++) { acc ^= sk[i]; acc *= 1099511628211ULL; }
            R->extra = acc;
            (void) sdb;
            free(pk); free(sk);
        });
        snprintf(nm, sizeof nm, "%s_enc/dec", km[j].pfx);
        CASE(nm, 64, {
            char sq[160], se[160], sd[160], sk2[160];
            snprintf(sq, sizeof sq, "%s_publickeybytes", km[j].pfx);
            GETF(fp24, pkb, sq);
            snprintf(sq, sizeof sq, "%s_secretkeybytes", km[j].pfx);
            GETF(fp24, skb, sq);
            snprintf(sq, sizeof sq, "%s_ciphertextbytes", km[j].pfx);
            GETF(fp24, ctb, sq);
            snprintf(sk2, sizeof sk2, "%s_seed_keypair", km[j].pfx);
            snprintf(se, sizeof se, "%s_enc", km[j].pfx);
            snprintf(sd, sizeof sd, "%s_dec", km[j].pfx);
            GETF(fp55, kp, sk2);
            GETF(fp55, e, se);
            GETF(fp50, d, sd);
            uc *pk = malloc(pkb() + 16), *sk = malloc(skb() + 16), *ct = malloc(ctb() + 16);
            kp(pk, sk, seed);
            R->ret = e(ct, R->out, pk);
            R->ret |= d(R->out + 32, ct, sk);
            free(pk); free(sk); free(ct);
        });
        if (strcmp(km[j].pfx, "crypto_kem") == 0) continue;
        snprintf(nm, sizeof nm, "%s_enc_deterministic", km[j].pfx);
        CASE(nm, 32, {
            char sq[160], se[160], sk2[160];
            snprintf(sq, sizeof sq, "%s_publickeybytes", km[j].pfx);
            GETF(fp24, pkb, sq);
            snprintf(sq, sizeof sq, "%s_secretkeybytes", km[j].pfx);
            GETF(fp24, skb, sq);
            snprintf(sq, sizeof sq, "%s_ciphertextbytes", km[j].pfx);
            GETF(fp24, ctb, sq);
            snprintf(sk2, sizeof sk2, "%s_seed_keypair", km[j].pfx);
            snprintf(se, sizeof se, "%s_enc_deterministic", km[j].pfx);
            GETF(fp55, kp, sk2);
            GETF(fp64, e, se);
            uc *pk = malloc(pkb() + 16), *sk = malloc(skb() + 16), *ct = malloc(ctb() + 16);
            kp(pk, sk, seed);
            R->ret = e(ct, R->out, pk, seed + 16);
            unsigned long long acc = 1469598103934665603ULL;
            for (size_t i = 0; i < ctb(); i++) { acc ^= ct[i]; acc *= 1099511628211ULL; }
            R->extra = acc;
            free(pk); free(sk); free(ct);
        });
    }
}

/* =================== kx =================== */
{
    uc seed[32];
    fillr(seed, 32);
    CASE("crypto_kx_seed_keypair", 64, {
        GETF(fp55, f, "crypto_kx_seed_keypair");
        R->ret = f(R->out, R->out + 32, seed);
    });
    CASE("crypto_kx_keypair", 64, {
        GETF(fp56, f, "crypto_kx_keypair");
        R->ret = f(R->out, R->out + 32);
    });
    CASE("crypto_kx_session_keys", 128, {
        GETF(fp55, kp, "crypto_kx_seed_keypair");
        GETF(fp65, cl, "crypto_kx_client_session_keys");
        GETF(fp65, sv, "crypto_kx_server_session_keys");
        uc cpk[32], csk[32], spk[32], ssk[32], seed2[32];
        memcpy(seed2, seed, 32); seed2[0] ^= 0xff;
        kp(cpk, csk, seed); kp(spk, ssk, seed2);
        R->ret  = cl(R->out, R->out + 32, cpk, csk, spk);
        R->ret |= sv(R->out + 64, R->out + 96, spk, ssk, cpk);
    });
}

/* =================== ipcrypt =================== */
{
    uc key[64], ip[16], tweak[16];
    fillr(key, 64); fillr(ip, 16); fillr(tweak, 16);
    for (int cls = 0; cls < 4; cls++) {
        uc ip2[16], key2[64], tw[16];
        char nm[80];
        memcpy(ip2, ip, 16); memcpy(key2, key, 64); memcpy(tw, tweak, 16);
        if (cls == 1) { memset(ip2, 0, 10); ip2[10] = 0xff; ip2[11] = 0xff; }
        if (cls == 2) { memcpy(key2 + 16, key2, 16); memcpy(key2 + 48, key2 + 32, 16); }
        if (cls == 3) { memset(ip2, 0, 16); memset(key2, 0, 64); }
        snprintf(nm, sizeof nm, "crypto_ipcrypt_encrypt/decrypt c%d", cls);
        CASE(nm, 32, {
            GETF(fpv_ip, e, "crypto_ipcrypt_encrypt");
            GETF(fpv_ip, d, "crypto_ipcrypt_decrypt");
            e(R->out, ip2, key2);
            d(R->out + 16, R->out, key2);
        });
        snprintf(nm, sizeof nm, "crypto_ipcrypt_nd c%d", cls);
        CASE(nm, 48, {
            GETF(fpv_ipt, e, "crypto_ipcrypt_nd_encrypt");
            GETF(fpv_ip, d, "crypto_ipcrypt_nd_decrypt");
            e(R->out, ip2, tw, key2);
            d(R->out + 24, R->out, key2);
        });
        snprintf(nm, sizeof nm, "crypto_ipcrypt_ndx c%d", cls);
        CASE(nm, 64, {
            GETF(fpv_ipt, e, "crypto_ipcrypt_ndx_encrypt");
            GETF(fpv_ip, d, "crypto_ipcrypt_ndx_decrypt");
            e(R->out, ip2, tw, key2);
            d(R->out + 32, R->out, key2);
        });
        snprintf(nm, sizeof nm, "crypto_ipcrypt_pfx c%d", cls);
        CASE(nm, 32, {
            GETF(fpv_ip, e, "crypto_ipcrypt_pfx_encrypt");
            GETF(fpv_ip, d, "crypto_ipcrypt_pfx_decrypt");
            e(R->out, ip2, key2);
            d(R->out + 16, R->out, key2);
        });
    }
}

/* =================== pwhash =================== */
{
    const char *pw = "Correct Horse Battery Staple";
    uc salt[32];
    fillr(salt, 32);
    for (int alg = 1; alg <= 2; alg++) {
        char nm[96];
        snprintf(nm, sizeof nm, "crypto_pwhash alg%d", alg);
        CASE(nm, 64, {
            GETF(fp66, f, "crypto_pwhash");
            R->ret = f(R->out, 64, pw, strlen(pw), salt, 3, 16384, alg);
        });
    }
    CASE("crypto_pwhash_argon2i", 64, {
        GETF(fp66, f, "crypto_pwhash");
        R->ret = f(R->out, 64, pw, strlen(pw), salt, 4, 32768, 1);
    });
    CASE("crypto_pwhash_str_verify argon2id", 128, {
        GETF(fp67, f, "crypto_pwhash_str");
        GETF(fp68, v, "crypto_pwhash_str_verify");
        GETF(fp69, nr, "crypto_pwhash_str_needs_rehash");
        R->ret = f((char *) R->out, pw, strlen(pw), 3, 16384);
        R->extra = (unsigned long long) (v((char *) R->out, pw, strlen(pw)) + 10) * 1000
                 + (unsigned long long) (nr((char *) R->out, 3, 16384) + 10);
    });
    CASE("crypto_pwhash_str_alg argon2i", 128, {
        GETF(fp70, f, "crypto_pwhash_str_alg");
        GETF(fp68, v, "crypto_pwhash_str_verify");
        R->ret = f((char *) R->out, pw, strlen(pw), 3, 16384, 1);
        R->extra = (unsigned long long) (v((char *) R->out, pw, strlen(pw)) + 10);
    });
    CASE("crypto_pwhash_argon2i_str", 128, {
        GETF(fp67, f, "crypto_pwhash_argon2i_str");
        GETF(fp68, v, "crypto_pwhash_argon2i_str_verify");
        R->ret = f((char *) R->out, pw, strlen(pw), 3, 16384);
        R->extra = (unsigned long long) (v((char *) R->out, pw, strlen(pw)) + 10);
    });
    CASE("crypto_pwhash_argon2id_str", 128, {
        GETF(fp67, f, "crypto_pwhash_argon2id_str");
        GETF(fp68, v, "crypto_pwhash_argon2id_str_verify");
        R->ret = f((char *) R->out, pw, strlen(pw), 3, 16384);
        R->extra = (unsigned long long) (v((char *) R->out, pw, strlen(pw)) + 10);
    });
    CASE("crypto_pwhash_argon2i", 64, {
        GETF(fp66, f, "crypto_pwhash_argon2i");
        R->ret = f(R->out, 64, pw, strlen(pw), salt, 3, 16384, 1);
    });
    CASE("crypto_pwhash_argon2id", 64, {
        GETF(fp66, f, "crypto_pwhash_argon2id");
        R->ret = f(R->out, 64, pw, strlen(pw), salt, 3, 16384, 2);
    });
    CASE("crypto_pwhash_scryptsalsa208sha256", 64, {
        GETF(fp71, f, "crypto_pwhash_scryptsalsa208sha256");
        R->ret = f(R->out, 64, pw, strlen(pw), salt, 32768, 16777216);
    });
    CASE("crypto_pwhash_scryptsalsa208sha256_ll", 64, {
        GETF(fp72, f, "crypto_pwhash_scryptsalsa208sha256_ll");
        R->ret = f((const uc *) pw, strlen(pw), salt, 32, 1024, 8, 1, R->out, 64);
    });
    CASE("crypto_pwhash_scryptsalsa208sha256_str", 128, {
        GETF(fp67, f, "crypto_pwhash_scryptsalsa208sha256_str");
        GETF(fp68, v, "crypto_pwhash_scryptsalsa208sha256_str_verify");
        GETF(fp69, nr, "crypto_pwhash_scryptsalsa208sha256_str_needs_rehash");
        R->ret = f((char *) R->out, pw, strlen(pw), 32768, 16777216);
        R->extra = (unsigned long long) (v((char *) R->out, pw, strlen(pw)) + 10) * 1000
                 + (unsigned long long) (nr((char *) R->out, 32768, 16777216) + 10);
    });
}

/* =================== randombytes deterministic =================== */
{
    uc seed[32];
    fillr(seed, 32);
    CASE("randombytes_buf_deterministic", 256, {
        GETF(fp73, f, "randombytes_buf_deterministic");
        f(R->out, 256, seed);
    });
    CASE("randombytes_uniform", 0, {
        GETF(fp74, f, "randombytes_uniform");
        unsigned long long acc = 0;
        for (int i = 0; i < 32; i++) acc = acc * 131 + f(1000 + i);
        R->ret = (long long) (acc & 0x7fffffff);
    });
    CASE("randombytes_implementation_name", 32, {
        GETF(fp1, f, "randombytes_implementation_name");
        const char *s = f();
        size_t n = strlen(s);
        if (n > 31) n = 31;
        memcpy(R->out, s, n + 1);
    });
}

/* =================== internal ed25519 / softaes =================== */
{
    uc a[32], b[64];
    fillr(a, 32); fillr(b, 64);
    CASE("_sodium_sc25519_reduce", 64, {
        GETF(fp22, f, "_sodium_sc25519_reduce");
        memcpy(R->out, b, 64);
        f(R->out);
    });
    CASE("_sodium_sc25519_muladd", 32, {
        GETF(fp75, f, "_sodium_sc25519_muladd");
        f(R->out, a, b, b + 32);
    });
    CASE("_sodium_sc25519_mul", 32, {
        GETF(fp53, f, "_sodium_sc25519_mul");
        f(R->out, a, b);
    });
    CASE("_sodium_sc25519_invert", 32, {
        GETF(fp52, f, "_sodium_sc25519_invert");
        f(R->out, a);
    });
    CASE("_sodium_sc25519_is_canonical", 0, {
        GETF(fp51, f, "_sodium_sc25519_is_canonical");
        R->ret = f(a) * 10 + f(b);
    });
    CASE("_sodium_fe25519_frombytes/tobytes", 32, {
        GETF(fp76, fb, "_sodium_fe25519_frombytes");
        GETF(fp77, tb, "_sodium_fe25519_tobytes");
        GETF(fp78, iv, "_sodium_fe25519_invert");
        int32_t h[10], g[10];
        fb(h, a);
        iv(g, h);
        tb(R->out, g);
    });
    CASE("_sodium_ge25519_scalarmult_base", 32, {
        GETF(fp79, sm, "_sodium_ge25519_scalarmult_base");
        GETF(fp80, tb, "_sodium_ge25519_p3_tobytes");
        unsigned char p3[512];
        memset(p3, 0, sizeof p3);
        sm(p3, a);
        tb(R->out, p3);
    });
    CASE("_sodium_ge25519_has_small_order", 0, {
        GETF(fp51, f, "_sodium_ge25519_has_small_order");
        R->ret = f(a);
    });
    CASE("_sodium_ge25519_is_canonical", 0, {
        GETF(fp51, f, "_sodium_ge25519_is_canonical");
        R->ret = f(a);
    });
    CASE("_sodium_ge25519_from_uniform", 32, {
        GETF(fp52, f, "_sodium_ge25519_from_uniform");
        f(R->out, a);
    });
    CASE("_sodium_ge25519_from_hash", 32, {
        GETF(fp52, f, "_sodium_ge25519_from_hash");
        f(R->out, b);
    });
    CASE("_sodium_ristretto255_from_hash", 32, {
        GETF(fpv_2p, f, "_sodium_ristretto255_from_hash");
        f(R->out, b);
    });
    CASE("_sodium_softaes_expand_key128", 176, {
        GETF(fp79, f, "_sodium_softaes_expand_key128");
        f(R->out, a);
    });
    CASE("_sodium_softaes_expand_key256", 240, {
        GETF(fp79, f, "_sodium_softaes_expand_key256");
        f(R->out, a);
    });
    CASE("_sodium_blake2b_long", 100, {
        GETF(fp82, f, "_sodium_blake2b_long");
        R->ret = f(R->out, 100, b, 64);
    });
    CASE("crypto_core_keccak1600", 200, {
        GETF(fp24, sb, "crypto_core_keccak1600_statebytes");
        GETF(fp48, fi, "crypto_core_keccak1600_init");
        GETF(fp83, fx, "crypto_core_keccak1600_xor_bytes");
        GETF(fp84, fe, "crypto_core_keccak1600_extract_bytes");
        GETF(fp48, p24, "crypto_core_keccak1600_permute_24");
        GETF(fp48, p12, "crypto_core_keccak1600_permute_12");
        void *st = calloc(1, sb() + 64);
        fi(st);
        fx(st, b, 0, 64);
        p24(st);
        fe(st, R->out, 0, 100);
        p12(st);
        fe(st, R->out + 100, 0, 100);
        R->extra = sb();
        free(st);
    });
}

#undef CASE
#undef GETF
