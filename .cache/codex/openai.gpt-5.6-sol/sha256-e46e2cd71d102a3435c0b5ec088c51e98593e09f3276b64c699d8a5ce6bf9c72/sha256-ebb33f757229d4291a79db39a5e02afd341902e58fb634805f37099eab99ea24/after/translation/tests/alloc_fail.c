#include <stdatomic.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

extern void *__libc_malloc(size_t);
extern void *__libc_calloc(size_t, size_t);
extern void __libc_free(void *);

static _Atomic long malloc_countdown = -1;
static _Atomic long calloc_countdown = -1;
static _Atomic long deterministic_allocations = 0;
static _Atomic size_t arena_offset = 0;

union Arena {
    max_align_t alignment;
    unsigned char bytes[4096];
};

static union Arena arena;

void arm_malloc_failure(long successful_calls_before_failure) {
    atomic_store(&malloc_countdown, successful_calls_before_failure);
}

void arm_calloc_failure(long successful_calls_before_failure) {
    atomic_store(&calloc_countdown, successful_calls_before_failure);
}

void arm_deterministic_allocations(void) {
    atomic_store(&arena_offset, 0);
    atomic_store(&deterministic_allocations, 4);
}

static void *arena_allocate(size_t size, int clear) {
    const size_t alignment = _Alignof(max_align_t);
    if (size == 0) {
        size = 1;
    }
    size_t old_offset = atomic_load(&arena_offset);
    size_t offset = (old_offset + alignment - 1) & ~(alignment - 1);
    if (offset > sizeof(arena.bytes) || size > sizeof(arena.bytes) - offset) {
        return NULL;
    }
    atomic_store(&arena_offset, offset + size);
    void *result = &arena.bytes[offset];
    if (clear) {
        memset(result, 0, size);
    }
    atomic_fetch_sub(&deterministic_allocations, 1);
    return result;
}

void *malloc(size_t size) {
    if (atomic_load(&deterministic_allocations) > 0) {
        return arena_allocate(size, 0);
    }
    long remaining = atomic_load(&malloc_countdown);
    if (remaining >= 0) {
        remaining = atomic_fetch_sub(&malloc_countdown, 1);
        if (remaining == 0) {
            atomic_store(&malloc_countdown, -1);
            return NULL;
        }
    }
    return __libc_malloc(size);
}

void *calloc(size_t count, size_t size) {
    if (atomic_load(&deterministic_allocations) > 0) {
        if (size != 0 && count > SIZE_MAX / size) {
            return NULL;
        }
        return arena_allocate(count * size, 1);
    }
    long remaining = atomic_load(&calloc_countdown);
    if (remaining >= 0) {
        remaining = atomic_fetch_sub(&calloc_countdown, 1);
        if (remaining == 0) {
            atomic_store(&calloc_countdown, -1);
            return NULL;
        }
    }
    return __libc_calloc(count, size);
}

void free(void *pointer) {
    uintptr_t address = (uintptr_t)pointer;
    uintptr_t start = (uintptr_t)&arena.bytes[0];
    uintptr_t end = (uintptr_t)&arena.bytes[sizeof(arena.bytes)];
    if (address >= start && address < end) {
        return;
    }
    __libc_free(pointer);
}
