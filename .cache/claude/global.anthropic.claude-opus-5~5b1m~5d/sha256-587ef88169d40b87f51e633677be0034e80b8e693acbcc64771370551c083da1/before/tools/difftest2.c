/* Differential test harness, phase 2: error paths, edge cases, allocator,
 * streaming APIs, and the less-travelled entry points. */
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>

#include <sodium.h>

static void P(const char *tag, const void *p, size_t n)
{
    size_t i;
    printf("%-46s ", tag);
    for (i = 0; i < n; i++) { printf("%02x", ((const unsigned char *) p)[i]); }
    printf("\n");
}
static void I(const char *tag, long long v) { printf("%-46s %lld\n", tag, v); }
static void IE(const char *tag, long long v, int e) { printf("%-46s %lld errno=%d\n", tag, v, e); }
static void S(const char *tag, const char *v) { printf("%-46s %s\n", tag, v ? v : "(null)"); }

/* deterministic randombytes */
static unsigned char det_state[32];
static uint64_t det_ctr;
static const char *det_name(void) { return "det"; }
static void det_buf(void *buf, size_t size)
{
    unsigned char nonce[8]; size_t i;
    for (i = 0; i < 8; i++) { nonce[i] = (unsigned char) (det_ctr >> (8 * i)); }
    det_ctr++;
    crypto_stream_salsa20((unsigned char *) buf, (unsigned long long) size, nonce, det_state);
}
static uint32_t det_random(void) { uint32_t r; det_buf(&r, sizeof r); return r; }
static void det_stir(void) { det_ctr = 0; }
static uint32_t det_uniform(const uint32_t ub)
{
    uint32_t min, r;
    if (ub < 2) { return 0; }
    min = (uint32_t) (-ub % ub);
    do { r = det_random(); } while (r < min);
    return r % ub;
}
static int det_close(void) { return 0; }
static randombytes_implementation det_impl = {
    det_name, det_random, det_stir, det_uniform, det_buf, det_close
};

static unsigned char k32[32], k64[64], n24[24], msg[300];

int main(void)
{
    unsigned char out[65536];
    char strbuf[600];
    size_t i;

    for (i = 0; i < 32; i++) { det_state[i] = (unsigned char) (i + 1); }
    for (i = 0; i < 32; i++) { k32[i] = (unsigned char) (i * 7 + 1); }
    for (i = 0; i < 64; i++) { k64[i] = (unsigned char) (i * 3 + 5); }
    for (i = 0; i < 24; i++) { n24[i] = (unsigned char) (i * 11 + 2); }
    for (i = 0; i < 300; i++) { msg[i] = (unsigned char) (i * 5 + 9); }

    randombytes_set_implementation(&det_impl);
    if (sodium_init() < 0) { printf("init failed\n"); return 1; }
    randombytes_stir();

    /* ---------- allocator ---------- */
    {
        void *p;
        errno = 0;
        p = sodium_malloc(100);
        I("malloc_nonnull", p != NULL);
        if (p != NULL) {
            memset(p, 7, 100);
            errno = 0; IE("mprotect_noaccess", sodium_mprotect_noaccess(p), errno);
            errno = 0; IE("mprotect_readonly", sodium_mprotect_readonly(p), errno);
            errno = 0; IE("mprotect_readwrite", sodium_mprotect_readwrite(p), errno);
            sodium_free(p);
        }
        errno = 0;
        p = sodium_allocarray(10, 20);
        I("allocarray_nonnull", p != NULL);
        sodium_free(p);
        errno = 0;
        p = sodium_allocarray((size_t) -1, 20);
        IE("allocarray_overflow_null", p == NULL, errno);
        sodium_free(NULL);
        I("free_null_ok", 1);
        errno = 0; IE("mlock", sodium_mlock(k32, 32), errno);
        errno = 0; IE("munlock", sodium_munlock(k32, 32), errno);
        sodium_stackzero(64);
        I("stackzero_ok", 1);
    }

    /* ---------- pad/unpad edge cases ---------- */
    {
        size_t pl;
        unsigned char b[80];
        memcpy(b, msg, 16);
        errno = 0; IE("pad_bs0", sodium_pad(&pl, b, 16, 0, sizeof b), errno);
        errno = 0; IE("pad_bs1", sodium_pad(&pl, b, 16, 1, sizeof b), errno);
        if (errno == 0) { I("pad_bs1_len", (long long) pl); }
        errno = 0; IE("pad_toosmall", sodium_pad(&pl, b, 16, 16, 16), errno);
        memcpy(b, msg, 16);
        sodium_pad(&pl, b, 16, 16, sizeof b);
        errno = 0; IE("unpad_ok", sodium_unpad(&pl, b, pl, 16), errno);
        I("unpad_len", (long long) pl);
        errno = 0; IE("unpad_bs0", sodium_unpad(&pl, b, 32, 0), errno);
        errno = 0; IE("unpad_short", sodium_unpad(&pl, b, 8, 16), errno);
        memset(b, 0, sizeof b);
        errno = 0; IE("unpad_nopad", sodium_unpad(&pl, b, 32, 16), errno);
        for (i = 0; i <= 40; i++) {
            unsigned char c[128];
            char tag[64];
            size_t l;
            memcpy(c, msg, i);
            if (sodium_pad(&l, c, i, 8, sizeof c) == 0) {
                snprintf(tag, sizeof tag, "pad8[%zu]", i);
                P(tag, c, l);
            }
        }
        for (i = 0; i < 32; i++) {
            char tag[64];
            snprintf(tag, sizeof tag, "cmp0[%zu]", i);
            I(tag, sodium_compare(msg, msg + 1, i));
        }
    }

    /* ---------- hex/base64 error paths ---------- */
    {
        unsigned char bin[64];
        size_t binlen;
        const char *hexend;
        errno = 0;
        I("hex2bin_ignore",
          sodium_hex2bin(bin, sizeof bin, "de:ad:be:ef", 11, ":", &binlen, &hexend));
        I("hex2bin_ignore_len", (long long) binlen);
        P("hex2bin_ignore_bin", bin, binlen);
        errno = 0;
        IE("hex2bin_overflow", sodium_hex2bin(bin, 2, "deadbeef", 8, NULL, &binlen, NULL), errno);
        errno = 0;
        IE("hex2bin_odd", sodium_hex2bin(bin, sizeof bin, "abc", 3, NULL, &binlen, &hexend), errno);
        I("hex2bin_odd_len", (long long) binlen);
        {
            char small[4];
            S("bin2hex_small", sodium_bin2hex(small, sizeof small, k32, 1));
        }
        errno = 0;
        IE("b642bin_badchar",
           sodium_base642bin(bin, sizeof bin, "AB*D", 4, NULL, &binlen, NULL,
                             sodium_base64_VARIANT_ORIGINAL), errno);
        errno = 0;
        I("b642bin_ignore",
          sodium_base642bin(bin, sizeof bin, "QUJD\nRUZH", 9, "\n", &binlen, NULL,
                            sodium_base64_VARIANT_ORIGINAL));
        I("b642bin_ignore_len", (long long) binlen);
        P("b642bin_ignore_bin", bin, binlen);
        errno = 0;
        IE("b642bin_nopad_with_pad",
           sodium_base642bin(bin, sizeof bin, "QQ==", 4, NULL, &binlen, NULL,
                             sodium_base64_VARIANT_ORIGINAL_NO_PADDING), errno);
        for (i = 0; i <= 20; i++) {
            char tag[64];
            sodium_bin2base64(strbuf, sizeof strbuf, k64, i, sodium_base64_VARIANT_URLSAFE);
            snprintf(tag, sizeof tag, "b64u[%zu]", i); S(tag, strbuf);
            sodium_bin2base64(strbuf, sizeof strbuf, k64, i,
                              sodium_base64_VARIANT_ORIGINAL_NO_PADDING);
            snprintf(tag, sizeof tag, "b64onp[%zu]", i); S(tag, strbuf);
        }
    }

    /* ---------- ip codecs ---------- */
    {
        static const char *ips[] = {
            "0.0.0.0", "255.255.255.255", "::", "::0", "1::", "1:2:3:4:5:6:7:8",
            "1:2:3:4:5:6:7:8:9", "1::2::3", "::ffff:0.0.0.0", "::ffff:255.255.255.255",
            "0:0:0:0:0:0:0:1", "fe80::1%1234567890", "1.2.3", "1.2.3.4.5", "",
            "  1.2.3.4", "1.2.3.4 ", "0x1.2.3.4", "01.2.3.4", "1:2:3:4:5:6:1.2.3.4"
        };
        for (i = 0; i < sizeof ips / sizeof ips[0]; i++) {
            unsigned char ip[16];
            char tag[80];
            int r = sodium_ip2bin(ip, ips[i], strlen(ips[i]));
            snprintf(tag, sizeof tag, "ip2bin2[%s]", ips[i]);
            if (r == 0) { P(tag, ip, 16); } else { S(tag, "FAIL"); }
            if (r == 0) {
                char o[64];
                snprintf(tag, sizeof tag, "bin2ip2[%s]", ips[i]);
                S(tag, sodium_bin2ip(o, sizeof o, ip));
                snprintf(tag, sizeof tag, "bin2ip2small[%s]", ips[i]);
                S(tag, sodium_bin2ip(o, 4, ip));
            }
        }
    }

    /* ---------- generichash bounds ---------- */
    {
        crypto_generichash_state st;
        I("gh_out_too_small", crypto_generichash(out, 15, msg, 100, NULL, 0));
        I("gh_out_too_big", crypto_generichash(out, 65, msg, 100, NULL, 0));
        I("gh_key_too_big", crypto_generichash(out, 32, msg, 100, k64, 65));
        I("gh_nokey", crypto_generichash(out, 32, msg, 100, NULL, 0));
        P("gh_nokey_out", out, 32);
        I("gh_init_bad", crypto_generichash_init(&st, NULL, 0, 65));
        for (i = 16; i <= 64; i++) {
            char tag[64];
            crypto_generichash(out, i, msg, 300, k32, 32);
            snprintf(tag, sizeof tag, "gh[%zu]", i); P(tag, out, i);
        }
        for (i = 0; i <= 64; i += 8) {
            char tag[64];
            crypto_generichash(out, 32, msg, 300, k64, i);
            snprintf(tag, sizeof tag, "ghk[%zu]", i); P(tag, out, 32);
        }
        crypto_generichash_blake2b_init_salt_personal(&st, k32, 32, 48, k32, k32 + 16);
        crypto_generichash_blake2b_update(&st, msg, 300);
        crypto_generichash_blake2b_final(&st, out, 48);
        P("gh_isp", out, 48);
        /* many-chunk streaming to exercise buffer boundaries */
        crypto_generichash_init(&st, NULL, 0, 32);
        for (i = 0; i < 40; i++) { crypto_generichash_update(&st, msg, i * 3); }
        crypto_generichash_final(&st, out, 32);
        P("gh_chunks", out, 32);
        crypto_generichash_keygen(out); P("gh_keygen", out, 32);
    }

    /* ---------- sha/keccak streaming boundaries ---------- */
    {
        crypto_hash_sha256_state s2;
        crypto_hash_sha512_state s5;
        crypto_hash_sha3256_state s3;
        crypto_hash_sha3512_state s6;
        crypto_xof_shake128_state x1;
        crypto_hash_sha256_init(&s2);
        crypto_hash_sha512_init(&s5);
        crypto_hash_sha3256_init(&s3);
        crypto_hash_sha3512_init(&s6);
        crypto_xof_shake128_init(&x1);
        for (i = 0; i < 40; i++) {
            crypto_hash_sha256_update(&s2, msg, i * 7);
            crypto_hash_sha512_update(&s5, msg, i * 7);
            crypto_hash_sha3256_update(&s3, msg, i * 7);
            crypto_hash_sha3512_update(&s6, msg, i * 7);
            crypto_xof_shake128_update(&x1, msg, i * 7);
        }
        crypto_hash_sha256_final(&s2, out); P("sha256_chunks", out, 32);
        crypto_hash_sha512_final(&s5, out); P("sha512_chunks", out, 64);
        crypto_hash_sha3256_final(&s3, out); P("sha3256_chunks", out, 32);
        crypto_hash_sha3512_final(&s6, out); P("sha3512_chunks", out, 64);
        for (i = 0; i < 20; i++) { crypto_xof_shake128_squeeze(&x1, out + i * 13, 13); }
        P("shake128_chunks", out, 260);
    }

    /* ---------- auth / onetimeauth failure paths ---------- */
    {
        unsigned char mac[64];
        crypto_auth(mac, msg, 300, k32);
        mac[0] ^= 1;
        I("auth_verify_bad", crypto_auth_verify(mac, msg, 300, k32));
        crypto_auth_hmacsha256(mac, msg, 300, k32); mac[0] ^= 1;
        I("hmac256_verify_bad", crypto_auth_hmacsha256_verify(mac, msg, 300, k32));
        crypto_auth_hmacsha512(mac, msg, 300, k32); mac[0] ^= 1;
        I("hmac512_verify_bad", crypto_auth_hmacsha512_verify(mac, msg, 300, k32));
        crypto_auth_hmacsha512256(mac, msg, 300, k32); mac[0] ^= 1;
        I("hmac512256_verify_bad", crypto_auth_hmacsha512256_verify(mac, msg, 300, k32));
        crypto_onetimeauth(mac, msg, 300, k32); mac[0] ^= 1;
        I("ota_verify_bad", crypto_onetimeauth_verify(mac, msg, 300, k32));
        /* long keys (> block size) get hashed */
        {
            unsigned char lk[200];
            memcpy(lk, msg, 200);
            crypto_auth_hmacsha256_state h2;
            crypto_auth_hmacsha256_init(&h2, lk, 200);
            crypto_auth_hmacsha256_update(&h2, msg, 300);
            crypto_auth_hmacsha256_final(&h2, out); P("hmac256_longkey", out, 32);
            crypto_auth_hmacsha512_state h5;
            crypto_auth_hmacsha512_init(&h5, lk, 200);
            crypto_auth_hmacsha512_update(&h5, msg, 300);
            crypto_auth_hmacsha512_final(&h5, out); P("hmac512_longkey", out, 64);
        }
        crypto_auth_keygen(out); P("auth_keygen", out, 32);
        crypto_onetimeauth_keygen(out); P("ota_keygen", out, 32);
        crypto_auth_hmacsha256_keygen(out); P("hmac256_keygen", out, 32);
        crypto_auth_hmacsha512_keygen(out); P("hmac512_keygen", out, 32);
        crypto_auth_hmacsha512256_keygen(out); P("hmac512256_keygen", out, 32);
        crypto_shorthash_keygen(out); P("sh_keygen", out, 16);
        crypto_secretbox_keygen(out); P("sb_keygen", out, 32);
        crypto_stream_keygen(out); P("stream_keygen", out, 32);
        crypto_stream_chacha20_keygen(out); P("cc20_keygen", out, 32);
        crypto_stream_chacha20_ietf_keygen(out); P("cc20ietf_keygen", out, 32);
        crypto_stream_xchacha20_keygen(out); P("xcc20_keygen", out, 32);
        crypto_stream_salsa20_keygen(out); P("s20_keygen", out, 32);
        crypto_stream_xsalsa20_keygen(out); P("xs20_keygen", out, 32);
        crypto_stream_salsa2012_keygen(out); P("s2012_keygen", out, 32);
        crypto_stream_salsa208_keygen(out); P("s208_keygen", out, 32);
        crypto_aead_chacha20poly1305_keygen(out); P("aead_ccp_keygen", out, 32);
        crypto_aead_chacha20poly1305_ietf_keygen(out); P("aead_ccpi_keygen", out, 32);
        crypto_aead_xchacha20poly1305_ietf_keygen(out); P("aead_xccp_keygen", out, 32);
        crypto_aead_aegis128l_keygen(out); P("aegis128l_keygen", out, 16);
        crypto_aead_aegis256_keygen(out); P("aegis256_keygen", out, 32);
        crypto_aead_aes256gcm_keygen(out); P("aes256gcm_keygen", out, 32);
        crypto_kdf_keygen(out); P("kdf_keygen", out, 32);
        crypto_kdf_hkdf_sha256_keygen(out); P("hkdf256_keygen", out, 32);
        crypto_kdf_hkdf_sha512_keygen(out); P("hkdf512_keygen", out, 64);
        crypto_generichash_blake2b_keygen(out); P("b2b_keygen", out, 32);
        crypto_secretbox_xsalsa20poly1305_keygen(out); P("sbx_keygen", out, 32);
        crypto_secretstream_xchacha20poly1305_keygen(out); P("ss_keygen", out, 32);
        crypto_ipcrypt_keygen(out); P("ipc_keygen", out, 16);
        crypto_ipcrypt_nd_keygen(out); P("ipcnd_keygen", out, 16);
        crypto_ipcrypt_ndx_keygen(out); P("ipcndx_keygen", out, 32);
        crypto_ipcrypt_pfx_keygen(out); P("ipcpfx_keygen", out, 32);
    }

    /* ---------- aead tamper / error paths ---------- */
    {
        unsigned char ct[512], pt[512], mac[32];
        unsigned long long clen, mlen, maclen;
        crypto_aead_chacha20poly1305_ietf_encrypt(ct, &clen, msg, 100, msg, 7, NULL, n24, k32);
        ct[3] ^= 0x80;
        I("aead_ccpi_tamper",
          crypto_aead_chacha20poly1305_ietf_decrypt(pt, &mlen, NULL, ct, clen, msg, 7, n24, k32));
        ct[3] ^= 0x80;
        I("aead_ccpi_short",
          crypto_aead_chacha20poly1305_ietf_decrypt(pt, &mlen, NULL, ct, 5, msg, 7, n24, k32));
        I("aead_ccpi_wrongad",
          crypto_aead_chacha20poly1305_ietf_decrypt(pt, &mlen, NULL, ct, clen, msg, 8, n24, k32));
        crypto_aead_aegis128l_encrypt_detached(ct, mac, &maclen, msg, 100, msg, 7, NULL, n24, k32);
        mac[0] ^= 1;
        memset(pt, 0xAA, sizeof pt);
        I("aegis128l_tamper",
          crypto_aead_aegis128l_decrypt_detached(pt, NULL, ct, 100, mac, msg, 7, n24, k32));
        P("aegis128l_tamper_out", pt, 32);
        mac[0] ^= 1;
        I("aegis128l_ok",
          crypto_aead_aegis128l_decrypt_detached(pt, NULL, ct, 100, mac, msg, 7, n24, k32));
        crypto_aead_aegis256_encrypt_detached(ct, mac, &maclen, msg, 100, msg, 7, NULL, n24, k32);
        mac[0] ^= 1;
        memset(pt, 0xBB, sizeof pt);
        I("aegis256_tamper",
          crypto_aead_aegis256_decrypt_detached(pt, NULL, ct, 100, mac, msg, 7, n24, k32));
        P("aegis256_tamper_out", pt, 32);
        /* nsec must be NULL; zero-length messages */
        I("aead_ccp_zero",
          crypto_aead_chacha20poly1305_encrypt(ct, &clen, NULL, 0, NULL, 0, NULL, n24, k32));
        P("aead_ccp_zero_ct", ct, (size_t) clen);
        I("aegis128l_zero",
          crypto_aead_aegis128l_encrypt(ct, &clen, NULL, 0, NULL, 0, NULL, n24, k32));
        P("aegis128l_zero_ct", ct, (size_t) clen);
        I("aegis256_zero",
          crypto_aead_aegis256_encrypt(ct, &clen, NULL, 0, NULL, 0, NULL, n24, k32));
        P("aegis256_zero_ct", ct, (size_t) clen);
        /* long messages crossing internal chunk logic */
        {
            static unsigned char lm[70000], lc[70100], lp[70100];
            size_t j;
            for (j = 0; j < sizeof lm; j++) { lm[j] = (unsigned char) (j * 31 + 7); }
            crypto_aead_xchacha20poly1305_ietf_encrypt(lc, &clen, lm, sizeof lm, msg, 9,
                                                       NULL, n24, k32);
            crypto_hash_sha256(out, lc, (size_t) clen); P("aead_xccp_long_sha", out, 32);
            I("aead_xccp_long_dec",
              crypto_aead_xchacha20poly1305_ietf_decrypt(lp, &mlen, NULL, lc, clen, msg, 9,
                                                         n24, k32));
            crypto_secretbox_easy(lc, lm, sizeof lm, n24, k32);
            crypto_hash_sha256(out, lc, sizeof lm + 16); P("sb_long_sha", out, 32);
            I("sb_long_open", crypto_secretbox_open_easy(lp, lc, sizeof lm + 16, n24, k32));
            crypto_secretbox_xchacha20poly1305_easy(lc, lm, sizeof lm, n24, k32);
            crypto_hash_sha256(out, lc, sizeof lm + 16); P("sbx_long_sha", out, 32);
            crypto_stream_chacha20(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("cc20_long_sha", out, 32);
            crypto_stream_salsa20(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("s20_long_sha", out, 32);
            crypto_stream_chacha20_ietf(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("cc20i_long_sha", out, 32);
            crypto_stream_xchacha20(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("xcc20_long_sha", out, 32);
            crypto_stream_salsa2012(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("s2012_long_sha", out, 32);
            crypto_stream_salsa208(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("s208_long_sha", out, 32);
            crypto_stream_chacha20_ietf_ext(lc, 70000, n24, k32);
            crypto_hash_sha256(out, lc, 70000); P("cc20ext_long_sha", out, 32);
            crypto_stream_chacha20_ietf_ext_xor_ic(lc, lm, 70000, n24, 3, k32);
            crypto_hash_sha256(out, lc, 70000); P("cc20ext_xoric_sha", out, 32);
            crypto_generichash(out, 32, lm, sizeof lm, k32, 32); P("gh_long", out, 32);
            crypto_hash_sha512(out, lm, sizeof lm); P("sha512_long", out, 64);
            crypto_hash_sha3512(out, lm, sizeof lm); P("sha3512_long", out, 64);
            crypto_xof_shake256(out, 64, lm, sizeof lm); P("shake256_long", out, 64);
            crypto_shorthash(out, lm, sizeof lm, k32); P("sh_long", out, 8);
            crypto_shorthash_siphashx24(out, lm, sizeof lm, k32); P("shx_long", out, 16);
        }
    }

    /* ---------- secretbox / secretstream error paths ---------- */
    {
        unsigned char ct[512], pt[512];
        crypto_secretbox_easy(ct, msg, 100, n24, k32);
        ct[0] ^= 1;
        I("sb_open_tamper", crypto_secretbox_open_easy(pt, ct, 116, n24, k32));
        ct[0] ^= 1;
        I("sb_open_short", crypto_secretbox_open_easy(pt, ct, 15, n24, k32));
        I("sbx_open_short", crypto_secretbox_xchacha20poly1305_open_easy(pt, ct, 15, n24, k32));
        {
            crypto_secretstream_xchacha20poly1305_state ss;
            unsigned char header[24];
            unsigned long long clen, mlen;
            unsigned char tag;
            crypto_secretstream_xchacha20poly1305_init_push(&ss, header, k32);
            crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, 100, NULL, 0, 0);
            ct[0] ^= 1;
            crypto_secretstream_xchacha20poly1305_init_pull(&ss, header, k32);
            I("ss_pull_tamper",
              crypto_secretstream_xchacha20poly1305_pull(&ss, pt, &mlen, &tag, ct, clen, NULL, 0));
            ct[0] ^= 1;
            crypto_secretstream_xchacha20poly1305_init_pull(&ss, header, k32);
            I("ss_pull_short",
              crypto_secretstream_xchacha20poly1305_pull(&ss, pt, &mlen, &tag, ct, 5, NULL, 0));
            /* long stream, many pushes, exercising nonce increment + rekey */
            crypto_secretstream_xchacha20poly1305_init_push(&ss, header, k32);
            for (i = 0; i < 30; i++) {
                unsigned char t = (i % 7 == 6)
                    ? crypto_secretstream_xchacha20poly1305_TAG_REKEY : 0;
                crypto_secretstream_xchacha20poly1305_push(&ss, ct, &clen, msg, i * 5,
                                                           msg, i % 3, t);
                crypto_hash_sha256(out, ct, (size_t) clen);
                { char tg[64]; snprintf(tg, sizeof tg, "ss_seq[%zu]", i); P(tg, out, 32); }
            }
        }
    }

    /* ---------- scalarmult / core error paths ---------- */
    {
        unsigned char q[32], z[32];
        memset(z, 0, 32);
        I("scalarmult_zero_pk", crypto_scalarmult(q, k32, z));
        I("sm_c25519_zero_pk", crypto_scalarmult_curve25519(q, k32, z));
        I("sm_ed25519_zero_n", crypto_scalarmult_ed25519(q, z, k32));
        I("sm_ed25519_bad_p", crypto_scalarmult_ed25519(q, k32, z));
        I("sm_ristretto_zero", crypto_scalarmult_ristretto255(q, k32, z));
        I("ed25519_is_valid_zero", crypto_core_ed25519_is_valid_point(z));
        I("ristretto_is_valid_zero", crypto_core_ristretto255_is_valid_point(z));
        I("ed25519_add_bad", crypto_core_ed25519_add(q, z, z));
        I("ristretto_add_bad", crypto_core_ristretto255_add(q, z, z));
        I("ed25519_sc_invert_zero", crypto_core_ed25519_scalar_invert(q, z));
        crypto_core_ed25519_random(q); P("ed25519_random", q, 32);
        crypto_core_ed25519_scalar_random(q); P("ed25519_sc_random", q, 32);
        crypto_core_ristretto255_random(q); P("ristretto_random", q, 32);
        crypto_core_ristretto255_scalar_random(q); P("ristretto_sc_random", q, 32);
        I("ed25519_sc_is_canon_L", crypto_core_ed25519_scalar_is_canonical(k32));
        {
            /* the group order L itself is not canonical */
            static const unsigned char L[32] = {
                0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7,
                0xa2, 0xde, 0xf9, 0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10
            };
            I("ed25519_sc_is_canon_exactL", crypto_core_ed25519_scalar_is_canonical(L));
            crypto_core_ed25519_scalar_negate(q, L); P("ed25519_sc_neg_L", q, 32);
            crypto_core_ed25519_scalar_complement(q, L); P("ed25519_sc_comp_L", q, 32);
        }
        for (i = 0; i < 8; i++) {
            unsigned char s[64], r[32];
            char tag[64];
            size_t j;
            for (j = 0; j < 64; j++) { s[j] = (unsigned char) (j * (i + 3) + i); }
            crypto_core_ed25519_scalar_reduce(r, s);
            snprintf(tag, sizeof tag, "sc_reduce[%zu]", i); P(tag, r, 32);
            crypto_core_ed25519_scalar_mul(r, s, s + 32);
            snprintf(tag, sizeof tag, "sc_mul[%zu]", i); P(tag, r, 32);
            if (crypto_core_ed25519_from_string(r, (const unsigned char *) "CTX", 3, s, 64, 1) == 0) {
                snprintf(tag, sizeof tag, "ed_from_str_sha256[%zu]", i); P(tag, r, 32);
            }
            if (crypto_core_ed25519_from_string(r, (const unsigned char *) "CTX", 3, s, 64, 2) == 0) {
                snprintf(tag, sizeof tag, "ed_from_str_sha512[%zu]", i); P(tag, r, 32);
            }
            if (crypto_core_ristretto255_from_string(r, (const unsigned char *) "CTX", 3, s, 64, 1) == 0) {
                snprintf(tag, sizeof tag, "ri_from_str_sha256[%zu]", i); P(tag, r, 32);
            }
            if (crypto_core_ristretto255_from_hash(r, s) == 0) {
                snprintf(tag, sizeof tag, "ri_from_hash[%zu]", i); P(tag, r, 32);
            }
            crypto_core_ristretto255_scalar_from_string(r, (const unsigned char *) "C", 1, s, 64, 2);
            snprintf(tag, sizeof tag, "ri_sc_from_str[%zu]", i); P(tag, r, 32);
        }
    }

    /* ---------- sign error paths ---------- */
    {
        unsigned char pk[32], sk[64], sig[64], sm[512], m2[512], z[32];
        unsigned long long siglen, smlen, mlen;
        memset(z, 0, 32);
        crypto_sign_keypair(pk, sk); P("sign_kp_pk", pk, 32); P("sign_kp_sk", sk, 64);
        crypto_sign_detached(sig, &siglen, msg, 300, sk);
        P("sign_sig2", sig, 64);
        sig[0] ^= 1;
        I("sign_verify_tamper", crypto_sign_verify_detached(sig, msg, 300, pk));
        sig[0] ^= 1;
        I("sign_verify_zero_pk", crypto_sign_verify_detached(sig, msg, 300, z));
        {
            unsigned char sig2[64];
            memcpy(sig2, sig, 64);
            memset(sig2 + 32, 0xff, 32); /* non-canonical S */
            I("sign_verify_noncanon_s", crypto_sign_verify_detached(sig2, msg, 300, pk));
        }
        crypto_sign(sm, &smlen, msg, 300, sk);
        sm[10] ^= 1;
        I("sign_open_tamper", crypto_sign_open(m2, &mlen, sm, smlen, pk));
        sm[10] ^= 1;
        I("sign_open_short", crypto_sign_open(m2, &mlen, sm, 10, pk));
        I("sign_ed25519_sk_to_curve25519_z", crypto_sign_ed25519_sk_to_curve25519(out, sk));
        P("sk2c", out, 32);
        I("sign_pk_to_c25519_bad", crypto_sign_ed25519_pk_to_curve25519(out, z));
        {
            crypto_sign_ed25519ph_state ph;
            crypto_sign_ed25519ph_init(&ph);
            for (i = 0; i < 20; i++) { crypto_sign_ed25519ph_update(&ph, msg, i * 11); }
            I("ph_create", crypto_sign_ed25519ph_final_create(&ph, sig, &siglen, sk));
            P("ph_sig", sig, 64);
            crypto_sign_ed25519ph_init(&ph);
            for (i = 0; i < 20; i++) { crypto_sign_ed25519ph_update(&ph, msg, i * 11); }
            I("ph_verify", crypto_sign_ed25519ph_final_verify(&ph, sig, pk));
            I("ph_statebytes", (long long) crypto_sign_ed25519ph_statebytes());
        }
    }

    /* ---------- box error paths ---------- */
    {
        unsigned char pk1[32], sk1[32], pk2[32], sk2[32], ct[512], pt[512], z[32];
        memset(z, 0, 32);
        crypto_box_keypair(pk1, sk1); P("box_kp_pk", pk1, 32); P("box_kp_sk", sk1, 32);
        crypto_box_keypair(pk2, sk2);
        I("box_beforenm_zero", crypto_box_beforenm(out, z, sk1));
        crypto_box_easy(ct, msg, 100, n24, pk2, sk1);
        ct[0] ^= 1;
        I("box_open_tamper", crypto_box_open_easy(pt, ct, 116, n24, pk1, sk2));
        ct[0] ^= 1;
        I("box_open_short", crypto_box_open_easy(pt, ct, 15, n24, pk1, sk2));
        crypto_box_seal(ct, msg, 100, pk1);
        ct[0] ^= 1;
        I("box_seal_open_tamper",
          crypto_box_seal_open(pt, ct, 100 + crypto_box_SEALBYTES, pk1, sk1));
        I("box_seal_open_short", crypto_box_seal_open(pt, ct, 10, pk1, sk1));
        I("box_c25519xccp_kp",
          crypto_box_curve25519xchacha20poly1305_keypair(pk1, sk1));
        P("xccp_pk", pk1, 32); P("xccp_sk", sk1, 32);
        I("box_c25519xccp_seed_kp",
          crypto_box_curve25519xchacha20poly1305_seed_keypair(pk2, sk2, k32));
        P("xccp_seed_pk", pk2, 32);
        I("xccp_beforenm", crypto_box_curve25519xchacha20poly1305_beforenm(out, pk2, sk1));
        P("xccp_nm", out, 32);
        I("xccp_easy", crypto_box_curve25519xchacha20poly1305_easy(ct, msg, 100, n24, pk2, sk1));
        P("xccp_easy_ct", ct, 116);
        I("xccp_open_easy",
          crypto_box_curve25519xchacha20poly1305_open_easy(pt, ct, 116, n24, pk1, sk2));
        I("xccp_easy_afternm",
          crypto_box_curve25519xchacha20poly1305_easy_afternm(ct, msg, 100, n24, out));
        P("xccp_easy_afternm_ct", ct, 116);
        I("xccp_seal_open",
          crypto_box_curve25519xchacha20poly1305_seal_open(pt, ct, 10, pk1, sk1));
    }

    /* ---------- kdf error paths ---------- */
    {
        unsigned char sub[64];
        I("kdf_short", crypto_kdf_derive_from_key(sub, 15, 1, "context8", k32));
        I("kdf_long", crypto_kdf_derive_from_key(sub, 65, 1, "context8", k32));
        for (i = 16; i <= 64; i += 6) {
            char tag[64];
            crypto_kdf_derive_from_key(sub, i, i, "context8", k32);
            snprintf(tag, sizeof tag, "kdf[%zu]", i); P(tag, sub, i);
        }
        errno = 0;
        IE("hkdf256_expand_big",
           crypto_kdf_hkdf_sha256_expand(out, 32 * 255 + 1, "i", 1, k32), errno);
        I("hkdf256_expand_max", crypto_kdf_hkdf_sha256_expand(out, 32 * 255, "i", 1, k32));
        crypto_hash_sha256(out + 20000, out, 32 * 255); P("hkdf256_max_sha", out + 20000, 32);
        errno = 0;
        IE("hkdf512_expand_big",
           crypto_kdf_hkdf_sha512_expand(out, 64 * 255 + 1, "i", 1, k64), errno);
        I("hkdf512_expand_max", crypto_kdf_hkdf_sha512_expand(out, 64 * 255, "i", 1, k64));
        crypto_hash_sha256(out + 20000, out, 64 * 255); P("hkdf512_max_sha", out + 20000, 32);
        for (i = 0; i <= 40; i += 7) {
            char tag[64];
            crypto_kdf_hkdf_sha256_expand(out, i, (const char *) msg, i, k32);
            snprintf(tag, sizeof tag, "hkdf256_exp[%zu]", i); P(tag, out, i);
            crypto_kdf_hkdf_sha512_expand(out, i, (const char *) msg, i, k64);
            snprintf(tag, sizeof tag, "hkdf512_exp[%zu]", i); P(tag, out, i);
        }
    }

    /* ---------- pwhash argon2 / scrypt error + encoding paths ---------- */
    {
        char str[600];
        I("pw_str_argon2i",
          crypto_pwhash_argon2i_str(str, "pw", 2, 3, 16384)); S("pw_str_argon2i_s", str);
        I("pw_str_argon2i_v", crypto_pwhash_argon2i_str_verify(str, "pw", 2));
        I("pw_str_argon2i_v_bad", crypto_pwhash_argon2i_str_verify(str, "px", 2));
        I("pw_str_argon2i_rehash",
          crypto_pwhash_argon2i_str_needs_rehash(str, 3, 16384));
        I("pw_str_argon2i_rehash2",
          crypto_pwhash_argon2i_str_needs_rehash(str, 4, 16384));
        I("pw_str_argon2id",
          crypto_pwhash_argon2id_str(str, "pw", 2, 3, 16384)); S("pw_str_argon2id_s", str);
        I("pw_str_argon2id_v", crypto_pwhash_argon2id_str_verify(str, "pw", 2));
        I("pw_str_argon2id_rehash",
          crypto_pwhash_argon2id_str_needs_rehash(str, 3, 16384));
        /* known-good reference strings */
        S("verify_ref_i", "");
        I("verify_ref_i_r",
          crypto_pwhash_str_verify(
            "$argon2i$v=19$m=4096,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$"
            "PzuMHRHfWvBNEUxA9OQ8xY0YFRJhVsSPkkkLnCEQPCE", "password", 8));
        I("needs_rehash_ref_bad", crypto_pwhash_str_needs_rehash("$argon2i$bogus", 3, 16384));
        I("needs_rehash_ref_bad2", crypto_pwhash_str_needs_rehash("", 3, 16384));
        I("str_verify_bogus", crypto_pwhash_str_verify("$argon2i$bogus", "pw", 2));
        I("str_verify_empty", crypto_pwhash_str_verify("", "pw", 2));
        errno = 0;
        IE("pw_opslimit_zero",
           crypto_pwhash(out, 32, "pw", 2, k32, 0, 16384, crypto_pwhash_ALG_ARGON2I13), errno);
        errno = 0;
        IE("pw_memlimit_small",
           crypto_pwhash(out, 32, "pw", 2, k32, 3, 1, crypto_pwhash_ALG_ARGON2I13), errno);
        for (i = 1; i <= 4; i++) {
            char tag[64];
            if (crypto_pwhash(out, 32, "pw", 2, k32, i, 8192 * i,
                              crypto_pwhash_ALG_ARGON2ID13) == 0) {
                snprintf(tag, sizeof tag, "pw_id[%zu]", i); P(tag, out, 32);
            }
            if (crypto_pwhash(out, 32, "pw", 2, k32, i, 8192 * i,
                              crypto_pwhash_ALG_ARGON2I13) == 0) {
                snprintf(tag, sizeof tag, "pw_i[%zu]", i); P(tag, out, 32);
            }
        }
        I("scrypt_ll_bad",
          crypto_pwhash_scryptsalsa208sha256_ll((const uint8_t *) "pw", 2, k32, 32,
                                                3, 8, 1, out, 64));
        for (i = 0; i < 4; i++) {
            char tag[64];
            if (crypto_pwhash_scryptsalsa208sha256_ll((const uint8_t *) "pw", 2, k32, 32,
                                                      1u << (4 + i), 4 + i, 1 + i,
                                                      out, 48) == 0) {
                snprintf(tag, sizeof tag, "scrypt_ll[%zu]", i); P(tag, out, 48);
            }
        }
        I("scrypt_str2",
          crypto_pwhash_scryptsalsa208sha256_str(str, "pw", 2, 32768, 1 << 21));
        S("scrypt_str2_s", str);
        I("scrypt_str2_v",
          crypto_pwhash_scryptsalsa208sha256_str_verify(str, "pw", 2));
        I("scrypt_str2_v_bad",
          crypto_pwhash_scryptsalsa208sha256_str_verify(str, "px", 2));
        I("scrypt_str2_rehash",
          crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(str, 32768, 1 << 21));
        I("scrypt_str2_rehash2",
          crypto_pwhash_scryptsalsa208sha256_str_needs_rehash(str, 65536, 1 << 22));
        I("scrypt_str_verify_bogus",
          crypto_pwhash_scryptsalsa208sha256_str_verify("$7$bogus", "pw", 2));
        errno = 0;
        IE("scrypt_out_small",
           crypto_pwhash_scryptsalsa208sha256(out, 8, "pw", 2, k32, 16384, 1 << 20), errno);
    }

    /* ---------- kem paths ---------- */
    {
        static unsigned char pk[3000], sk[3000], ct[3000], ss1[64], ss2[64];
        I("mlkem_keypair", crypto_kem_mlkem768_keypair(pk, sk));
        crypto_hash_sha256(out, pk, crypto_kem_mlkem768_PUBLICKEYBYTES);
        P("mlkem_kp_pk_sha", out, 32);
        I("mlkem_enc", crypto_kem_mlkem768_enc(ct, ss1, pk));
        I("mlkem_dec2", crypto_kem_mlkem768_dec(ss2, ct, sk));
        I("mlkem_ss_match", memcmp(ss1, ss2, 32) == 0);
        I("xwing_keypair", crypto_kem_xwing_keypair(pk, sk));
        crypto_hash_sha256(out, pk, crypto_kem_xwing_PUBLICKEYBYTES);
        P("xwing_kp_pk_sha", out, 32);
        I("xwing_enc", crypto_kem_xwing_enc(ct, ss1, pk));
        I("xwing_dec2", crypto_kem_xwing_dec(ss2, ct, sk));
        I("xwing_ss_match", memcmp(ss1, ss2, 32) == 0);
        I("kem_keypair", crypto_kem_keypair(pk, sk));
        I("kem_seed_keypair", crypto_kem_seed_keypair(pk, sk, k32));
        crypto_hash_sha256(out, pk, crypto_kem_publickeybytes());
        P("kem_seed_pk_sha", out, 32);
        I("kem_enc", crypto_kem_enc(ct, ss1, pk));
        I("kem_dec", crypto_kem_dec(ss2, ct, sk));
        I("kem_ss_match", memcmp(ss1, ss2, 32) == 0);
        for (i = 0; i < 4; i++) {
            unsigned char seed[64];
            size_t j;
            char tag[64];
            for (j = 0; j < 64; j++) { seed[j] = (unsigned char) (j * (i + 2) + 1); }
            crypto_kem_mlkem768_seed_keypair(pk, sk, seed);
            crypto_kem_mlkem768_enc_deterministic(ct, ss1, pk, seed);
            crypto_hash_sha256(out, ct, crypto_kem_mlkem768_CIPHERTEXTBYTES);
            snprintf(tag, sizeof tag, "mlkem_ct[%zu]", i); P(tag, out, 32);
            snprintf(tag, sizeof tag, "mlkem_ss[%zu]", i); P(tag, ss1, 32);
            crypto_kem_xwing_seed_keypair(pk, sk, seed);
            crypto_kem_xwing_enc_deterministic(ct, ss1, pk, seed);
            crypto_hash_sha256(out, ct, crypto_kem_xwing_CIPHERTEXTBYTES);
            snprintf(tag, sizeof tag, "xwing_ct[%zu]", i); P(tag, out, 32);
            snprintf(tag, sizeof tag, "xwing_ss[%zu]", i); P(tag, ss1, 32);
        }
    }

    /* ---------- ipcrypt exhaustive-ish ---------- */
    {
        unsigned char ip[16], enc[32], dec[16];
        for (i = 0; i < 12; i++) {
            char tag[64];
            size_t j;
            for (j = 0; j < 16; j++) { ip[j] = (unsigned char) (j * (i + 1) + i); }
            if (i == 0) { memset(ip, 0, 16); }
            if (i == 1) { memset(ip, 0xff, 16); }
            if (i == 2) { memset(ip, 0, 10); ip[10] = 0xff; ip[11] = 0xff; }
            crypto_ipcrypt_encrypt(enc, ip, k32);
            snprintf(tag, sizeof tag, "ipc_e[%zu]", i); P(tag, enc, 16);
            crypto_ipcrypt_decrypt(dec, enc, k32);
            snprintf(tag, sizeof tag, "ipc_d[%zu]", i); P(tag, dec, 16);
            crypto_ipcrypt_nd_encrypt(enc, ip, k32, n24);
            snprintf(tag, sizeof tag, "ipc_nde[%zu]", i);
            P(tag, enc, crypto_ipcrypt_ND_OUTPUTBYTES);
            crypto_ipcrypt_nd_decrypt(dec, enc, k32);
            snprintf(tag, sizeof tag, "ipc_ndd[%zu]", i); P(tag, dec, 16);
            crypto_ipcrypt_ndx_encrypt(enc, ip, k32, n24);
            snprintf(tag, sizeof tag, "ipc_ndxe[%zu]", i);
            P(tag, enc, crypto_ipcrypt_NDX_OUTPUTBYTES);
            crypto_ipcrypt_ndx_decrypt(dec, enc, k32);
            snprintf(tag, sizeof tag, "ipc_ndxd[%zu]", i); P(tag, dec, 16);
            crypto_ipcrypt_pfx_encrypt(enc, ip, k32);
            snprintf(tag, sizeof tag, "ipc_pfxe[%zu]", i); P(tag, enc, 16);
            crypto_ipcrypt_pfx_decrypt(dec, enc, k32);
            snprintf(tag, sizeof tag, "ipc_pfxd[%zu]", i); P(tag, dec, 16);
        }
    }

    /* ---------- keccak / xof edge cases ---------- */
    {
        crypto_core_keccak1600_state ks;
        for (i = 0; i < 8; i++) {
            char tag[64];
            crypto_core_keccak1600_init(&ks);
            crypto_core_keccak1600_xor_bytes(&ks, msg, i * 3, 50 + i);
            crypto_core_keccak1600_permute_24(&ks);
            crypto_core_keccak1600_xor_bytes(&ks, msg, i, 30);
            crypto_core_keccak1600_permute_12(&ks);
            crypto_core_keccak1600_extract_bytes(&ks, out, i * 5, 100);
            snprintf(tag, sizeof tag, "keccak_mix[%zu]", i); P(tag, out, 100);
        }
        for (i = 0; i <= 400; i += 33) {
            char tag[64];
            crypto_xof_shake128(out, i, msg, 300);
            snprintf(tag, sizeof tag, "shake128o[%zu]", i); P(tag, out, i);
            crypto_xof_turboshake128(out, i, msg, 300);
            snprintf(tag, sizeof tag, "ts128o[%zu]", i); P(tag, out, i);
            crypto_xof_turboshake256(out, i, msg, 300);
            snprintf(tag, sizeof tag, "ts256o[%zu]", i); P(tag, out, i);
        }
        for (i = 1; i < 0x80; i += 17) {
            crypto_xof_turboshake128_state t;
            char tag[64];
            crypto_xof_turboshake128_init_with_domain(&t, (unsigned char) i);
            crypto_xof_turboshake128_update(&t, msg, 300);
            crypto_xof_turboshake128_squeeze(&t, out, 64);
            snprintf(tag, sizeof tag, "ts128dom[%zu]", i); P(tag, out, 64);
        }
    }

    /* ---------- randombytes ---------- */
    {
        unsigned char buf[128];
        randombytes_stir();
        randombytes(buf, 64); P("randombytes64", buf, 64);
        for (i = 0; i < 8; i++) {
            char tag[64];
            randombytes_buf_deterministic(buf, 100, k32);
            snprintf(tag, sizeof tag, "rb_det[%zu]", i); P(tag, buf, 100);
            k32[0]++;
        }
        for (i = 1; i < 20; i++) { I("rb_uniform", (long long) randombytes_uniform(i)); }
        I("rb_close", randombytes_close());
        S("rb_name", randombytes_implementation_name());
    }

    printf("DONE2\n");
    return 0;
}
