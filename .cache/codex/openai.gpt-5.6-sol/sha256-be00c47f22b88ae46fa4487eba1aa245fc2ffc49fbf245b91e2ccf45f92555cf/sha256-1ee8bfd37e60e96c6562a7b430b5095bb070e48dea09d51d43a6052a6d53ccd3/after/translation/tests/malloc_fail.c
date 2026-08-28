#include <stddef.h>

extern void *__libc_malloc(size_t size);

static _Thread_local int should_fail_next_malloc;

void fail_next_malloc(void)
{
  should_fail_next_malloc = 1;
}

void *malloc(size_t size)
{
  if(should_fail_next_malloc) {
    should_fail_next_malloc = 0;
    return NULL;
  }

  return __libc_malloc(size);
}
