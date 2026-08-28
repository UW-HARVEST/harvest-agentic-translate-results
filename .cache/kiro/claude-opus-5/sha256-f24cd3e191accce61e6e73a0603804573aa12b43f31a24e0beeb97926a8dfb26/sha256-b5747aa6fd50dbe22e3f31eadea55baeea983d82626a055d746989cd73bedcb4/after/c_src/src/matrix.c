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
#include <string.h>

#include "matrix.h"
#include "write.h"

matrix_t* allocate_matrix(int width, int height) {
    matrix_t* mat = malloc(sizeof(matrix_t));
    if (mat == NULL) {
        perror("Failed to allocate memory for matrix struct");
        return NULL;
    }

    mat->width = width;
    mat->height = height;

    mat->matrix = malloc(height * sizeof(int*));
    if (mat->matrix == NULL) {
        perror("Failed to allocate memory for matrix rows");
        free(mat);
        return NULL;
    }

    for (int i = 0; i < height; i++) {
        mat->matrix[i] = malloc(width * sizeof(int));
        if (mat->matrix[i] == NULL) {
            perror("Failed to allocate memory for matrix columns");
            for (int j = 0; j <= i; j++) {
                free(mat->matrix[j]);
            }
            free(mat->matrix);
            free(mat);
            return NULL;
        }
    }

    return mat;
}

void free_matrix(matrix_t* mat) {
    if (mat == NULL) {
        return;
    }

    for (int i = 0; i < mat->height; i++) {
        free(mat->matrix[i]);
    }
    free(mat->matrix);
    free(mat);
}

matrix_t* initialize_matrix_from_string(const char* input, int width, int height) {
    matrix_t* mat = allocate_matrix(width, height);

    char* input_copy = strdup(input);
    if (input_copy == NULL) {
        perror("Failed to duplicate input string");
        free_matrix(mat);
        return NULL;
    }

    char* saveptr_row;
    char* row_token = strtok_r(input_copy, "\n", &saveptr_row);
    for (int i = 0; i < height; i++) {
        if (row_token == NULL) {
            fprintf(stderr, "Insufficient rows in input string.\n");
            free(input_copy);
            free_matrix(mat);
            return NULL;
        }

        char* saveptr_col;
        char* col_token = strtok_r(row_token, " ", &saveptr_col); 
        for (int j = 0; j < width; j++) {
            if (col_token == NULL) {
                fprintf(stderr, "Insufficient columns in row %d.\n", i + 1);
                free(input_copy);
                free_matrix(mat);
                return NULL;
            }
            mat->matrix[i][j] = atoi(col_token);
            col_token = strtok_r(NULL, " ", &saveptr_col);
        }

        row_token = strtok_r(NULL, "\n", &saveptr_row);
    }

    free(input_copy);
    return mat;
}

matrix_t* multiply_matrices(matrix_t* mat_a, matrix_t* mat_b) {
    if (mat_a->width != mat_b->height) {
        fprintf(stderr, "Matrix dimensions do not allow multiplication.\n");
        return NULL;
    }

    matrix_t* result = allocate_matrix(mat_b->width, mat_a->height);
    for (int i = 0; i < mat_a->height; i++) {
        for (int j = 0; j < mat_b->width; j++) {
            result->matrix[i][j] = 0;
            for (int k = 0; k < mat_a->width; k++) {
                result->matrix[i][j] += mat_a->matrix[i][k] * mat_b->matrix[k][j];
            }
        }
    }

    return result;
}

char* matrix_to_string(matrix_t* mat) {
    if (mat == NULL) {
        fprintf(stderr, "Error: Matrix is NULL.\n");
        return NULL;
    }

    int buffer_size = mat->height * (mat->width * 10 + mat->width) + mat->height + 1;
    char* result = malloc(buffer_size);
    if (result == NULL) {
        perror("Failed to allocate memory for matrix string");
        return NULL;
    }

    result[0] = '\0';

    for (int i = 0; i < mat->height; i++) {
        for (int j = 0; j < mat->width; j++) {
            char buffer[12];
            snprintf(buffer, sizeof(buffer), "%d", mat->matrix[i][j]);
            strcat(result, buffer);

            if (j < mat->width - 1) {
                strcat(result, " ");
            }
        }
        strcat(result, "\n");
    }

    return result;
}
