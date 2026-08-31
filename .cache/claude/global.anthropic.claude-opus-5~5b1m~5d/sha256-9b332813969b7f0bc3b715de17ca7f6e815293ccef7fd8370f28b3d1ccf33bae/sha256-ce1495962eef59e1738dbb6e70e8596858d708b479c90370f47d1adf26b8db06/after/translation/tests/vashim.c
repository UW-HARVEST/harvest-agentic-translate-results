/*
 * va_list trampoline for the differential tests.
 *
 * Three jansson entry points take a `va_list` rather than `...`:
 *   json_vpack_ex, json_vunpack_ex, json_vsprintf   (plus jsonp_error_vset)
 *
 * Rust on stable cannot *construct* a va_list, but it can call a C variadic
 * function. So each shim below is variadic on the Rust-facing side, turns its
 * arguments into a real va_list, and forwards that to whichever
 * implementation's function pointer it is handed. That lets the tests drive the
 * v* symbols of BOTH libraries with identical arguments.
 *
 * This file lives under translation/tests/ and is compiled into its own
 * separate .so; nothing in c_src/ is touched.
 */

#include <stdarg.h>
#include <stddef.h>

typedef struct json_t json_t;
typedef struct json_error_t json_error_t;

typedef json_t *(*vpack_fn)(json_error_t *, size_t, const char *, va_list);
typedef int (*vunpack_fn)(json_t *, json_error_t *, size_t, const char *, va_list);
typedef json_t *(*vsprintf_fn)(const char *, va_list);
typedef void (*verror_fn)(json_error_t *, int, int, size_t, int, const char *, va_list);

json_t *shim_vpack_ex(void *fn, json_error_t *error, size_t flags, const char *fmt, ...) {
    va_list ap;
    json_t *ret;
    va_start(ap, fmt);
    ret = ((vpack_fn)fn)(error, flags, fmt, ap);
    va_end(ap);
    return ret;
}

int shim_vunpack_ex(void *fn, json_t *root, json_error_t *error, size_t flags,
                    const char *fmt, ...) {
    va_list ap;
    int ret;
    va_start(ap, fmt);
    ret = ((vunpack_fn)fn)(root, error, flags, fmt, ap);
    va_end(ap);
    return ret;
}

json_t *shim_vsprintf(void *fn, const char *fmt, ...) {
    va_list ap;
    json_t *ret;
    va_start(ap, fmt);
    ret = ((vsprintf_fn)fn)(fmt, ap);
    va_end(ap);
    return ret;
}

void shim_error_vset(void *fn, json_error_t *error, int line, int column, size_t position,
                     int code, const char *msg, ...) {
    va_list ap;
    va_start(ap, msg);
    ((verror_fn)fn)(error, line, column, position, code, msg, ap);
    va_end(ap);
}
