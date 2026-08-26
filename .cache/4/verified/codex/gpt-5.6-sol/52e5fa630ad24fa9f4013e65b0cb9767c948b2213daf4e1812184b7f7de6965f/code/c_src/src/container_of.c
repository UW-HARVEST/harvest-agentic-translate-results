#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define offsetof(TYPE, MEMBER)  ((size_t) (&(((TYPE *)(0))->MEMBER)))

#define container_of(ptr, type, member) ({         \
    (type *)( (char *)ptr - offsetof(type, member) );})

struct test {
    int a;
    int b;
};

struct test* find_container_of_a(int *i) {
    return (container_of(i, struct test, a));
}

struct test* find_container_of_b(int *i) {
    return (container_of(i, struct test, b));
}

int main(int argc, char** argv) {
    int a = atoi(argv[1]);
    int b = atoi(argv[2]);

    struct test t;

    memset(&t, 0, sizeof(t));
    t.a = a;
    t.b = b;

     printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
}
