#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

typedef void (*no_arg_fn)(void);
typedef void (*driver_fn)(int);
typedef void (*print_hex_char_line_fn)(char);
typedef void (*print_line_fn)(const char *);

static void *load_symbol(void *library, const char *name)
{
    void *symbol;

    dlerror();
    symbol = dlsym(library, name);
    if (dlerror() != NULL) {
        fprintf(stderr, "missing symbol: %s\n", name);
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv)
{
    void *library;
    no_arg_fn bad;
    no_arg_fn good;
    driver_fn driver;
    print_hex_char_line_fn print_hex_char_line;
    print_line_fn print_line;
    const char non_ascii[] = {(char)0xff, '%', 'x', '\0'};
    int value;

    if (argc != 2) {
        return 64;
    }

    library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return 1;
    }

    bad = (no_arg_fn)load_symbol(library, "bad");
    driver = (driver_fn)load_symbol(library, "driver");
    good = (no_arg_fn)load_symbol(library, "good");
    print_hex_char_line =
        (print_hex_char_line_fn)load_symbol(library, "printHexCharLine");
    print_line = (print_line_fn)load_symbol(library, "printLine");

    print_line(NULL);
    print_line("");
    print_line("plain text");
    print_line("format tokens: %s %x %%");
    print_line(non_ascii);

    for (value = SCHAR_MIN; value <= SCHAR_MAX; ++value) {
        print_hex_char_line((char)value);
    }

    bad();
    good();
    driver(INT_MIN);
    driver(-1);
    driver(0);
    driver(1);
    driver(INT_MAX);

    return dlclose(library) == 0 ? 0 : 3;
}
