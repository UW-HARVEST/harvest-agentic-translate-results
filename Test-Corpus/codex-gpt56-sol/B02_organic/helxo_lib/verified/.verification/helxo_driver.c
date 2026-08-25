#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*helxo_fn)(char);

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 1;
    }
    void *symbol = dlsym(handle, "helxo");
    if (!symbol) {
        fprintf(stderr, "dlsym: %s\n", dlerror());
        return 1;
    }
    helxo_fn helxo;
    memcpy(&helxo, &symbol, sizeof(helxo));

    for (int value = -128; value <= 127; ++value)
        helxo((char)value);

    dlclose(handle);
    return 0;
}
