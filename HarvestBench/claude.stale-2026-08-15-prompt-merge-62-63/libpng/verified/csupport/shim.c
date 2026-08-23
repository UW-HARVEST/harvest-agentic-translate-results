/* setjmp shim for the Rust translation.
 *
 * `setjmp` has to be executed in the frame that the matching `longjmp` is
 * meant to resume, so it cannot be wrapped in a helper that returns before
 * the `longjmp` happens.  Instead the protected code is passed in as a
 * callback: the shim frame (which owns the jmp_buf and the landing pad) stays
 * alive for exactly as long as the callback runs, which is precisely the
 * lifetime the C code gives to `png_safe_execute`'s `safe_jmpbuf` and to
 * `png_create_png_struct`'s `create_jmp_buf`.
 */
#include <setjmp.h>

/* Run body(arg) with a setjmp landing pad whose address is published through
 * *env_slot (exactly what C libpng does when it stores `safe_jmpbuf` in
 * image->opaque->error_buf, or `&create_jmp_buf` in
 * create_struct.jmp_buf_ptr).  Returns body's return value, or `on_longjmp`
 * if a longjmp back to the pad occurred.
 *
 * Note: none of the parameters is modified after setjmp, so their values are
 * well defined on the longjmp return path without `volatile`.
 */
int png_rust_protect(void **env_slot, int (*body)(void *), void *arg,
    int on_longjmp)
{
    jmp_buf env;

    if (setjmp(env) == 0)
    {
        *env_slot = (void *)env;  /* jmp_buf is an array type: decays */
        return body(arg);
    }

    return on_longjmp;
}
