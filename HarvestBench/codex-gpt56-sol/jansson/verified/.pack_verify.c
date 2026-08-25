#include "jansson.h"
#include <stdio.h>
#include <stdlib.h>

static void show(const char *name, json_t *value, json_error_t *error) {
    char *text = value ? json_dumps(value, JSON_COMPACT | JSON_ENCODE_ANY) : NULL;
    printf("%s=%s", name, text ? text : "<null>");
    if (!value && error)
        printf(",%d,%s", json_error_code(error), error->text);
    puts("");
    free(text);
    json_decref(value);
}

int main(void) {
    json_error_t error;
    show("lengths", json_pack_ex(&error, 0, "[s#,s%,s+s#]",
                                 "abcdef", 3, "uvwxyz", (size_t)4,
                                 "ab", "cdef", 2), &error);
    show("optional-null", json_pack_ex(&error, 0, "[s?,s*,O?,o*]",
                                      NULL, NULL, NULL, NULL), &error);
    show("optional-object", json_pack_ex(&error, 0, "{s:o*,s:i}",
                                        "skip", NULL, "keep", 4), &error);
    show("bad-null", json_pack_ex(&error, 0, "{s:s}", "x", NULL), &error);
    show("bad-format", json_pack_ex(&error, 0, "[z]"), &error);

    json_t *root = json_pack("{s:[i,s,{s:b}],s:f}",
                             "items", 7, "word", "yes", 1, "real", 2.5);
    int integer = 0, boolean = 0;
    const char *string = NULL;
    double real = 0;
    json_t *borrowed = NULL;
    printf("unpack=%d,", json_unpack_ex(root, &error, JSON_STRICT,
                                        "{s:[i,s,{s:b}],s:F}",
                                        "items", &integer, &string, "yes",
                                        &boolean, "real", &real));
    printf("%d,%s,%d,%.1f\n", integer, string, boolean, real);
    printf("optional=%d\n", json_unpack_ex(root, &error, 0,
                                           "{s?:s,s:o}",
                                           "missing", &string,
                                           "items", &borrowed));
    printf("strict=%d,%d,%s\n", json_unpack_ex(root, &error, JSON_STRICT,
                                               "{s:o}", "items", &borrowed),
           json_error_code(&error), error.text);
    json_decref(root);
    return 0;
}
