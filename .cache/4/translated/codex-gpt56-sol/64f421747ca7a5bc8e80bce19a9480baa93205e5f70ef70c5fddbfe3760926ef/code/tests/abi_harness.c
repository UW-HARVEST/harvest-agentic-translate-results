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
} Header;

typedef struct StringBlock StringBlock;
typedef struct {
    StringBlock *storage;
    size_t remaining;
    unsigned char block;
    unsigned char mode;
} StringArena;

typedef struct {
    size_t hash[8];
    ptrdiff_t index[8];
} HashBucket;

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
    StringArena string;
    HashBucket *storage;
} HashIndex;

typedef struct {
    int key;
    int value;
} IntEntry;

typedef struct {
    char *key;
    int value;
} StringEntry;

static void *(*arrgrowf)(void *, size_t, size_t, size_t);
static void (*arrfreef)(void *);
static void (*rand_seed)(size_t);
static size_t (*hash_string)(char *, size_t);
static size_t (*hash_bytes)(void *, size_t, size_t);
static void *(*hmget_key_ts)(void *, size_t, void *, size_t, ptrdiff_t *, int);
static void *(*hmget_key)(void *, size_t, void *, size_t, int);
static void *(*hmput_default)(void *, size_t);
static void *(*shmode_func)(size_t, int);
static void *(*hmdel_key)(void *, size_t, void *, size_t, size_t, int);
static char *(*stralloc_fn)(StringArena *, char *);
static void *(*hmput_key)(void *, size_t, void *, size_t, int);
static void (*strreset_fn)(StringArena *);
static void (*hmfree_func)(void *, size_t);
static char *(*strkey_fn)(int);
static void (*hm_geti_fn)(int);

static Header *header(void *array) {
    return (Header *)array - 1;
}

static void *raw_map(void *map, size_t elem_size) {
    return (char *)map - elem_size;
}

static HashIndex *map_table(void *map, size_t elem_size) {
    return header(raw_map(map, elem_size))->hash_table;
}

static uint64_t mix(uint64_t value, uint64_t input) {
    value ^= input;
    return value * UINT64_C(1099511628211);
}

static uint64_t hash_map_table(HashIndex *table) {
    uint64_t value = UINT64_C(1469598103934665603);
    size_t i;
    size_t j;
    value = mix(value, table->slot_count);
    value = mix(value, table->used_count);
    value = mix(value, table->tombstone_count);
    value = mix(value, table->seed);
    for (i = 0; i < table->slot_count / 8; ++i) {
        for (j = 0; j < 8; ++j) {
            value = mix(value, table->storage[i].hash[j]);
            value = mix(value, (uint64_t)table->storage[i].index[j]);
        }
    }
    return value;
}

static void load_symbol(void *library, void *target, const char *name) {
    void *symbol = dlsym(library, name);
    if (symbol == NULL) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    memcpy(target, &symbol, sizeof(symbol));
}

#define LOAD(name) load_symbol(library, &name, #name)

static void load_exports(void *library) {
    load_symbol(library, &arrgrowf, "stbds_arrgrowf");
    load_symbol(library, &arrfreef, "stbds_arrfreef");
    load_symbol(library, &rand_seed, "stbds_rand_seed");
    load_symbol(library, &hash_string, "stbds_hash_string");
    load_symbol(library, &hash_bytes, "stbds_hash_bytes");
    load_symbol(library, &hmget_key_ts, "stbds_hmget_key_ts");
    load_symbol(library, &hmget_key, "stbds_hmget_key");
    load_symbol(library, &hmput_default, "stbds_hmput_default");
    load_symbol(library, &shmode_func, "stbds_shmode_func");
    load_symbol(library, &hmdel_key, "stbds_hmdel_key");
    load_symbol(library, &stralloc_fn, "stbds_stralloc");
    load_symbol(library, &hmput_key, "stbds_hmput_key");
    load_symbol(library, &strreset_fn, "stbds_strreset");
    load_symbol(library, &hmfree_func, "stbds_hmfree_func");
    load_symbol(library, &strkey_fn, "strkey");
    load_symbol(library, &hm_geti_fn, "hm_geti");
}

static void test_hashes(void) {
    unsigned char bytes[80];
    char high_string[] = {'A', (char)0xff, 'z', '\0'};
    const size_t seeds[] = {0, 1, UINT64_C(0x0123456789abcdef), SIZE_MAX};
    const size_t lengths[] = {0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 31, 32, 63, 79};
    size_t i;
    size_t j;
    for (i = 0; i < sizeof(bytes); ++i)
        bytes[i] = (unsigned char)(i * 37 + 11);

    for (i = 0; i < sizeof(seeds) / sizeof(seeds[0]); ++i) {
        printf("hs %zu %016zx %016zx %016zx %016zx\n",
               i,
               hash_string("", seeds[i]),
               hash_string("hello", seeds[i]),
               hash_string("hash-table/key", seeds[i]),
               hash_string(high_string, seeds[i]));
        for (j = 0; j < sizeof(lengths) / sizeof(lengths[0]); ++j)
            printf("hb %zu %zu %016zx\n",
                   i, lengths[j], hash_bytes(bytes, lengths[j], seeds[i]));
    }
}

static void test_array(void) {
    unsigned char *array = arrgrowf(NULL, 3, 0, 1);
    Header *h = header(array);
    size_t i;
    printf("arr0 %zu %zu %td\n", h->length, h->capacity, h->temp);
    for (i = 0; i < 9; ++i)
        array[i] = (unsigned char)(i + 20);
    h->length = 3;
    array = arrgrowf(array, 3, 10, 0);
    h = header(array);
    printf("arr1 %zu %zu", h->length, h->capacity);
    for (i = 0; i < 9; ++i)
        printf(" %u", array[i]);
    putchar('\n');
    arrfreef(array);
}

static void test_arena(void) {
    StringArena arena = {0};
    char long_string[701];
    char *a;
    char *b;
    char *c;
    uint64_t checksum = 0;
    size_t i;
    for (i = 0; i < sizeof(long_string) - 1; ++i)
        long_string[i] = (char)('a' + i % 26);
    long_string[sizeof(long_string) - 1] = '\0';

    a = stralloc_fn(&arena, "alpha");
    b = stralloc_fn(&arena, long_string);
    c = stralloc_fn(&arena, "omega");
    for (i = 0; b[i] != '\0'; ++i)
        checksum = mix(checksum, (unsigned char)b[i]);
    printf("arena %s %s %zu %u %016llx\n",
           a, c, arena.remaining, arena.block,
           (unsigned long long)checksum);
    strreset_fn(&arena);
    printf("arena-reset %d %zu %u %u\n",
           arena.storage == NULL, arena.remaining, arena.block, arena.mode);
}

static void test_strkey(void) {
    const int values[] = {0, 1, -1, INT_MAX, INT_MIN};
    char copy[256];
    size_t i;
    for (i = 0; i < sizeof(values) / sizeof(values[0]); ++i) {
        strcpy(copy, strkey_fn(values[i]));
        printf("strkey %d %s\n", values[i], copy);
    }
}

static void put_int(void **map, int key, int value) {
    IntEntry *entries;
    Header *h;
    *map = hmput_key(*map, sizeof(IntEntry), &key, sizeof(key), 0);
    entries = *map;
    h = header(raw_map(*map, sizeof(IntEntry)));
    entries[h->temp].key = key;
    entries[h->temp].value = value;
}

static int get_int(void **map, int key, int threaded) {
    IntEntry *entries;
    ptrdiff_t index;
    if (threaded)
        *map = hmget_key_ts(*map, sizeof(IntEntry), &key, sizeof(key), &index, 0);
    else {
        *map = hmget_key(*map, sizeof(IntEntry), &key, sizeof(key), 0);
        index = header(raw_map(*map, sizeof(IntEntry)))->temp;
    }
    entries = *map;
    return entries[index].value;
}

static void test_binary_map(void) {
    void *map = NULL;
    IntEntry *entries;
    Header *h;
    HashIndex *table;
    uint64_t entries_hash = UINT64_C(1469598103934665603);
    long long sum = 0;
    int i;
    int key;

    rand_seed(UINT64_C(0x123456789abcdef0));
    map = hmput_default(map, sizeof(IntEntry));
    ((IntEntry *)map)[-1].value = -777;
    for (i = 0; i < 64; ++i) {
        key = i * 37 % 97;
        put_int(&map, key, key * 11);
    }
    for (i = -10; i < 111; ++i)
        sum += get_int(&map, i, i & 1);
    for (i = 0; i < 64; i += 3) {
        key = i * 37 % 97;
        put_int(&map, key, -key * 7);
    }
    h = header(raw_map(map, sizeof(IntEntry)));
    table = map_table(map, sizeof(IntEntry));
    printf("imap0 %zu %zu %lld %016llx\n",
           h->length - 1, h->capacity, sum,
           (unsigned long long)hash_map_table(table));

    for (i = 0; i < 64; i += 4) {
        key = i * 37 % 97;
        map = hmdel_key(map, sizeof(IntEntry), &key, sizeof(key), 0, 0);
    }
    entries = map;
    h = header(raw_map(map, sizeof(IntEntry)));
    for (i = 0; i < (int)h->length - 1; ++i) {
        entries_hash = mix(entries_hash, (uint32_t)entries[i].key);
        entries_hash = mix(entries_hash, (uint32_t)entries[i].value);
    }
    table = map_table(map, sizeof(IntEntry));
    printf("imap1 %zu %zu %td %016llx %016llx\n",
           h->length - 1, h->capacity, h->temp,
           (unsigned long long)entries_hash,
           (unsigned long long)hash_map_table(table));
    key = 1001;
    printf("imap-miss %d\n", get_int(&map, key, 0));
    hmfree_func(raw_map(map, sizeof(IntEntry)), sizeof(IntEntry));
}

static void test_string_mode(int ownership_mode) {
    char names[24][32];
    char query[32];
    void *map;
    StringEntry *entries;
    Header *h;
    HashIndex *table;
    uint64_t checksum = UINT64_C(1469598103934665603);
    int i;

    rand_seed((size_t)(700 + ownership_mode));
    map = ownership_mode == 1 ? NULL : shmode_func(sizeof(StringEntry), ownership_mode);
    map = hmput_default(map, sizeof(StringEntry));
    ((StringEntry *)map)[-1].value = -91;
    for (i = 0; i < 24; ++i) {
        snprintf(names[i], sizeof(names[i]), "name_%02d", i);
        strcpy(query, names[i]);
        map = hmput_key(map, sizeof(StringEntry),
                        ownership_mode == 1 ? names[i] : query,
                        sizeof(char *), 1);
        h = header(raw_map(map, sizeof(StringEntry)));
        ((StringEntry *)map)[h->temp].value = i * i + ownership_mode;
        if (ownership_mode != 1)
            memset(query, 'X', strlen(query));
    }

    for (i = 23; i >= 0; --i) {
        snprintf(query, sizeof(query), "name_%02d", i);
        map = hmget_key(map, sizeof(StringEntry), query, sizeof(char *), 1);
        h = header(raw_map(map, sizeof(StringEntry)));
        entries = map;
        checksum = mix(checksum, (uint32_t)entries[h->temp].value);
        checksum = mix(checksum, (unsigned char)entries[h->temp].key[5]);
    }
    for (i = 2; i < 24; i += 5) {
        snprintf(query, sizeof(query), "name_%02d", i);
        map = hmdel_key(map, sizeof(StringEntry), query, sizeof(char *), 0, 1);
    }
    h = header(raw_map(map, sizeof(StringEntry)));
    table = map_table(map, sizeof(StringEntry));
    printf("smap%d %zu %zu %u %zu %016llx %016llx\n",
           ownership_mode, h->length - 1, h->capacity,
           table->string.mode, table->string.remaining,
           (unsigned long long)checksum,
           (unsigned long long)hash_map_table(table));
    hmfree_func(raw_map(map, sizeof(StringEntry)), sizeof(StringEntry));
}

static void test_hm_geti(void) {
    hm_geti_fn(-1);
    hm_geti_fn(0);
    hm_geti_fn(1);
    hm_geti_fn(2);
    hm_geti_fn(31);
    hm_geti_fn(1024);
    puts("hm_geti ok");
}

int main(int argc, char **argv) {
    void *library;
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }
    library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }
    load_exports(library);
    puts("exports 16");
    test_hashes();
    test_array();
    test_arena();
    test_strkey();
    test_binary_map();
    test_string_mode(1);
    test_string_mode(2);
    test_string_mode(3);
    test_hm_geti();
    dlclose(library);
    return 0;
}
