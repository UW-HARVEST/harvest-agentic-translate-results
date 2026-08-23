/* Test-harness shim: provides a real setjmp landing pad so that the Rust
 * differential tests can drive libpng exactly like a C application does
 * (png_error -> longjmp -> back to the caller).
 *
 * This file is NOT part of the library; it is compiled at test time into a
 * small shared object which the integration tests dlopen.
 */
#include <setjmp.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>

typedef void (*th_fn)(void *);

static __thread jmp_buf th_env;
static __thread int th_active = 0;

/* Run f(ctx) with a longjmp landing pad installed.  Returns 0 if f returned
 * normally, or the non-zero value passed to th_longjmp otherwise.
 */
int th_protect(th_fn f, void *ctx)
{
    jmp_buf saved;
    volatile int was = th_active;
    int v;

    if (was)
        memcpy(&saved, &th_env, sizeof(jmp_buf));

    v = setjmp(th_env);

    if (v == 0)
    {
        th_active = 1;
        f(ctx);
    }

    th_active = was;
    if (was)
        memcpy(&th_env, &saved, sizeof(jmp_buf));

    return v;
}

/* Matches png_longjmp_ptr: void (*)(jmp_buf, int).  The buffer libpng hands us
 * is ignored; we jump to the pad installed by th_protect.
 */
void th_longjmp(void *env, int val)
{
    (void)env;
    if (th_active)
        longjmp(th_env, val != 0 ? val : 1);
    abort();
}

size_t th_jmp_buf_size(void)
{
    return sizeof(jmp_buf);
}
