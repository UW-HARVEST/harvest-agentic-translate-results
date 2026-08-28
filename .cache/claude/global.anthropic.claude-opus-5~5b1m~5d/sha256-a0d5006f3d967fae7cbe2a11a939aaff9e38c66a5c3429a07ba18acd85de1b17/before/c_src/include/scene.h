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
// scene.h
#ifndef SCENE_H
#define SCENE_H

#include "shape.h"

#define MAX_SHAPES_IN_SCENE 50
#define MAX_SCENE_NAME 64

typedef struct {
    char name[MAX_SCENE_NAME];
    shape_t *shapes[MAX_SHAPES_IN_SCENE];
    int shape_count;
} scene_t;

// Create a new empty scene
scene_t* scene_create(const char *name);

// Destroy a scene
void scene_destroy(scene_t *scene);

// Add a shape to the scene
int scene_add_shape(scene_t *scene, shape_t *shape);

// Remove a shape at index
int scene_remove_shape(scene_t *scene, int index);

// Print the scene
void scene_print(const scene_t *scene);

// Compare two scenes for equality (1:1 correspondence)
int scene_equals(const scene_t *s1, const scene_t *s2);

// Save scene to file
int scene_save(const scene_t *scene, const char *filename);

// Load scene from file
scene_t* scene_load(const char *filename);

// List all shapes in scene
void scene_list_shapes(const scene_t *scene);

#endif // SCENE_H
