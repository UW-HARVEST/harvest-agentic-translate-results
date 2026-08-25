#include <stddef.h>

extern void *__libc_malloc(size_t size);

enum {
    FAULT_NONE = 0,
    FAULT_RETURN_NULL = 1,
    FAULT_ZERO_STATUS = 2,
    FAULT_FILL_CAPACITY = 3,
};

typedef struct {
    int *results;
    size_t capacity;
    size_t count;
    void *operation;
    char status;
} ProcessorStateLayout;

static int fault_action;
static size_t fault_allocation;
static size_t allocation_count;
static ProcessorStateLayout *state_allocation;

void fault_malloc_configure(int action, size_t allocation) {
    fault_action = action;
    fault_allocation = allocation;
    allocation_count = 0;
    state_allocation = NULL;
}

void fault_malloc_disable(void) {
    fault_action = FAULT_NONE;
    allocation_count = 0;
    state_allocation = NULL;
}

void *malloc(size_t size) {
    if (fault_action == FAULT_NONE) {
        return __libc_malloc(size);
    }

    allocation_count++;
    if (fault_action == FAULT_RETURN_NULL &&
        allocation_count == fault_allocation) {
        return NULL;
    }

    void *allocation = __libc_malloc(size);
    if (allocation_count == 1) {
        state_allocation = allocation;
    }

    if (allocation_count == fault_allocation && state_allocation != NULL) {
        if (fault_action == FAULT_ZERO_STATUS) {
            state_allocation->status = 0;
        } else if (fault_action == FAULT_FILL_CAPACITY) {
            state_allocation->count = state_allocation->capacity;
        }
    }

    return allocation;
}
