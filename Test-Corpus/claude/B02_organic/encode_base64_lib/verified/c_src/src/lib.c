#include "lib.h"

#include <stdlib.h>
#include <string.h>

static char encode(unsigned char u)
{
    if (u < 26) {
        return 'A' + u;
    }
    if (u < 52) {
        return 'a' + (u - 26);
    }
    if (u < 62) {
        return '0' + (u - 52);
    }
    if (u == 62) {
        return '+';
    }

    return '/';
}

/* Base64 encode and return size data in 'src'. The caller must free the
 * returned string.
 * Returns encoded string otherwise NULL
 */
char *encode_base64(int size, const char *src)
{
    int i;
    char *out, *p;

    if (!src) {
        return NULL;
    }

    if (!size) {
        size = strlen((char *)src);
    }

    out = (char *)calloc(sizeof(char), size * 4 / 3 + 4);
    if (!out) {
        return NULL;
    }

    p = out;

    for (i = 0; i < size; i += 3) {
        unsigned char b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0, b7 = 0;

        b1 = src[i];

        if (i + 1 < size) {
            b2 = src[i + 1];
        }

        if (i + 2 < size) {
            b3 = src[i + 2];
        }

        b4 = b1 >> 2;
        b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        b7 = b3 & 0x3f;

        *p++ = encode(b4);
        *p++ = encode(b5);

        if (i + 1 < size) {
            *p++ = encode(b6);
        } else {
            *p++ = '=';
        }

        if (i + 2 < size) {
            *p++ = encode(b7);
        } else {
            *p++ = '=';
        }
    }

    return out;
}
