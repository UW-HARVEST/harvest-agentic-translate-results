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

typedef struct {
    uint32_t key;
    int value;
} IntEntry;

typedef struct {
    char *key;
    int value;
} StringEntry;

typedef void *(*arrgrow_fn)(void *, size_t, size_t, size_t);
typedef void (*arrfree_fn)(void *);
typedef void (*seed_fn)(size_t);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);
typedef void *(*hmget_ts_fn)(void *, size_t, void *, size_t, ptrdiff_t *, int);
typedef void *(*hmget_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmdefault_fn)(void *, size_t);
typedef void *(*hmput_fn)(void *, size_t, void *, size_t, int);
typedef void *(*shmode_fn)(size_t, int);
typedef void *(*hmdel_fn)(void *, size_t, void *, size_t, size_t, int);
typedef char *(*stralloc_fn)(StringArena *, char *);
typedef void (*strreset_fn)(StringArena *);
typedef void (*hmfree_fn)(void *, size_t);
typedef char *(*strkey_fn)(int);
typedef void (*arr_ins_fn)(int);

static ArrayHeader *array_header(void *array) {
    return (ArrayHeader *)array - 1;
}

static ArrayHeader *map_header(void *entries, size_t element_size) {
    return array_header((char *)entries - element_size);
}

static void *symbol(void *library, const char *name) {
    void *result = dlsym(library, name);
    if (!result) {
        fprintf(stderr, "missing symbol: %s\n", name);
        exit(2);
    }
    return result;
}

#define LOAD(type, name) type name = (type)symbol(library, #name)

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    LOAD(arrgrow_fn, stbds_arrgrowf);
    LOAD(arrfree_fn, stbds_arrfreef);
    LOAD(seed_fn, stbds_rand_seed);
    LOAD(hash_string_fn, stbds_hash_string);
    LOAD(hash_bytes_fn, stbds_hash_bytes);
    LOAD(hmget_ts_fn, stbds_hmget_key_ts);
    LOAD(hmget_fn, stbds_hmget_key);
    LOAD(hmdefault_fn, stbds_hmput_default);
    LOAD(hmput_fn, stbds_hmput_key);
    LOAD(shmode_fn, stbds_shmode_func);
    LOAD(hmdel_fn, stbds_hmdel_key);
    LOAD(stralloc_fn, stbds_stralloc);
    LOAD(strreset_fn, stbds_strreset);
    LOAD(hmfree_fn, stbds_hmfree_func);
    LOAD(strkey_fn, strkey);
    LOAD(arr_ins_fn, arr_ins);

    unsigned char bytes[64];
    for (size_t i = 0; i < sizeof(bytes); ++i) {
        bytes[i] = (unsigned char)(i * 37u + 131u);
    }
    char *strings[] = {"", "a", "test_0", "high\x80\xff", "a longer test string"};
    size_t seeds[] = {0, 1, 0x31415926u, (size_t)0xfedcba9876543210ull};
    for (size_t seed_index = 0; seed_index < 4; ++seed_index) {
        for (size_t string_index = 0; string_index < 5; ++string_index) {
            printf("hs %zu %zu %016zx\n", seed_index, string_index,
                   stbds_hash_string(strings[string_index], seeds[seed_index]));
        }
        for (size_t length = 0; length <= 33; ++length) {
            printf("hb %zu %zu %016zx\n", seed_index, length,
                   stbds_hash_bytes(bytes, length, seeds[seed_index]));
        }
    }

    int *array = NULL;
    array = stbds_arrgrowf(array, sizeof(*array), 1, 0);
    array[0] = 17;
    array_header(array)->length = 1;
    array = stbds_arrgrowf(array, sizeof(*array), 7, 3);
    for (int i = 1; i < 8; ++i) {
        array[i] = i * i - 3;
    }
    array_header(array)->length = 8;
    array = stbds_arrgrowf(array, sizeof(*array), 0, 19);
    printf("array %zu %zu", array_header(array)->length,
           array_header(array)->capacity);
    for (int i = 0; i < 8; ++i) {
        printf(" %d", array[i]);
    }
    putchar('\n');
    stbds_arrfreef(array);

    stbds_rand_seed((size_t)0x123456789abcdef0ull);
    IntEntry *map = stbds_hmput_default(NULL, sizeof(*map));
    map[-1].value = -7001;
    for (uint32_t key = 0; key < 73; ++key) {
        uint32_t mixed = key * 2654435761u;
        map = stbds_hmput_key(map, sizeof(*map), &mixed, sizeof(mixed), 0);
        ptrdiff_t index = map_header(map, sizeof(*map))->temp;
        map[index].key = mixed;
        map[index].value = (int)(key * key) - 99;
    }
    printf("map-insert %zu %zu %td\n", map_header(map, sizeof(*map))->length,
           map_header(map, sizeof(*map))->capacity,
           map_header(map, sizeof(*map))->temp);

    for (uint32_t key = 0; key < 80; key += 7) {
        uint32_t mixed = key * 2654435761u;
        ptrdiff_t temporary = 12345;
        map = stbds_hmget_key_ts(map, sizeof(*map), &mixed, sizeof(mixed),
                                &temporary, 0);
        printf("map-get-ts %u %td %d\n", key, temporary,
               temporary < 0 ? map[-1].value : map[temporary].value);
        map = stbds_hmget_key(map, sizeof(*map), &mixed, sizeof(mixed), 0);
        ptrdiff_t index = map_header(map, sizeof(*map))->temp;
        printf("map-get %u %td %d\n", key, index,
               index < 0 ? map[-1].value : map[index].value);
    }

    for (uint32_t key = 3; key < 73; key += 4) {
        uint32_t mixed = key * 2654435761u;
        map = stbds_hmdel_key(map, sizeof(*map), &mixed, sizeof(mixed), 0, 0);
        printf("map-del %u %td %zu\n", key,
               map_header(map, sizeof(*map))->temp,
               map_header(map, sizeof(*map))->length);
    }
    uint32_t absent = 0xdeadbeefu;
    map = stbds_hmdel_key(map, sizeof(*map), &absent, sizeof(absent), 0, 0);
    printf("map-del-absent %td %zu\n", map_header(map, sizeof(*map))->temp,
           map_header(map, sizeof(*map))->length);
    stbds_hmfree_func(map - 1, sizeof(*map));

    char mutable_keys[24][32];
    StringEntry *string_map = stbds_shmode_func(sizeof(*string_map), 2);
    for (int i = 0; i < 24; ++i) {
        snprintf(mutable_keys[i], sizeof(mutable_keys[i]), "word_%02d_%c", i,
                 'a' + (i % 26));
        string_map = stbds_hmput_key(string_map, sizeof(*string_map),
                                    mutable_keys[i], sizeof(char *), 1);
        ptrdiff_t index = map_header(string_map, sizeof(*string_map))->temp;
        string_map[index].value = i * 11 - 5;
    }
    memset(mutable_keys, 'X', sizeof(mutable_keys));
    for (int i = 0; i < 24; i += 5) {
        char query[32];
        snprintf(query, sizeof(query), "word_%02d_%c", i, 'a' + (i % 26));
        string_map = stbds_hmget_key(string_map, sizeof(*string_map), query,
                                    sizeof(char *), 1);
        ptrdiff_t index = map_header(string_map, sizeof(*string_map))->temp;
        printf("str-get %d %td %s %d\n", i, index,
               index < 0 ? "<missing>" : string_map[index].key,
               index < 0 ? -1 : string_map[index].value);
    }
    char delete_key[] = "word_10_k";
    string_map = stbds_hmdel_key(string_map, sizeof(*string_map), delete_key,
                                sizeof(char *), 0, 1);
    printf("str-del %td %zu\n", map_header(string_map, sizeof(*string_map))->temp,
           map_header(string_map, sizeof(*string_map))->length);
    stbds_hmfree_func(string_map - 1, sizeof(*string_map));

    StringEntry *arena_map = stbds_shmode_func(sizeof(*arena_map), 3);
    char arena_key[1400];
    for (int i = 0; i < 18; ++i) {
        size_t length = i == 7 ? 1200u : (size_t)(17 + i * 13);
        for (size_t j = 0; j < length; ++j) {
            arena_key[j] = (char)('a' + (i + (int)j) % 26);
        }
        arena_key[length] = '\0';
        arena_map = stbds_hmput_key(arena_map, sizeof(*arena_map), arena_key,
                                   sizeof(char *), 1);
        ptrdiff_t index = map_header(arena_map, sizeof(*arena_map))->temp;
        arena_map[index].value = i + 100;
        printf("arena-put %d %zu %td %c %c\n", i,
               strlen(arena_map[index].key), index, arena_map[index].key[0],
               arena_map[index].key[length - 1]);
    }
    stbds_hmfree_func(arena_map - 1, sizeof(*arena_map));

    StringArena arena = {0};
    char short_text[] = "short";
    char *short_copy = stbds_stralloc(&arena, short_text);
    char long_text[701];
    memset(long_text, 'q', 700);
    long_text[700] = '\0';
    char *long_copy = stbds_stralloc(&arena, long_text);
    printf("arena-direct %s %zu %c %c %zu %u %u\n", short_copy,
           strlen(long_copy), long_copy[0], long_copy[699], arena.remaining,
           arena.block, arena.mode);
    stbds_strreset(&arena);
    printf("arena-reset %d %zu %u %u\n", arena.storage == NULL, arena.remaining,
           arena.block, arena.mode);

    int key_numbers[] = {0, -1, 2147483647, -2147483647 - 1};
    for (size_t i = 0; i < 4; ++i) {
        printf("strkey %d %s\n", key_numbers[i], strkey(key_numbers[i]));
    }
    arr_ins(-1234567);
    stbds_hmfree_func(NULL, sizeof(IntEntry));
    dlclose(library);
    return 0;
}
