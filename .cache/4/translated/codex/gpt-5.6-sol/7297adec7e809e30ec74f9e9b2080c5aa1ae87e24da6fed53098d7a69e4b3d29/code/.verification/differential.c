#define _GNU_SOURCE

#include <dlfcn.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    size_t length;
    size_t capacity;
    void *hash_table;
    ptrdiff_t temp;
} ArrayHeader;

typedef struct StringBlock StringBlock;
typedef struct {
    StringBlock *storage;
    size_t remaining;
    unsigned char block;
    unsigned char mode;
} StringArena;

typedef struct {
    uint64_t key;
    uint64_t value;
} BinaryEntry;

typedef struct {
    char *key;
    uint64_t value;
} StringEntry;

typedef struct {
    void *handle;
    void *(*arrgrowf)(void *, size_t, size_t, size_t);
    void (*arrfreef)(void *);
    void (*rand_seed)(size_t);
    size_t (*hash_bytes)(void *, size_t, size_t);
    size_t (*hash_string)(char *, size_t);
    void (*hmfree_func)(void *, size_t);
    void *(*hmget_key)(void *, size_t, void *, size_t, int);
    void *(*hmget_key_ts)(void *, size_t, void *, size_t, ptrdiff_t *, int);
    void *(*hmput_default)(void *, size_t);
    void *(*hmput_key)(void *, size_t, void *, size_t, int);
    void *(*hmdel_key)(void *, size_t, void *, size_t, size_t, int);
    void *(*shmode_func)(size_t, int);
    char *(*stralloc)(StringArena *, char *);
    void (*strreset)(StringArena *);
    char *(*strkey)(int);
} Api;

static void fail(const char *message)
{
    fprintf(stderr, "FAIL: %s\n", message);
    exit(1);
}

static void require(int condition, const char *message)
{
    if (!condition)
        fail(message);
}

static void load_symbol(void *handle, void *destination, const char *name)
{
    void *symbol = dlsym(handle, name);
    if (!symbol) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(1);
    }
    memcpy(destination, &symbol, sizeof(symbol));
}

static Api load_api(const char *path)
{
    Api api = {0};
    api.handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (!api.handle) {
        fprintf(stderr, "dlopen %s: %s\n", path, dlerror());
        exit(1);
    }

#define LOAD(field, name) load_symbol(api.handle, &api.field, name)
    LOAD(arrgrowf, "stbds_arrgrowf");
    LOAD(arrfreef, "stbds_arrfreef");
    LOAD(rand_seed, "stbds_rand_seed");
    LOAD(hash_bytes, "stbds_hash_bytes");
    LOAD(hash_string, "stbds_hash_string");
    LOAD(hmfree_func, "stbds_hmfree_func");
    LOAD(hmget_key, "stbds_hmget_key");
    LOAD(hmget_key_ts, "stbds_hmget_key_ts");
    LOAD(hmput_default, "stbds_hmput_default");
    LOAD(hmput_key, "stbds_hmput_key");
    LOAD(hmdel_key, "stbds_hmdel_key");
    LOAD(shmode_func, "stbds_shmode_func");
    LOAD(stralloc, "stbds_stralloc");
    LOAD(strreset, "stbds_strreset");
    LOAD(strkey, "strkey");
#undef LOAD
    return api;
}

static ArrayHeader *header(void *array)
{
    return (ArrayHeader *)array - 1;
}

static void *raw_array(void *hash, size_t element_size)
{
    return (char *)hash - element_size;
}

static void compare_array_header(void *left, void *right)
{
    require((left == NULL) == (right == NULL), "array nullness differs");
    if (left == NULL)
        return;
    ArrayHeader *a = header(left);
    ArrayHeader *b = header(right);
    require(a->length == b->length, "array length differs");
    require(a->capacity == b->capacity, "array capacity differs");
    require(a->temp == b->temp, "array temp differs");
    require((a->hash_table == NULL) == (b->hash_table == NULL),
            "array hash-table presence differs");
}

static void test_hashes(Api *c, Api *rust)
{
    unsigned char data[320];
    const size_t seeds[] = {
        0,
        1,
        0x31415926u,
        (size_t)0x0123456789abcdefULL,
        ~(size_t)0,
    };
    char high_bytes[] = {(char)0x80, (char)0xff, 'x', '\0'};
    char *strings[] = {"", "a", "test_0", "a longer hash-table key", high_bytes};

    for (size_t i = 0; i < sizeof(data); ++i)
        data[i] = (unsigned char)(i * 73u + 19u);

    for (size_t seed_index = 0; seed_index < sizeof(seeds) / sizeof(seeds[0]);
         ++seed_index) {
        for (size_t length = 0; length <= sizeof(data); ++length) {
            size_t a = c->hash_bytes(data, length, seeds[seed_index]);
            size_t b = rust->hash_bytes(data, length, seeds[seed_index]);
            if (a != b) {
                fprintf(stderr,
                        "byte hash differs at seed=%#zx length=%zu: C=%#zx Rust=%#zx\n",
                        seeds[seed_index], length, a, b);
                fail("byte hash differs");
            }
        }
        for (size_t string_index = 0;
             string_index < sizeof(strings) / sizeof(strings[0]); ++string_index) {
            size_t a = c->hash_string(strings[string_index], seeds[seed_index]);
            size_t b = rust->hash_string(strings[string_index], seeds[seed_index]);
            require(a == b, "string hash differs");
        }
    }
}

static void test_strkey(Api *c, Api *rust)
{
    const int values[] = {INT_MIN, -1000000, -1, 0, 1, 42, 1000000, INT_MAX};
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); ++i) {
        char expected[256];
        strcpy(expected, c->strkey(values[i]));
        require(strcmp(expected, rust->strkey(values[i])) == 0, "strkey differs");
    }
}

static void test_arrays(Api *c, Api *rust)
{
    const size_t requests[][2] = {
        {0, 0}, {1, 0}, {3, 0}, {1, 9}, {20, 0}, {0, 100}, {150, 0},
    };
    uint64_t *a = NULL;
    uint64_t *b = NULL;

    for (size_t request = 0; request < sizeof(requests) / sizeof(requests[0]);
         ++request) {
        a = c->arrgrowf(a, sizeof(*a), requests[request][0], requests[request][1]);
        b = rust->arrgrowf(b, sizeof(*b), requests[request][0], requests[request][1]);
        compare_array_header(a, b);
        if (a == NULL)
            continue;

        size_t old_length = header(a)->length;
        size_t added = requests[request][0];
        if (old_length + added <= header(a)->capacity) {
            for (size_t i = 0; i < added; ++i) {
                a[old_length + i] = 0x9e3779b97f4a7c15ULL ^ (old_length + i);
                b[old_length + i] = a[old_length + i];
            }
            header(a)->length += added;
            header(b)->length += added;
        }
        compare_array_header(a, b);
        require(memcmp(a, b, header(a)->length * sizeof(*a)) == 0,
                "array payload differs");
    }

    c->arrfreef(a);
    rust->arrfreef(b);
}

static void test_arenas(Api *c, Api *rust)
{
    const size_t lengths[] = {1, 12, 700, 400, 3000, 63, 700000, 1100000, 9};
    StringArena a = {0};
    StringArena b = {0};

    for (size_t i = 0; i < sizeof(lengths) / sizeof(lengths[0]); ++i) {
        char *input = malloc(lengths[i] + 1);
        require(input != NULL, "test allocation failed");
        for (size_t j = 0; j < lengths[i]; ++j)
            input[j] = (char)('a' + (j * 11 + i) % 26);
        input[lengths[i]] = '\0';

        char *left = c->stralloc(&a, input);
        char *right = rust->stralloc(&b, input);
        require(strcmp(left, right) == 0, "arena string differs");
        require(strcmp(left, input) == 0, "arena string content is wrong");
        require(a.remaining == b.remaining, "arena remaining differs");
        require(a.block == b.block, "arena block progression differs");
        require(a.mode == b.mode, "arena mode differs");
        free(input);
    }

    c->strreset(&a);
    rust->strreset(&b);
    require(memcmp(&a, &b, sizeof(a)) == 0, "reset arena differs");
}

static void compare_binary_maps(BinaryEntry *a, BinaryEntry *b)
{
    void *raw_a = raw_array(a, sizeof(*a));
    void *raw_b = raw_array(b, sizeof(*b));
    compare_array_header(raw_a, raw_b);
    require(memcmp(raw_a, raw_b, header(raw_a)->length * sizeof(*a)) == 0,
            "binary map entries differ");
}

static void test_binary_maps(Api *c, Api *rust)
{
    BinaryEntry *a = NULL;
    BinaryEntry *b = NULL;

    c->rand_seed(0x1020304050607080ULL);
    rust->rand_seed(0x1020304050607080ULL);
    a = c->hmput_default(a, sizeof(*a));
    b = rust->hmput_default(b, sizeof(*b));
    compare_binary_maps(a, b);
    a[-1].value = 0xfeedface;
    b[-1].value = 0xfeedface;

    for (uint64_t i = 0; i < 320; ++i) {
        uint64_t key = i * 0x9e3779b97f4a7c15ULL + 17;
        a = c->hmput_key(a, sizeof(*a), &key, sizeof(key), 0);
        b = rust->hmput_key(b, sizeof(*b), &key, sizeof(key), 0);
        require(header(raw_array(a, sizeof(*a)))->temp ==
                    header(raw_array(b, sizeof(*b)))->temp,
                "binary insertion index differs");
        ptrdiff_t index = header(raw_array(a, sizeof(*a)))->temp;
        a[index].value = i ^ 0xa5a5a5a5;
        b[index].value = i ^ 0xa5a5a5a5;
        compare_binary_maps(a, b);
    }

    for (uint64_t i = 0; i < 340; ++i) {
        uint64_t key = i * 0x9e3779b97f4a7c15ULL + 17;
        ptrdiff_t left_index = 99;
        ptrdiff_t right_index = 98;
        a = c->hmget_key_ts(a, sizeof(*a), &key, sizeof(key), &left_index, 0);
        b = rust->hmget_key_ts(b, sizeof(*b), &key, sizeof(key), &right_index, 0);
        require(left_index == right_index, "thread-safe lookup index differs");
        a = c->hmget_key(a, sizeof(*a), &key, sizeof(key), 0);
        b = rust->hmget_key(b, sizeof(*b), &key, sizeof(key), 0);
        compare_binary_maps(a, b);
    }

    for (uint64_t i = 0; i < 320; i += 2) {
        uint64_t key = i * 0x9e3779b97f4a7c15ULL + 17;
        a = c->hmdel_key(a, sizeof(*a), &key, sizeof(key), 0, 0);
        b = rust->hmdel_key(b, sizeof(*b), &key, sizeof(key), 0, 0);
        compare_binary_maps(a, b);
    }

    c->hmfree_func(raw_array(a, sizeof(*a)), sizeof(*a));
    rust->hmfree_func(raw_array(b, sizeof(*b)), sizeof(*b));

    ptrdiff_t left_index = 0;
    ptrdiff_t right_index = 0;
    uint64_t missing = 123;
    a = c->hmget_key_ts(NULL, sizeof(*a), &missing, sizeof(missing), &left_index, 0);
    b = rust->hmget_key_ts(NULL, sizeof(*b), &missing, sizeof(missing),
                           &right_index, 0);
    require(left_index == right_index && left_index == -1,
            "null lookup result differs");
    compare_binary_maps(a, b);
    c->hmfree_func(raw_array(a, sizeof(*a)), sizeof(*a));
    rust->hmfree_func(raw_array(b, sizeof(*b)), sizeof(*b));
}

static void compare_string_maps(StringEntry *a, StringEntry *b)
{
    void *raw_a = raw_array(a, sizeof(*a));
    void *raw_b = raw_array(b, sizeof(*b));
    compare_array_header(raw_a, raw_b);
    size_t count = header(raw_a)->length - 1;
    for (size_t i = 0; i < count; ++i) {
        require(strcmp(a[i].key, b[i].key) == 0, "string map key differs");
        require(a[i].value == b[i].value, "string map value differs");
    }
}

static void test_string_mode(Api *c, Api *rust, int string_mode)
{
    char keys[220][32];
    StringEntry *a;
    StringEntry *b;

    for (size_t i = 0; i < 220; ++i)
        snprintf(keys[i], sizeof(keys[i]), "key_%03zu_%08zx", i, i * 2654435761u);

    c->rand_seed(0x8877665544332211ULL + (size_t)string_mode);
    rust->rand_seed(0x8877665544332211ULL + (size_t)string_mode);
    if (string_mode == 1) {
        a = NULL;
        b = NULL;
    } else {
        a = c->shmode_func(sizeof(*a), string_mode);
        b = rust->shmode_func(sizeof(*b), string_mode);
        compare_string_maps(a, b);
    }

    for (size_t i = 0; i < 220; ++i) {
        a = c->hmput_key(a, sizeof(*a), keys[i], sizeof(a->key), 1);
        b = rust->hmput_key(b, sizeof(*b), keys[i], sizeof(b->key), 1);
        ptrdiff_t left_index = header(raw_array(a, sizeof(*a)))->temp;
        ptrdiff_t right_index = header(raw_array(b, sizeof(*b)))->temp;
        require(left_index == right_index, "string insertion index differs");
        a[left_index].value = i * 7 + 3;
        b[right_index].value = i * 7 + 3;
        compare_string_maps(a, b);
    }

    for (size_t i = 0; i < 220; i += 3) {
        ptrdiff_t left_index;
        ptrdiff_t right_index;
        a = c->hmget_key_ts(a, sizeof(*a), keys[i], sizeof(a->key),
                            &left_index, 1);
        b = rust->hmget_key_ts(b, sizeof(*b), keys[i], sizeof(b->key),
                               &right_index, 1);
        require(left_index == right_index, "string lookup index differs");
    }

    for (size_t i = 1; i < 220; i += 2) {
        a = c->hmdel_key(a, sizeof(*a), keys[i], sizeof(a->key), 0, 1);
        b = rust->hmdel_key(b, sizeof(*b), keys[i], sizeof(b->key), 0, 1);
        compare_string_maps(a, b);
    }

    c->hmfree_func(raw_array(a, sizeof(*a)), sizeof(*a));
    rust->hmfree_func(raw_array(b, sizeof(*b)), sizeof(*b));
}

int main(int argc, char **argv)
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    Api c = load_api(argv[1]);
    Api rust = load_api(argv[2]);
    fprintf(stderr, "hashes\n");
    test_hashes(&c, &rust);
    fprintf(stderr, "strkey\n");
    test_strkey(&c, &rust);
    fprintf(stderr, "arrays\n");
    test_arrays(&c, &rust);
    fprintf(stderr, "arenas\n");
    test_arenas(&c, &rust);
    fprintf(stderr, "binary maps\n");
    test_binary_maps(&c, &rust);
    fprintf(stderr, "string default\n");
    test_string_mode(&c, &rust, 1);
    fprintf(stderr, "string strdup\n");
    test_string_mode(&c, &rust, 2);
    fprintf(stderr, "string arena\n");
    test_string_mode(&c, &rust, 3);
    dlclose(rust.handle);
    dlclose(c.handle);
    puts("differential checks passed");
    return 0;
}
