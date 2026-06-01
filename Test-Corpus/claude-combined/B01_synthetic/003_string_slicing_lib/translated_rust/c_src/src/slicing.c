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
#include <string.h>

#include "slicing.h"

/*
Index into a passed string
and print the substring indexed by [*start_ptr, *stop_ptr).
If there is no start, use 0.
If there is no stop, use the end of the string. 
*/

int slice(char *mystr, int *start_ptr, int *stop_ptr) {

    size_t len = strlen(mystr);

    char *end;
    int start, stop;

    if (start_ptr) {
        start = *start_ptr;
        if (start > len) {
            printf("Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if (stop_ptr) {
        stop = *stop_ptr;
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
    printf("%.*s\n", stop - start, mystr + start);

    return 0;
}
