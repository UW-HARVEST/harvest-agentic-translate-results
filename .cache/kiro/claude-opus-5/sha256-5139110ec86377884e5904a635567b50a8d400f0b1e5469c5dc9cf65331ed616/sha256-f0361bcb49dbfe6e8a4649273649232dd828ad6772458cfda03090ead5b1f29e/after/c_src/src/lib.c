#include "lib.h"

#include <stdlib.h>
#include <string.h>

#define TRUE    1
#define FALSE   0

/* Decode a base64 character */
static unsigned char decode(char c)
{
    if (c >= 'A' && c <= 'Z') {
        return (c - 'A');
    }
    if (c >= 'a' && c <= 'z') {
        return (c - 'a' + 26);
    }
    if (c >= '0' && c <= '9') {
        return (c - '0' + 52);
    }
    if (c == '+') {
        return 62;
    }

    return 63;
}

/* Returns TRUE if 'c' is a valid base64 character, otherwise FALSE */
static int is_base64(char c)
{
    if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
        (c >= '0' && c <= '9') || (c == '+')             ||
        (c == '/')             || (c == '=')) {

        return TRUE;
    }
    return FALSE;
}

/* Decode the base64 encoded string 'src' into the memory pointed to by
 * 'dest'. The dest buffer is NUL terminated.
 * Returns NULL in case of error
 */
char *decode_base64(const char *src)
{
    if (src && *src) {
        char *dest;
        unsigned char *p;
        int k, l = strlen(src) + 1;
        unsigned char *buf;

        /* The size of the dest will always be less than the source */
        dest = (char *)calloc(sizeof(char), l + 13);
        if (!dest) {
            return (NULL);
        }

        p = (unsigned char *)dest;

        buf = (unsigned char *) malloc(l);
        if (!buf) {
            free(dest);
            return (NULL);
        }

        /* Ignore non base64 chars as per the POSIX standard */
        for (k = 0, l = 0; src[k]; k++) {
            if (is_base64(src[k])) {
                buf[l++] = src[k];
            }
        }

        for (k = 0; k < l; k += 4) {
            char c1 = 'A', c2 = 'A', c3 = 'A', c4 = 'A';
            unsigned char b1 = 0, b2 = 0, b3 = 0, b4 = 0;

            c1 = buf[k];

            if (k + 1 < l) {
                c2 = buf[k + 1];
            }

            if (k + 2 < l) {
                c3 = buf[k + 2];
            }

            if (k + 3 < l) {
                c4 = buf[k + 3];
            }

            b1 = decode(c1);
            b2 = decode(c2);
            b3 = decode(c3);
            b4 = decode(c4);

            *p++ = ((b1 << 2) | (b2 >> 4) );

            if (c3 != '=') {
                *p++ = (((b2 & 0xf) << 4) | (b3 >> 2) );
            }

            if (c4 != '=') {
                *p++ = (((b3 & 0x3) << 6) | b4 );
            }

        }

        free(buf);

        return (dest);
    }
    return (NULL);
}
