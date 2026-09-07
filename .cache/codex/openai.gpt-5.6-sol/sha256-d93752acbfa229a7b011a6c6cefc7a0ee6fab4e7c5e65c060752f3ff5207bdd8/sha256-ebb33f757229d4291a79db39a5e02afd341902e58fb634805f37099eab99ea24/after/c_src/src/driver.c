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

#include "driver.h"

#include <ctype.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>

void driver(char c) {
    setlocale(LC_ALL, "C");
    
    printf("alphanumeric: %d\n", isalnum(c));
    printf("alphabetic: %d\n", isalpha(c));
    printf("lowercase: %d\n", islower(c));
    printf("uppercase: %d\n", isupper(c));
    printf("digit: %d\n", isdigit(c));
    printf("hexadecimal: %d\n", isxdigit(c));
    printf("control: %d\n", iscntrl(c));
    printf("graphical: %d\n", isgraph(c));
    printf("space: %d\n", isspace(c));
    printf("blank: %d\n", isblank(c));
    printf("printing: %d\n", isprint(c));
    printf("punctuation: %d\n", ispunct(c));
    printf("to lower: %c\n", tolower(c));
    printf("to upper: %c\n", toupper(c));
}
