#include <dlfcn.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

typedef void (*print_line_fn)(const char *);
typedef void (*print_int_line_fn)(int);
typedef void (*bad_fn)(float);
typedef void (*good_fn)(float);
typedef void (*driver_fn)(float, float);

static void *required_symbol(void *library, const char *name)
{
    void *symbol = dlsym(library, name);
    if (symbol == NULL)
    {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv)
{
    void *library;
    print_line_fn print_line;
    print_int_line_fn print_int_line;
    bad_fn bad_function;
    good_fn good_function;
    driver_fn driver_function;
    const float bad_values[] = {
        2.0F, -2.0F, 0.0F, -0.0F, NAN, INFINITY, -INFINITY,
        1.0e-30F, -1.0e-30F, 5.0e-8F, 4.0e-8F
    };
    const float good_values[] = {
        2.0F, -2.0F, 0.0F, -0.0F, NAN, INFINITY, -INFINITY,
        0.000001F, -0.000001F, 0.0000011F, -0.0000011F
    };
    size_t index;

    if (argc != 2)
    {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL)
    {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    *(void **)(&print_line) = required_symbol(library, "printLine");
    *(void **)(&print_int_line) = required_symbol(library, "printIntLine");
    *(void **)(&bad_function) = required_symbol(library, "bad");
    *(void **)(&good_function) = required_symbol(library, "good");
    *(void **)(&driver_function) = required_symbol(library, "driver");

    print_line(NULL);
    print_line("");
    print_line("plain text");
    print_line("%d %s");

    print_int_line(0);
    print_int_line(1);
    print_int_line(-1);
    print_int_line(INT_MAX);
    print_int_line(INT_MIN);

    for (index = 0; index < sizeof(bad_values) / sizeof(bad_values[0]); ++index)
    {
        bad_function(bad_values[index]);
    }
    for (index = 0; index < sizeof(good_values) / sizeof(good_values[0]); ++index)
    {
        good_function(good_values[index]);
    }

    driver_function(2.0F, 4.0F);
    driver_function(0.0F, 0.0F);

    if (dlclose(library) != 0)
    {
        fprintf(stderr, "dlclose failed: %s\n", dlerror());
        return 2;
    }
    return 0;
}
