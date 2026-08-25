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
    int key;
    int value;
} int_pair;

typedef struct {
    char *key;
    int value;
} string_pair;

typedef void *(*arrgrow_fn)(void *, size_t, size_t, size_t);
typedef void (*arrfree_fn)(void *);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef void *(*hmdel_fn)(void *, size_t, void *, size_t, size_t, int);
typedef void (*hmfree_fn)(void *, size_t);
typedef void *(*hmget_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmget_ts_fn)(void *, size_t, void *, size_t, ptrdiff_t *, int);
typedef void *(*hmput_default_fn)(void *, size_t);
typedef void *(*hmput_fn)(void *, size_t, void *, size_t, int);
typedef void (*rand_seed_fn)(size_t);
typedef void *(*shmode_fn)(size_t, int);
typedef char *(*stralloc_fn)(string_arena *, char *);
typedef void (*strreset_fn)(string_arena *);
typedef char *(*strkey_fn)(int);
typedef void (*intput_fn)(int);

static arrgrow_fn arrgrow;
static arrfree_fn arrfree;
static hash_bytes_fn hash_bytes;
static hash_string_fn hash_string;
static hmdel_fn hmdel;
static hmfree_fn hmfree;
static hmget_fn hmget;
static hmget_ts_fn hmget_ts;
static hmput_default_fn hmput_default;
static hmput_fn hmput;
static rand_seed_fn rand_seed;
static shmode_fn shmode;
static stralloc_fn stralloc;
static strreset_fn strreset;
static strkey_fn make_strkey;
static intput_fn intput;

#define LOAD(handle, variable, symbol) \
    (*(void **)(&(variable)) = dlsym((handle), (symbol)))

static array_header *header(void *array)
{
    return (array_header *)array - 1;
}

static void load_api(void *handle)
{
    LOAD(handle, arrgrow, "stbds_arrgrowf");
    LOAD(handle, arrfree, "stbds_arrfreef");
    LOAD(handle, hash_bytes, "stbds_hash_bytes");
    LOAD(handle, hash_string, "stbds_hash_string");
    LOAD(handle, hmdel, "stbds_hmdel_key");
    LOAD(handle, hmfree, "stbds_hmfree_func");
    LOAD(handle, hmget, "stbds_hmget_key");
    LOAD(handle, hmget_ts, "stbds_hmget_key_ts");
    LOAD(handle, hmput_default, "stbds_hmput_default");
    LOAD(handle, hmput, "stbds_hmput_key");
    LOAD(handle, rand_seed, "stbds_rand_seed");
    LOAD(handle, shmode, "stbds_shmode_func");
    LOAD(handle, stralloc, "stbds_stralloc");
    LOAD(handle, strreset, "stbds_strreset");
    LOAD(handle, make_strkey, "strkey");
    LOAD(handle, intput, "intput");

    if (!arrgrow || !arrfree || !hash_bytes || !hash_string || !hmdel ||
        !hmfree || !hmget || !hmget_ts || !hmput_default || !hmput ||
        !rand_seed || !shmode || !stralloc || !strreset || !make_strkey ||
        !intput) {
        fprintf(stderr, "missing symbol: %s\n", dlerror());
        exit(2);
    }
}

static int_pair *put_int(int_pair *map, int key, int value)
{
    map = hmput(map, sizeof(*map), &key, sizeof(key), 0);
    ptrdiff_t index = header(map - 1)->temp;
    map[index].key = key;
    map[index].value = value;
    return map;
}

static int get_int(int_pair **map, int key)
{
    *map = hmget(*map, sizeof(**map), &key, sizeof(key), 0);
    return (*map)[header(*map - 1)->temp].value;
}

static string_pair *put_string(string_pair *map, char *key, int value)
{
    map = hmput(map, sizeof(*map), key, sizeof(map->key), 1);
    map[header(map - 1)->temp].value = value;
    return map;
}

static int get_string(string_pair **map, char *key)
{
    *map = hmget(*map, sizeof(**map), key, sizeof((*map)->key), 1);
    return (*map)[header(*map - 1)->temp].value;
}

static void test_hashes(void)
{
    static char high_bytes[] = {(char)0x80, (char)0xff, 'x', '\0'};
    char *strings[] = {"", "a", "test_0", "a longer string with spaces", high_bytes};
    size_t seeds[] = {0, 1, 0x31415926, (size_t)-1};
    unsigned char bytes[40];

    for (size_t i = 0; i < sizeof(bytes); ++i)
        bytes[i] = (unsigned char)(i * 37 + 11);

    for (size_t i = 0; i < sizeof(strings) / sizeof(strings[0]); ++i)
        for (size_t j = 0; j < sizeof(seeds) / sizeof(seeds[0]); ++j)
            printf("hs %zu %zu %016zx\n", i, j, hash_string(strings[i], seeds[j]));

    for (size_t length = 0; length <= sizeof(bytes); ++length)
        printf("hb %zu %016zx %016zx\n", length,
               hash_bytes(bytes, length, 0),
               hash_bytes(bytes, length, 0xdeadbeef12345678ULL));
}

static void test_array(void)
{
    int *array = NULL;
    array = arrgrow(array, sizeof(*array), 0, 1);
    printf("arr 0 %zu %zu\n", header(array)->length, header(array)->capacity);
    for (int i = 0; i < 4; ++i)
        array[header(array)->length++] = 100 + i;
    array = arrgrow(array, sizeof(*array), 1, 0);
    printf("arr 1 %zu %zu %d %d\n", header(array)->length,
           header(array)->capacity, array[0], array[3]);
    array[header(array)->length++] = 104;
    array = arrgrow(array, sizeof(*array), 20, 7);
    printf("arr 2 %zu %zu %d %d\n", header(array)->length,
           header(array)->capacity, array[0], array[4]);
    arrfree(array);
}

static void test_binary_map(void)
{
    int_pair *map = NULL;
    ptrdiff_t temporary = 99;
    int key = 17;

    rand_seed(0x10203040);
    map = hmget_ts(map, sizeof(*map), &key, sizeof(key), &temporary, 0);
    printf("hm init %td %zu %zu\n", temporary, header(map - 1)->length,
           header(map - 1)->capacity);
    map[-1].value = -777;

    for (int i = 0; i < 80; ++i)
        map = put_int(map, (i * 29) % 101, i * i - 3);
    map = put_int(map, 29, 9999);

    printf("hm populated %zu %zu", header(map - 1)->length,
           header(map - 1)->capacity);
    for (int i = 0; i < 12; ++i) {
        int lookup = (i * 17) % 101;
        printf(" %d:%d", lookup, get_int(&map, lookup));
    }
    printf(" absent:%d\n", get_int(&map, 10000));

    for (int i = 0; i < 55; ++i) {
        int remove = (i * 29) % 101;
        map = hmdel(map, sizeof(*map), &remove, sizeof(remove), 0, 0);
        printf("del %d %td %zu\n", remove, header(map - 1)->temp,
               header(map - 1)->length);
    }

    printf("hm remain");
    for (int i = 55; i < 80; ++i) {
        int lookup = (i * 29) % 101;
        printf(" %d:%d", lookup, get_int(&map, lookup));
    }
    printf("\n");
    hmfree(map - 1, sizeof(*map));
}

static void test_default_map(void)
{
    int_pair *map = NULL;
    int missing = 12;
    map = hmput_default(map, sizeof(*map));
    map[-1].key = -1;
    map[-1].value = 4242;
    printf("default %d %zu\n", get_int(&map, missing), header(map - 1)->length);
    hmfree(map - 1, sizeof(*map));
}

static void test_string_mode(int mode)
{
    char keys[48][24];
    string_pair *map;
    rand_seed(0x55667788 + (size_t)mode);
    map = shmode(sizeof(*map), mode);
    map[-1].value = -313;

    for (int i = 0; i < 48; ++i) {
        snprintf(keys[i], sizeof(keys[i]), "key_%02d_%c", i, 'a' + i % 7);
        map = put_string(map, keys[i], i * 13);
    }
    map = put_string(map, keys[7], 7007);

    printf("sh %d %zu %zu", mode, header(map - 1)->length,
           header(map - 1)->capacity);
    for (int i = 0; i < 48; i += 5)
        printf(" %d", get_string(&map, keys[i]));
    printf(" miss:%d own:%d\n", get_string(&map, "not-present"),
           map[0].key != keys[0]);

    for (int i = 0; i < 33; ++i) {
        map = hmdel(map, sizeof(*map), keys[i], sizeof(map->key), 0, 1);
        printf("sdel %d %d %td %zu\n", mode, i, header(map - 1)->temp,
               header(map - 1)->length);
    }

    printf("shremain %d", mode);
    for (int i = 33; i < 48; ++i)
        printf(" %s:%d", keys[i], get_string(&map, keys[i]));
    printf("\n");
    hmfree(map - 1, sizeof(*map));
}

static void test_arena(void)
{
    string_arena arena = {0};
    char medium[500];
    char large[800];
    memset(medium, 'm', sizeof(medium) - 1);
    medium[sizeof(medium) - 1] = '\0';
    memset(large, 'L', sizeof(large) - 1);
    large[sizeof(large) - 1] = '\0';

    char *a = stralloc(&arena, "alpha");
    char *b = stralloc(&arena, medium);
    char *c = stralloc(&arena, large);
    char *d = stralloc(&arena, "omega");
    printf("arena %s %zu:%c%c %zu:%c%c %s %zu %u\n", a, strlen(b), b[0],
           b[498], strlen(c), c[0], c[798], d, arena.remaining, arena.block);
    strreset(&arena);
    printf("arena reset %d %zu %u %u\n", arena.storage == NULL,
           arena.remaining, arena.block, arena.mode);
}

static void test_misc(void)
{
    int values[] = {0, 1, -1, 2147483647, -2147483647 - 1};
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); ++i)
        printf("strkey %d %s\n", values[i], make_strkey(values[i]));
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); ++i)
        intput(values[i]);
    printf("intput ok\n");
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }
    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    load_api(handle);
    test_hashes();
    test_array();
    test_binary_map();
    test_default_map();
    test_string_mode(1);
    test_string_mode(2);
    test_string_mode(3);
    test_arena();
    test_misc();
    dlclose(handle);
    return 0;
}
