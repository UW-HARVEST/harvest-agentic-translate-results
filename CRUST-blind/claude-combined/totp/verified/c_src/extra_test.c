#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "totp.h"

static void
to_hex(const uint8_t *a, size_t len, char *buf)
{
    size_t i;
    for (i=0; i<len; i++) {
        buf[i*2]   = "0123456789abcdef"[a[i] >> 4];
        buf[i*2+1] = "0123456789abcdef"[a[i] & 0xF];
    }
    buf[len*2] = '\0';
}

int main(void) {
    uint8_t buf[1024], hash[20];
    char str[41];

    /* SHA-1 of empty string */
    buf[0] = 0;
    sha1(buf, 0, sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(\"\") = %s\n", str);

    /* SHA-1 of "abc" */
    snprintf((char *)buf, sizeof(buf), "%s", "abc");
    sha1(buf, 3, sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(\"abc\") = %s\n", str);

    /* SHA-1 of fox */
    const char *fox = "The quick brown fox jumps over the lazy dog";
    snprintf((char *)buf, sizeof(buf), "%s", fox);
    sha1(buf, strlen(fox), sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(fox) = %s\n", str);

    /* SHA-1 of long input - 64 bytes */
    memset(buf, 0, sizeof(buf));
    for (int i = 0; i < 64; i++) buf[i] = 'a';
    sha1(buf, 64, sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(64*'a') = %s\n", str);

    /* SHA-1 with len=55 (boundary) */
    memset(buf, 0, sizeof(buf));
    for (int i = 0; i < 55; i++) buf[i] = 'b';
    sha1(buf, 55, sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(55*'b') = %s\n", str);

    /* SHA-1 with len=56 (boundary - must roll to next block) */
    memset(buf, 0, sizeof(buf));
    for (int i = 0; i < 56; i++) buf[i] = 'c';
    sha1(buf, 56, sizeof(buf), hash);
    to_hex(hash, 20, str);
    printf("sha1(56*'c') = %s\n", str);

    /* HMAC-SHA1 RFC 2202 test vector 1 */
    uint8_t key[64], text[64];
    memset(key, 0, sizeof(key));
    memset(text, 0, sizeof(text));
    for (int i = 0; i < 20; i++) key[i] = 0x0b;
    snprintf((char *)text, sizeof(text), "Hi There");
    hmac_sha1(key, text, 8, hash);
    to_hex(hash, 20, str);
    printf("hmac_sha1(0x0b*20, \"Hi There\") = %s\n", str);

    /* HMAC-SHA1 RFC 2202 test vector 2 */
    memset(key, 0, sizeof(key));
    memset(text, 0, sizeof(text));
    snprintf((char *)key, sizeof(key), "Jefe");
    snprintf((char *)text, sizeof(text), "what do ya want for nothing?");
    hmac_sha1(key, text, 28, hash);
    to_hex(hash, 20, str);
    printf("hmac_sha1(\"Jefe\", \"what do ya want for nothing?\") = %s\n", str);

    /* HMAC-SHA1 RFC 2202 test vector 3 */
    memset(key, 0, sizeof(key));
    memset(text, 0, sizeof(text));
    for (int i = 0; i < 20; i++) key[i] = 0xAA;
    for (int i = 0; i < 50; i++) text[i] = 0xDD;
    hmac_sha1(key, text, 50, hash);
    to_hex(hash, 20, str);
    printf("hmac_sha1(0xAA*20, 0xDD*50) = %s\n", str);

    /* HOTP - RFC 4226 Appendix D */
    static const uint8_t secret[64] = {
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30 };
    for (int i = 0; i <= 9; i++) {
        printf("hotp(secret, %d) = %d\n", i, hotp(secret, i));
    }

    /* TOTP at varying times */
    printf("totp(secret, 0) = %d\n", totp(secret, 0));
    printf("totp(secret, 30) = %d\n", totp(secret, 30));
    printf("totp(secret, 59) = %d\n", totp(secret, 59));
    printf("totp(secret, 60) = %d\n", totp(secret, 60));
    printf("totp(secret, 90) = %d\n", totp(secret, 90));
    printf("totp(secret, 1234567890) = %d\n", totp(secret, 1234567890));

    /* from_base32 */
    {
        uint8_t b[20];
        size_t n;
        memset(b, 0, sizeof(b));
        n = from_base32("MZxw6===", b, sizeof(b));
        printf("from_base32(\"MZxw6===\") = %lu, ", (unsigned long)n);
        for (size_t i = 0; i < n; i++) printf("%02x", b[i]);
        printf("\n");

        memset(b, 0, sizeof(b));
        n = from_base32("MZxw6YQ=", b, sizeof(b));
        printf("from_base32(\"MZxw6YQ=\") = %lu, ", (unsigned long)n);
        for (size_t i = 0; i < n; i++) printf("%02x", b[i]);
        printf("\n");

        memset(b, 0, sizeof(b));
        n = from_base32("MZxw6YTB", b, sizeof(b));
        printf("from_base32(\"MZxw6YTB\") = %lu, ", (unsigned long)n);
        for (size_t i = 0; i < n; i++) printf("%02x", b[i]);
        printf("\n");

        memset(b, 0, sizeof(b));
        n = from_base32("MZxw6YTBOI======", b, sizeof(b));
        printf("from_base32(\"MZxw6YTBOI======\") = %lu, ", (unsigned long)n);
        for (size_t i = 0; i < n; i++) printf("%02x", b[i]);
        printf("\n");

        /* Invalid: wrong length */
        n = from_base32("ABC", b, sizeof(b));
        printf("from_base32(\"ABC\") = %lu (invalid len)\n", (unsigned long)n);

        /* Invalid: bad char */
        n = from_base32("AB!DEFGH", b, sizeof(b));
        printf("from_base32(\"AB!DEFGH\") = %lu (bad char)\n", (unsigned long)n);

        /* Empty string */
        n = from_base32("", b, sizeof(b));
        printf("from_base32(\"\") = %lu (empty)\n", (unsigned long)n);

        /* Cap too small */
        n = from_base32("MZxw6YTB", b, 4);
        printf("from_base32(\"MZxw6YTB\", cap=4) = %lu (cap too small)\n", (unsigned long)n);

        /* "1" -> 0 (1 not in base32 alphabet) */
        n = from_base32("11111111", b, sizeof(b));
        printf("from_base32(\"11111111\") = %lu\n", (unsigned long)n);
    }

    /* pack32/unpack32 */
    {
        uint8_t a[4];
        unpack32(0xDEADBEEF, a);
        printf("unpack32(0xDEADBEEF) = %02x%02x%02x%02x\n", a[0], a[1], a[2], a[3]);
        printf("pack32 back = %x\n", pack32(a));
    }

    /* unpack64 */
    {
        uint8_t a[8];
        unpack64(0x0102030405060708ULL, a);
        printf("unpack64(0x0102030405060708) = %02x%02x%02x%02x%02x%02x%02x%02x\n",
            a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
    }

    return 0;
}
