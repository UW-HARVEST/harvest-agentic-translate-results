/* setjmp shim so the Rust translation of png_safe_execute can catch a longjmp
 * from png_safe_error, exactly as the C simplified API does. */
#include <setjmp.h>

/* jmp_buf is an array type; wrap it so Rust passes a pointer. */
int png_rust_setjmp(jmp_buf *env) {
    return setjmp(*env);
}
