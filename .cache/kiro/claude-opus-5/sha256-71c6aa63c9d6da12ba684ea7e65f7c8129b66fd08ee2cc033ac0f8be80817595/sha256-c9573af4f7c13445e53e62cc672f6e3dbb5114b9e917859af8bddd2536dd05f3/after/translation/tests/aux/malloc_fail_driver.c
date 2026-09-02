/* Driver for ERRORS.md row 18 — the `checkshift` malloc-failure branch.
 *
 * Loads BOTH shared objects with dlopen and calls their exported `checkshift`
 * through function pointers, exactly like the Rust integration tests do, but from
 * a plain C process so that no Rust runtime allocation can interfere with the
 * interposed malloc.
 *
 * usage: malloc_fail_driver <c.so> <rust.so>
 * exit:  0 = C and Rust agree, 1 = they diverge, 2 = harness problem
 *
 * Not part of the library under test; lives under translation/tests/aux/.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef int (*checkshift_t)(int, int, int, int);

#define CAP 65536

/* Call `fn(1,2,3,4)` with stdout redirected to a temp file, optionally with
 * malloc(12) failing. Returns the return value; fills `out` with the bytes
 * written. */
static int run_capture(checkshift_t fn, int *flag, int arm, char *out, int *hits,
                       const int *hit_counter) {
    char tmpl[] = "/tmp/checkshift-mfail-XXXXXX";
    int before = *hit_counter;

    fflush(NULL);
    int saved = dup(1);
    int fd = mkstemp(tmpl);
    if (saved < 0 || fd < 0) {
        fprintf(stderr, "driver: cannot set up capture\n");
        exit(2);
    }
    if (dup2(fd, 1) < 0) {
        fprintf(stderr, "driver: dup2 failed\n");
        exit(2);
    }

    if (arm) {
        *flag = 1;
    }
    int rc = fn(1, 2, 3, 4);
    *flag = 0;

    fflush(NULL);
    dup2(saved, 1);
    close(saved);

    lseek(fd, 0, SEEK_SET);
    ssize_t n = read(fd, out, CAP - 1);
    if (n < 0) {
        n = 0;
    }
    out[n] = '\0';
    close(fd);
    unlink(tmpl);

    *hits = *hit_counter - before;
    return rc;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <c.so> <rust.so>\n", argv[0]);
        return 2;
    }

    int *flag = (int *)dlsym(RTLD_DEFAULT, "checkshift_fail_malloc_12");
    int *hit_counter = (int *)dlsym(RTLD_DEFAULT, "checkshift_fail_malloc_12_hits");
    if (flag == NULL || hit_counter == NULL) {
        fprintf(stderr, "driver: malloc shim is not LD_PRELOADed\n");
        return 2;
    }

    void *hc = dlopen(argv[1], RTLD_NOW);
    if (hc == NULL) {
        fprintf(stderr, "driver: dlopen(%s): %s\n", argv[1], dlerror());
        return 2;
    }
    void *hr = dlopen(argv[2], RTLD_NOW);
    if (hr == NULL) {
        fprintf(stderr, "driver: dlopen(%s): %s\n", argv[2], dlerror());
        return 2;
    }
    checkshift_t fc = (checkshift_t)dlsym(hc, "checkshift");
    checkshift_t fr = (checkshift_t)dlsym(hr, "checkshift");
    if (fc == NULL || fr == NULL) {
        fprintf(stderr, "driver: dlsym(checkshift) failed\n");
        return 2;
    }

    static char c_ok[CAP], r_ok[CAP], c_fail[CAP], r_fail[CAP];
    int hits;
    int rc;

    /* Control: no injected failure. Also warms up both libraries so that any
     * one-time runtime allocation happens outside the failure window. */
    int c_rc_ok = run_capture(fc, flag, 0, c_ok, &hits, hit_counter);
    int r_rc_ok = run_capture(fr, flag, 0, r_ok, &hits, hit_counter);
    if (c_rc_ok != r_rc_ok) {
        fprintf(stderr, "control: C returned %d but Rust returned %d\n", c_rc_ok, r_rc_ok);
        return 1;
    }
    if (strcmp(c_ok, r_ok) != 0) {
        fprintf(stderr, "control: stdout differs\n--- C ---\n%s\n--- Rust ---\n%s\n", c_ok, r_ok);
        return 1;
    }

    /* The failure branch. */
    int c_rc = run_capture(fc, flag, 1, c_fail, &hits, hit_counter);
    if (hits != 1) {
        fprintf(stderr, "C: expected exactly 1 intercepted malloc(12), saw %d\n", hits);
        return 2;
    }
    int r_rc = run_capture(fr, flag, 1, r_fail, &hits, hit_counter);
    if (hits != 1) {
        fprintf(stderr, "Rust: expected exactly 1 intercepted malloc(12), saw %d\n", hits);
        return 2;
    }

    rc = 0;
    if (c_rc != -1) {
        fprintf(stderr, "C did not return the -1 sentinel on malloc failure (got %d)\n", c_rc);
        rc = 2;
    }
    if (r_rc != c_rc) {
        fprintf(stderr, "malloc failure: C returned %d but Rust returned %d\n", c_rc, r_rc);
        rc = 1;
    }
    if (strcmp(c_fail, r_fail) != 0) {
        fprintf(stderr,
                "malloc failure: stdout differs\n--- C ---\n%s\n--- Rust ---\n%s\n",
                c_fail, r_fail);
        rc = 1;
    }

    const char *expected =
        "\n=== Starting foo function ===\n"
        "Parameters: 1, 2, 3, 4\n"
        "Error: Failed to allocate memory for state\n";
    if (strcmp(c_fail, expected) != 0) {
        fprintf(stderr, "C failure output was not the expected text:\n%s\n", c_fail);
        rc = 2;
    }

    if (rc == 0) {
        fprintf(stderr,
                "row 18 ok: both returned %d with identical output (%zu bytes)\n",
                c_rc, strlen(c_fail));
    }
    return rc;
}
