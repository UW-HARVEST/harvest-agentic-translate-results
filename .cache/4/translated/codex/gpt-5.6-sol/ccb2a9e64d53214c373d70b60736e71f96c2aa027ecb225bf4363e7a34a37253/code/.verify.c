#include "jansson.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void print_dump(const char *name, json_t *value, size_t flags) {
    char *text = json_dumps(value, flags);
    printf("%s=%s\n", name, text ? text : "<null>");
    free(text);
}

static size_t allocation_count;
static void *count_malloc(size_t size) {
    allocation_count++;
    return malloc(size);
}
static void *count_realloc(void *ptr, size_t size) {
    allocation_count++;
    return realloc(ptr, size);
}
static void count_free(void *ptr) {
    free(ptr);
}

int main(void) {
    json_error_t error;
    printf("version=%s,%d,%d\n", jansson_version_str(),
           jansson_version_cmp(2, 15, 0), jansson_version_cmp(2, 14, 9));

    const char input[] =
        "{\"z\":1,\"text\":\"h\xC3\xA9llo\\n\",\"a\":[true,false,null,-12,1.25e3]}";
    json_t *root = json_loads(input, 0, &error);
    printf("load=%d,%zu,%d\n", root != NULL, json_object_size(root),
           json_error_code(&error));
    print_dump("compact", root, JSON_COMPACT);
    print_dump("sorted", root, JSON_SORT_KEYS | JSON_ENSURE_ASCII);
    print_dump("indent", root, JSON_INDENT(2) | JSON_SORT_KEYS);

    json_t *copy = json_deep_copy(root);
    printf("copy=%d,%d\n", json_equal(root, copy), copy != root);
    json_t *array = json_object_get(root, "a");
    json_array_insert_new(array, 1, json_integer(44));
    json_array_set_new(array, 0, json_string("first"));
    json_array_remove(array, 2);
    print_dump("mutated", root, JSON_COMPACT);

    json_t *extra = json_object();
    json_object_set_new(extra, "nested", json_pack("{s:i,s:s}", "x", 7, "q", "v"));
    json_object_set_new(extra, "z", json_integer(99));
    json_object_update_recursive(root, extra);
    print_dump("updated", root, JSON_COMPACT | JSON_SORT_KEYS);

    printf("iter=");
    void *iter = json_object_iter(root);
    while (iter) {
        printf("%.*s:%d;", (int)json_object_iter_key_len(iter),
               json_object_iter_key(iter),
               (int)json_typeof(json_object_iter_value(iter)));
        iter = json_object_iter_next(root, iter);
    }
    puts("");

    json_t *packed = json_pack("[s,i,I,f,b,n,{s:s#}]",
                               "word", -4, (json_int_t)1234567890123LL,
                               2.5, 1, "key", "abcdef", 3);
    print_dump("packed", packed, JSON_COMPACT);
    const char *word = NULL;
    int small = 0;
    json_int_t large = 0;
    double real = 0;
    int boolean = 0;
    int unpack_result =
        json_unpack(packed, "[s,i,I,F,b,n,o]", &word, &small, &large,
                    &real, &boolean, &extra);
    printf("unpack=%d,%s,%d,%lld,%.3f,%d,%d\n", unpack_result,
           word ? word : "<null>", small, (long long)large, real, boolean,
           extra != NULL);

    json_t *formatted = json_sprintf("%s:%d:%.2f", "fmt", 17, 1.5);
    print_dump("sprintf", formatted, JSON_ENCODE_ANY);

    json_t *bad = json_loads("{\"a\":1,\"a\":2}", JSON_REJECT_DUPLICATES, &error);
    printf("bad=%d,%d,%d,%d,%s\n", bad == NULL, error.line, error.column,
           json_error_code(&error), error.text);

    char buffer[512];
    size_t needed = json_dumpb(root, buffer, sizeof(buffer), JSON_COMPACT);
    printf("dumpb=%zu,%.*s\n", needed, (int)needed, buffer);

    json_decref(formatted);
    json_decref(packed);
    json_decref(extra);
    json_decref(copy);
    json_decref(root);

    json_set_alloc_funcs2(count_malloc, count_realloc, count_free);
    json_t *allocated = json_pack("{s:[i,i,i]}", "x", 1, 2, 3);
    char *allocated_dump = json_dumps(allocated, JSON_COMPACT);
    printf("allocator=%zu,%s\n", allocation_count, allocated_dump);
    count_free(allocated_dump);
    json_decref(allocated);
    return 0;
}
