#include <sodium.h>

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned char transcript[32768];
static size_t transcript_len;

static void
record(const void *data, size_t len)
{
    if (transcript_len + len > sizeof transcript) {
        fprintf(stderr, "transcript overflow\n");
        exit(2);
    }
    memcpy(transcript + transcript_len, data, len);
    transcript_len += len;
}

static void
require(int condition, const char *operation)
{
    if (!condition) {
        fprintf(stderr, "%s failed\n", operation);
        exit(3);
    }
}

static void
fill(unsigned char *out, size_t len, unsigned char domain)
{
    size_t i;

    for (i = 0; i < len; i++) {
        out[i] = (unsigned char) (domain + i * 29U + (i >> 1));
    }
}

static void
record_crypto_primitives(void)
{
    static const unsigned char message[] =
        "libsodium Rust differential transcript";
    unsigned char key[64];
    unsigned char nonce[32];
    unsigned char ad[23];
    unsigned char out[512];
    unsigned char opened[sizeof message];
    unsigned char mac[64];
    unsigned long long out_len;

    fill(key, sizeof key, 7);
    fill(nonce, sizeof nonce, 31);
    fill(ad, sizeof ad, 79);

    require(crypto_hash_sha256(out, message, sizeof message) == 0, "sha256");
    record(out, crypto_hash_sha256_bytes());
    require(crypto_hash_sha512(out, message, sizeof message) == 0, "sha512");
    record(out, crypto_hash_sha512_bytes());
    require(crypto_hash_sha3256(out, message, sizeof message) == 0, "sha3-256");
    record(out, crypto_hash_sha3256_bytes());
    require(crypto_hash_sha3512(out, message, sizeof message) == 0, "sha3-512");
    record(out, crypto_hash_sha3512_bytes());
    require(crypto_generichash(out, 48, message, sizeof message, key, 32) == 0,
            "generichash");
    record(out, 48);
    require(crypto_xof_shake128(out, 73, message, sizeof message) == 0,
            "shake128");
    record(out, 73);
    require(crypto_xof_shake256(out, 79, message, sizeof message) == 0,
            "shake256");
    record(out, 79);
    require(crypto_xof_turboshake128(out, 83, message, sizeof message) == 0,
            "turboshake128");
    record(out, 83);
    require(crypto_xof_turboshake256(out, 89, message, sizeof message) == 0,
            "turboshake256");
    record(out, 89);

    require(crypto_auth_hmacsha256(mac, message, sizeof message, key) == 0,
            "hmacsha256");
    record(mac, crypto_auth_hmacsha256_bytes());
    require(crypto_auth_hmacsha512(mac, message, sizeof message, key) == 0,
            "hmacsha512");
    record(mac, crypto_auth_hmacsha512_bytes());
    require(crypto_auth_hmacsha512256(mac, message, sizeof message, key) == 0,
            "hmacsha512256");
    record(mac, crypto_auth_hmacsha512256_bytes());
    require(crypto_onetimeauth_poly1305(mac, message, sizeof message, key) == 0,
            "poly1305");
    record(mac, crypto_onetimeauth_poly1305_bytes());
    require(crypto_shorthash_siphash24(mac, message, sizeof message, key) == 0,
            "siphash24");
    record(mac, crypto_shorthash_siphash24_bytes());
    require(crypto_shorthash_siphashx24(mac, message, sizeof message, key) == 0,
            "siphashx24");
    record(mac, crypto_shorthash_siphashx24_bytes());

    require(crypto_core_hchacha20(out, nonce, key, NULL) == 0, "hchacha20");
    record(out, crypto_core_hchacha20_outputbytes());
    require(crypto_core_hsalsa20(out, nonce, key, NULL) == 0, "hsalsa20");
    record(out, crypto_core_hsalsa20_outputbytes());
    require(crypto_stream_chacha20_xor(out, message, sizeof message, nonce, key) == 0,
            "chacha20");
    record(out, sizeof message);
    require(crypto_stream_xchacha20_xor(out, message, sizeof message, nonce, key) == 0,
            "xchacha20");
    record(out, sizeof message);
    require(crypto_stream_salsa20_xor(out, message, sizeof message, nonce, key) == 0,
            "salsa20");
    record(out, sizeof message);

    require(crypto_secretbox_easy(out, message, sizeof message, nonce, key) == 0,
            "secretbox");
    record(out, sizeof message + crypto_secretbox_macbytes());
    require(crypto_secretbox_open_easy(opened, out,
                                       sizeof message + crypto_secretbox_macbytes(),
                                       nonce, key) == 0,
            "secretbox open");
    require(memcmp(opened, message, sizeof message) == 0, "secretbox plaintext");

    require(crypto_aead_chacha20poly1305_ietf_encrypt(
                out, &out_len, message, sizeof message, ad, sizeof ad, NULL,
                nonce, key) == 0,
            "chacha20poly1305");
    record(out, (size_t) out_len);
    require(crypto_aead_xchacha20poly1305_ietf_encrypt(
                out, &out_len, message, sizeof message, ad, sizeof ad, NULL,
                nonce, key) == 0,
            "xchacha20poly1305");
    record(out, (size_t) out_len);
    require(crypto_aead_aegis128l_encrypt(
                out, &out_len, message, sizeof message, ad, sizeof ad, NULL,
                nonce, key) == 0,
            "aegis128l");
    record(out, (size_t) out_len);
    require(crypto_aead_aegis256_encrypt(
                out, &out_len, message, sizeof message, ad, sizeof ad, NULL,
                nonce, key) == 0,
            "aegis256");
    record(out, (size_t) out_len);
}

static void
record_public_key_primitives(void)
{
    static const unsigned char message[] = "public key transcript";
    unsigned char seed[crypto_kem_mlkem768_SEEDBYTES];
    unsigned char seed2[64];
    unsigned char pk[crypto_kem_xwing_PUBLICKEYBYTES];
    unsigned char sk[crypto_kem_mlkem768_SECRETKEYBYTES];
    unsigned char ct[crypto_kem_xwing_CIPHERTEXTBYTES];
    unsigned char shared[64];
    unsigned char shared2[64];
    unsigned char sig[crypto_sign_BYTES];
    unsigned char scalar[crypto_scalarmult_SCALARBYTES];
    unsigned char point[crypto_scalarmult_BYTES];
    unsigned long long sig_len;

    fill(seed, sizeof seed, 13);
    fill(seed2, sizeof seed2, 47);
    fill(scalar, sizeof scalar, 101);

    require(crypto_scalarmult_base(point, scalar) == 0, "scalarmult base");
    record(point, sizeof point);

    require(crypto_sign_seed_keypair(pk, sk, seed) == 0, "sign seed keypair");
    record(pk, crypto_sign_publickeybytes());
    require(crypto_sign_detached(sig, &sig_len, message, sizeof message, sk) == 0,
            "sign detached");
    require(crypto_sign_verify_detached(sig, message, sizeof message, pk) == 0,
            "sign verify");
    record(sig, (size_t) sig_len);

    require(crypto_box_seed_keypair(pk, sk, seed) == 0, "box seed keypair");
    require(crypto_box_easy(ct, message, sizeof message, seed2, pk, sk) == 0,
            "box easy");
    record(ct, sizeof message + crypto_box_macbytes());

    require(crypto_kem_mlkem768_seed_keypair(pk, sk, seed) == 0,
            "mlkem seed keypair");
    record(pk, crypto_kem_mlkem768_PUBLICKEYBYTES);
    require(crypto_kem_mlkem768_enc_deterministic(ct, shared, pk, seed2) == 0,
            "mlkem deterministic encapsulation");
    require(crypto_kem_mlkem768_dec(shared2, ct, sk) == 0,
            "mlkem decapsulation");
    require(memcmp(shared, shared2, crypto_kem_mlkem768_SHAREDSECRETBYTES) == 0,
            "mlkem shared secret");
    record(ct, crypto_kem_mlkem768_CIPHERTEXTBYTES);
    record(shared, crypto_kem_mlkem768_SHAREDSECRETBYTES);

    require(crypto_kem_xwing_seed_keypair(pk, sk, seed2) == 0,
            "xwing seed keypair");
    record(pk, crypto_kem_xwing_PUBLICKEYBYTES);
    require(crypto_kem_xwing_enc_deterministic(ct, shared, pk, seed) == 0,
            "xwing deterministic encapsulation");
    require(crypto_kem_xwing_dec(shared2, ct, sk) == 0, "xwing decapsulation");
    require(memcmp(shared, shared2, crypto_kem_xwing_SHAREDSECRETBYTES) == 0,
            "xwing shared secret");
    record(ct, crypto_kem_xwing_CIPHERTEXTBYTES);
    record(shared, crypto_kem_xwing_SHAREDSECRETBYTES);
}

static void
record_miscellaneous(void)
{
    static const char password[] = "correct horse battery staple";
    unsigned char key[64];
    unsigned char salt[crypto_pwhash_SALTBYTES];
    unsigned char context[crypto_kdf_CONTEXTBYTES] = "DiffTest";
    unsigned char input[32];
    unsigned char output[128];
    unsigned char inverse[32];
    char encoded[256];
    size_t encoded_len;

    fill(key, sizeof key, 5);
    fill(salt, sizeof salt, 17);
    fill(input, sizeof input, 89);

    require(crypto_kdf_derive_from_key(output, 64, 0x1020304050607080ULL,
                                       (const char *) context, key) == 0,
            "kdf");
    record(output, 64);
    require(crypto_pwhash_argon2id(
                output, 32, password, strlen(password), salt,
                crypto_pwhash_argon2id_opslimit_min(),
                crypto_pwhash_argon2id_memlimit_min(),
                crypto_pwhash_argon2id_alg_argon2id13()) == 0,
            "argon2id");
    record(output, 32);

    crypto_ipcrypt_encrypt(output, input, key);
    crypto_ipcrypt_decrypt(inverse, output, key);
    require(memcmp(input, inverse, crypto_ipcrypt_BYTES) == 0, "ipcrypt");
    record(output, crypto_ipcrypt_BYTES);
    crypto_ipcrypt_nd_encrypt(output, input, key, key);
    crypto_ipcrypt_nd_decrypt(inverse, output, key);
    require(memcmp(input, inverse, crypto_ipcrypt_ND_INPUTBYTES) == 0,
            "ipcrypt nd");
    record(output, crypto_ipcrypt_ND_OUTPUTBYTES);
    crypto_ipcrypt_ndx_encrypt(output, input, key, key);
    crypto_ipcrypt_ndx_decrypt(inverse, output, key);
    require(memcmp(input, inverse, crypto_ipcrypt_NDX_INPUTBYTES) == 0,
            "ipcrypt ndx");
    record(output, crypto_ipcrypt_NDX_OUTPUTBYTES);
    crypto_ipcrypt_pfx_encrypt(output, input, key);
    crypto_ipcrypt_pfx_decrypt(inverse, output, key);
    require(memcmp(input, inverse, crypto_ipcrypt_PFX_BYTES) == 0,
            "ipcrypt pfx");
    record(output, crypto_ipcrypt_PFX_BYTES);

    randombytes_buf_deterministic(output, 97, key);
    record(output, 97);
    require(sodium_bin2hex(encoded, sizeof encoded, output, 97) != NULL,
            "bin2hex");
    encoded_len = strlen(encoded);
    record(encoded, encoded_len);
    require(sodium_hex2bin(inverse, sizeof inverse, encoded, 32, NULL,
                           NULL, NULL) == 0,
            "hex2bin");
    record(inverse, 16);

    sodium_increment(input, sizeof input);
    sodium_add(input, key, sizeof input);
    sodium_sub(input, salt, sizeof salt);
    record(input, sizeof input);
}

int
main(void)
{
    unsigned char digest[crypto_hash_sha512_BYTES];
    size_t i;

    require(sodium_init() >= 0, "sodium_init");
    record_crypto_primitives();
    record_public_key_primitives();
    record_miscellaneous();
    require(crypto_hash_sha512(digest, transcript, transcript_len) == 0,
            "transcript digest");

    printf("%s %d.%d %zu\n", sodium_version_string(),
           sodium_library_version_major(), sodium_library_version_minor(),
           transcript_len);
    for (i = 0; i < sizeof digest; i++) {
        printf("%02x", digest[i]);
    }
    putchar('\n');

    return 0;
}
