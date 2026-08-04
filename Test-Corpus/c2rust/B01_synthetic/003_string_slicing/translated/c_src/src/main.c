// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
// 
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
Index into a passed string
and print the substring indexed by [start, stop).
If there is no start, use 0.
If there is no stop, use the end of the string. 
*/
int main(int argc, char **argv) {

    if ((argc > 4) || (argc == 1)) {
        printf("Error: there should be one to three arguments passed:\n");
        printf("<string> [start] [stop]\n");
        return 1;
    }

    size_t len = strlen(argv[1]);
    int start, stop;

    char *end;

    if (argc >= 3) {
        start = strtol(argv[2], &end, 10);
        if (end == argv[2]) {
            printf("Second argument must be an integer!");
            return 1;
        }
        if (start > len) {
            printf("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if (argc == 4) {
        stop = strtol(argv[3], NULL, 10);
        if (end == argv[3]) {
            printf("Third argument must be an integer!");
            return 1;
        }

        if (stop > len) {
            printf("Error: stop is off the end of the string!\n");
            return 1;
        }
    
        if (stop <= start) {
            printf("Error: stop must come after start!\n");
            return 1;
        }
    // single-line else statement just to make style checking sad
    } else stop = len;


    /* char arithmetic: skip ahead `start` characters in the array */
    printf("%.*s\n", stop - start, argv[1] + start);

    return 0;
}
