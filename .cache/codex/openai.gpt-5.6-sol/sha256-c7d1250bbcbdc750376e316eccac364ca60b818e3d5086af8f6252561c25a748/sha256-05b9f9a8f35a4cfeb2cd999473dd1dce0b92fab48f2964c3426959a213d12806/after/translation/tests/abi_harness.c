#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

typedef void (*line_fn)(const char *);
typedef void (*void_fn)(void);

static void *load_symbol(void *library, const char *name)
{
    void *symbol = dlsym(library, name);
    const char *error = dlerror();

    if (error != NULL)
    {
        fprintf(stderr, "%s\n", error);
        exit(2);
    }

    return symbol;
}

int main(int argc, char **argv)
{
    if (argc != 2)
    {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL)
    {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    line_fn print_line = (line_fn)load_symbol(library, "printLine");
    void_fn bad_fn = (void_fn)load_symbol(library, "bad");
    void_fn good_fn = (void_fn)load_symbol(library, "good");
    void_fn driver_fn = (void_fn)load_symbol(library, "driver");
    const char raw[] = {'r', 'a', 'w', ':', (char)0x80, (char)0xff, '\0'};

    print_line(NULL);
    print_line("");
    print_line("plain %s");
    print_line(raw);
    bad_fn();
    good_fn();
    driver_fn();

    return dlclose(library) == 0 ? 0 : 2;
}
