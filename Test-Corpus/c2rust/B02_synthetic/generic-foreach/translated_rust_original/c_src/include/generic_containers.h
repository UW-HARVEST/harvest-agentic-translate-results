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
// generic_containers.h
#ifndef GENERIC_CONTAINERS_H
#define GENERIC_CONTAINERS_H

#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// ============================================================================
// GENERIC DYNAMIC ARRAY
// ============================================================================

#define DECLARE_ARRAY(TYPE) \
    typedef struct { \
        TYPE *data; \
        size_t size; \
        size_t capacity; \
    } array_##TYPE##_t; \
    \
    array_##TYPE##_t* array_##TYPE##_create(size_t initial_capacity); \
    void array_##TYPE##_destroy(array_##TYPE##_t *arr); \
    int array_##TYPE##_push(array_##TYPE##_t *arr, TYPE value); \
    TYPE array_##TYPE##_get(array_##TYPE##_t *arr, size_t index); \
    size_t array_##TYPE##_size(array_##TYPE##_t *arr); \
    void array_##TYPE##_clear(array_##TYPE##_t *arr);

#define DEFINE_ARRAY(TYPE) \
    array_##TYPE##_t* array_##TYPE##_create(size_t initial_capacity) { \
        array_##TYPE##_t *arr = malloc(sizeof(array_##TYPE##_t)); \
        if (!arr) return NULL; \
        arr->capacity = initial_capacity > 0 ? initial_capacity : 16; \
        arr->size = 0; \
        arr->data = malloc(sizeof(TYPE) * arr->capacity); \
        if (!arr->data) { \
            free(arr); \
            return NULL; \
        } \
        return arr; \
    } \
    \
    void array_##TYPE##_destroy(array_##TYPE##_t *arr) { \
        if (arr) { \
            free(arr->data); \
            free(arr); \
        } \
    } \
    \
    int array_##TYPE##_push(array_##TYPE##_t *arr, TYPE value) { \
        if (!arr) return -1; \
        if (arr->size >= arr->capacity) { \
            size_t new_capacity = arr->capacity * 2; \
            TYPE *new_data = realloc(arr->data, sizeof(TYPE) * new_capacity); \
            if (!new_data) return -1; \
            arr->data = new_data; \
            arr->capacity = new_capacity; \
        } \
        arr->data[arr->size++] = value; \
        return 0; \
    } \
    \
    TYPE array_##TYPE##_get(array_##TYPE##_t *arr, size_t index) { \
        return arr->data[index]; \
    } \
    \
    size_t array_##TYPE##_size(array_##TYPE##_t *arr) { \
        return arr ? arr->size : 0; \
    } \
    \
    void array_##TYPE##_clear(array_##TYPE##_t *arr) { \
        if (arr) arr->size = 0; \
    }

// FOR_EACH macro for arrays
#define ARRAY_FOREACH(TYPE, var, arr) \
    for (size_t _i = 0; _i < (arr)->size && (var = (arr)->data[_i], 1); _i++)

// ============================================================================
// GENERIC LINKED LIST
// ============================================================================

#define DECLARE_LIST(TYPE) \
    typedef struct list_node_##TYPE { \
        TYPE data; \
        struct list_node_##TYPE *next; \
    } list_node_##TYPE##_t; \
    \
    typedef struct { \
        list_node_##TYPE##_t *head; \
        list_node_##TYPE##_t *tail; \
        size_t size; \
    } list_##TYPE##_t; \
    \
    list_##TYPE##_t* list_##TYPE##_create(void); \
    void list_##TYPE##_destroy(list_##TYPE##_t *list); \
    int list_##TYPE##_append(list_##TYPE##_t *list, TYPE value); \
    int list_##TYPE##_prepend(list_##TYPE##_t *list, TYPE value); \
    size_t list_##TYPE##_size(list_##TYPE##_t *list); \
    void list_##TYPE##_clear(list_##TYPE##_t *list);

#define DEFINE_LIST(TYPE) \
    list_##TYPE##_t* list_##TYPE##_create(void) { \
        list_##TYPE##_t *list = malloc(sizeof(list_##TYPE##_t)); \
        if (!list) return NULL; \
        list->head = NULL; \
        list->tail = NULL; \
        list->size = 0; \
        return list; \
    } \
    \
    void list_##TYPE##_destroy(list_##TYPE##_t *list) { \
        if (!list) return; \
        list_node_##TYPE##_t *current = list->head; \
        while (current) { \
            list_node_##TYPE##_t *next = current->next; \
            free(current); \
            current = next; \
        } \
        free(list); \
    } \
    \
    int list_##TYPE##_append(list_##TYPE##_t *list, TYPE value) { \
        if (!list) return -1; \
        list_node_##TYPE##_t *node = malloc(sizeof(list_node_##TYPE##_t)); \
        if (!node) return -1; \
        node->data = value; \
        node->next = NULL; \
        if (!list->head) { \
            list->head = list->tail = node; \
        } else { \
            list->tail->next = node; \
            list->tail = node; \
        } \
        list->size++; \
        return 0; \
    } \
    \
    int list_##TYPE##_prepend(list_##TYPE##_t *list, TYPE value) { \
        if (!list) return -1; \
        list_node_##TYPE##_t *node = malloc(sizeof(list_node_##TYPE##_t)); \
        if (!node) return -1; \
        node->data = value; \
        node->next = list->head; \
        list->head = node; \
        if (!list->tail) { \
            list->tail = node; \
        } \
        list->size++; \
        return 0; \
    } \
    \
    size_t list_##TYPE##_size(list_##TYPE##_t *list) { \
        return list ? list->size : 0; \
    } \
    \
    void list_##TYPE##_clear(list_##TYPE##_t *list) { \
        if (!list) return; \
        list_node_##TYPE##_t *current = list->head; \
        while (current) { \
            list_node_##TYPE##_t *next = current->next; \
            free(current); \
            current = next; \
        } \
        list->head = list->tail = NULL; \
        list->size = 0; \
    }

// FOR_EACH macro for linked lists
#define LIST_FOREACH(TYPE, var, list) \
    for (list_node_##TYPE##_t *_node = (list)->head; \
         _node && (var = _node->data, 1); \
         _node = _node->next)

#endif // GENERIC_CONTAINERS_H
