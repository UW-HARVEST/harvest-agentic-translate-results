#include <stdio.h>
#include <string.h>
#include <inttypes.h>
#include "murmurhash.h"

int main(void) {
    /* Use explicit length so we can include zero-byte strings. */
    struct Case { const char* s; uint32_t len; uint32_t seed; } cases[] = {
        {"a", 1, 0},
        {"ab", 2, 0},
        {"abc", 3, 0},
        {"abcd", 4, 0},
        {"abcde", 5, 0},
        {"abcdef", 6, 0},
        {"abcdefg", 7, 0},
        {"abcdefgh", 8, 0},
        {"The quick brown fox jumps over the lazy dog", 43, 0},
        {"The quick brown fox jumps over the lazy dog", 43, 1},
        {"The quick brown fox jumps over the lazy dog", 43, 42},
        {"x", 1, 12345},
        {"xx", 2, 12345},
        {"xxx", 3, 12345},
        {"xxxx", 4, 12345},
        {"Lorem ipsum", 11, 0xdeadbeef},
        {"\x00\x00\x00\x00", 4, 0},
        {"\xff\xff\xff\xff", 4, 0},
        {"\x01\x02\x03\x04\x05", 5, 0xcafebabe},
    };
    int n = (int)(sizeof(cases)/sizeof(cases[0]));
    for (int i = 0; i < n; i++) {
        uint32_t h = murmurhash(cases[i].s, cases[i].len, cases[i].seed);
        printf("idx=%d len=%u seed=0x%08x -> 0x%08x (%u)\n",
               i, cases[i].len, cases[i].seed, h, h);
    }
    return 0;
}
