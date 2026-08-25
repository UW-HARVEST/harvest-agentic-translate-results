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
} array_header;

typedef struct string_block string_block;
typedef struct {
    string_block *storage;
    size_t remaining;
    unsigned char block;
    unsigned char mode;
} string_arena;

typedef struct {
    uint64_t key;
    int value;
} map_entry;

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
typedef void (*hmfree_fn)(void *, size_t);
typedef void *(*shmode_fn)(size_t, int);
typedef char *(*stralloc_fn)(string_arena *, char *);
typedef void (*strreset_fn)(string_arena *);
typedef char *(*strkey_fn)(int);

typedef struct {
    arrgrow_fn arrgrow;
    arrfree_fn arrfree;
    seed_fn seed;
    hash_string_fn hash_string;
    hash_bytes_fn hash_bytes;
    hmget_fn hmget;
    hmget_ts_fn hmget_ts;
    hmdefault_fn hmdefault;
    hmput_fn hmput;
    hmdel_fn hmdel;
    hmfree_fn hmfree;
    shmode_fn shmode;
    stralloc_fn stralloc;
    strreset_fn strreset;
    strkey_fn strkey;
} api;

static array_header *header(void *array)
{
    return (array_header *)array - 1;
}

static void *raw_map(void *map)
{
    return (char *)map - sizeof(map_entry);
}

static api load_api(void *library)
{
    api result;
#define LOAD(field, symbol) result.field = (field##_fn)dlsym(library, symbol)
    LOAD(arrgrow, "stbds_arrgrowf");
    LOAD(arrfree, "stbds_arrfreef");
    LOAD(seed, "stbds_rand_seed");
    LOAD(hash_string, "stbds_hash_string");
    LOAD(hash_bytes, "stbds_hash_bytes");
    LOAD(hmget, "stbds_hmget_key");
    LOAD(hmget_ts, "stbds_hmget_key_ts");
    LOAD(hmdefault, "stbds_hmput_default");
    LOAD(hmput, "stbds_hmput_key");
    LOAD(hmdel, "stbds_hmdel_key");
    LOAD(hmfree, "stbds_hmfree_func");
    LOAD(shmode, "stbds_shmode_func");
    LOAD(stralloc, "stbds_stralloc");
    LOAD(strreset, "stbds_strreset");
    LOAD(strkey, "strkey");
#undef LOAD
    return result;
}

static void exercise_hashes(api *functions)
{
    static const size_t seeds[] = {
        0, 1, 0x31415926, 0xdeadbeef, SIZE_MAX,
        0x0123456789abcdefULL
    };
    unsigned char bytes[160];
    uint64_t state = 0x9e3779b97f4a7c15ULL;

    puts("hash-string");
    for (size_t seed_index = 0; seed_index < sizeof(seeds) / sizeof(seeds[0]); ++seed_index) {
        char *strings[] = {"", "a", "foo", "test_0", "with spaces",
                           "abcdefghijklmnopqrstuvwxyz"};
        for (size_t index = 0; index < sizeof(strings) / sizeof(strings[0]); ++index)
            printf("%016zx\n", functions->hash_string(strings[index], seeds[seed_index]));
    }

    puts("hash-bytes");
    for (size_t index = 0; index < sizeof(bytes); ++index) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes[index] = (unsigned char)(state >> 23);
    }
    for (size_t seed_index = 0; seed_index < sizeof(seeds) / sizeof(seeds[0]); ++seed_index)
        for (size_t length = 0; length <= sizeof(bytes); ++length)
            printf("%016zx\n", functions->hash_bytes(bytes, length, seeds[seed_index]));
}

static void exercise_array(api *functions)
{
    uint32_t *array = NULL;

    puts("array");
    array = functions->arrgrow(array, sizeof(*array), 0, 1);
    printf("%zu %zu\n", header(array)->length, header(array)->capacity);
    for (uint32_t index = 0; index < 4; ++index)
        array[index] = 0x10203040U + index;
    header(array)->length = 4;
    array = functions->arrgrow(array, sizeof(*array), 1, 0);
    printf("%zu %zu", header(array)->length, header(array)->capacity);
    for (uint32_t index = 0; index < 4; ++index)
        printf(" %08x", array[index]);
    putchar('\n');
    array = functions->arrgrow(array, sizeof(*array), 0, 20);
    printf("%zu %zu\n", header(array)->length, header(array)->capacity);
    functions->arrfree(array);
}

static void exercise_arena(api *functions)
{
    string_arena arena = {0};
    char medium[701];
    char large[1100001];
    char *stored;

    memset(medium, 'm', sizeof(medium) - 1);
    medium[sizeof(medium) - 1] = 0;
    memset(large, 'L', sizeof(large) - 1);
    large[sizeof(large) - 1] = 0;

    puts("arena");
    stored = functions->stralloc(&arena, "one");
    printf("%s %zu %u %u\n", stored, arena.remaining, arena.block, arena.mode);
    stored = functions->stralloc(&arena, medium);
    printf("%zu %c %c %zu %u\n", strlen(stored), stored[0],
           stored[strlen(stored) - 1], arena.remaining, arena.block);
    stored = functions->stralloc(&arena, large);
    printf("%zu %c %c %zu %u\n", strlen(stored), stored[0],
           stored[strlen(stored) - 1], arena.remaining, arena.block);
    functions->strreset(&arena);
    printf("%d %zu %u %u\n", arena.storage == NULL, arena.remaining,
           arena.block, arena.mode);
}

static void exercise_map(api *functions)
{
    map_entry *map = NULL;
    uint64_t key;
    ptrdiff_t temporary;

    puts("map");
    functions->seed(0x123456789abcdef0ULL);
    map = functions->hmdefault(map, sizeof(*map));
    map[-1].value = -99;
    for (uint64_t index = 0; index < 180; ++index) {
        key = index * 17 + 3;
        map = functions->hmput(map, sizeof(*map), &key, sizeof(key), 0);
        map[header(raw_map(map))->temp].value = (int)(index * 11);
    }
    printf("%zu %zu %td\n", header(raw_map(map))->length - 1,
           header(raw_map(map))->capacity, header(raw_map(map))->temp);

    long long total = 0;
    for (uint64_t index = 0; index < 220; ++index) {
        key = index * 17 + 3;
        map = functions->hmget(map, sizeof(*map), &key, sizeof(key), 0);
        total += map[header(raw_map(map))->temp].value;
    }
    printf("%lld %zu\n", total, header(raw_map(map))->length - 1);

    key = 20;
    map = functions->hmput(map, sizeof(*map), &key, sizeof(key), 0);
    map[header(raw_map(map))->temp].value = 777777;
    map = functions->hmget_ts(map, sizeof(*map), &key, sizeof(key), &temporary, 0);
    printf("%td %d\n", temporary, map[temporary].value);

    for (uint64_t index = 0; index < 180; index += 3) {
        key = index * 17 + 3;
        map = functions->hmdel(map, sizeof(*map), &key, sizeof(key), 0, 0);
    }
    printf("%zu", header(raw_map(map))->length - 1);
    for (size_t index = 0; index < header(raw_map(map))->length - 1; ++index)
        printf(" %llu:%d", (unsigned long long)map[index].key, map[index].value);
    putchar('\n');

    for (uint64_t index = 0; index < 180; ++index) {
        key = index * 17 + 3;
        map = functions->hmdel(map, sizeof(*map), &key, sizeof(key), 0, 0);
    }
    printf("%zu %d\n", header(raw_map(map))->length - 1, map[-1].value);
    functions->hmfree(raw_map(map), sizeof(*map));
}

static void exercise_misc(api *functions)
{
    static const int values[] = {0, 1, -1, 42, 2147483647, -2147483647 - 1};

    puts("strkey");
    for (size_t index = 0; index < sizeof(values) / sizeof(values[0]); ++index)
        puts(functions->strkey(values[index]));

    puts("shmode");
    for (int mode = 2; mode <= 3; ++mode) {
        map_entry *map = functions->shmode(sizeof(*map), mode);
        printf("%d %zu %zu\n", mode, header(raw_map(map))->length,
               header(raw_map(map))->capacity);
        functions->hmfree(raw_map(map), sizeof(*map));
    }
}

int main(int argc, char **argv)
{
    void *library;
    api functions;

    if (argc != 2)
        return 2;
    library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fputs(dlerror(), stderr);
        return 3;
    }
    functions = load_api(library);
    exercise_hashes(&functions);
    exercise_array(&functions);
    exercise_arena(&functions);
    exercise_map(&functions);
    exercise_misc(&functions);
    dlclose(library);
    return 0;
}
