#include <dlfcn.h>
#include <inttypes.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct string_block {
    struct string_block *next;
    char storage[8];
} string_block;

typedef struct {
    string_block *storage;
    size_t remaining;
    unsigned char block;
    unsigned char mode;
} string_arena;

typedef struct {
    size_t length;
    size_t capacity;
    void *hash_table;
    ptrdiff_t temp;
} array_header;

typedef struct {
    size_t hash[8];
    ptrdiff_t index[8];
} hash_bucket;

typedef struct {
    char *temp_key;
    size_t slot_count;
    size_t used_count;
    size_t used_count_threshold;
    size_t used_count_shrink_threshold;
    size_t tombstone_count;
    size_t tombstone_count_threshold;
    size_t seed;
    size_t slot_count_log2;
    string_arena string;
    hash_bucket *storage;
} hash_index;

typedef void *(*arrgrowf_fn)(void *, size_t, size_t, size_t);
typedef void (*arrfreef_fn)(void *);
typedef void (*rand_seed_fn)(size_t);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef void *(*hmget_key_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmget_key_ts_fn)(void *, size_t, void *, size_t, ptrdiff_t *, int);
typedef void *(*hmput_default_fn)(void *, size_t);
typedef void *(*hmput_key_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmdel_key_fn)(void *, size_t, void *, size_t, size_t, int);
typedef void (*hmfree_func_fn)(void *, size_t);
typedef void *(*shmode_func_fn)(size_t, int);
typedef char *(*stralloc_fn)(string_arena *, char *);
typedef void (*strreset_fn)(string_arena *);
typedef char *(*strkey_fn)(int);
typedef void (*arr_del_fn)(int);

typedef struct {
    arrgrowf_fn arrgrowf;
    arrfreef_fn arrfreef;
    rand_seed_fn rand_seed;
    hash_bytes_fn hash_bytes;
    hash_string_fn hash_string;
    hmget_key_fn hmget_key;
    hmget_key_ts_fn hmget_key_ts;
    hmput_default_fn hmput_default;
    hmput_key_fn hmput_key;
    hmdel_key_fn hmdel_key;
    hmfree_func_fn hmfree_func;
    shmode_func_fn shmode_func;
    stralloc_fn stralloc;
    strreset_fn strreset;
    strkey_fn strkey;
    arr_del_fn arr_del;
} api;

#define LOAD(api_value, handle, field, symbol)                              \
    do {                                                                    \
        *(void **) &(api_value).field = dlsym((handle), (symbol));          \
        if (!(api_value).field) {                                           \
            fprintf(stderr, "missing %s: %s\n", (symbol), dlerror());       \
            exit(2);                                                        \
        }                                                                   \
    } while (0)

static array_header *header(void *array)
{
    return (array_header *) array - 1;
}

static void *raw_hash_array(void *array, size_t element_size)
{
    return (unsigned char *) array - element_size;
}

static void print_bytes(const void *pointer, size_t length)
{
    const unsigned char *bytes = pointer;
    for (size_t i = 0; i < length; ++i)
        printf("%02x", bytes[i]);
}

static void test_hashes(api *library)
{
    unsigned char bytes[96];
    char strings[][32] = {
        "",
        "a",
        "test_0",
        "The quick brown fox",
        "\x7f\x80\xfe\xff",
    };
    size_t seeds[] = {0, 1, 0x31415926u, SIZE_MAX, UINT64_C(0x123456789abcdef0)};

    for (size_t i = 0; i < sizeof(bytes); ++i)
        bytes[i] = (unsigned char) (i * 73u + 0x81u);

    puts("hash-string");
    for (size_t seed_index = 0; seed_index < sizeof(seeds) / sizeof(seeds[0]); ++seed_index) {
        for (size_t string_index = 0; string_index < sizeof(strings) / sizeof(strings[0]); ++string_index) {
            printf("%016zx\n", library->hash_string(strings[string_index], seeds[seed_index]));
        }
    }

    puts("hash-bytes");
    for (size_t seed_index = 0; seed_index < sizeof(seeds) / sizeof(seeds[0]); ++seed_index) {
        for (size_t length = 0; length <= sizeof(bytes); ++length)
            printf("%02zu:%016zx\n", length, library->hash_bytes(bytes, length, seeds[seed_index]));
    }
}

static void test_strkey(api *library)
{
    int values[] = {INT_MIN, -1000000, -1, 0, 1, 42, INT_MAX};
    puts("strkey");
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); ++i)
        printf("%s\n", library->strkey(values[i]));
}

static void test_array(api *library)
{
    uint32_t *array = NULL;
    size_t requests[][2] = {{1, 0}, {3, 0}, {1, 20}, {30, 0}, {0, 100}};

    puts("array");
    array = library->arrgrowf(array, sizeof(*array), 0, 0);
    printf("zero:%d\n", array == NULL);
    for (size_t request = 0; request < sizeof(requests) / sizeof(requests[0]); ++request) {
        array = library->arrgrowf(array, sizeof(*array), requests[request][0], requests[request][1]);
        array_header *metadata = header(array);
        while (metadata->length < metadata->capacity &&
               metadata->length < request * 7 + 3) {
            array[metadata->length] = UINT32_C(0x9e3779b9) ^ (uint32_t) metadata->length;
            ++metadata->length;
        }
        printf("%zu:%zu:", metadata->length, metadata->capacity);
        print_bytes(array, metadata->length * sizeof(*array));
        putchar('\n');
    }
    library->arrfreef(array);
    library->arr_del(INT_MIN);
    library->arr_del(17);
    puts("arr-del:ok");
}

static void test_arena(api *library)
{
    string_arena arena = {0};
    char small[][24] = {"alpha", "beta", "", "gamma-delta"};
    char *large = malloc(1401);
    memset(large, 'Q', 1400);
    large[1400] = '\0';

    puts("arena");
    for (size_t round = 0; round < 20; ++round) {
        char *source = small[round % (sizeof(small) / sizeof(small[0]))];
        char *result = library->stralloc(&arena, source);
        printf("%zu:%u:%zu:%s\n", round, arena.block, arena.remaining, result);
    }
    char *large_result = library->stralloc(&arena, large);
    printf("large:%u:%zu:%zu:%c:%c\n", arena.block, arena.remaining,
           strlen(large_result), large_result[0], large_result[1399]);
    library->strreset(&arena);
    printf("reset:");
    print_bytes(&arena, sizeof(arena));
    putchar('\n');
    free(large);
}

typedef struct {
    uint64_t key;
    uint32_t value;
    unsigned char tag[4];
} binary_entry;

static hash_index *table_for(void *map, size_t element_size)
{
    void *raw = raw_hash_array(map, element_size);
    return header(raw)->hash_table;
}

static void print_binary_map(binary_entry *map)
{
    void *raw = raw_hash_array(map, sizeof(*map));
    array_header *metadata = header(raw);
    hash_index *table = metadata->hash_table;
    printf("meta:%zu:%zu:%td", metadata->length - 1, metadata->capacity, metadata->temp);
    if (table) {
        printf(":%zu:%zu:%zu:%zu:%zu:%zu:%016zx",
               table->slot_count, table->used_count,
               table->used_count_threshold, table->used_count_shrink_threshold,
               table->tombstone_count, table->tombstone_count_threshold,
               table->seed);
    }
    putchar('\n');

    for (size_t i = 0; i + 1 < metadata->length; ++i) {
        printf("entry:%zu:%016" PRIx64 ":%08" PRIx32 ":", i, map[i].key, map[i].value);
        print_bytes(map[i].tag, sizeof(map[i].tag));
        putchar('\n');
    }
    if (table) {
        for (size_t i = 0; i < table->slot_count; ++i) {
            hash_bucket *bucket = &table->storage[i >> 3];
            size_t slot = i & 7;
            printf("slot:%zu:%016zx:%td\n", i, bucket->hash[slot], bucket->index[slot]);
        }
    }
}

static void test_binary_map(api *library)
{
    binary_entry *map = NULL;
    uint64_t missing = UINT64_C(0xffffeeee11112222);
    ptrdiff_t temporary = 99;

    puts("binary-map");
    map = library->hmget_key_ts(map, sizeof(*map), &missing, sizeof(missing), &temporary, 0);
    printf("initial:%td:%zu:%zu\n", temporary,
           header(raw_hash_array(map, sizeof(*map)))->length,
           header(raw_hash_array(map, sizeof(*map)))->capacity);
    map = library->hmput_default(map, sizeof(*map));
    map[-1].key = UINT64_C(0xdddddddddddddddd);
    map[-1].value = UINT32_C(0xabcdef01);

    library->rand_seed(UINT64_C(0x1020304050607080));
    for (uint64_t i = 0; i < 48; ++i) {
        uint64_t key = (i * UINT64_C(0x9e3779b97f4a7c15)) ^ (i << 33);
        map = library->hmput_key(map, sizeof(*map), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_hash_array(map, sizeof(*map)))->temp;
        map[index].value = (uint32_t) (i * 101u + 7u);
        for (size_t j = 0; j < sizeof(map[index].tag); ++j)
            map[index].tag[j] = (unsigned char) (i + j * 17u);
    }
    print_binary_map(map);

    for (uint64_t i = 0; i < 48; i += 5) {
        uint64_t key = (i * UINT64_C(0x9e3779b97f4a7c15)) ^ (i << 33);
        map = library->hmget_key(map, sizeof(*map), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_hash_array(map, sizeof(*map)))->temp;
        printf("get:%" PRIu64 ":%td:%08" PRIx32 "\n", i, index, map[index].value);
    }
    map = library->hmget_key(map, sizeof(*map), &missing, sizeof(missing), 0);
    printf("missing:%td:%08" PRIx32 "\n",
           header(raw_hash_array(map, sizeof(*map)))->temp, map[-1].value);

    for (uint64_t i = 0; i < 48; i += 2) {
        uint64_t key = (i * UINT64_C(0x9e3779b97f4a7c15)) ^ (i << 33);
        map = library->hmdel_key(map, sizeof(*map), &key, sizeof(key), 0, 0);
        printf("del:%" PRIu64 ":%td:%zu:%zu\n", i,
               header(raw_hash_array(map, sizeof(*map)))->temp,
               header(raw_hash_array(map, sizeof(*map)))->length - 1,
               table_for(map, sizeof(*map))->slot_count);
    }
    print_binary_map(map);
    library->hmfree_func(raw_hash_array(map, sizeof(*map)), sizeof(*map));
}

typedef struct {
    char *key;
    uint64_t value;
} string_entry;

static void test_string_mode(api *library, int allocation_mode)
{
    char keys[36][32];
    string_entry *map = NULL;
    if (allocation_mode)
        map = library->shmode_func(sizeof(*map), allocation_mode);

    for (size_t i = 0; i < 36; ++i) {
        snprintf(keys[i], sizeof(keys[i]), "key_%02zu_%c", i, (char) ('A' + i % 26));
        map = library->hmput_key(map, sizeof(*map), keys[i], sizeof(map->key), 1);
        ptrdiff_t index = header(raw_hash_array(map, sizeof(*map)))->temp;
        map[index].value = UINT64_C(0xfeed000000000000) + i;
        printf("put:%d:%zu:%td:%d:%s\n", allocation_mode, i, index,
               map[index].key == keys[i], map[index].key);
    }

    for (size_t i = 0; i < 36; i += 7) {
        map = library->hmput_key(map, sizeof(*map), keys[i], sizeof(map->key), 1);
        ptrdiff_t index = header(raw_hash_array(map, sizeof(*map)))->temp;
        printf("update:%d:%zu:%td:%s\n", allocation_mode, i, index, map[index].key);
    }

    for (size_t i = 1; i < 36; i += 4) {
        map = library->hmdel_key(map, sizeof(*map), keys[i], sizeof(map->key), 0, 1);
        printf("sdel:%d:%zu:%td:%zu\n", allocation_mode, i,
               header(raw_hash_array(map, sizeof(*map)))->temp,
               header(raw_hash_array(map, sizeof(*map)))->length - 1);
    }

    array_header *metadata = header(raw_hash_array(map, sizeof(*map)));
    hash_index *table = metadata->hash_table;
    printf("string-meta:%d:%zu:%zu:%zu:%zu:%zu:%u:%zu\n",
           allocation_mode, metadata->length - 1, metadata->capacity,
           table->slot_count, table->used_count, table->tombstone_count,
           table->string.block, table->string.remaining);
    for (size_t i = 0; i + 1 < metadata->length; ++i)
        printf("string-entry:%d:%zu:%s:%016" PRIx64 "\n",
               allocation_mode, i, map[i].key, map[i].value);

    library->hmfree_func(raw_hash_array(map, sizeof(*map)), sizeof(*map));
}

static void test_string_maps(api *library)
{
    puts("string-maps");
    library->rand_seed(UINT64_C(0x8877665544332211));
    test_string_mode(library, 0);
    library->rand_seed(UINT64_C(0x8877665544332211));
    test_string_mode(library, 2);
    library->rand_seed(UINT64_C(0x8877665544332211));
    test_string_mode(library, 3);
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s library.so\n", argv[0]);
        return 2;
    }
    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    api library;
    LOAD(library, handle, arrgrowf, "stbds_arrgrowf");
    LOAD(library, handle, arrfreef, "stbds_arrfreef");
    LOAD(library, handle, rand_seed, "stbds_rand_seed");
    LOAD(library, handle, hash_bytes, "stbds_hash_bytes");
    LOAD(library, handle, hash_string, "stbds_hash_string");
    LOAD(library, handle, hmget_key, "stbds_hmget_key");
    LOAD(library, handle, hmget_key_ts, "stbds_hmget_key_ts");
    LOAD(library, handle, hmput_default, "stbds_hmput_default");
    LOAD(library, handle, hmput_key, "stbds_hmput_key");
    LOAD(library, handle, hmdel_key, "stbds_hmdel_key");
    LOAD(library, handle, hmfree_func, "stbds_hmfree_func");
    LOAD(library, handle, shmode_func, "stbds_shmode_func");
    LOAD(library, handle, stralloc, "stbds_stralloc");
    LOAD(library, handle, strreset, "stbds_strreset");
    LOAD(library, handle, strkey, "strkey");
    LOAD(library, handle, arr_del, "arr_del");

    test_hashes(&library);
    test_strkey(&library);
    test_array(&library);
    test_arena(&library);
    test_binary_map(&library);
    test_string_maps(&library);
    dlclose(handle);
    return 0;
}
