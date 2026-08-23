/* C shim providing variadic public functions that the Rust cdylib cannot
 * define on stable. Each formats its arguments with vsnprintf (exactly like
 * the C original) then delegates to a Rust `*_impl` function. The public
 * linker symbols therefore match the original MuJS names.
 *
 * Compiled with -fexceptions/-funwind-tables so the Rust panic (used to model
 * longjmp) can unwind through these frames for the NORETURN error functions.
 */
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

typedef struct js_State js_State;

/* Rust-side implementations (take a pre-formatted message). */
extern void rs_js_error(js_State *J, const char *msg);
extern void rs_js_evalerror(js_State *J, const char *msg);
extern void rs_js_rangeerror(js_State *J, const char *msg);
extern void rs_js_referenceerror(js_State *J, const char *msg);
extern void rs_js_syntaxerror(js_State *J, const char *msg);
extern void rs_js_typeerror(js_State *J, const char *msg);
extern void rs_js_urierror(js_State *J, const char *msg);

extern void rs_jsC_error(js_State *J, void *node, const char *msg);
extern void rs_jsP_error(js_State *J, const char *msg);
extern void rs_jsP_warning(js_State *J, const char *msg);
extern void rs_jsY_error(js_State *J, const char *msg);

#define FMT(buf) do { \
        va_list ap; va_start(ap, fmt); \
        vsnprintf((buf), sizeof(buf), fmt, ap); \
        va_end(ap); \
    } while (0)

void js_error(js_State *J, const char *fmt, ...)          { char b[256]; FMT(b); rs_js_error(J, b); }
void js_evalerror(js_State *J, const char *fmt, ...)      { char b[256]; FMT(b); rs_js_evalerror(J, b); }
void js_rangeerror(js_State *J, const char *fmt, ...)     { char b[256]; FMT(b); rs_js_rangeerror(J, b); }
void js_referenceerror(js_State *J, const char *fmt, ...) { char b[256]; FMT(b); rs_js_referenceerror(J, b); }
void js_syntaxerror(js_State *J, const char *fmt, ...)    { char b[256]; FMT(b); rs_js_syntaxerror(J, b); }
void js_typeerror(js_State *J, const char *fmt, ...)      { char b[256]; FMT(b); rs_js_typeerror(J, b); }
void js_urierror(js_State *J, const char *fmt, ...)       { char b[256]; FMT(b); rs_js_urierror(J, b); }

/* jsC_error/jsP_error/jsY_error format "file:line: " + message. The Rust side
 * builds the "file:line: " prefix; here we only format the user message part,
 * matching the C code which does: vsnprintf(msgbuf,...); snprintf(buf,"file:line: "); strcat. */
void jsC_error(js_State *J, void *node, const char *fmt, ...) { char b[256]; FMT(b); rs_jsC_error(J, node, b); }
void jsP_error_shim(js_State *J, const char *fmt, ...)   { char b[256]; FMT(b); rs_jsP_error(J, b); }
void jsP_warning_shim(js_State *J, const char *fmt, ...) { char b[256]; FMT(b); rs_jsP_warning(J, b); }
void jsY_error_shim(js_State *J, const char *fmt, ...)   { char b[256]; FMT(b); rs_jsY_error(J, b); }
