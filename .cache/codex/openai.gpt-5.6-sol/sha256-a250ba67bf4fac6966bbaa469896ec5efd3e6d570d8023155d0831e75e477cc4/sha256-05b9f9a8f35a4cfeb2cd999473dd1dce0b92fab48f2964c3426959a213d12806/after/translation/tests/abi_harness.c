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
    uint32_t key;
    int32_t value;
} binary_entry;

typedef struct {
    char *key;
    int32_t value;
} string_entry;

typedef void *(*arrgrow_fn)(void *, size_t, size_t, size_t);
typedef void (*arrfree_fn)(void *);
typedef void (*seed_fn)(size_t);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);
typedef void (*hmfree_fn)(void *, size_t);
typedef void *(*hmget_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmget_ts_fn)(void *, size_t, void *, size_t, ptrdiff_t *, int);
typedef void *(*hmdefault_fn)(void *, size_t);
typedef void *(*hmput_fn)(void *, size_t, void *, size_t, int);
typedef void *(*hmdel_fn)(void *, size_t, void *, size_t, size_t, int);
typedef void *(*shmode_fn)(size_t, int);
typedef char *(*stralloc_fn)(string_arena *, char *);
typedef void (*strreset_fn)(string_arena *);
typedef char *(*strkey_fn)(int);
typedef void (*sh_geti_fn)(int);

static void *load_symbol(void *library, const char *name)
{
    void *symbol = dlsym(library, name);
    if (!symbol) {
        fprintf(stderr, "missing symbol: %s\n", name);
        exit(2);
    }
    return symbol;
}

static array_header *header(void *array)
{
    return (array_header *) array - 1;
}

static void *raw_map(void *map, size_t element_size)
{
    return (char *) map - element_size;
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s library.so\n", argv[0]);
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    arrgrow_fn arrgrow = (arrgrow_fn) load_symbol(library, "stbds_arrgrowf");
    arrfree_fn arrfree = (arrfree_fn) load_symbol(library, "stbds_arrfreef");
    seed_fn rand_seed = (seed_fn) load_symbol(library, "stbds_rand_seed");
    hash_string_fn hash_string = (hash_string_fn) load_symbol(library, "stbds_hash_string");
    hash_bytes_fn hash_bytes = (hash_bytes_fn) load_symbol(library, "stbds_hash_bytes");
    hmfree_fn hmfree = (hmfree_fn) load_symbol(library, "stbds_hmfree_func");
    hmget_fn hmget = (hmget_fn) load_symbol(library, "stbds_hmget_key");
    hmget_ts_fn hmget_ts = (hmget_ts_fn) load_symbol(library, "stbds_hmget_key_ts");
    hmdefault_fn hmdefault = (hmdefault_fn) load_symbol(library, "stbds_hmput_default");
    hmput_fn hmput = (hmput_fn) load_symbol(library, "stbds_hmput_key");
    hmdel_fn hmdel = (hmdel_fn) load_symbol(library, "stbds_hmdel_key");
    shmode_fn shmode = (shmode_fn) load_symbol(library, "stbds_shmode_func");
    stralloc_fn stralloc = (stralloc_fn) load_symbol(library, "stbds_stralloc");
    strreset_fn strreset = (strreset_fn) load_symbol(library, "stbds_strreset");
    strkey_fn strkey = (strkey_fn) load_symbol(library, "strkey");
    sh_geti_fn sh_geti = (sh_geti_fn) load_symbol(library, "sh_geti");

    char high_string[] = { (char) 0x80, (char) 0xff, 'Z', 0 };
    char *strings[] = { "", "a", "hello", "test_12345", high_string };
    size_t seeds[] = { 0, 1, 0x31415926u, (size_t) 0xfedcba9876543210ull };
    for (size_t i = 0; i < sizeof(strings) / sizeof(strings[0]); ++i) {
        for (size_t j = 0; j < sizeof(seeds) / sizeof(seeds[0]); ++j)
            printf("hs %zu %zu %016zx\n", i, j, hash_string(strings[i], seeds[j]));
    }

    unsigned char bytes[80];
    for (size_t i = 0; i < sizeof(bytes); ++i)
        bytes[i] = (unsigned char) (i * 73u + 0x80u);
    for (size_t len = 0; len <= 40; ++len)
        printf("hb %zu %016zx %016zx\n", len,
               hash_bytes(bytes, len, 0),
               hash_bytes(bytes, len, (size_t) 0xfedcba9876543210ull));

    int *array = arrgrow(NULL, sizeof(*array), 0, 1);
    header(array)->length = 3;
    array[0] = 17;
    array[1] = -4;
    array[2] = 99;
    printf("arr %zu %zu", header(array)->length, header(array)->capacity);
    array = arrgrow(array, sizeof(*array), 2, 0);
    printf(" %zu %zu %d %d %d\n", header(array)->length, header(array)->capacity,
           array[0], array[1], array[2]);
    arrfree(array);

    string_arena arena = { 0 };
    char large[701];
    for (size_t i = 0; i < sizeof(large) - 1; ++i)
        large[i] = (char) ('a' + i % 26);
    large[sizeof(large) - 1] = 0;
    char *arena_a = stralloc(&arena, "alpha");
    char *arena_b = stralloc(&arena, large);
    char *arena_c = stralloc(&arena, "omega");
    printf("arena %s %zu %c %c %s %zu %u\n", arena_a, strlen(arena_b),
           arena_b[0], arena_b[699], arena_c, arena.remaining, arena.block);
    strreset(&arena);
    printf("arena-reset %d %zu %u %u\n", arena.storage == NULL, arena.remaining,
           arena.block, arena.mode);

    rand_seed(0x12345678u);
    binary_entry *binary = hmdefault(NULL, sizeof(*binary));
    binary[-1].value = -777;
    for (uint32_t n = 0; n < 120; ++n) {
        uint32_t key = (n * 37u) % 120u;
        binary = hmput(binary, sizeof(*binary), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_map(binary, sizeof(*binary)))->temp;
        binary[index].value = (int32_t) key * 11 - 5;
    }

    long long binary_sum = 0;
    for (uint32_t key = 0; key < 125; ++key) {
        binary = hmget(binary, sizeof(*binary), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_map(binary, sizeof(*binary)))->temp;
        binary_sum += binary[index].value;
    }
    ptrdiff_t ts_index = 999;
    uint32_t ts_key = 73;
    ptrdiff_t old_temp = header(raw_map(binary, sizeof(*binary)))->temp;
    binary = hmget_ts(binary, sizeof(*binary), &ts_key, sizeof(ts_key), &ts_index, 0);
    printf("binary-get %lld %td %d %d\n", binary_sum, ts_index,
           binary[ts_index].value,
           header(raw_map(binary, sizeof(*binary)))->temp == old_temp);

    int deleted = 0;
    for (uint32_t key = 0; key < 120; key += 3) {
        binary = hmdel(binary, sizeof(*binary), &key, sizeof(key), 0, 0);
        deleted += (int) header(raw_map(binary, sizeof(*binary)))->temp;
    }
    long long retained_sum = 0;
    for (uint32_t key = 0; key < 120; ++key) {
        binary = hmget(binary, sizeof(*binary), &key, sizeof(key), 0);
        ptrdiff_t index = header(raw_map(binary, sizeof(*binary)))->temp;
        retained_sum += binary[index].value;
    }
    array_header *binary_header = header(raw_map(binary, sizeof(*binary)));
    printf("binary-del %d %zu %lld", deleted, binary_header->length - 1, retained_sum);
    for (size_t i = 0; i + 1 < binary_header->length; ++i)
        printf(" %u:%d", binary[i].key, binary[i].value);
    putchar('\n');
    hmfree(raw_map(binary, sizeof(*binary)), sizeof(*binary));

    binary = shmode(sizeof(*binary), 4);
    uint32_t unusual_key = 0xfeedbeefu;
    binary = hmput(binary, sizeof(*binary), &unusual_key, sizeof(unusual_key), 0);
    ptrdiff_t unusual_index = header(raw_map(binary, sizeof(*binary)))->temp;
    binary[unusual_index].value = 2468;
    binary = hmget(binary, sizeof(*binary), &unusual_key, sizeof(unusual_key), 0);
    unusual_index = header(raw_map(binary, sizeof(*binary)))->temp;
    printf("unusual-mode %08x %d\n", binary[unusual_index].key,
           binary[unusual_index].value);
    hmfree(raw_map(binary, sizeof(*binary)), sizeof(*binary));

    for (int mode = 2; mode <= 3; ++mode) {
        rand_seed(0xabcdefu + (size_t) mode);
        string_entry *map = shmode(sizeof(*map), mode);
        map = hmdefault(map, sizeof(*map));
        map[-1].value = -33;
        char keys[32][24];
        for (int i = 0; i < 32; ++i) {
            snprintf(keys[i], sizeof(keys[i]), "key-%02d-%c", i, 'A' + i % 7);
            map = hmput(map, sizeof(*map), keys[i], sizeof(char *), 1);
            ptrdiff_t index = header(raw_map(map, sizeof(*map)))->temp;
            map[index].value = i * i;
        }
        long long sum = 0;
        for (int i = 0; i < 35; ++i) {
            char lookup[24];
            snprintf(lookup, sizeof(lookup), "key-%02d-%c", i, 'A' + i % 7);
            map = hmget(map, sizeof(*map), lookup, sizeof(char *), 1);
            ptrdiff_t index = header(raw_map(map, sizeof(*map)))->temp;
            sum += map[index].value;
        }
        for (int i = 1; i < 32; i += 4)
            map = hmdel(map, sizeof(*map), keys[i], sizeof(char *), 0, 1);
        array_header *map_header = header(raw_map(map, sizeof(*map)));
        printf("string-map %d %lld %zu", mode, sum, map_header->length - 1);
        for (size_t i = 0; i + 1 < map_header->length; ++i)
            printf(" %s:%d", map[i].key, map[i].value);
        putchar('\n');
        hmfree(raw_map(map, sizeof(*map)), sizeof(*map));
    }

    printf("strkey %s ", strkey(0));
    printf("%s ", strkey(-12));
    printf("%s\n", strkey(999));

    puts("sh_geti-begin");
    rand_seed(0x31415926u);
    sh_geti(18);
    puts("sh_geti-end");

    dlclose(library);
    return 0;
}
