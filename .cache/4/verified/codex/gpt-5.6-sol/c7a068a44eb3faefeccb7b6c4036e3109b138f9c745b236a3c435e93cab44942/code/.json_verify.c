#include "jansson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    static const char *inputs[] = {
        "{}", "[]", "{\"\":0}", "[0,-0,1.0,1e2,1E-2]",
        "{\"emoji\":\"\\uD83D\\uDE00\",\"nul\":\"\\u0000\"}",
        "{\"esc\":\"\\\\\\\"\\/\\b\\f\\n\\r\\t\"}",
        "{\"b\":2,\"a\":1,\"aa\":3,\"A\":4}",
        "[[[[[null]]]]]", "true", "\"text\"", "9223372036854775807",
        "-9223372036854775808", "{\"x\":1} trailing",
        "{\"x\":1,\"x\":2}", "[1,]", "{\"x\":}", "\"\\uD800\""
    };
    static const size_t load_flags[] = {
        0, JSON_DECODE_ANY, JSON_ALLOW_NUL | JSON_DECODE_ANY,
        JSON_REJECT_DUPLICATES | JSON_DECODE_ANY,
        JSON_DISABLE_EOF_CHECK | JSON_DECODE_ANY
    };
    static const size_t dump_flags[] = {
        JSON_COMPACT, JSON_SORT_KEYS, JSON_INDENT(1),
        JSON_ENSURE_ASCII | JSON_COMPACT,
        JSON_ESCAPE_SLASH | JSON_COMPACT,
        JSON_ENCODE_ANY | JSON_EMBED | JSON_COMPACT
    };
    for (size_t i = 0; i < sizeof(inputs) / sizeof(inputs[0]); i++) {
        for (size_t j = 0; j < sizeof(load_flags) / sizeof(load_flags[0]); j++) {
            json_error_t error;
            json_t *value = json_loads(inputs[i], load_flags[j], &error);
            if (!value) {
                printf("%zu/%zu=E:%d:%d:%d:%s\n", i, j, error.line,
                       error.column, json_error_code(&error), error.text);
                continue;
            }
            printf("%zu/%zu=V", i, j);
            for (size_t k = 0; k < sizeof(dump_flags) / sizeof(dump_flags[0]); k++) {
                char *text = json_dumps(value, dump_flags[k]);
                printf("|%s", text ? text : "<null>");
                free(text);
            }
            puts("");
            json_decref(value);
        }
    }
    return 0;
}
