#include <dlfcn.h>
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

typedef void *(*arrgrow_fn)(void *, size_t, size_t, size_t);
typedef void (*arrfree_fn)(void *);
typedef void (*seed_fn)(size_t);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);
typedef void *(*hmget_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmget_ts_fn)(void *, size_t, void *, size_t, ptrdiff_t *, int);
typedef void *(*hmdefault_fn)(void *, size_t);
typedef void *(*hmput_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmdel_fn)(void *, size_t, void *, size_t, size_t, int);
typedef void *(*shmode_fn)(size_t, int);
typedef void (*hmfree_fn)(void *, size_t);
typedef char *(*stralloc_fn)(StringArena *, char *);
typedef void (*strreset_fn)(StringArena *);
typedef char *(*strkey_fn)(int);
typedef void (*arr_push_fn)(int);

typedef struct {
    uint64_t key;
    uint64_t value;
    uint32_t marker;
    uint32_t padding;
} BinaryEntry;

typedef struct {
    char *key;
    uint64_t value;
} StringEntry;

static ArrayHeader *header(void *array) {
    return (ArrayHeader *)array - 1;
}

static void *raw_hash_array(void *array, size_t element_size) {
    return (char *)array - element_size;
}

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    if (!symbol) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s library.so\n", argv[0]);
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

#define LOAD(type, name) type name = (type)load_symbol(library, #name)
    LOAD(arrgrow_fn, stbds_arrgrowf);
    LOAD(arrfree_fn, stbds_arrfreef);
    LOAD(seed_fn, stbds_rand_seed);
    LOAD(hash_string_fn, stbds_hash_string);
    LOAD(hash_bytes_fn, stbds_hash_bytes);
    LOAD(hmget_fn, stbds_hmget_key);
    LOAD(hmget_ts_fn, stbds_hmget_key_ts);
    LOAD(hmdefault_fn, stbds_hmput_default);
    LOAD(hmput_fn, stbds_hmput_key);
    LOAD(hmdel_fn, stbds_hmdel_key);
    LOAD(shmode_fn, stbds_shmode_func);
    LOAD(hmfree_fn, stbds_hmfree_func);
    LOAD(stralloc_fn, stbds_stralloc);
    LOAD(strreset_fn, stbds_strreset);
    LOAD(strkey_fn, strkey);
    LOAD(arr_push_fn, arr_push);
#undef LOAD

    unsigned char bytes[65];
    for (size_t i = 0; i < sizeof(bytes); ++i)
        bytes[i] = (unsigned char)(i * 29u + 0xa7u);

    size_t seeds[] = {0, 1, 0x31415926u, (size_t)0xfedcba9876543210ull};
    size_t lengths[] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 33, 64, 65};
    printf("hash-bytes\n");
    for (size_t s = 0; s < sizeof(seeds) / sizeof(seeds[0]); ++s) {
        for (size_t i = 0; i < sizeof(lengths) / sizeof(lengths[0]); ++i)
            printf("%016zx ", stbds_hash_bytes(bytes, lengths[i], seeds[s]));
        putchar('\n');
    }

    char high_string[] = {'A', (char)0x80, (char)0xff, 'z', '\0'};
    char *strings[] = {"", "a", "test_123", "a considerably longer string", high_string};
    printf("hash-strings\n");
    for (size_t s = 0; s < sizeof(seeds) / sizeof(seeds[0]); ++s) {
        for (size_t i = 0; i < sizeof(strings) / sizeof(strings[0]); ++i)
            printf("%016zx ", stbds_hash_string(strings[i], seeds[s]));
        putchar('\n');
    }

    printf("array\n");
    uint32_t *array = NULL;
    array = stbds_arrgrowf(array, sizeof(*array), 0, 1);
    for (uint32_t i = 0; i < 4; ++i)
        array[i] = 100 + i;
    header(array)->length = 4;
    printf("%zu/%zu", header(array)->length, header(array)->capacity);
    array = stbds_arrgrowf(array, sizeof(*array), 1, 0);
    array[4] = 104;
    header(array)->length = 5;
    printf(" %zu/%zu", header(array)->length, header(array)->capacity);
    array = stbds_arrgrowf(array, sizeof(*array), 0, 33);
    printf(" %zu/%zu:", header(array)->length, header(array)->capacity);
    for (size_t i = 0; i < header(array)->length; ++i)
        printf("%u,", array[i]);
    putchar('\n');
    stbds_arrfreef(array);

    printf("binary-map\n");
    stbds_rand_seed(0x1020304050607080ull);
    BinaryEntry *binary = NULL;
    binary = stbds_hmput_default(binary, sizeof(*binary));
    memset(&binary[-1], 0, sizeof(*binary));
    binary[-1].value = 0xdeadbeef;
    for (uint64_t i = 0; i < 173; ++i) {
        uint64_t key = (i * 37) % 173;
        binary = stbds_hmput_key(binary, sizeof(*binary), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_hash_array(binary, sizeof(*binary)))->temp;
        memset(&binary[index], 0, sizeof(*binary));
        binary[index].key = key;
        binary[index].value = key * key + 17;
        binary[index].marker = (uint32_t)(key ^ 0xa5a5u);
    }
    for (uint64_t key = 0; key < 173; key += 11) {
        ptrdiff_t temp = -99;
        binary = stbds_hmget_key_ts(
            binary, sizeof(*binary), &key, sizeof(key), &temp, 0);
        printf("%llu=%llu/%u,", (unsigned long long)key,
               (unsigned long long)binary[temp].value, binary[temp].marker);
    }
    putchar('\n');
    for (uint64_t key = 0; key < 173; key += 3)
        binary = stbds_hmdel_key(
            binary, sizeof(*binary), &key, sizeof(key), 0, 0);
    uint64_t missing = 10000;
    binary = stbds_hmget_key(
        binary, sizeof(*binary), &missing, sizeof(missing), 0);
    ArrayHeader *binary_header = header(raw_hash_array(binary, sizeof(*binary)));
    printf("len=%zu missing=%td default=%llu\n",
           binary_header->length - 1, binary_header->temp,
           (unsigned long long)binary[-1].value);
    for (size_t i = 0; i + 1 < binary_header->length; ++i)
        printf("%llu:%llu:%u\n",
               (unsigned long long)binary[i].key,
               (unsigned long long)binary[i].value,
               binary[i].marker);
    stbds_hmfree_func(raw_hash_array(binary, sizeof(*binary)), sizeof(*binary));

    printf("null-get\n");
    uint64_t null_key = 7;
    ptrdiff_t null_temp = 123;
    BinaryEntry *empty = stbds_hmget_key_ts(
        NULL, sizeof(*empty), &null_key, sizeof(null_key), &null_temp, 0);
    printf("%td/%zu/%llu\n", null_temp,
           header(raw_hash_array(empty, sizeof(*empty)))->length,
           (unsigned long long)empty[-1].value);
    stbds_hmfree_func(raw_hash_array(empty, sizeof(*empty)), sizeof(*empty));

    for (int string_mode = 1; string_mode <= 3; ++string_mode) {
        printf("string-map-%d\n", string_mode);
        stbds_rand_seed(0x8877665544332211ull + (size_t)string_mode);
        StringEntry *map = string_mode == 1
            ? NULL
            : stbds_shmode_func(sizeof(*map), string_mode);
        char keys[80][32];
        for (int i = 0; i < 80; ++i) {
            snprintf(keys[i], sizeof(keys[i]), "key_%02d_%c", i, 'a' + (i * 7) % 26);
            map = stbds_hmput_key(
                map, sizeof(*map), keys[i], sizeof(map->key), 1);
            ptrdiff_t index = header(raw_hash_array(map, sizeof(*map)))->temp;
            map[index].value = (uint64_t)i * 101 + (uint64_t)string_mode;
        }
        for (int i = 0; i < 80; i += 9) {
            ptrdiff_t temp = -99;
            map = stbds_hmget_key_ts(
                map, sizeof(*map), keys[i], sizeof(map->key), &temp, 1);
            printf("%s=%llu,", map[temp].key,
                   (unsigned long long)map[temp].value);
        }
        putchar('\n');
        for (int i = 2; i < 80; i += 4)
            map = stbds_hmdel_key(
                map, sizeof(*map), keys[i], sizeof(map->key), 0, 1);
        ArrayHeader *map_header = header(raw_hash_array(map, sizeof(*map)));
        printf("len=%zu\n", map_header->length - 1);
        for (size_t i = 0; i + 1 < map_header->length; ++i)
            printf("%s:%llu\n", map[i].key, (unsigned long long)map[i].value);
        stbds_hmfree_func(raw_hash_array(map, sizeof(*map)), sizeof(*map));
    }

    printf("arena\n");
    StringArena arena = {0};
    char short_one[] = "short";
    char medium[601];
    char short_two[] = "tail";
    memset(medium, 'm', sizeof(medium) - 1);
    medium[sizeof(medium) - 1] = '\0';
    char *stored_medium = stbds_stralloc(&arena, medium);
    char *stored_one = stbds_stralloc(&arena, short_one);
    char *stored_two = stbds_stralloc(&arena, short_two);
    printf("%d/%d/%d rem=%zu block=%u mode=%u\n",
           strcmp(stored_medium, medium) == 0,
           strcmp(stored_one, short_one) == 0,
           strcmp(stored_two, short_two) == 0,
           arena.remaining, arena.block, arena.mode);
    stbds_strreset(&arena);
    printf("%d/%zu/%u/%u\n", arena.storage == NULL, arena.remaining,
           arena.block, arena.mode);

    printf("strkey\n");
    printf("%s %s %s\n", strkey(-2147483647), strkey(0), strkey(2147483647));
    arr_push(-1);
    arr_push(0);
    arr_push(1);
    arr_push(51);
    arr_push(126);
    printf("arr-push-ok\n");

    dlclose(library);
    return 0;
}
