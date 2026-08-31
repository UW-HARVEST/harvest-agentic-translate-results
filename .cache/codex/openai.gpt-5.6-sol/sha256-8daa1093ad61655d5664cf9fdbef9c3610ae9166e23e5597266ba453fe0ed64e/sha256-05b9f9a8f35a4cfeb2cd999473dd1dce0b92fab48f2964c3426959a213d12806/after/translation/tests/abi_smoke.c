#include "jansson.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static void print_dump(const char *label, json_t *value, size_t flags) {
    char *dump = json_dumps(value, flags);
    printf("%s=%s\n", label, dump ? dump : "(null)");
    if (dump)
        free(dump);
}

struct callback_input {
    const char *data;
    size_t length;
    size_t offset;
};

static size_t load_callback(void *buffer, size_t buflen, void *opaque) {
    struct callback_input *input = opaque;
    size_t left = input->length - input->offset;
    size_t amount = left < 3 ? left : 3;
    if (amount > buflen)
        amount = buflen;
    memcpy(buffer, input->data + input->offset, amount);
    input->offset += amount;
    return amount;
}

struct dump_output {
    char data[512];
    size_t length;
};

static int dump_callback(const char *buffer, size_t size, void *opaque) {
    struct dump_output *output = opaque;
    memcpy(output->data + output->length, buffer, size);
    output->length += size;
    return 0;
}

int main(void) {
    static const char input[] =
        "{\"z\":-0.0,\"unicode\":\"\\u20ac\\/x\",\"array\":[1,2.5,true,false,null],"
        "\"nested\":{\"b\":2,\"a\":1}}";
    json_error_t error;
    json_t *root = json_loadb(input, sizeof(input) - 1, 0, &error);
    if (!root) {
        printf("unexpected_error=%d:%d:%d:%s\n", error.line, error.column,
               error.position, error.text);
        return 1;
    }

    printf("version=%s:%d:%d\n", jansson_version_str(),
           jansson_version_cmp(2, 15, 0), jansson_version_cmp(2, 14, 99));
    print_dump("default", root, 0);
    print_dump("compact_sorted_ascii", root,
               JSON_COMPACT | JSON_SORT_KEYS | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH);
    print_dump("indent", root, JSON_INDENT(2) | JSON_SORT_KEYS);
    print_dump("precision", json_real(1.0 / 7.0), JSON_ENCODE_ANY | JSON_REAL_PRECISION(6));

    char small[19];
    memset(small, '#', sizeof(small));
    size_t needed = json_dumpb(root, small, sizeof(small), JSON_COMPACT);
    printf("dumpb=%zu:%.*s:%02x\n", needed, (int)sizeof(small), small,
           (unsigned char)small[sizeof(small) - 1]);

    json_t *object = json_object();
    json_object_set_new(object, "first", json_integer(INT64_C(9223372036854775807)));
    json_object_setn_new(object, "nul\0key", 7, json_stringn_nocheck("v\0x", 3));
    json_t *array = json_array();
    json_array_append_new(array, json_string("zero"));
    json_array_insert_new(array, 0, json_integer(-4));
    json_array_set_new(array, 1, json_real(3.25));
    json_object_set_new(object, "array", array);
    print_dump("constructed", object, JSON_COMPACT | JSON_ENCODE_ANY);
    printf("object=%zu:%zu:%lld:%.2f\n", json_object_size(object),
           json_array_size(array), (long long)json_integer_value(json_object_get(object, "first")),
           json_number_value(json_array_get(array, 1)));

    const char *key;
    void *iter = json_object_iter(object);
    printf("iteration=");
    while (iter) {
        key = json_object_iter_key(iter);
        printf("%zu:%.*s;", json_object_iter_key_len(iter),
               (int)json_object_iter_key_len(iter), key);
        iter = json_object_iter_next(object, iter);
    }
    printf("\n");

    json_t *packed =
        json_pack("{s:i,s:[s,b,n],s:f}", "n", 7, "a", "x", 1, "r", 1.25);
    print_dump("packed", packed, JSON_COMPACT | JSON_SORT_KEYS);
    int n = 0, boolean = 0;
    const char *string = NULL;
    double real = 0.0;
    int unpack_result = json_unpack(
        packed, "{s:i,s:[s,b,n],s:f}", "n", &n, "a", &string, &boolean, "r", &real);
    printf("unpack=%d:%d:%s:%d:%.2f\n", unpack_result, n, string, boolean, real);

    json_t *formatted = json_sprintf("%s:%04d:%.2f", "fmt", 9, 2.5);
    print_dump("sprintf", formatted, JSON_ENCODE_ANY);

    const char bad[] = "{\"a\":1,\"a\":2,}";
    json_t *invalid = json_loadb(bad, sizeof(bad) - 1, JSON_REJECT_DUPLICATES, &error);
    printf("invalid=%p:%d:%d:%d:%d:%s\n", (void *)invalid, error.line, error.column,
           error.position, (unsigned char)error.text[JSON_ERROR_TEXT_LENGTH - 1],
           error.text);

    struct callback_input callback_input = {
        .data = "[\"callback\",123]",
        .length = strlen("[\"callback\",123]"),
        .offset = 0,
    };
    json_t *callback_root = json_load_callback(load_callback, &callback_input, 0, &error);
    struct dump_output callback_output = {{0}, 0};
    int callback_result =
        json_dump_callback(callback_root, dump_callback, &callback_output, JSON_COMPACT);
    printf("callback=%d:%zu:%.*s\n", callback_result, callback_output.length,
           (int)callback_output.length, callback_output.data);

    int fds[2];
    pipe(fds);
    json_dumpfd(callback_root, fds[1], JSON_COMPACT);
    close(fds[1]);
    char fd_buffer[128] = {0};
    ssize_t fd_length = read(fds[0], fd_buffer, sizeof(fd_buffer));
    close(fds[0]);
    printf("dumpfd=%zd:%.*s\n", fd_length, (int)fd_length, fd_buffer);

    printf("equal_copy=%d:%d\n", json_equal(root, json_deep_copy(root)),
           json_equal(root, json_copy(root)));

    json_decref(callback_root);
    json_decref(formatted);
    json_decref(packed);
    json_decref(object);
    json_decref(root);
    return 0;
}
