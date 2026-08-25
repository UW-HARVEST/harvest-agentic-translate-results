#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char *data;
    int capacity;
    int length;
} StringBuffer;

typedef StringBuffer *(*create_buffer_fn)(int);
typedef int (*append_to_buffer_fn)(StringBuffer *, const char *);
typedef void (*destroy_buffer_fn)(StringBuffer *);
typedef const char *(*get_operation_name_fn)(int);
typedef int (*perform_operation_fn)(int, int, const char *);
typedef int (*buffapp_fn)(int, int, int, int);

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    if (!symbol) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc < 2 || argc > 3) {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW);
    if (!library) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    create_buffer_fn create_buffer = (create_buffer_fn)load_symbol(library, "create_buffer");
    append_to_buffer_fn append_to_buffer =
        (append_to_buffer_fn)load_symbol(library, "append_to_buffer");
    destroy_buffer_fn destroy_buffer =
        (destroy_buffer_fn)load_symbol(library, "destroy_buffer");
    get_operation_name_fn get_operation_name =
        (get_operation_name_fn)load_symbol(library, "get_operation_name");
    perform_operation_fn perform_operation =
        (perform_operation_fn)load_symbol(library, "perform_operation");
    buffapp_fn buffapp = (buffapp_fn)load_symbol(library, "buffapp");

    if (argc == 3) {
        if (strcmp(argv[2], "division-fault") != 0) {
            return 2;
        }
        return perform_operation(INT_MIN, -1, "divide");
    }

    StringBuffer *buffer = create_buffer(4);
    printf("buffer-initial:%d:%d:%s\n", buffer->capacity, buffer->length, buffer->data);
    printf("append-1:%d\n", append_to_buffer(buffer, "abc"));
    printf("buffer-1:%d:%d:%s\n", buffer->capacity, buffer->length, buffer->data);
    printf("append-2:%d\n", append_to_buffer(buffer, "defgh"));
    printf("buffer-2:%d:%d:%s\n", buffer->capacity, buffer->length, buffer->data);
    destroy_buffer(buffer);
    destroy_buffer(NULL);

    for (int operation_code = -2; operation_code <= 5; ++operation_code) {
        printf("name:%d:%s\n", operation_code, get_operation_name(operation_code));
    }

    static const char *operations[] = {
        "add", "subtract", "multiply", "divide", "Divide", "",
    };
    for (size_t index = 0; index < sizeof(operations) / sizeof(operations[0]); ++index) {
        printf("operation:%s:%d\n",
               operations[index],
               perform_operation(-21, index == 3 ? 0 : 5, operations[index]));
    }
    printf("operation:divide-nonzero:%d\n", perform_operation(-21, 5, "divide"));

    static const int inputs[][4] = {
        {4, 2, 6, 3},
        {1, 8, 3, 2},
        {2, -3, 0, 7},
        {-1, 10, -2, 4},
        {0, 0, 0, 0},
    };
    for (size_t index = 0; index < sizeof(inputs) / sizeof(inputs[0]); ++index) {
        int result = buffapp(inputs[index][0], inputs[index][1], inputs[index][2],
                             inputs[index][3]);
        printf("buffapp-result:%zu:%d\n", index, result);
    }

    for (int a = -4; a <= 4; ++a) {
        for (int b = -4; b <= 4; ++b) {
            for (int c = -4; c <= 4; ++c) {
                for (int d = -4; d <= 4; ++d) {
                    printf("grid-result:%d:%d:%d:%d:%d\n", a, b, c, d,
                           buffapp(a, b, c, d));
                }
            }
        }
    }

    dlclose(library);
    return 0;
}
