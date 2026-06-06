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
#include <stdint.h>
#include <stddef.h>

/* External function declaration */
int process_strings(char *input, size_t input_len, 
                   const char *reference, size_t ref_len,
                   int operation, uint32_t flags);

#define MAX_BUFFER_SIZE 1024

int main(void) {
    int operation;
    uint32_t flags;
    size_t input_len, ref_len;
    char input_buffer[MAX_BUFFER_SIZE];
    char ref_buffer[MAX_BUFFER_SIZE];
    
    /* Read operation */
    if (scanf("%d", &operation) != 1) {
        fprintf(stderr, "Error reading operation\n");
        return 1;
    }
    
    /* Read flags */
    if (scanf("%u", &flags) != 1) {
        fprintf(stderr, "Error reading flags\n");
        return 1;
    }
    
    /* Read input length */
    if (scanf("%zu", &input_len) != 1) {
        fprintf(stderr, "Error reading input length\n");
        return 1;
    }
    
    if (input_len > MAX_BUFFER_SIZE) {
        fprintf(stderr, "Error: input length %zu exceeds maximum %d\n", 
                input_len, MAX_BUFFER_SIZE);
        return 1;
    }
    
    /* Read input buffer data */
    for (size_t i = 0; i < input_len; i++) {
        unsigned int byte;
        if (scanf("%u", &byte) != 1) {
            fprintf(stderr, "Error reading input byte %zu\n", i);
            return 1;
        }
        input_buffer[i] = (char)byte;
    }
    
    /* Read reference length */
    if (scanf("%zu", &ref_len) != 1) {
        fprintf(stderr, "Error reading reference length\n");
        return 1;
    }
    
    if (ref_len > MAX_BUFFER_SIZE) {
        fprintf(stderr, "Error: reference length %zu exceeds maximum %d\n", 
                ref_len, MAX_BUFFER_SIZE);
        return 1;
    }
    
    /* Read reference buffer data */
    for (size_t i = 0; i < ref_len; i++) {
        unsigned int byte;
        if (scanf("%u", &byte) != 1) {
            fprintf(stderr, "Error reading reference byte %zu\n", i);
            return 1;
        }
        ref_buffer[i] = (char)byte;
    }
    
    /* Call the library function */
    int result = process_strings(input_buffer, input_len, 
                                 ref_buffer, ref_len, 
                                 operation, flags);
    
    /* Print result to stdout */
    printf("%d\n", result);
    
    return 0;
}
