#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*sh_geti_fn)(int);
typedef size_t (*hash_string_fn)(char *, size_t);
typedef size_t (*hash_bytes_fn)(void *, size_t, size_t);

int main(int argc, char **argv)
{
    void *library;

    if (argc < 3)
        return 2;
    library = dlopen(argv[1], RTLD_NOW);
    if (!library) {
        fputs(dlerror(), stderr);
        return 3;
    }

    if (strcmp(argv[2], "sh_geti") == 0) {
        sh_geti_fn function = (sh_geti_fn)dlsym(library, "sh_geti");
        for (int argument = 3; argument < argc; ++argument)
            function(atoi(argv[argument]));
    } else if (strcmp(argv[2], "hash_string") == 0) {
        hash_string_fn function = (hash_string_fn)dlsym(library, "stbds_hash_string");
        size_t seed = (size_t)strtoull(argv[3], NULL, 0);
        for (int argument = 4; argument < argc; ++argument)
            printf("%016zx\n", function(argv[argument], seed));
    } else if (strcmp(argv[2], "hash_bytes") == 0) {
        hash_bytes_fn function = (hash_bytes_fn)dlsym(library, "stbds_hash_bytes");
        size_t seed = (size_t)strtoull(argv[3], NULL, 0);
        unsigned char bytes[512];
        size_t length = 0;
        for (char *cursor = argv[4]; cursor[0] && cursor[1]; cursor += 2)
            bytes[length++] = (unsigned char)strtoul((char[3]){cursor[0], cursor[1], 0}, NULL, 16);
        printf("%016zx\n", function(bytes, length, seed));
    } else {
        return 4;
    }

    dlclose(library);
    return 0;
}
