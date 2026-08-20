#include "lib.h"

#include <stdio.h>
#include <string.h>
#include <errno.h>

const char*
extractFilename(const char* path, char separator)
{
    const char* search = strrchr(path, separator);
    if (search == NULL) return path;
    return search+1;
}

/* FIO_createFilename_fromOutDir() :
 * Takes a source file name and specified output directory, and
 * allocates memory for and returns a pointer to final path.
 * This function never returns an error (it may abort() in case of pb)
 */
char*
FIO_createFilename_fromOutDir(const char* path, const char* outDirName, const size_t suffixLen)
{
    const char* filenameStart;
    char separator;
    char* result;

#if defined(_MSC_VER) || defined(__MINGW32__) || defined (__MSVCRT__) /* windows support */
    separator = '\\';
#else
    separator = '/';
#endif

    filenameStart = extractFilename(path, separator);
#if defined(_MSC_VER) || defined(__MINGW32__) || defined (__MSVCRT__) /* windows support */
    filenameStart = extractFilename(filenameStart, '/');  /* sometimes, '/' separator is also used on Windows (mingw+msys2) */
#endif

    result = (char*) calloc(1, strlen(outDirName) + 1 + strlen(filenameStart) + suffixLen + 1);
    if (!result) {
        fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno));
        exit(30);
    }

    memcpy(result, outDirName, strlen(outDirName));
    if (outDirName[strlen(outDirName)-1] == separator) {
        memcpy(result + strlen(outDirName), filenameStart, strlen(filenameStart));
    } else {
        memcpy(result + strlen(outDirName), &separator, 1);
        memcpy(result + strlen(outDirName) + 1, filenameStart, strlen(filenameStart));
    }

    return result;
}
