#include "lib.h"

#include <stdlib.h>
#include <string.h>

#include <stdio.h>

char *searchAndReplace(const char *orig, const char *search, const char *value)
{
    char *p;
    const size_t orig_len = strlen(orig);
    const size_t search_len = strlen(search);
    const size_t value_len = strlen(value);

    size_t inx_start;
    char *tmp = NULL;
    size_t tmp_offset = 0;
    size_t total_bytes_allocated = 1;
    size_t from;

    /* Check for any match */
    p = strstr(orig, search);
    if (p == NULL) {
        tmp = strdup(orig);
        return tmp;
    }

    inx_start = (size_t) (p - orig);
    from = inx_start + search_len;

    /* Copy content before first match, if any */
    if (inx_start > 0) {
        total_bytes_allocated = inx_start + 1;
        tmp = malloc(sizeof(char) * total_bytes_allocated);
        if (tmp == NULL) {
            return NULL;
        }
        strncpy(tmp, orig, inx_start);
        tmp_offset = inx_start;
    }

    while (p != NULL) {
        /* Copy replacement */
        total_bytes_allocated += value_len;
        tmp = realloc(tmp, total_bytes_allocated);
        if (tmp == NULL) {
            return NULL;
        }

        strncpy(tmp + tmp_offset, value, total_bytes_allocated - tmp_offset);
        tmp_offset += value_len;

        /* Search for further occurrences */
        p = strstr(orig + inx_start + search_len, search);
        if (p != NULL) {
            size_t inx_start2 = (size_t) (p - orig);

            /* Copy content between matches, if any */
            if (inx_start2 > from) {
                size_t gap = inx_start2 - from;
                total_bytes_allocated += gap;
                tmp = realloc(tmp, total_bytes_allocated);
                if (tmp == NULL) {
                    return NULL;
                }
                strncpy(tmp + tmp_offset, orig + from, gap);
                tmp_offset += gap;
            }

            inx_start = inx_start2;
        }

        /* Set position for copying content after last match */
        from = inx_start + search_len;
    }

    /* Copy content after last match, if any */
    if ((from < orig_len) && from > 0) {
        total_bytes_allocated += orig_len - from;
        tmp = realloc(tmp, total_bytes_allocated);
        if (tmp == NULL) {
            return NULL;
        }
        strncpy(tmp + tmp_offset, orig + from, orig_len - from);
    }

    tmp[total_bytes_allocated - 1] = '\0';

    return tmp;
}
