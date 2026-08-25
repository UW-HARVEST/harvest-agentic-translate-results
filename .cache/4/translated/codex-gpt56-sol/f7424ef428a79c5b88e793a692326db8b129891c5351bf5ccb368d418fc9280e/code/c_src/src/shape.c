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
// shape.c
#include "shape.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <stdint.h>

// Singleton shape instances
static shape_t *shapes[SHAPE_COUNT] = {NULL};

static void init_tree(shape_t *shape) {
    shape->type = SHAPE_TREE;
    strcpy(shape->name, "Tree");
    shape->height = 7;
    shape->width = 11;
    
    strcpy(shape->art[0], "    /\\    ");
    strcpy(shape->art[1], "   /  \\   ");
    strcpy(shape->art[2], "  /____\\  ");
    strcpy(shape->art[3], "  /    \\  ");
    strcpy(shape->art[4], " /______\\ ");
    strcpy(shape->art[5], "    ||    ");
    strcpy(shape->art[6], "    ||    ");
}

static void init_tractor(shape_t *shape) {
    shape->type = SHAPE_TRACTOR;
    strcpy(shape->name, "Tractor");
    shape->height = 6;
    shape->width = 20;
    
    strcpy(shape->art[0], "      ________     ");
    strcpy(shape->art[1], "     |        |___ ");
    strcpy(shape->art[2], "     |  []  []|   |");
    strcpy(shape->art[3], "  ___|________|___|");
    strcpy(shape->art[4], " /  o        o   \\");
    strcpy(shape->art[5], "|___|        |___| ");
}

static void init_house(shape_t *shape) {
    shape->type = SHAPE_HOUSE;
    strcpy(shape->name, "House");
    shape->height = 7;
    shape->width = 13;
    
    strcpy(shape->art[0], "     /\\     ");
    strcpy(shape->art[1], "    /  \\    ");
    strcpy(shape->art[2], "   /____\\   ");
    strcpy(shape->art[3], "   |    |   ");
    strcpy(shape->art[4], "   | [] |   ");
    strcpy(shape->art[5], "   |    |   ");
    strcpy(shape->art[6], "   |____|   ");
}

static void init_sun(shape_t *shape) {
    shape->type = SHAPE_SUN;
    strcpy(shape->name, "Sun");
    shape->height = 7;
    shape->width = 11;
    
    strcpy(shape->art[0], "  \\  |  / ");
    strcpy(shape->art[1], "   \\ | /  ");
    strcpy(shape->art[2], "--- (@) ---");
    strcpy(shape->art[3], "   / | \\  ");
    strcpy(shape->art[4], "  /  |  \\ ");
    strcpy(shape->art[5], "          ");
    strcpy(shape->art[6], "          ");
}

static void init_cloud(shape_t *shape) {
    shape->type = SHAPE_CLOUD;
    strcpy(shape->name, "Cloud");
    shape->height = 4;
    shape->width = 16;
    
    strcpy(shape->art[0], "   _____       ");
    strcpy(shape->art[1], "  /     \\_    ");
    strcpy(shape->art[2], " /  ___  _\\  ");
    strcpy(shape->art[3], "(__/   \\_)   ");
}

static void init_flower(shape_t *shape) {
    shape->type = SHAPE_FLOWER;
    strcpy(shape->name, "Flower");
    shape->height = 7;
    shape->width = 9;
    
    strcpy(shape->art[0], "  \\|/  ");
    strcpy(shape->art[1], " -(@)- ");
    strcpy(shape->art[2], "  /|\\  ");
    strcpy(shape->art[3], "   |   ");
    strcpy(shape->art[4], "   |   ");
    strcpy(shape->art[5], "  / \\  ");
    strcpy(shape->art[6], " /   \\ ");
}

static void init_car(shape_t *shape) {
    shape->type = SHAPE_CAR;
    strcpy(shape->name, "Car");
    shape->height = 4;
    shape->width = 16;
    
    strcpy(shape->art[0], "  ____       ");
    strcpy(shape->art[1], " /|_||_\\____ ");
    strcpy(shape->art[2], "( o     o  ) ");
    strcpy(shape->art[3], " -----------  ");
}

static void init_star(shape_t *shape) {
    shape->type = SHAPE_STAR;
    strcpy(shape->name, "Star");
    shape->height = 5;
    shape->width = 9;
    
    strcpy(shape->art[0], "    *    ");
    strcpy(shape->art[1], "   ***   ");
    strcpy(shape->art[2], "  *****  ");
    strcpy(shape->art[3], " ******* ");
    strcpy(shape->art[4], "*********");
}

static void init_heart(shape_t *shape) {
    shape->type = SHAPE_HEART;
    strcpy(shape->name, "Heart");
    shape->height = 6;
    shape->width = 11;
    
    strcpy(shape->art[0], " *** ***  ");
    strcpy(shape->art[1], "*********  ");
    strcpy(shape->art[2], "*********  ");
    strcpy(shape->art[3], " ******* ");
    strcpy(shape->art[4], "  *****  ");
    strcpy(shape->art[5], "   ***   ");
}

static void init_rainbow(shape_t *shape) {
    shape->type = SHAPE_RAINBOW;
    strcpy(shape->name, "Rainbow");
    shape->height = 5;
    shape->width = 21;
    
    strcpy(shape->art[0], "      _______      ");
    strcpy(shape->art[1], "    /         \\    ");
    strcpy(shape->art[2], "   /           \\   ");
    strcpy(shape->art[3], "  /             \\  ");
    strcpy(shape->art[4], " /               \\ ");
}

void shape_manager_init(void) {
    // Allocate each shape once (singleton pattern)
    for (int i = 0; i < SHAPE_COUNT; i++) {
        shapes[i] = malloc(sizeof(shape_t));
        if (!shapes[i]) {
            fprintf(stderr, "Error: Failed to allocate shape\n");
            exit(1);
        }
    }
    
    // Initialize each shape
    init_tree(shapes[SHAPE_TREE]);
    init_tractor(shapes[SHAPE_TRACTOR]);
    init_house(shapes[SHAPE_HOUSE]);
    init_sun(shapes[SHAPE_SUN]);
    init_cloud(shapes[SHAPE_CLOUD]);
    init_flower(shapes[SHAPE_FLOWER]);
    init_car(shapes[SHAPE_CAR]);
    init_star(shapes[SHAPE_STAR]);
    init_heart(shapes[SHAPE_HEART]);
    init_rainbow(shapes[SHAPE_RAINBOW]);
}

void shape_manager_cleanup(void) {
    for (int i = 0; i < SHAPE_COUNT; i++) {
        free(shapes[i]);
        shapes[i] = NULL;
    }
}

shape_t* shape_get(shape_type_t type) {
    if (type < 0 || type >= SHAPE_COUNT) {
        return NULL;
    }
    return shapes[type];
}

void shape_print(const shape_t *shape) {
    if (!shape) {
        printf("(null shape)\n");
        return;
    }
    
    printf("%s:\n", shape->name);
    for (int i = 0; i < shape->height; i++) {
        printf("%s\n", shape->art[i]);
    }
}

int shape_equals(const shape_t *s1, const shape_t *s2) {
    return s1 == s2 ? 1 : 0;
}

const char* shape_type_name(shape_type_t type) {
    switch (type) {
        case SHAPE_TREE: return "Tree";
        case SHAPE_TRACTOR: return "Tractor";
        case SHAPE_HOUSE: return "House";
        case SHAPE_SUN: return "Sun";
        case SHAPE_CLOUD: return "Cloud";
        case SHAPE_FLOWER: return "Flower";
        case SHAPE_CAR: return "Car";
        case SHAPE_STAR: return "Star";
        case SHAPE_HEART: return "Heart";
        case SHAPE_RAINBOW: return "Rainbow";
        default: return "Unknown";
    }
}
