/*
 * External-caller harness for the C-vs-Rust differential tests.
 *
 * This is NOT part of c_src; it is test scaffolding compiled on demand by
 * translation/tests/external_caller.rs.
 *
 * It dlopen()s whichever `libdriver.so` it is handed (the C one or the Rust
 * one) and calls the requested scenario. Because the program, the call sites
 * and the process startup path are identical for both libraries, the two runs
 * are directly comparable -- including for `bad()`, whose behaviour depends on
 * the state of the stack below the call.
 *
 * usage: c_caller <path-to-libdriver.so> <scenario> [arg]
 */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*fn_void)(void);
typedef void (*fn_int)(int);
typedef void (*fn_str)(const char *);

int main(int argc, char **argv)
{
    if (argc < 3)
    {
        fprintf(stderr, "usage: %s <lib> <scenario> [arg]\n", argv[0]);
        return 2;
    }

    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (h == NULL)
    {
        fprintf(stderr, "dlopen(%s): %s\n", argv[1], dlerror());
        return 3;
    }

    fn_str printLine = (fn_str)dlsym(h, "printLine");
    fn_void bad = (fn_void)dlsym(h, "bad");
    fn_void good = (fn_void)dlsym(h, "good");
    fn_int driver = (fn_int)dlsym(h, "driver");
    if (!printLine || !bad || !good || !driver)
    {
        fprintf(stderr, "missing symbol in %s: %s\n", argv[1], dlerror());
        return 4;
    }

    const char *scenario = argv[2];
    const char *arg = argc > 3 ? argv[3] : "";

    if (strcmp(scenario, "driver") == 0)
    {
        driver((int)strtol(arg, NULL, 0));
    }
    else if (strcmp(scenario, "bad") == 0)
    {
        bad();
    }
    else if (strcmp(scenario, "good") == 0)
    {
        good();
    }
    else if (strcmp(scenario, "printLine") == 0)
    {
        printLine(arg);
    }
    else if (strcmp(scenario, "printLine_null") == 0)
    {
        printLine(NULL);
    }
    /* Repetition and ordering: state that leaks between calls shows up here. */
    else if (strcmp(scenario, "bad_x8") == 0)
    {
        for (int i = 0; i < 8; i++)
            bad();
    }
    else if (strcmp(scenario, "good_x8") == 0)
    {
        for (int i = 0; i < 8; i++)
            good();
    }
    else if (strcmp(scenario, "driver_alternating") == 0)
    {
        for (int i = 0; i < 8; i++)
            driver(i % 2);
    }
    else if (strcmp(scenario, "mixed") == 0)
    {
        good();
        bad();
        printLine("mixed");
        driver(1);
        driver(0);
        printLine(NULL);
        bad();
        driver(-7);
    }
    /* bad() reached right after printLine(arg) at the same stack depth. On
       gcc -O0 x86_64 this makes the C bad() re-print `arg`, because printLine
       saves its argument at exactly the [rbp-8] slot that bad() then reads
       uninitialized. Used to document the mechanism. */
    else if (strcmp(scenario, "printLine_then_bad") == 0)
    {
        printLine(arg);
        bad();
    }
    /* Discriminating cases: the producing call and bad() sit at *different*
       stack depths, so the [rbp-8] slots do not alias. */
    else if (strcmp(scenario, "printLine_then_driver0") == 0)
    {
        printLine(arg);
        driver(0);
    }
    else if (strcmp(scenario, "good_then_driver0") == 0)
    {
        good();
        driver(0);
    }
    else if (strcmp(scenario, "driver1_then_bad") == 0)
    {
        driver(1);
        bad();
    }
    else if (strcmp(scenario, "driver0_then_bad") == 0)
    {
        driver(0);
        bad();
    }
    /* bad() reached after the stack has been churned by unrelated work. */
    else if (strcmp(scenario, "bad_after_churn") == 0)
    {
        volatile char pad[512];
        memset((void *)pad, 0x5A, sizeof pad);
        printLine("churn");
        bad();
    }
    else
    {
        fprintf(stderr, "unknown scenario: %s\n", scenario);
        return 5;
    }

    fflush(stdout);
    dlclose(h);
    return 0;
}
