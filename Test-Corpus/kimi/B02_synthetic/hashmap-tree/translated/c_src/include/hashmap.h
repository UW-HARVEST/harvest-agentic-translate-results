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
// hashmap.h
#ifndef HASHMAP_H
#define HASHMAP_H

#include <stddef.h>
#include <stdint.h>

#define HASHMAP_INITIAL_CAPACITY 16
#define HASHMAP_LOAD_FACTOR 0.75

typedef uint64_t tree_id_t;

typedef struct hashmap_entry {
    tree_id_t key;
    void *value;
    int occupied;
    int deleted;
} hashmap_entry_t;

typedef struct {
    hashmap_entry_t *entries;
    size_t capacity;
    size_t size;
    size_t deleted_count;
} hashmap_t;

// Create a new hashmap
hashmap_t* hashmap_create(void);

// Destroy hashmap (does not free values)
void hashmap_destroy(hashmap_t *map);

// Insert or update a key-value pair
int hashmap_put(hashmap_t *map, tree_id_t key, void *value);

// Get value by key (returns NULL if not found)
void* hashmap_get(hashmap_t *map, tree_id_t key);

// Remove a key-value pair (returns the value or NULL)
void* hashmap_remove(hashmap_t *map, tree_id_t key);

// Check if key exists
int hashmap_contains(hashmap_t *map, tree_id_t key);

// Get current size
size_t hashmap_size(hashmap_t *map);

// Clear all entries
void hashmap_clear(hashmap_t *map);

#endif // HASHMAP_H
