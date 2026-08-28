/* Loads one implementation of the library with dlopen and drives it while an
 * allocation failure is armed. Run once against the C .so and once against the
 * Rust .so; the two stdout streams and exit codes must match exactly.
 *
 * usage: oom_driver <library.so> <scenario> <fail_size> [p1 p2 p3 p4]
 *        scenario = create_state | confusion
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void *(*create_state_t)(int, int);
typedef void (*destroy_state_t)(void *);
typedef int (*process_buffer_t)(void *, char);
typedef int (*confusion_t)(int, int, int, int);

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <so> <scenario> <fail_size> [p1 p2 p3 p4]\n", argv[0]);
        return 2;
    }
    void (*arm)(size_t) = (void (*)(size_t))dlsym(RTLD_DEFAULT, "oom_arm");
    int (*fired)(void) = (int (*)(void))dlsym(RTLD_DEFAULT, "oom_fired");
    void (*disarm)(void) = (void (*)(void))dlsym(RTLD_DEFAULT, "oom_disarm");
    void (*areset)(void) = (void (*)(void))dlsym(RTLD_DEFAULT, "oom_reset");
    unsigned long (*amallocs)(void) = (unsigned long (*)(void))dlsym(RTLD_DEFAULT, "oom_mallocs");
    unsigned long (*afrees)(void) = (unsigned long (*)(void))dlsym(RTLD_DEFAULT, "oom_frees");
    unsigned long (*abytes)(void) = (unsigned long (*)(void))dlsym(RTLD_DEFAULT, "oom_bytes");
    if (!arm || !fired || !disarm || !areset || !amallocs || !afrees || !abytes) {
        fprintf(stderr, "oom shim symbols not found: LD_PRELOAD missing\n");
        return 3;
    }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) {
        fprintf(stderr, "dlopen(%s): %s\n", argv[1], dlerror());
        return 4;
    }

    const char *scen = argv[2];
    size_t fail_size = (size_t)strtoull(argv[3], NULL, 10);
    int p1 = argc > 4 ? (int)strtol(argv[4], NULL, 10) : 0;
    int p2 = argc > 5 ? (int)strtol(argv[5], NULL, 10) : 0;
    int p3 = argc > 6 ? (int)strtol(argv[6], NULL, 10) : 0;
    int p4 = argc > 7 ? (int)strtol(argv[7], NULL, 10) : 0;

    create_state_t cs = (create_state_t)dlsym(h, "create_state");
    destroy_state_t ds = (destroy_state_t)dlsym(h, "destroy_state");
    process_buffer_t pb = (process_buffer_t)dlsym(h, "process_buffer");
    confusion_t cf = (confusion_t)dlsym(h, "confusion");
    if (!cs || !ds || !pb || !cf) {
        fprintf(stderr, "dlsym failed on %s\n", argv[1]);
        return 5;
    }

    /* Warm stdio up so printf's own buffer is already allocated and cannot
     * interact with the armed failure. */
    printf("driver ready\n");
    fflush(stdout);

    if (strcmp(scen, "create_state") == 0) {
        arm(fail_size);
        void *s = cs(p1, p2);
        printf("create_state -> %s\n", s ? "NON-NULL" : "NULL");
        printf("oom_fired = %d\n", fired());
        if (s) {
            unsigned int flags;
            int cap;
            char *buf;
            memcpy(&flags, (char *)s + 0, sizeof flags);
            memcpy(&buf, (char *)s + 8, sizeof buf);
            memcpy(&cap, (char *)s + 16, sizeof cap);
            printf("flags = 0x%08x cap = %d buf_null = %d\n", flags, cap, buf == NULL);
            if (buf && cap > 0) printf("buf = \"%s\"\n", buf);
            printf("process_buffer('0') -> %d\n", pb(s, '0'));
            ds(s);
        }
    } else if (strcmp(scen, "confusion") == 0) {
        arm(fail_size);
        int r = cf(p1, p2, p3, p4);
        printf("confusion -> %d\n", r);
        printf("oom_fired = %d\n", fired());
    } else if (strcmp(scen, "alloc_trace") == 0) {
        /* Exercise the full pipeline and report the allocator trace. */
        create_state_t cs2 = cs;
        void (*uf)(void *, int) = (void (*)(void *, int))dlsym(h, "update_flags");
        int (*ct)(void *, int) = (int (*)(void *, int))dlsym(h, "confuse_types");
        areset();
        void *s = cs2(p1, p2 > 0 ? p2 : 128);
        if (s) {
            uf(s, p3);
            (void)pb(s, (char)('0' + (p3 % 10)));
            (void)ct(s, p4 % 4);
            ds(s);
        }
        printf("create/destroy: mallocs=%lu frees=%lu bytes=%lu\n",
               amallocs(), afrees(), abytes());
        areset();
        (void)cf(p1, p2, p3, p4);
        printf("confusion:      mallocs=%lu frees=%lu bytes=%lu\n",
               amallocs(), afrees(), abytes());
        areset();
        for (int i = 0; i < 5; i++) {
            void *t = cs2(p1 + i, 64 + i);
            if (t) ds(t);
        }
        printf("five rounds:    mallocs=%lu frees=%lu bytes=%lu\n",
               amallocs(), afrees(), abytes());
    } else {
        fprintf(stderr, "unknown scenario %s\n", scen);
        return 6;
    }

    disarm();
    fflush(stdout);
    return 0;
}
