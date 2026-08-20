#ifndef SHARED_H
#define SHARED_H

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#define OS_MAXSTR 1024

#define os_free(x) if(x){free(x);x=NULL;};
#define os_clearnl(x,p) if((p = strrchr(x, '\n')))*p = '\0';

void *os_calloc(size_t num, size_t size) {
    void *out = calloc(num, size);
    if (!out) {
        fprintf(stderr, "Memory allocation failed in os_calloc");
        exit(EXIT_FAILURE);
    }
    return out;
}

void *os_realloc(void *ptr, size_t new_size) {
    void *out = realloc(ptr, new_size);
    if (!out) {
        fprintf(stderr, "Memory allocation failed in os_realloc");
        exit(EXIT_FAILURE);
    }
    return out;
}

char *os_strdup(const char *str) {
    if (!str) {
        fprintf(stderr, "NULL string passed to os_strdup");
        exit(EXIT_FAILURE);
    }
    char *dup = strdup(str);
    if (!dup) {
        fprintf(stderr, "Memory allocation failed in os_strdup");
        exit(EXIT_FAILURE);
    }
    return dup;
}

#endif
