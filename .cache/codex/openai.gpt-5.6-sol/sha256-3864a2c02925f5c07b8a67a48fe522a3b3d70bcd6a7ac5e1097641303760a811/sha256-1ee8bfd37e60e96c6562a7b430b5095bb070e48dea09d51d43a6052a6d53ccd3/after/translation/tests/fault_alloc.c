#include <stddef.h>
#include <stdint.h>

/*
 * Test-only malloc interposer. The integration test enables it only around a
 * gotomach call, after both libraries and stdio have been warmed up.
 */
extern void *__libc_malloc(size_t size);
extern void __libc_free(void *pointer);

static int enabled;
static int fault_mode;
static size_t fail_at;
static size_t allocation_count;
static void *state_allocation;

void fault_configure(int mode, size_t nth_allocation)
{
    fault_mode = mode;
    fail_at = nth_allocation;
    allocation_count = 0;
    state_allocation = NULL;
    enabled = 1;
}

void fault_disable(void)
{
    enabled = 0;
}

void *malloc(size_t size)
{
    void *result;

    if (!enabled) {
        return __libc_malloc(size);
    }

    allocation_count++;
    if (fault_mode == 1 && allocation_count == fail_at) {
        return NULL;
    }

    result = __libc_malloc(size);
    if (allocation_count == 1) {
        state_allocation = result;
    }

    if (allocation_count == 3 && state_allocation) {
        if (fault_mode == 2) {
            /* offsetof(ProcessorState, status) on the tested 64-bit ABI. */
            ((unsigned char *)state_allocation)[32] = 0;
        } else if (fault_mode == 3) {
            /* offsetof(ProcessorState, capacity) on the tested 64-bit ABI. */
            *((size_t *)((unsigned char *)state_allocation + 8)) = 0;
        }
    }

    return result;
}

void free(void *pointer)
{
    __libc_free(pointer);
}
