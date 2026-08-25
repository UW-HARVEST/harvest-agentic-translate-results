#include "jansson.h"
#include "jansson_private.h"
#include "hashtable.h"
#include "strbuffer.h"
#include "utf.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern char *dtoa(double, int, int, int *, int *, char **);
extern char *dtoa_r(double, int, int, int *, int *, char **, char *, size_t);
extern void freedtoa(char *);
extern double strtod__unused(const char *, char **);

int main(void) {
    static const unsigned char utf[][5] = {
        {0x41, 0}, {0xc3, 0xa9, 0}, {0xf0, 0x9f, 0x98, 0x80, 0},
        {0xc0, 0x80, 0}, {0xed, 0xa0, 0x80, 0}, {0xf5, 0x80, 0x80, 0x80, 0}
    };
    for (size_t i = 0; i < sizeof(utf) / sizeof(utf[0]); i++) {
        size_t len = strlen((const char *)utf[i]);
        int32_t cp = -1;
        const char *next = utf8_iterate((const char *)utf[i], len, &cp);
        printf("utf=%zu,%zu,%zu,%d,%ld\n", i, len,
               utf8_check_first((char)utf[i][0]),
               utf8_check_string((const char *)utf[i], len),
               next ? (long)(next - (const char *)utf[i]) : -1L);
    }
    char encoded[8] = {0};
    size_t encoded_size = 0;
    printf("encode=%d,", utf8_encode(0x1f600, encoded, &encoded_size));
    for (size_t i = 0; i < encoded_size; i++)
        printf("%02X", (unsigned char)encoded[i]);
    puts("");

    strbuffer_t buffer;
    printf("sbinit=%d\n", strbuffer_init(&buffer));
    strbuffer_append_bytes(&buffer, "abcdefghijklmnop", 16);
    strbuffer_append_byte(&buffer, '!');
    printf("sb=%zu,%zu,%s,%c,%s\n", buffer.length, buffer.size,
           strbuffer_value(&buffer), strbuffer_pop(&buffer),
           strbuffer_value(&buffer));
    char *stolen = strbuffer_steal_value(&buffer);
    printf("steal=%s,%d\n", stolen, buffer.value == NULL);
    jsonp_free(stolen);
    strbuffer_close(&buffer);

    hashtable_t table;
    hashtable_init(&table);
    hashtable_set(&table, "a", 1, json_integer(1));
    hashtable_set(&table, "b\0x", 3, json_integer(2));
    hashtable_set(&table, "a", 1, json_integer(3));
    printf("hash=%zu,%lld,%lld\n", table.size,
           (long long)json_integer_value(hashtable_get(&table, "a", 1)),
           (long long)json_integer_value(hashtable_get(&table, "b\0x", 3)));
    printf("hashiter=");
    void *iter = hashtable_iter(&table);
    while (iter) {
        printf("%zu:%lld;", hashtable_iter_key_len(iter),
               (long long)json_integer_value(hashtable_iter_value(iter)));
        iter = hashtable_iter_next(&table, iter);
    }
    puts("");
    printf("hashdel=%d,%d,%zu\n", hashtable_del(&table, "a", 1),
           hashtable_del(&table, "missing", 7), table.size);
    hashtable_close(&table);

    static const double values[] = {0.0, -0.0, 0.1, 1e-7, 1e16, 1.23456789};
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); i++) {
        for (int mode = 0; mode <= 3; mode++) {
            int decpt, sign;
            char *end;
            char *digits = dtoa(values[i], mode, 6, &decpt, &sign, &end);
            printf("dtoa=%zu,%d,%s,%d,%d,%ld\n", i, mode, digits,
                   decpt, sign, (long)(end - digits));
            freedtoa(digits);
        }
    }
    char number[] = "1.25e3";
    strbuffer_t numeric = {number, 6, 7};
    double parsed = 0;
    printf("strtod=%d,%.1f\n", jsonp_strtod(&numeric, &parsed), parsed);
    return 0;
}
