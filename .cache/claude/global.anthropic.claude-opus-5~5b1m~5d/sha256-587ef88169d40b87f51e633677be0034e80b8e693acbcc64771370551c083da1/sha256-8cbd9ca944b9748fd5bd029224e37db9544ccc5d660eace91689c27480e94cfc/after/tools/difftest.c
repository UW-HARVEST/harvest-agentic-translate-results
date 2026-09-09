/* Differential test harness: link once against the C libsodium.so and once
 * against the Rust libsodium.so, run both, and diff stdout.
 *
 * Every input is deterministic; nothing depends on the system RNG except where
 * explicitly overridden with a fixed randombytes implementation.
 */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>

#include <sodium.h>

static void P(const char *tag, const void *p, size_t n)
{
    size_t i;
    printf("%-42s ", tag);
    for (i = 0; i < n; i++) {
        printf("%02x", ((const unsigned char *) p)[i]);
    }
    printf("\n");
}
static void I(const char *tag, long long v) { printf("%-42s %lld\n", tag, v); }
static void S(const char *tag, const char *v) { printf("%-42s %s\n", tag, v ? v : "(null)"); }

/* ---- deterministic randombytes implementation -------------------------- */
static unsigned char det_state[32] = {
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
};
static uint64_t det_ctr;

static const char *det_name(void) { return "det"; }
static void det_buf(void *buf, size_t size)
{
    unsigned char nonce[8];
    size_t i;
    for (i = 0; i < 8; i++) { nonce[i] = (unsigned char) (det_ctr >> (8 * i)); }
    det_ctr++;
    crypto_stream_salsa20((unsigned char *) buf, (unsigned long long) size, nonce, det_state);
}
static uint32_t det_random(void) { uint32_t r; det_buf(&r, sizeof r); return r; }
static void det_stir(void) { det_ctr = 0; }
static uint32_t det_uniform(const uint32_t upper_bound)
{
    uint32_t min, r;
    if (upper_bound < 2) { return 0; }
    min = (uint32_t) (-upper_bound % upper_bound);
    do { r = det_random(); } while (r < min);
    return r % upper_bound;
}
static int det_close(void) { return 0; }
static randombytes_implementation det_impl = {
    det_name, det_random, det_stir, det_uniform, det_buf, det_close
};

/* ---- fixed test vectors ------------------------------------------------ */
static unsigned char k32[32], k64[64], n24[24], n32[32], msg[256], big[600];

static void mkinputs(void)
{
    size_t i;
    for (i = 0; i < 32; i++) { k32[i] = (unsigned char) (i * 7 + 1); }
    for (i = 0; i < 64; i++) { k64[i] = (unsigned char) (i * 3 + 5); }
    for (i = 0; i < 24; i++) { n24[i] = (unsigned char) (i * 11 + 2); }
    for (i = 0; i < 32; i++) { n32[i] = (unsigned char) (i * 13 + 3); }
    for (i = 0; i < 256; i++) { msg[i] = (unsigned char) (i * 5 + 9); }
    for (i = 0; i < 600; i++) { big[i] = (unsigned char) (i * 17 + 4); }
}

int main(void)
{
    unsigned char out[2048];
    char strbuf[512];
    size_t i;

    mkinputs();
    if (randombytes_set_implementation(&det_impl) != 0) { printf("setimpl failed\n"); }
    if (sodium_init() < 0) { printf("init failed\n"); return 1; }
    randombytes_stir();

    S("version_string", sodium_version_string());
    I("version_major", sodium_library_version_major());
    I("version_minor", sodium_library_version_minor());
    I("library_minimal", sodium_library_minimal());
    I("runtime_sse2", sodium_runtime_has_sse2());
    I("runtime_aesni", sodium_runtime_has_aesni());
    I("runtime_neon", sodium_runtime_has_neon());
    I("runtime_avx2", sodium_runtime_has_avx2());

    /* ---- verify ---- */
    I("verify16_eq", crypto_verify_16(k32, k32));
    I("verify16_ne", crypto_verify_16(k32, k32 + 1));
    I("verify32_eq", crypto_verify_32(k32, k32));
    I("verify64_eq", crypto_verify_64(k64, k64));
    I("verify16_bytes", (long long) crypto_verify_16_bytes());
    I("verify32_bytes", (long long) crypto_verify_32_bytes());
    I("verify64_bytes", (long long) crypto_verify_64_bytes());

    /* ---- utils ---- */
    {
        unsigned char a[16], b[16];
        memcpy(a, k32, 16); memcpy(b, k32, 16);
        I("memcmp", sodium_memcmp(a, b, 16));
        I("compare", sodium_compare(a, b, 16));
        b[0]++;
        I("compare_lt", sodium_compare(a, b, 16));
        I("is_zero", sodium_is_zero(a, 16));
        memset(a, 0, 16);
        I("is_zero_yes", sodium_is_zero(a, 16));
        memcpy(a, k32, 16);
        sodium_increment(a, 16); P("increment", a, 16);
        sodium_add(a, b, 16); P("add", a, 16);
        sodium_sub(a, b, 16); P("sub", a, 16);
    }
    {
        size_t padlen;
        unsigned char buf[64];
        memcpy(buf, msg, 20);
        if (sodium_pad(&padlen, buf, 20, 16, sizeof buf) == 0) {
            I("pad_len", (long long) padlen);
            P("pad", buf, padlen);
            if (sodium_unpad(&padlen, buf, padlen, 16) == 0) { I("unpad_len", (long long) padlen); }
        }
    }
    {
        char hex[129];
        unsigned char bin[64];
        size_t binlen;
        S("bin2hex", sodium_bin2hex(hex, sizeof hex, k64, 64));
        if (sodium_hex2bin(bin, sizeof bin, hex, strlen(hex), NULL, &binlen, NULL) == 0) {
            I("hex2bin_len", (long long) binlen);
            P("hex2bin", bin, binlen);
        }
        I("hex2bin_bad", sodium_hex2bin(bin, sizeof bin, "zz", 2, NULL, &binlen, NULL));
    }
    {
        int variants[4] = { sodium_base64_VARIANT_ORIGINAL,
                            sodium_base64_VARIANT_ORIGINAL_NO_PADDING,
                            sodium_base64_VARIANT_URLSAFE,
                            sodium_base64_VARIANT_URLSAFE_NO_PADDING };
        for (i = 0; i < 4; i++) {
            unsigned char bin[80];
            size_t binlen;
            char tag[64];
            I("b64_encoded_len", (long long) sodium_base64_encoded_len(37, variants[i]));
            sodium_bin2base64(strbuf, sizeof strbuf, k64, 37, variants[i]);
            snprintf(tag, sizeof tag, "bin2base64[%d]", variants[i]);
            S(tag, strbuf);
            if (sodium_base642bin(bin, sizeof bin, strbuf, strlen(strbuf), NULL, &binlen,
                                  NULL, variants[i]) == 0) {
                snprintf(tag, sizeof tag, "base642bin[%d]", variants[i]);
                P(tag, bin, binlen);
            } else {
                snprintf(tag, sizeof tag, "base642bin[%d]", variants[i]);
                S(tag, "FAIL");
            }
        }
    }
    {
        static const char *ips[] = { "127.0.0.1", "1.2.3.4", "::1",
                                     "2001:db8::ff00:42:8329", "::ffff:1.2.3.4",
                                     "bad", "256.1.1.1", "fe80::1%eth0" };
        for (i = 0; i < sizeof ips / sizeof ips[0]; i++) {
            unsigned char ip[16];
            char tag[64];
            int r = sodium_ip2bin(ip, ips[i], strlen(ips[i]));
            snprintf(tag, sizeof tag, "ip2bin[%s]", ips[i]);
            if (r == 0) { P(tag, ip, 16); } else { S(tag, "FAIL"); }
            if (r == 0) {
                if (sodium_bin2ip(strbuf, sizeof strbuf, ip) != NULL) {
                    snprintf(tag, sizeof tag, "bin2ip[%s]", ips[i]);
                    S(tag, strbuf);
                }
            }
        }
    }

    /* ---- hashes ---- */
    for (i = 0; i <= 256; i += 37) {
        char tag[64];
        crypto_hash_sha256(out, msg, i);
        snprintf(tag, sizeof tag, "sha256[%zu]", i); P(tag, out, 32);
        crypto_hash_sha512(out, msg, i);
        snprintf(tag, sizeof tag, "sha512[%zu]", i); P(tag, out, 64);
        crypto_hash(out, msg, i);
        snprintf(tag, sizeof tag, "hash[%zu]", i); P(tag, out, 64);
        crypto_hash_sha3256(out, msg, i);
        snprintf(tag, sizeof tag, "sha3256[%zu]", i); P(tag, out, 32);
        crypto_hash_sha3512(out, msg, i);
        snprintf(tag, sizeof tag, "sha3512[%zu]", i); P(tag, out, 64);
    }
    S("hash_primitive", crypto_hash_primitive());
    {
        crypto_hash_sha256_state st256;
        crypto_hash_sha512_state st512;
        crypto_hash_sha256_init(&st256);
        crypto_hash_sha256_update(&st256, msg, 100);
        crypto_hash_sha256_update(&st256, big, 600);
        crypto_hash_sha256_final(&st256, out); P("sha256_stream", out, 32);
        crypto_hash_sha512_init(&st512);
        crypto_hash_sha512_update(&st512, msg, 100);
        crypto_hash_sha512_update(&st512, big, 600);
        crypto_hash_sha512_final(&st512, out); P("sha512_stream", out, 64);
    }
    {
        crypto_hash_sha3256_state s3;
        crypto_hash_sha3512_state s5;
        crypto_hash_sha3256_init(&s3);
        crypto_hash_sha3256_update(&s3, big, 600);
        crypto_hash_sha3256_final(&s3, out); P("sha3256_stream", out, 32);
        crypto_hash_sha3512_init(&s5);
        crypto_hash_sha3512_update(&s5, big, 600);
        crypto_hash_sha3512_final(&s5, out); P("sha3512_stream", out, 64);
    }

    /* ---- XOF ---- */
    {
        crypto_xof_shake128_state x1;
        crypto_xof_shake256_state x2;
        crypto_xof_turboshake128_state t1;
        crypto_xof_turboshake256_state t2;
        crypto_xof_shake128(out, 200, big, 600); P("shake128", out, 200);
        crypto_xof_shake256(out, 200, big, 600); P("shake256", out, 200);
        crypto_xof_turboshake128(out, 200, big, 600); P("turboshake128", out, 200);
        crypto_xof_turboshake256(out, 200, big, 600); P("turboshake256", out, 200);
        crypto_xof_shake128_init(&x1);
        crypto_xof_shake128_update(&x1, big, 600);
        crypto_xof_shake128_squeeze(&x1, out, 100);
        crypto_xof_shake128_squeeze(&x1, out + 100, 100); P("shake128_stream", out, 200);
        crypto_xof_shake256_init(&x2);
        crypto_xof_shake256_update(&x2, big, 600);
        crypto_xof_shake256_squeeze(&x2, out, 200); P("shake256_stream", out, 200);
        crypto_xof_turboshake128_init_with_domain(&t1, 0x1f);
        crypto_xof_turboshake128_update(&t1, big, 600);
        crypto_xof_turboshake128_squeeze(&t1, out, 200); P("turboshake128_dom", out, 200);
        crypto_xof_turboshake256_init_with_domain(&t2, 0x0b);
        crypto_xof_turboshake256_update(&t2, big, 600);
        crypto_xof_turboshake256_squeeze(&t2, out, 200); P("turboshake256_dom", out, 200);
        I("shake128_blockbytes", (long long) crypto_xof_shake128_blockbytes());
        I("shake256_blockbytes", (long long) crypto_xof_shake256_blockbytes());
        I("ts128_domain", (long long) crypto_xof_turboshake128_domain_standard());
        I("ts256_domain", (long long) crypto_xof_turboshake256_domain_standard());
    }

    /* ---- keccak1600 core ---- */
    {
        crypto_core_keccak1600_state ks;
        crypto_core_keccak1600_init(&ks);
        crypto_core_keccak1600_xor_bytes(&ks, big, 0, 136);
        crypto_core_keccak1600_permute_24(&ks);
        crypto_core_keccak1600_extract_bytes(&ks, out, 0, 200); P("keccak24", out, 200);
        crypto_core_keccak1600_init(&ks);
        crypto_core_keccak1600_xor_bytes(&ks, big, 3, 100);
        crypto_core_keccak1600_permute_12(&ks);
        crypto_core_keccak1600_extract_bytes(&ks, out, 7, 150); P("keccak12", out, 150);
        I("keccak_statebytes", (long long) crypto_core_keccak1600_statebytes());
    }

    /* ---- generichash / blake2b ---- */
    for (i = 16; i <= 64; i += 16) {
        char tag[64];
        crypto_generichash(out, i, big, 600, k32, 32);
        snprintf(tag, sizeof tag, "generichash[%zu]", i); P(tag, out, i);
        crypto_generichash_blake2b_salt_personal(out, i, big, 600, k64, 64, n32, n32 + 16);
        snprintf(tag, sizeof tag, "blake2b_sp[%zu]", i); P(tag, out, i);
    }
    {
        crypto_generichash_state gh;
        crypto_generichash_init(&gh, k32, 32, 32);
        crypto_generichash_update(&gh, msg, 100);
        crypto_generichash_update(&gh, big, 600);
        crypto_generichash_final(&gh, out, 32); P("generichash_stream", out, 32);
    }
    S("generichash_primitive", crypto_generichash_primitive());

    /* ---- shorthash ---- */
    crypto_shorthash(out, big, 600, k32); P("siphash24", out, 8);
    crypto_shorthash_siphashx24(out, big, 600, k32); P("siphashx24", out, 16);
    S("shorthash_primitive", crypto_shorthash_primitive());

    /* ---- auth ---- */
    crypto_auth(out, big, 600, k32); P("auth", out, 32);
    I("auth_verify", crypto_auth_verify(out, big, 600, k32));
    crypto_auth_hmacsha256(out, big, 600, k32); P("hmacsha256", out, 32);
    crypto_auth_hmacsha512(out, big, 600, k32); P("hmacsha512", out, 64);
    crypto_auth_hmacsha512256(out, big, 600, k32); P("hmacsha512256", out, 32);
    {
        crypto_auth_hmacsha256_state h2;
        crypto_auth_hmacsha512_state h5;
        crypto_auth_hmacsha512256_state h6;
        crypto_auth_hmacsha256_init(&h2, k64, 64);
        crypto_auth_hmacsha256_update(&h2, big, 600);
        crypto_auth_hmacsha256_final(&h2, out); P("hmacsha256_stream", out, 32);
        crypto_auth_hmacsha512_init(&h5, k64, 64);
        crypto_auth_hmacsha512_update(&h5, big, 600);
        crypto_auth_hmacsha512_final(&h5, out); P("hmacsha512_stream", out, 64);
        crypto_auth_hmacsha512256_init(&h6, k64, 64);
        crypto_auth_hmacsha512256_update(&h6, big, 600);
        crypto_auth_hmacsha512256_final(&h6, out); P("hmacsha512256_stream", out, 32);
    }
    S("auth_primitive", crypto_auth_primitive());

    /* ---- onetimeauth ---- */
    crypto_onetimeauth(out, big, 600, k32); P("onetimeauth", out, 16);
    I("onetimeauth_verify", crypto_onetimeauth_verify(out, big, 600, k32));
    {
        crypto_onetimeauth_state ot;
        crypto_onetimeauth_init(&ot, k32);
        crypto_onetimeauth_update(&ot, msg, 100);
        crypto_onetimeauth_update(&ot, big, 600);
        crypto_onetimeauth_final(&ot, out); P("onetimeauth_stream", out, 16);
    }
    S("onetimeauth_primitive", crypto_onetimeauth_primitive());

    /* ---- core ---- */
    crypto_core_salsa20(out, n24, k32, NULL); P("core_salsa20", out, 64);
    crypto_core_salsa2012(out, n24, k32, NULL); P("core_salsa2012", out, 64);
    crypto_core_salsa208(out, n24, k32, NULL); P("core_salsa208", out, 64);
    crypto_core_hsalsa20(out, n24, k32, NULL); P("core_hsalsa20", out, 32);
    crypto_core_hchacha20(out, n24, k32, NULL); P("core_hchacha20", out, 32);

    /* ---- streams ---- */
    for (i = 0; i <= 600; i += 131) {
        char tag[64];
        crypto_stream_salsa20(out, i, n24, k32);
        snprintf(tag, sizeof tag, "stream_salsa20[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_salsa20_xor_ic(out, big, i, n24, 5, k32);
        snprintf(tag, sizeof tag, "salsa20_xor_ic[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_chacha20(out, i, n24, k32);
        snprintf(tag, sizeof tag, "stream_chacha20[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_chacha20_xor_ic(out, big, i, n24, 7, k32);
        snprintf(tag, sizeof tag, "chacha20_xor_ic[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_chacha20_ietf(out, i, n24, k32);
        snprintf(tag, sizeof tag, "chacha20_ietf[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_chacha20_ietf_xor_ic(out, big, i, n24, 3, k32);
        snprintf(tag, sizeof tag, "chacha20_ietf_xor_ic[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_xchacha20(out, i, n24, k32);
        snprintf(tag, sizeof tag, "xchacha20[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_xchacha20_xor_ic(out, big, i, n24, 9, k32);
        snprintf(tag, sizeof tag, "xchacha20_xor_ic[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_xsalsa20(out, i, n24, k32);
        snprintf(tag, sizeof tag, "xsalsa20[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_xsalsa20_xor_ic(out, big, i, n24, 2, k32);
        snprintf(tag, sizeof tag, "xsalsa20_xor_ic[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_salsa2012(out, i, n24, k32);
        snprintf(tag, sizeof tag, "salsa2012[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream_salsa208(out, i, n24, k32);
        snprintf(tag, sizeof tag, "salsa208[%zu]", i); P(tag, out, i > 64 ? 64 : i);
        crypto_stream(out, i, n24, k32);
        snprintf(tag, sizeof tag, "stream[%zu]", i); P(tag, out, i > 64 ? 64 : i);
    }
    S("stream_primitive", crypto_stream_primitive());

    /* ---- aead ---- */
    for (i = 0; i <= 256; i += 61) {
        char tag[64];
        unsigned long long clen, mlen;
        unsigned char ct[512], pt[512], mac[64];
        unsigned long long maclen;

        if (crypto_aead_chacha20poly1305_encrypt(ct, &clen, msg, i, big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_ccp[%zu]", i); P(tag, ct, (size_t) clen);
            I("aead_ccp_dec",
              crypto_aead_chacha20poly1305_decrypt(pt, &mlen, NULL, ct, clen, big, 17, n24, k32));
        }
        if (crypto_aead_chacha20poly1305_ietf_encrypt(ct, &clen, msg, i, big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_ccp_ietf[%zu]", i); P(tag, ct, (size_t) clen);
            I("aead_ccp_ietf_dec",
              crypto_aead_chacha20poly1305_ietf_decrypt(pt, &mlen, NULL, ct, clen, big, 17, n24, k32));
        }
        if (crypto_aead_xchacha20poly1305_ietf_encrypt(ct, &clen, msg, i, big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_xccp[%zu]", i); P(tag, ct, (size_t) clen);
            I("aead_xccp_dec",
              crypto_aead_xchacha20poly1305_ietf_decrypt(pt, &mlen, NULL, ct, clen, big, 17, n24, k32));
        }
        if (crypto_aead_aegis128l_encrypt(ct, &clen, msg, i, big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_aegis128l[%zu]", i); P(tag, ct, (size_t) clen);
            I("aead_aegis128l_dec",
              crypto_aead_aegis128l_decrypt(pt, &mlen, NULL, ct, clen, big, 17, n24, k32));
        }
        if (crypto_aead_aegis256_encrypt(ct, &clen, msg, i, big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_aegis256[%zu]", i); P(tag, ct, (size_t) clen);
            I("aead_aegis256_dec",
              crypto_aead_aegis256_decrypt(pt, &mlen, NULL, ct, clen, big, 17, n24, k32));
        }
        if (crypto_aead_chacha20poly1305_ietf_encrypt_detached(ct, mac, &maclen, msg, i,
                                                              big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_ccp_ietf_det[%zu]", i);
            P(tag, ct, i); P("  mac", mac, (size_t) maclen);
        }
        if (crypto_aead_aegis128l_encrypt_detached(ct, mac, &maclen, msg, i,
                                                   big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_aegis128l_det[%zu]", i);
            P(tag, ct, i); P("  mac", mac, (size_t) maclen);
        }
        if (crypto_aead_aegis256_encrypt_detached(ct, mac, &maclen, msg, i,
                                                  big, 17, NULL, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "aead_aegis256_det[%zu]", i);
            P(tag, ct, i); P("  mac", mac, (size_t) maclen);
        }
    }
    I("aes256gcm_is_available", crypto_aead_aes256gcm_is_available());
    {
        unsigned long long clen;
        unsigned char ct[512];
        errno = 0;
        I("aes256gcm_encrypt",
          crypto_aead_aes256gcm_encrypt(ct, &clen, msg, 32, big, 17, NULL, n24, k32));
        I("aes256gcm_errno", errno);
        I("aes256gcm_abytes", (long long) crypto_aead_aes256gcm_abytes());
        I("aes256gcm_keybytes", (long long) crypto_aead_aes256gcm_keybytes());
        I("aes256gcm_statebytes", (long long) crypto_aead_aes256gcm_statebytes());
    }

    /* ---- secretbox ---- */
    for (i = 0; i <= 256; i += 61) {
        char tag[64];
        unsigned char ct[512], pt[512], mac[64];
        if (crypto_secretbox_easy(ct, msg, i, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "secretbox_easy[%zu]", i);
            P(tag, ct, i + crypto_secretbox_MACBYTES);
            I("secretbox_open", crypto_secretbox_open_easy(pt, ct, i + crypto_secretbox_MACBYTES, n24, k32));
        }
        if (crypto_secretbox_detached(ct, mac, msg, i, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "secretbox_det[%zu]", i);
            P(tag, ct, i); P("  mac", mac, 16);
        }
        if (crypto_secretbox_xchacha20poly1305_easy(ct, msg, i, n24, k32) == 0) {
            snprintf(tag, sizeof tag, "secretbox_xccp[%zu]", i);
            P(tag, ct, i + 16);
        }
    }
    {
        unsigned char m[288], c[288];
        memset(m, 0, crypto_secretbox_ZEROBYTES);
        memcpy(m + crypto_secretbox_ZEROBYTES, msg, 256);
        if (crypto_secretbox(c, m, sizeof m, n24, k32) == 0) { P("secretbox_nacl", c, sizeof c); }
        I("secretbox_nacl_open", crypto_secretbox_open(m, c, sizeof c, n24, k32));
    }
    S("secretbox_primitive", crypto_secretbox_primitive());

    /* ---- secretstream ---- */
    {
        crypto_secretstream_xchacha20poly1305_state ss;
        unsigned char header[crypto_secretstream_xchacha20poly1305_HEADERBYTES];
        unsigned char ct[512];
        unsigned long long clen;
        unsigned char tag;
        unsigned long long mlen;
        crypto_secretstream_xchacha20poly1305_init_push(&ss, header, k32);
        P("ss_header", header, sizeof header);
        crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, 100, big, 5, 0);
        P("ss_push1", ct, (size_t) clen);
        crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, 50, NULL, 0,
            crypto_secretstream_xchacha20poly1305_TAG_REKEY);
        P("ss_push2", ct, (size_t) clen);
        crypto_secretstream_xchacha20poly1305_rekey(&ss);
        crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, 10, NULL, 0,
            crypto_secretstream_xchacha20poly1305_TAG_FINAL);
        P("ss_push3", ct, (size_t) clen);

        crypto_secretstream_xchacha20poly1305_init_push(&ss, header, k32);
        crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, 100, big, 5, 0);
        crypto_secretstream_xchacha20poly1305_init_pull(&ss, header, k32);
        I("ss_pull", crypto_secretstream_xchacha20poly1305_pull(&ss, out, &mlen, &tag,
                                                                ct, clen, big, 5));
        I("ss_pull_tag", tag);
        P("ss_pull_msg", out, (size_t) mlen);
        I("ss_tag_message", crypto_secretstream_xchacha20poly1305_tag_message());
        I("ss_tag_push", crypto_secretstream_xchacha20poly1305_tag_push());
        I("ss_tag_rekey", crypto_secretstream_xchacha20poly1305_tag_rekey());
        I("ss_tag_final", crypto_secretstream_xchacha20poly1305_tag_final());
    }

    /* ---- scalarmult ---- */
    {
        unsigned char q[32];
        I("scalarmult_base", crypto_scalarmult_base(q, k32)); P("scalarmult_base", q, 32);
        I("scalarmult", crypto_scalarmult(out, k32, q)); P("scalarmult", out, 32);
        I("scalarmult_c25519_base", crypto_scalarmult_curve25519_base(q, k32));
        P("sm_c25519_base", q, 32);
        I("sm_ed25519_base", crypto_scalarmult_ed25519_base(out, k32)); P("sm_ed25519_base", out, 32);
        I("sm_ed25519_base_nc", crypto_scalarmult_ed25519_base_noclamp(out, k32));
        P("sm_ed25519_base_nc", out, 32);
        {
            unsigned char p[32];
            crypto_scalarmult_ed25519_base(p, k32);
            I("sm_ed25519", crypto_scalarmult_ed25519(out, k32, p)); P("sm_ed25519", out, 32);
            I("sm_ed25519_nc", crypto_scalarmult_ed25519_noclamp(out, k32, p));
            P("sm_ed25519_nc", out, 32);
        }
        I("sm_ristretto_base", crypto_scalarmult_ristretto255_base(out, k32));
        P("sm_ristretto_base", out, 32);
        {
            unsigned char p[32];
            crypto_scalarmult_ristretto255_base(p, k32);
            I("sm_ristretto", crypto_scalarmult_ristretto255(out, k32, p));
            P("sm_ristretto", out, 32);
        }
        S("scalarmult_primitive", crypto_scalarmult_primitive());
    }

    /* ---- core_ed25519 / ristretto255 ---- */
    {
        unsigned char p[32], q[32], r[32], s[64];
        crypto_scalarmult_ed25519_base(p, k32);
        crypto_scalarmult_ed25519_base(q, k64);
        I("ed25519_is_valid", crypto_core_ed25519_is_valid_point(p));
        I("ed25519_add", crypto_core_ed25519_add(r, p, q)); P("ed25519_add", r, 32);
        I("ed25519_sub", crypto_core_ed25519_sub(r, p, q)); P("ed25519_sub", r, 32);
        memcpy(s, k64, 64);
        I("ed25519_from_string",
          crypto_core_ed25519_from_string(r, (const unsigned char *) "ctx", 3, msg, 20, 1));
        P("ed25519_from_string", r, 32);
        I("ed25519_from_string_nu",
          crypto_core_ed25519_from_string_nu(r, (const unsigned char *) "ctx", 3, msg, 20, 1));
        P("ed25519_from_string_nu", r, 32);
        I("ed25519_sc_from_string",
          crypto_core_ed25519_scalar_from_string(r, (const unsigned char *) "ctx", 3, msg, 20, 1));
        P("ed25519_sc_from_string", r, 32);
        crypto_core_ed25519_scalar_reduce(r, k64); P("ed25519_sc_reduce", r, 32);
        crypto_core_ed25519_scalar_add(r, k32, n32); P("ed25519_sc_add", r, 32);
        crypto_core_ed25519_scalar_sub(r, k32, n32); P("ed25519_sc_sub", r, 32);
        crypto_core_ed25519_scalar_mul(r, k32, n32); P("ed25519_sc_mul", r, 32);
        crypto_core_ed25519_scalar_negate(r, k32); P("ed25519_sc_negate", r, 32);
        crypto_core_ed25519_scalar_complement(r, k32); P("ed25519_sc_complement", r, 32);
        {
            unsigned char sc[32];
            memcpy(sc, k32, 32); sc[31] &= 15;
            I("ed25519_sc_invert", crypto_core_ed25519_scalar_invert(r, sc));
            P("ed25519_sc_invert", r, 32);
            I("ed25519_sc_is_canonical", crypto_core_ed25519_scalar_is_canonical(sc));
        }
        I("ristretto_is_valid", crypto_core_ristretto255_is_valid_point(p));
        I("ristretto_from_hash", crypto_core_ristretto255_from_hash(r, k64));
        P("ristretto_from_hash", r, 32);
        {
            unsigned char a[32], b[32];
            crypto_core_ristretto255_from_hash(a, k64);
            crypto_core_ristretto255_from_hash(b, n32); /* only 32 bytes; still deterministic */
            I("ristretto_add", crypto_core_ristretto255_add(r, a, a)); P("ristretto_add", r, 32);
            I("ristretto_sub", crypto_core_ristretto255_sub(r, a, a)); P("ristretto_sub", r, 32);
        }
        crypto_core_ristretto255_scalar_reduce(r, k64); P("ristretto_sc_reduce", r, 32);
        crypto_core_ristretto255_scalar_add(r, k32, n32); P("ristretto_sc_add", r, 32);
        crypto_core_ristretto255_scalar_mul(r, k32, n32); P("ristretto_sc_mul", r, 32);
        I("ristretto_from_string",
          crypto_core_ristretto255_from_string(r, (const unsigned char *) "ctx", 3, msg, 20, 1));
        P("ristretto_from_string", r, 32);
        I("ristretto_sc_from_string",
          crypto_core_ristretto255_scalar_from_string(r, (const unsigned char *) "ctx", 3, msg, 20, 1));
        P("ristretto_sc_from_string", r, 32);
        I("ed25519_bytes", (long long) crypto_core_ed25519_bytes());
        I("ristretto255_bytes", (long long) crypto_core_ristretto255_bytes());
    }

    /* ---- sign ---- */
    {
        unsigned char pk[32], sk[64], sig[64];
        unsigned long long siglen, smlen, mlen2;
        unsigned char sm[512], m2[512];
        I("sign_seed_keypair", crypto_sign_seed_keypair(pk, sk, k32));
        P("sign_pk", pk, 32); P("sign_sk", sk, 64);
        I("sign_detached", crypto_sign_detached(sig, &siglen, msg, 100, sk));
        P("sign_sig", sig, 64);
        I("sign_verify", crypto_sign_verify_detached(sig, msg, 100, pk));
        I("sign", crypto_sign(sm, &smlen, msg, 100, sk));
        P("sign_sm", sm, (size_t) smlen);
        I("sign_open", crypto_sign_open(m2, &mlen2, sm, smlen, pk));
        I("sign_sk_to_pk", crypto_sign_ed25519_sk_to_pk(out, sk)); P("sk_to_pk", out, 32);
        I("sign_sk_to_seed", crypto_sign_ed25519_sk_to_seed(out, sk)); P("sk_to_seed", out, 32);
        I("sign_pk_to_c25519", crypto_sign_ed25519_pk_to_curve25519(out, pk));
        P("pk_to_c25519", out, 32);
        I("sign_sk_to_c25519", crypto_sign_ed25519_sk_to_curve25519(out, sk));
        P("sk_to_c25519", out, 32);
        {
            crypto_sign_state st;
            crypto_sign_init(&st);
            crypto_sign_update(&st, msg, 100);
            crypto_sign_update(&st, big, 600);
            I("sign_final_create", crypto_sign_final_create(&st, sig, &siglen, sk));
            P("sign_ph_sig", sig, 64);
            crypto_sign_init(&st);
            crypto_sign_update(&st, msg, 100);
            crypto_sign_update(&st, big, 600);
            I("sign_final_verify", crypto_sign_final_verify(&st, sig, pk));
        }
        S("sign_primitive", crypto_sign_primitive());
    }

    /* ---- box ---- */
    {
        unsigned char pk1[32], sk1[32], pk2[32], sk2[32], ct[512], pt[512], nm[32], mac[32];
        I("box_seed_keypair", crypto_box_seed_keypair(pk1, sk1, k32));
        P("box_pk1", pk1, 32); P("box_sk1", sk1, 32);
        I("box_seed_keypair2", crypto_box_seed_keypair(pk2, sk2, n32));
        I("box_easy", crypto_box_easy(ct, msg, 100, n24, pk2, sk1));
        P("box_easy", ct, 100 + 16);
        I("box_open_easy", crypto_box_open_easy(pt, ct, 116, n24, pk1, sk2));
        I("box_beforenm", crypto_box_beforenm(nm, pk2, sk1)); P("box_nm", nm, 32);
        I("box_easy_afternm", crypto_box_easy_afternm(ct, msg, 100, n24, nm));
        P("box_easy_afternm", ct, 116);
        I("box_detached", crypto_box_detached(ct, mac, msg, 100, n24, pk2, sk1));
        P("box_det", ct, 100); P("box_det_mac", mac, 16);
        I("box_seal", crypto_box_seal(ct, msg, 100, pk1));
        P("box_seal", ct, 100 + crypto_box_SEALBYTES);
        I("box_seal_open", crypto_box_seal_open(pt, ct, 100 + crypto_box_SEALBYTES, pk1, sk1));
        I("box_c25519xccp_seal", crypto_box_curve25519xchacha20poly1305_seal(ct, msg, 100, pk1));
        P("box_xccp_seal", ct, 100 + crypto_box_curve25519xchacha20poly1305_SEALBYTES);
        {
            unsigned char m[288], c[288];
            memset(m, 0, crypto_box_ZEROBYTES);
            memcpy(m + crypto_box_ZEROBYTES, msg, 256);
            I("box_nacl", crypto_box(c, m, sizeof m, n24, pk2, sk1)); P("box_nacl", c, sizeof c);
            I("box_nacl_open", crypto_box_open(m, c, sizeof c, n24, pk1, sk2));
        }
        S("box_primitive", crypto_box_primitive());
    }

    /* ---- kx ---- */
    {
        unsigned char cpk[32], csk[32], spk[32], ssk[32], rx[32], tx[32];
        I("kx_seed_keypair", crypto_kx_seed_keypair(cpk, csk, k32));
        P("kx_cpk", cpk, 32); P("kx_csk", csk, 32);
        crypto_kx_seed_keypair(spk, ssk, n32);
        I("kx_client", crypto_kx_client_session_keys(rx, tx, cpk, csk, spk));
        P("kx_client_rx", rx, 32); P("kx_client_tx", tx, 32);
        I("kx_server", crypto_kx_server_session_keys(rx, tx, spk, ssk, cpk));
        P("kx_server_rx", rx, 32); P("kx_server_tx", tx, 32);
        S("kx_primitive", crypto_kx_primitive());
    }

    /* ---- kdf ---- */
    {
        unsigned char sub[64];
        I("kdf_derive", crypto_kdf_derive_from_key(sub, 32, 12345, "context8", k32));
        P("kdf_derive", sub, 32);
        I("kdf_blake2b", crypto_kdf_blake2b_derive_from_key(sub, 64, 7, "ctx12345", k32));
        P("kdf_blake2b", sub, 64);
        S("kdf_primitive", crypto_kdf_primitive());
        I("hkdf256_extract", crypto_kdf_hkdf_sha256_extract(out, n32, 32, big, 600));
        P("hkdf256_prk", out, 32);
        I("hkdf256_expand", crypto_kdf_hkdf_sha256_expand(sub, 64, "info", 4, out));
        P("hkdf256_okm", sub, 64);
        I("hkdf512_extract", crypto_kdf_hkdf_sha512_extract(out, n32, 32, big, 600));
        P("hkdf512_prk", out, 64);
        I("hkdf512_expand", crypto_kdf_hkdf_sha512_expand(sub, 64, "info", 4, out));
        P("hkdf512_okm", sub, 64);
        {
            crypto_kdf_hkdf_sha256_state s2;
            crypto_kdf_hkdf_sha512_state s5;
            crypto_kdf_hkdf_sha256_extract_init(&s2, n32, 32);
            crypto_kdf_hkdf_sha256_extract_update(&s2, big, 600);
            crypto_kdf_hkdf_sha256_extract_final(&s2, out); P("hkdf256_stream", out, 32);
            crypto_kdf_hkdf_sha512_extract_init(&s5, n32, 32);
            crypto_kdf_hkdf_sha512_extract_update(&s5, big, 600);
            crypto_kdf_hkdf_sha512_extract_final(&s5, out); P("hkdf512_stream", out, 64);
        }
    }

    /* ---- pwhash ---- */
    {
        char str[crypto_pwhash_STRBYTES];
        I("pwhash_i",
          crypto_pwhash(out, 64, "password", 8, n32,
                        crypto_pwhash_argon2i_OPSLIMIT_MIN, 16384,
                        crypto_pwhash_ALG_ARGON2I13));
        P("pwhash_i", out, 64);
        I("pwhash_id",
          crypto_pwhash(out, 64, "password", 8, n32,
                        crypto_pwhash_argon2id_OPSLIMIT_MIN, 16384,
                        crypto_pwhash_ALG_ARGON2ID13));
        P("pwhash_id", out, 64);
        I("pwhash_str",
          crypto_pwhash_str_alg(str, "password", 8, crypto_pwhash_argon2i_OPSLIMIT_MIN,
                                16384, crypto_pwhash_ALG_ARGON2I13));
        S("pwhash_str", str);
        I("pwhash_str_verify", crypto_pwhash_str_verify(str, "password", 8));
        I("pwhash_str_verify_bad", crypto_pwhash_str_verify(str, "wrong", 5));
        I("pwhash_needs_rehash",
          crypto_pwhash_str_needs_rehash(str, crypto_pwhash_argon2i_OPSLIMIT_MIN, 16384));
        I("pwhash_needs_rehash2",
          crypto_pwhash_str_needs_rehash(str, crypto_pwhash_OPSLIMIT_MODERATE, 1 << 26));
        I("pwhash_argon2i",
          crypto_pwhash_argon2i(out, 64, "password", 8, n32, 3, 16384, crypto_pwhash_argon2i_ALG_ARGON2I13));
        P("pwhash_argon2i", out, 64);
        I("pwhash_argon2id",
          crypto_pwhash_argon2id(out, 64, "password", 8, n32, 3, 16384, crypto_pwhash_argon2id_ALG_ARGON2ID13));
        P("pwhash_argon2id", out, 64);
        S("pwhash_primitive", crypto_pwhash_primitive());
        S("pwhash_strprefix", crypto_pwhash_strprefix());
        S("pwhash_argon2i_strprefix", crypto_pwhash_argon2i_strprefix());
        S("pwhash_argon2id_strprefix", crypto_pwhash_argon2id_strprefix());
        I("pwhash_alg_default", crypto_pwhash_alg_default());
        I("pwhash_opslimit_min", (long long) crypto_pwhash_opslimit_min());
        I("pwhash_memlimit_min", (long long) crypto_pwhash_memlimit_min());
        I("pwhash_bytes_min", (long long) crypto_pwhash_bytes_min());
        I("pwhash_bytes_max", (long long) crypto_pwhash_bytes_max());
        I("pwhash_saltbytes", (long long) crypto_pwhash_saltbytes());
        I("pwhash_strbytes", (long long) crypto_pwhash_strbytes());
        /* error paths */
        I("pwhash_badalg", crypto_pwhash(out, 64, "p", 1, n32, 3, 16384, 99));
        I("pwhash_lowmem", crypto_pwhash(out, 64, "p", 1, n32, 3, 100,
                                         crypto_pwhash_ALG_ARGON2I13));
        I("pwhash_shortout", crypto_pwhash(out, 8, "p", 1, n32, 3, 16384,
                                           crypto_pwhash_ALG_ARGON2I13));
    }

    /* ---- scrypt ---- */
    {
        char str[crypto_pwhash_scryptsalsa208sha256_STRBYTES];
        I("scrypt",
          crypto_pwhash_scryptsalsa208sha256(out, 64, "password", 8, n32, 16384, 1 << 20));
        P("scrypt", out, 64);
        I("scrypt_ll",
          crypto_pwhash_scryptsalsa208sha256_ll((const uint8_t *) "password", 8,
                                                n32, 32, 1024, 8, 1, out, 64));
        P("scrypt_ll", out, 64);
        I("scrypt_str",
          crypto_pwhash_scryptsalsa208sha256_str(str, "password", 8, 16384, 1 << 20));
        S("scrypt_str", str);
        I("scrypt_str_verify",
          crypto_pwhash_scryptsalsa208sha256_str_verify(str, "password", 8));
        I("scrypt_needs_rehash",
          crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(str, 16384, 1 << 20));
        I("scrypt_bytes_min", (long long) crypto_pwhash_scryptsalsa208sha256_bytes_min());
        I("scrypt_strbytes", (long long) crypto_pwhash_scryptsalsa208sha256_strbytes());
        S("scrypt_strprefix", crypto_pwhash_scryptsalsa208sha256_strprefix());
    }

    /* ---- kem ---- */
    {
        static unsigned char pk[3000], sk[3000], ct[3000], ss1[64], ss2[64];
        I("mlkem_seed_keypair", crypto_kem_mlkem768_seed_keypair(pk, sk, k64));
        P("mlkem_pk_hash_in", pk, 64);
        crypto_hash_sha256(out, pk, crypto_kem_mlkem768_PUBLICKEYBYTES);
        P("mlkem_pk_sha256", out, 32);
        crypto_hash_sha256(out, sk, crypto_kem_mlkem768_SECRETKEYBYTES);
        P("mlkem_sk_sha256", out, 32);
        I("mlkem_enc_det", crypto_kem_mlkem768_enc_deterministic(ct, ss1, pk, k32));
        crypto_hash_sha256(out, ct, crypto_kem_mlkem768_CIPHERTEXTBYTES);
        P("mlkem_ct_sha256", out, 32);
        P("mlkem_ss1", ss1, crypto_kem_mlkem768_SHAREDSECRETBYTES);
        I("mlkem_dec", crypto_kem_mlkem768_dec(ss2, ct, sk));
        P("mlkem_ss2", ss2, crypto_kem_mlkem768_SHAREDSECRETBYTES);
        /* corrupted ciphertext -> implicit rejection */
        ct[0] ^= 0xff;
        I("mlkem_dec_bad", crypto_kem_mlkem768_dec(ss2, ct, sk));
        P("mlkem_ss_bad", ss2, crypto_kem_mlkem768_SHAREDSECRETBYTES);
        ct[0] ^= 0xff;

        I("xwing_seed_keypair", crypto_kem_xwing_seed_keypair(pk, sk, k32));
        crypto_hash_sha256(out, pk, crypto_kem_xwing_PUBLICKEYBYTES);
        P("xwing_pk_sha256", out, 32);
        crypto_hash_sha256(out, sk, crypto_kem_xwing_SECRETKEYBYTES);
        P("xwing_sk_sha256", out, 32);
        I("xwing_enc_det", crypto_kem_xwing_enc_deterministic(ct, ss1, pk, k64));
        crypto_hash_sha256(out, ct, crypto_kem_xwing_CIPHERTEXTBYTES);
        P("xwing_ct_sha256", out, 32);
        P("xwing_ss1", ss1, crypto_kem_xwing_SHAREDSECRETBYTES);
        I("xwing_dec", crypto_kem_xwing_dec(ss2, ct, sk));
        P("xwing_ss2", ss2, crypto_kem_xwing_SHAREDSECRETBYTES);
        S("kem_primitive", crypto_kem_primitive());
        I("kem_publickeybytes", (long long) crypto_kem_publickeybytes());
    }

    /* ---- ipcrypt ---- */
    {
        unsigned char ip[16], enc[32], dec[16];
        memcpy(ip, k32, 16);
        crypto_ipcrypt_encrypt(enc, ip, k32); P("ipcrypt_enc", enc, 16);
        crypto_ipcrypt_decrypt(dec, enc, k32); P("ipcrypt_dec", dec, 16);
        crypto_ipcrypt_nd_encrypt(enc, ip, k32, n24); P("ipcrypt_nd_enc", enc,
            crypto_ipcrypt_ND_OUTPUTBYTES);
        crypto_ipcrypt_nd_decrypt(dec, enc, k32); P("ipcrypt_nd_dec", dec, 16);
        crypto_ipcrypt_ndx_encrypt(enc, ip, k32, n24); P("ipcrypt_ndx_enc", enc,
            crypto_ipcrypt_NDX_OUTPUTBYTES);
        crypto_ipcrypt_ndx_decrypt(dec, enc, k32); P("ipcrypt_ndx_dec", dec, 16);
        crypto_ipcrypt_pfx_encrypt(enc, ip, k32); P("ipcrypt_pfx_enc", enc, 16);
        crypto_ipcrypt_pfx_decrypt(dec, enc, k32); P("ipcrypt_pfx_dec", dec, 16);
        I("ipcrypt_bytes", (long long) crypto_ipcrypt_bytes());
        I("ipcrypt_keybytes", (long long) crypto_ipcrypt_keybytes());
        I("ipcrypt_nd_keybytes", (long long) crypto_ipcrypt_nd_keybytes());
        I("ipcrypt_ndx_keybytes", (long long) crypto_ipcrypt_ndx_keybytes());
        I("ipcrypt_pfx_keybytes", (long long) crypto_ipcrypt_pfx_keybytes());
    }

    /* ---- randombytes (deterministic impl) ---- */
    {
        unsigned char buf[64];
        S("randombytes_impl_name", randombytes_implementation_name());
        randombytes_stir();
        randombytes_buf(buf, sizeof buf); P("randombytes_buf", buf, sizeof buf);
        I("randombytes_random", (long long) randombytes_random());
        I("randombytes_uniform", (long long) randombytes_uniform(1000));
        I("randombytes_seedbytes", (long long) randombytes_seedbytes());
        randombytes_buf_deterministic(buf, sizeof buf, n32);
        P("randombytes_det", buf, sizeof buf);
    }

    printf("DONE\n");
    return 0;
}
