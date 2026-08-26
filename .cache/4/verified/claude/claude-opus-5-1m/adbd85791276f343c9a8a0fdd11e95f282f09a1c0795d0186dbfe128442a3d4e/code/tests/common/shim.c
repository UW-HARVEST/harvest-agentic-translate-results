/* Differential-test support shim.
 *
 * libpng reports fatal errors by calling png_longjmp(), which calls the
 * application-supplied longjmp_fn on the jmp_buf returned by
 * png_set_longjmp_fn().  A Rust test cannot host the setjmp() itself (setjmp
 * has no sound Rust binding), so this tiny C object provides the setjmp frame;
 * the body of the operation is a Rust callback.
 *
 * It also supplies the libpng `error_fn`: recording the message and jumping
 * straight out of the C error handler keeps libpng from falling through to
 * png_default_error(), which would spam stderr.  Warnings are recorded by a
 * Rust callback (they return normally).
 *
 * This is *test scaffolding only*: it is compiled into its own shared object,
 * dlopen'd by the test, and is never linked into either libpng under test.
 * Nothing in c_src/ is modified or used to build it.
 */
#include <setjmp.h>
#include <stddef.h>
#include <string.h>

/* png_longjmp() does `png_ptr->longjmp_fn(*png_ptr->jmp_buf_ptr, val)`, i.e. it
 * passes the decayed jmp_buf; the type therefore matches `longjmp` exactly,
 * which is what png.h's own png_jmpbuf() macro relies on. */
typedef void (*png_longjmp_ptr_t)(jmp_buf, int);
typedef jmp_buf *(*set_longjmp_fn_t)(void *png_ptr, png_longjmp_ptr_t fn,
                                     size_t jmp_buf_size);

static __thread jmp_buf *harness_jb;
static __thread char harness_msg[1024];
static __thread int harness_have_msg;

/* Installed as libpng's error_fn. */
void harness_error_fn(void *png_ptr, const char *msg)
{
   (void)png_ptr;
   harness_msg[0] = '\0';
   if (msg != NULL)
   {
      size_t n = strlen(msg);
      if (n > sizeof harness_msg - 1)
         n = sizeof harness_msg - 1;
      memcpy(harness_msg, msg, n);
      harness_msg[n] = '\0';
   }
   harness_have_msg = 1;

   if (harness_jb != NULL)
      longjmp(*harness_jb, 1);
   /* else: return and let libpng take its default path */
}

/* The message recorded by the most recent harness_run(), or NULL. */
const char *harness_last_error(void)
{
   return harness_have_msg ? harness_msg : NULL;
}

/* Run `body(arg)` with a libpng error trap armed on `png_ptr`.
 *
 * returns 0  -> body ran to completion
 *         1  -> libpng raised a fatal error (png_error/png_longjmp)
 *        -1  -> png_set_longjmp_fn() returned NULL (ABI/OOM failure)
 */
int harness_run(set_longjmp_fn_t set_longjmp_fn, void *png_ptr,
                void (*body)(void *), void *arg)
{
   jmp_buf local;
   jmp_buf *jb;
   jmp_buf *saved = harness_jb;

   harness_have_msg = 0;
   harness_msg[0] = '\0';

   if (set_longjmp_fn != NULL && png_ptr != NULL)
   {
      jb = set_longjmp_fn(png_ptr, longjmp, sizeof(jmp_buf));

      if (jb == NULL)
         return -1;
   }
   else
      jb = &local;

   harness_jb = jb;

   if (setjmp(*jb) != 0)
   {
      harness_jb = saved;
      return 1;
   }

   body(arg);
   harness_jb = saved;
   return 0;
}

size_t harness_jmp_buf_size(void)
{
   return sizeof(jmp_buf);
}
