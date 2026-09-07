/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 * 
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 * 
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

#include <stdio.h>
#include <stdlib.h>

int forward_goto_example(int x) {
  if (x < 0) {
    goto error;
  }

  printf("Processing: %d\n", x);
  return x * 2;

error:
  fprintf(stderr, "Error: negative input\n");
  return -1;
}

FILE* open_with_cleanup(const char *filename) {
  FILE* fp = fopen(filename, "r");
  if (!fp) {
    goto cleanup;
  }

  char buffer[100];
  while (fgets(buffer, sizeof(buffer), fp)) {
      printf("%s", buffer);
  }

  if (ferror(fp)) {
      goto cleanup;
  }

  return fp;

cleanup:
  fprintf(stderr, "Error: opening or processing file %s\n", filename);
  if(fp) fclose(fp);
  return NULL;
}

int driver(int num, const char* filename) {
  int res = forward_goto_example(num);
  if (res == -1) {
      return -1;
  } else {
      printf("Goto output: %d\n", res);
  }

  FILE* out = open_with_cleanup(filename);
  if (out == NULL) {
      return -2;
  } else {
     fclose(out);
  }

  return 0;
}
