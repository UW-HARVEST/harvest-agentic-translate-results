#define _GNU_SOURCE

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

extern void *__libc_malloc(size_t size);

enum {
    SHIM_FAIL_ALLOCATION = 1,
    SHIM_ZERO_STATUS_BEFORE_CHECK = 2,
    SHIM_ZERO_STATUS_DURING_LOOP = 3,
    SHIM_FILL_COUNT_DURING_LOOP = 4,
};

static volatile int armed;
static volatile int shim_mode;
static volatile int fail_at;
static volatile size_t allocation_count;
static unsigned char *processor_state;
static volatile int operation_count;

void shim_arm(int mode, int allocation_to_fail) {
    processor_state = NULL;
    allocation_count = 0;
    operation_count = 0;
    fail_at = allocation_to_fail;
    shim_mode = mode;
    armed = 1;
}

void shim_disarm(void) {
    armed = 0;
}

void *malloc(size_t size) {
    if (!armed) {
        return __libc_malloc(size);
    }

    size_t call = ++allocation_count;
    if (shim_mode == SHIM_FAIL_ALLOCATION && call == (size_t)fail_at) {
        return NULL;
    }

    void *result = __libc_malloc(size);
    if (call == 1 && size >= 40) {
        processor_state = result;
    }

    if (shim_mode == SHIM_ZERO_STATUS_BEFORE_CHECK && call == 3 &&
        processor_state != NULL) {
        processor_state[32] = 0;
    }

    return result;
}

static void mutate_state_after_operation(void) {
    if (!armed || processor_state == NULL || operation_count++ != 0) {
        return;
    }

    if (shim_mode == SHIM_ZERO_STATUS_DURING_LOOP) {
        processor_state[32] = 0;
    } else if (shim_mode == SHIM_FILL_COUNT_DURING_LOOP) {
        size_t capacity = *(size_t *)(processor_state + 8);
        *(size_t *)(processor_state + 16) = capacity;
    }
}

int process_value(int value, int unused_param, void *unused_context) {
    (void)unused_param;
    (void)unused_context;
    int result = value + 10;
    mutate_state_after_operation();
    return result;
}

int double_value(int value, int unused_param, void *unused_context) {
    (void)unused_param;
    (void)unused_context;
    int result = value * 2;
    mutate_state_after_operation();
    return result;
}

int triple_value(int value, int unused_param, void *unused_context) {
    (void)unused_param;
    (void)unused_context;
    int result = value * 3;
    mutate_state_after_operation();
    return result;
}
