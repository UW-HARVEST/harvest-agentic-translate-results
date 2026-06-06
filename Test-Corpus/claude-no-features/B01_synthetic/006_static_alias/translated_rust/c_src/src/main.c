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

int*
static_alias(int *outer) {
  static int inner = 1;
  if(*outer >= inner) {
    inner += *outer;
    return &inner;
  } else {
    *outer += inner;
    return outer;
  }
}

/*
  Maintain a sum leveraging multiple references to a static variable
 */
int
main(int argc, char **argv) {

  if (argc != 3) {
    printf("Error: should only be two (integer) arguments!\n");
    return 1;
  }

  char *end;
  int initial_value = strtol(argv[1], &end, 10);
  if (end == argv[1]) {
    // end is set to start of string if nothing parsed
    printf("Error: first argument must be an integer!\n");
    return 1;
  }

  int iterations = strtol(argv[2], &end, 10);
  if (end == argv[2]) {
    // end is set to start of string if nothing parsed
    printf("Error: second argument must be an integer!\n");
    return 1;
  }

  int *running_sum = &initial_value;
  for (int i = 0; i < iterations; i++) {
    running_sum = static_alias(running_sum);
    printf("%d\n", *running_sum);
  }

  return 0;
}
