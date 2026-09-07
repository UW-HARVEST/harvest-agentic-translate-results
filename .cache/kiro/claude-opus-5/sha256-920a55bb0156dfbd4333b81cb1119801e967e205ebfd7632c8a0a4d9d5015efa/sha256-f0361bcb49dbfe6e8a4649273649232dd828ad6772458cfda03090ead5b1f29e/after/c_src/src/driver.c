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

#include <stdlib.h>

#include "matrix.h"
#include "write.h"

#include <stdio.h>

#define OUT_FILE "matrix.txt"

int driver(int width_a, int height_a, const char* matrix_a, int width_b, int height_b, const char* matrix_b) {
    matrix_t* mat_a = initialize_matrix_from_string(matrix_a, width_a, height_a);
    if (mat_a == NULL) {
        return EXIT_FAILURE;
    }
    matrix_t* mat_b = initialize_matrix_from_string(matrix_b, width_b, height_b);
    if (mat_b == NULL) {
        free_matrix(mat_a);
        return EXIT_FAILURE;
    }

    matrix_t* res = multiply_matrices(mat_a, mat_b);
    if (res == NULL) {
        free_matrix(mat_a);
        free_matrix(mat_b);
        return EXIT_FAILURE;
    }
    char* res_str = matrix_to_string(res);
    if (res_str == NULL) {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free(res);
        return EXIT_FAILURE;
    }

    int res_write = write_to_file(OUT_FILE, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    free(res_str);

    if (res_write != 0) {
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
