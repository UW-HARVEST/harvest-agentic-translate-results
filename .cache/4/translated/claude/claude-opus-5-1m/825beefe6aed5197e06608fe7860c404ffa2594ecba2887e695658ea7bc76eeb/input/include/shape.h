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
#ifndef SHAPE_H
#define SHAPE_H

#include <stddef.h>

#define MAX_SHAPE_WIDTH 80
#define MAX_SHAPE_HEIGHT 30
#define MAX_SHAPE_NAME 32

typedef enum {
    SHAPE_TREE,
    SHAPE_TRACTOR,
    SHAPE_HOUSE,
    SHAPE_SUN,
    SHAPE_CLOUD,
    SHAPE_FLOWER,
    SHAPE_CAR,
    SHAPE_STAR,
    SHAPE_HEART,
    SHAPE_RAINBOW,
    SHAPE_COUNT
} shape_type_t;

typedef struct {
    shape_type_t type;
    char name[MAX_SHAPE_NAME];
    char art[MAX_SHAPE_HEIGHT][MAX_SHAPE_WIDTH];
    int width;
    int height;
} shape_t;

// Initialize the shape manager (allocate all shapes once)
void shape_manager_init(void);

// Clean up shape manager
void shape_manager_cleanup(void);

// Get a shape by type (returns singleton instance)
shape_t* shape_get(shape_type_t type);

// Print a shape to stdout
void shape_print(const shape_t *shape);

// Compare two shapes (equal if XOR of pointers is 0)
int shape_equals(const shape_t *s1, const shape_t *s2);

// Get shape type name
const char* shape_type_name(shape_type_t type);

#endif // SHAPE_H
