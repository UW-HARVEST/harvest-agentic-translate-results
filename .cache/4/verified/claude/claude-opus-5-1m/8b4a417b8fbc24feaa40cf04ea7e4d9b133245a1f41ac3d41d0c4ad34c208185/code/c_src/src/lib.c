#include "lib.h"

/* Create an array of pointers to the lines in a buffer */
const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize)
{
    size_t lineIndex = 0;
    size_t pos = 0;
    void* const bufferPtrs = malloc(numLines * sizeof(const char**));
    const char** const linePointers = (const char**)bufferPtrs;
    if (bufferPtrs == NULL) return NULL;

    while (lineIndex < numLines && pos < bufferSize) {
        size_t len = 0;
        linePointers[lineIndex++] = buffer+pos;

        /* Find the next null terminator, being careful not to go past the buffer */
        while ((pos + len < bufferSize) && buffer[pos + len] != '\0') {
            len++;
        }

        /* Move past this string and its null terminator */
        pos += len;
        if (pos < bufferSize) pos++;  /* Skip the null terminator if we're not at buffer end */
    }

    /* Verify we processed the expected number of lines */
    if (lineIndex != numLines) {
        /* Something went wrong - we didn't find as many lines as expected */
        free(bufferPtrs);
        return NULL;
    }

    return linePointers;
}
