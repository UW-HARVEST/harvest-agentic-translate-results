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
// main.c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "shape.h"
#include "scene.h"

#define MAX_SCENES 10

static scene_t *scenes[MAX_SCENES] = {NULL};
static int scene_count = 0;

void print_menu(void) {
    printf("\n");
    printf("=========================================\n");
    printf("  ASCII ART DRAWING APPLICATION\n");
    printf("=========================================\n");
    printf("1. View all available shapes\n");
    printf("2. Create new scene\n");
    printf("3. Add shape to scene\n");
    printf("4. Remove shape from scene\n");
    printf("5. View scene\n");
    printf("6. List all scenes\n");
    printf("7. Save scene\n");
    printf("8. Load scene\n");
    printf("9. Compare two shapes\n");
    printf("10. Compare two scenes\n");
    printf("11. Delete scene\n");
    printf("12. Exit\n");
    printf("=========================================\n");
    printf("Choice: ");
}

void view_all_shapes(void) {
    printf("\n=== Available Shapes ===\n");
    for (int i = 0; i < SHAPE_COUNT; i++) {
        printf("\n%d. ", i + 1);
        shape_print(shape_get((shape_type_t)i));
    }
}

void create_new_scene(void) {
    if (scene_count >= MAX_SCENES) {
        printf("Error: Maximum scenes reached\n");
        return;
    }
    
    char name[MAX_SCENE_NAME];
    printf("Enter scene name: ");
    if (fgets(name, MAX_SCENE_NAME, stdin) == NULL) {
        return;
    }
    name[strcspn(name, "\n")] = 0;
    
    scenes[scene_count] = scene_create(name);
    if (scenes[scene_count]) {
        printf("Scene '%s' created (index %d)\n", name, scene_count);
        scene_count++;
    } else {
        printf("Error creating scene\n");
    }
}

void add_shape_to_scene(void) {
    if (scene_count == 0) {
        printf("No scenes available. Create a scene first.\n");
        return;
    }
    
    printf("Select scene (0-%d): ", scene_count - 1);
    int scene_idx;
    if (scanf("%d", &scene_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_idx < 0 || scene_idx >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    printf("\nSelect shape to add:\n");
    for (int i = 0; i < SHAPE_COUNT; i++) {
        printf("%d. %s\n", i, shape_type_name((shape_type_t)i));
    }
    printf("Choice: ");
    
    int shape_type;
    if (scanf("%d", &shape_type) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (shape_type < 0 || shape_type >= SHAPE_COUNT) {
        printf("Invalid shape type\n");
        return;
    }
    
    shape_t *shape = shape_get((shape_type_t)shape_type);
    if (scene_add_shape(scenes[scene_idx], shape) == 0) {
        printf("Shape '%s' added to scene (reusing singleton at %p)\n", 
               shape->name, (void*)shape);
    } else {
        printf("Error adding shape\n");
    }
}

void remove_shape_from_scene(void) {
    if (scene_count == 0) {
        printf("No scenes available\n");
        return;
    }
    
    printf("Select scene (0-%d): ", scene_count - 1);
    int scene_idx;
    if (scanf("%d", &scene_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_idx < 0 || scene_idx >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    scene_list_shapes(scenes[scene_idx]);
    
    if (scenes[scene_idx]->shape_count == 0) {
        printf("Scene is empty\n");
        return;
    }
    
    printf("Select shape to remove (1-%d): ", scenes[scene_idx]->shape_count);
    int shape_idx;
    if (scanf("%d", &shape_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_remove_shape(scenes[scene_idx], shape_idx - 1) == 0) {
        printf("Shape removed\n");
    } else {
        printf("Error removing shape\n");
    }
}

void view_scene(void) {
    if (scene_count == 0) {
        printf("No scenes available\n");
        return;
    }
    
    printf("Select scene (0-%d): ", scene_count - 1);
    int scene_idx;
    if (scanf("%d", &scene_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_idx < 0 || scene_idx >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    scene_print(scenes[scene_idx]);
}

void list_all_scenes(void) {
    printf("\n=== All Scenes ===\n");
    if (scene_count == 0) {
        printf("No scenes created yet\n");
        return;
    }
    
    for (int i = 0; i < scene_count; i++) {
        printf("%d. %s (%d shapes)\n", i, scenes[i]->name, scenes[i]->shape_count);
    }
}

void save_scene_to_file(void) {
    if (scene_count == 0) {
        printf("No scenes available\n");
        return;
    }
    
    printf("Select scene (0-%d): ", scene_count - 1);
    int scene_idx;
    if (scanf("%d", &scene_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_idx < 0 || scene_idx >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    char filename[256];
    printf("Enter filename: ");
    if (fgets(filename, sizeof(filename), stdin) == NULL) {
        return;
    }
    filename[strcspn(filename, "\n")] = 0;
    
    scene_save(scenes[scene_idx], filename);
}

void load_scene_from_file(void) {
    if (scene_count >= MAX_SCENES) {
        printf("Error: Maximum scenes reached\n");
        return;
    }
    
    char filename[256];
    printf("Enter filename: ");
    if (fgets(filename, sizeof(filename), stdin) == NULL) {
        return;
    }
    filename[strcspn(filename, "\n")] = 0;
    
    scene_t *scene = scene_load(filename);
    if (scene) {
        scenes[scene_count++] = scene;
        printf("Scene loaded (index %d)\n", scene_count - 1);
    }
}

void compare_shapes(void) {
    printf("\nSelect first shape (0-%d):\n", SHAPE_COUNT - 1);
    for (int i = 0; i < SHAPE_COUNT; i++) {
        printf("%d. %s\n", i, shape_type_name((shape_type_t)i));
    }
    printf("Choice: ");
    
    int type1;
    if (scanf("%d", &type1) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    printf("\nSelect second shape (0-%d): ", SHAPE_COUNT - 1);
    int type2;
    if (scanf("%d", &type2) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (type1 < 0 || type1 >= SHAPE_COUNT || type2 < 0 || type2 >= SHAPE_COUNT) {
        printf("Invalid shape type\n");
        return;
    }
    
    shape_t *s1 = shape_get((shape_type_t)type1);
    shape_t *s2 = shape_get((shape_type_t)type2);
    
    printf("\nShape 1: %s (ptr: %p)\n", s1->name, (void*)s1);
    printf("Shape 2: %s (ptr: %p)\n", s2->name, (void*)s2);
    printf("Comparison of pointers: %d\n", s1 == s2);
    
    if (shape_equals(s1, s2)) {
        printf("Result: Shapes are EQUAL (same instance)\n");
    } else {
        printf("Result: Shapes are NOT EQUAL (different instances)\n");
    }
}

void compare_scenes(void) {
    if (scene_count < 2) {
        printf("Need at least 2 scenes to compare\n");
        return;
    }
    
    printf("Select first scene (0-%d): ", scene_count - 1);
    int idx1;
    if (scanf("%d", &idx1) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    printf("Select second scene (0-%d): ", scene_count - 1);
    int idx2;
    if (scanf("%d", &idx2) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (idx1 < 0 || idx1 >= scene_count || idx2 < 0 || idx2 >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    scene_t *sc1 = scenes[idx1];
    scene_t *sc2 = scenes[idx2];
    
    printf("\nScene 1: %s (%d shapes)\n", sc1->name, sc1->shape_count);
    scene_list_shapes(sc1);
    
    printf("\nScene 2: %s (%d shapes)\n", sc2->name, sc2->shape_count);
    scene_list_shapes(sc2);
    
    if (scene_equals(sc1, sc2)) {
        printf("\nResult: Scenes are EQUAL (1:1 correspondence)\n");
    } else {
        printf("\nResult: Scenes are NOT EQUAL\n");
    }
}

void delete_scene(void) {
    if (scene_count == 0) {
        printf("No scenes available\n");
        return;
    }
    
    printf("Select scene to delete (0-%d): ", scene_count - 1);
    int scene_idx;
    if (scanf("%d", &scene_idx) != 1) {
        printf("Invalid input\n");
        while (getchar() != '\n');
        return;
    }
    while (getchar() != '\n');
    
    if (scene_idx < 0 || scene_idx >= scene_count) {
        printf("Invalid scene index\n");
        return;
    }
    
    scene_destroy(scenes[scene_idx]);
    
    // Shift remaining scenes
    for (int i = scene_idx; i < scene_count - 1; i++) {
        scenes[i] = scenes[i + 1];
    }
    
    scene_count--;
    printf("Scene deleted\n");
}

int main(void) {
    printf("╔════════════════════════════════════════╗\n");
    printf("║  ASCII ART DRAWING APPLICATION        ║\n");
    printf("║  Child-Friendly Shape Editor           ║\n");
    printf("╚════════════════════════════════════════╝\n");
    
    // Initialize shape manager (allocate all shapes once)
    shape_manager_init();
    
    char input[256];
    int choice;
    
    while (1) {
        print_menu();
        
        if (fgets(input, sizeof(input), stdin) == NULL) {
            break;
        }
        
        if (sscanf(input, "%d", &choice) != 1) {
            printf("Invalid input\n");
            continue;
        }
        
        switch (choice) {
            case 1:
                view_all_shapes();
                break;
            case 2:
                create_new_scene();
                break;
            case 3:
                add_shape_to_scene();
                break;
            case 4:
                remove_shape_from_scene();
                break;
            case 5:
                view_scene();
                break;
            case 6:
                list_all_scenes();
                break;
            case 7:
                save_scene_to_file();
                break;
            case 8:
                load_scene_from_file();
                break;
            case 9:
                compare_shapes();
                break;
            case 10:
                compare_scenes();
                break;
            case 11:
                delete_scene();
                break;
            case 12:
                printf("\nCleaning up and exiting...\n");
                for (int i = 0; i < scene_count; i++) {
                    scene_destroy(scenes[i]);
                }
                shape_manager_cleanup();
                printf("Goodbye!\n");
                return 0;
            default:
                printf("Invalid choice\n");
                break;
        }
    }
    
    // Cleanup
    for (int i = 0; i < scene_count; i++) {
        scene_destroy(scenes[i]);
    }
    shape_manager_cleanup();
    
    return 0;
}
