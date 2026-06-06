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
// scene.c
#include "scene.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

scene_t* scene_create(const char *name) {
    scene_t *scene = malloc(sizeof(scene_t));
    if (!scene) {
        return NULL;
    }
    
    if (name) {
        strncpy(scene->name, name, MAX_SCENE_NAME - 1);
        scene->name[MAX_SCENE_NAME - 1] = '\0';
    } else {
        strcpy(scene->name, "Untitled Scene");
    }
    
    scene->shape_count = 0;
    
    return scene;
}

void scene_destroy(scene_t *scene) {
    if (scene) {
        // Note: We don't free the shapes themselves
        // because they are singletons managed by shape_manager
        free(scene);
    }
}

int scene_add_shape(scene_t *scene, shape_t *shape) {
    if (!scene || !shape) {
        return -1;
    }
    
    if (scene->shape_count >= MAX_SHAPES_IN_SCENE) {
        fprintf(stderr, "Error: Scene is full\n");
        return -1;
    }
    
    scene->shapes[scene->shape_count++] = shape;
    return 0;
}

int scene_remove_shape(scene_t *scene, int index) {
    if (!scene || index < 0 || index >= scene->shape_count) {
        return -1;
    }
    
    // Shift remaining shapes
    for (int i = index; i < scene->shape_count - 1; i++) {
        scene->shapes[i] = scene->shapes[i + 1];
    }
    
    scene->shape_count--;
    return 0;
}

void scene_print(const scene_t *scene) {
    if (!scene) {
        printf("(null scene)\n");
        return;
    }
    
    printf("\n=== Scene: %s ===\n", scene->name);
    printf("Contains %d shape(s)\n\n", scene->shape_count);
    
    for (int i = 0; i < scene->shape_count; i++) {
        printf("Shape #%d:\n", i + 1);
        shape_print(scene->shapes[i]);
        printf("\n");
    }
}

int scene_equals(const scene_t *s1, const scene_t *s2) {
    if (!s1 || !s2) {
        return 0;
    }
    
    // Scenes are equal if there's a 1:1 correspondence
    if (s1->shape_count != s2->shape_count) {
        return 0;
    }
    
    // For each shape in s1, find a matching shape in s2
    int matched[MAX_SHAPES_IN_SCENE] = {0};
    
    for (int i = 0; i < s1->shape_count; i++) {
        int found = 0;
        for (int j = 0; j < s2->shape_count; j++) {
            if (!matched[j] && shape_equals(s1->shapes[i], s2->shapes[j])) {
                matched[j] = 1;
                found = 1;
                break;
            }
        }
        if (!found) {
            return 0;
        }
    }
    
    return 1;
}

int scene_save(const scene_t *scene, const char *filename) {
    if (!scene || !filename) {
        return -1;
    }
    
    FILE *file = fopen(filename, "w");
    if (!file) {
        fprintf(stderr, "Error: Could not open file '%s' for writing\n", filename);
        return -1;
    }
    
    // Write scene name
    fprintf(file, "%s\n", scene->name);
    
    // Write shape count
    fprintf(file, "%d\n", scene->shape_count);
    
    // Write shape types (not the shapes themselves, just their types)
    for (int i = 0; i < scene->shape_count; i++) {
        fprintf(file, "%d\n", scene->shapes[i]->type);
    }
    
    fclose(file);
    printf("Scene saved to '%s'\n", filename);
    return 0;
}

scene_t* scene_load(const char *filename) {
    if (!filename) {
        return NULL;
    }
    
    FILE *file = fopen(filename, "r");
    if (!file) {
        fprintf(stderr, "Error: Could not open file '%s' for reading\n", filename);
        return NULL;
    }
    
    char name[MAX_SCENE_NAME];
    if (fgets(name, MAX_SCENE_NAME, file) == NULL) {
        fclose(file);
        return NULL;
    }
    
    // Remove newline
    name[strcspn(name, "\n")] = 0;
    
    scene_t *scene = scene_create(name);
    if (!scene) {
        fclose(file);
        return NULL;
    }
    
    int shape_count;
    if (fscanf(file, "%d\n", &shape_count) != 1) {
        scene_destroy(scene);
        fclose(file);
        return NULL;
    }
    
    for (int i = 0; i < shape_count; i++) {
        int type;
        if (fscanf(file, "%d\n", &type) != 1) {
            scene_destroy(scene);
            fclose(file);
            return NULL;
        }
        
        shape_t *shape = shape_get((shape_type_t)type);
        if (shape) {
            scene_add_shape(scene, shape);
        }
    }
    
    fclose(file);
    printf("Scene loaded from '%s'\n", filename);
    return scene;
}

void scene_list_shapes(const scene_t *scene) {
    if (!scene) {
        printf("(null scene)\n");
        return;
    }
    
    printf("\nScene: %s\n", scene->name);
    printf("Shapes (%d):\n", scene->shape_count);
    
    for (int i = 0; i < scene->shape_count; i++) {
        printf("  %d. %s (ptr: %p)\n", i + 1, 
               scene->shapes[i]->name, 
               (void*)scene->shapes[i]);
    }
}
