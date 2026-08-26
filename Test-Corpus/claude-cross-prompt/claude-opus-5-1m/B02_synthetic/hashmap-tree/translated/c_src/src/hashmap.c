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
// hashmap.c
#include "hashmap.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

static uint64_t hash_function(tree_id_t key) {
    // FNV-1a hash
    uint64_t hash = 14695981039346656037ULL;
    uint8_t *bytes = (uint8_t *)&key;
    
    for (size_t i = 0; i < sizeof(tree_id_t); i++) {
        hash ^= bytes[i];
        hash *= 1099511628211ULL;
    }
    
    return hash;
}

static int should_resize(hashmap_t *map) {
    double load = (double)(map->size + map->deleted_count) / map->capacity;
    return load > HASHMAP_LOAD_FACTOR;
}

static int hashmap_resize(hashmap_t *map) {
    size_t old_capacity = map->capacity;
    hashmap_entry_t *old_entries = map->entries;
    
    // Double capacity
    map->capacity *= 2;
    map->entries = calloc(map->capacity, sizeof(hashmap_entry_t));
    if (!map->entries) {
        map->entries = old_entries;
        map->capacity = old_capacity;
        return -1;
    }
    
    map->size = 0;
    map->deleted_count = 0;
    
    // Rehash all entries
    for (size_t i = 0; i < old_capacity; i++) {
        if (old_entries[i].occupied && !old_entries[i].deleted) {
            hashmap_put(map, old_entries[i].key, old_entries[i].value);
        }
    }
    
    free(old_entries);
    return 0;
}

hashmap_t* hashmap_create(void) {
    hashmap_t *map = malloc(sizeof(hashmap_t));
    if (!map) {
        return NULL;
    }
    
    map->capacity = HASHMAP_INITIAL_CAPACITY;
    map->size = 0;
    map->deleted_count = 0;
    map->entries = calloc(map->capacity, sizeof(hashmap_entry_t));
    
    if (!map->entries) {
        free(map);
        return NULL;
    }
    
    return map;
}

void hashmap_destroy(hashmap_t *map) {
    if (map) {
        free(map->entries);
        free(map);
    }
}

int hashmap_put(hashmap_t *map, tree_id_t key, void *value) {
    if (!map) {
        return -1;
    }
    
    if (should_resize(map)) {
        if (hashmap_resize(map) != 0) {
            return -1;
        }
    }
    
    uint64_t hash = hash_function(key);
    size_t index = hash % map->capacity;
    size_t probe = 0;
    
    // Linear probing
    while (probe < map->capacity) {
        size_t current = (index + probe) % map->capacity;
        
        if (!map->entries[current].occupied) {
            // Empty slot
            map->entries[current].key = key;
            map->entries[current].value = value;
            map->entries[current].occupied = 1;
            map->entries[current].deleted = 0;
            map->size++;
            return 0;
        } else if (map->entries[current].deleted) {
            // Reuse deleted slot
            map->entries[current].key = key;
            map->entries[current].value = value;
            map->entries[current].deleted = 0;
            map->size++;
            map->deleted_count--;
            return 0;
        } else if (map->entries[current].key == key) {
            // Update existing
            map->entries[current].value = value;
            return 0;
        }
        
        probe++;
    }
    
    return -1; // Map is full (shouldn't happen with resizing)
}

void* hashmap_get(hashmap_t *map, tree_id_t key) {
    if (!map) {
        return NULL;
    }
    
    uint64_t hash = hash_function(key);
    size_t index = hash % map->capacity;
    size_t probe = 0;
    
    while (probe < map->capacity) {
        size_t current = (index + probe) % map->capacity;
        
        if (!map->entries[current].occupied) {
            return NULL;
        }
        
        if (!map->entries[current].deleted && map->entries[current].key == key) {
            return map->entries[current].value;
        }
        
        probe++;
    }
    
    return NULL;
}

void* hashmap_remove(hashmap_t *map, tree_id_t key) {
    if (!map) {
        return NULL;
    }
    
    uint64_t hash = hash_function(key);
    size_t index = hash % map->capacity;
    size_t probe = 0;
    
    while (probe < map->capacity) {
        size_t current = (index + probe) % map->capacity;
        
        if (!map->entries[current].occupied) {
            return NULL;
        }
        
        if (!map->entries[current].deleted && map->entries[current].key == key) {
            void *value = map->entries[current].value;
            map->entries[current].deleted = 1;
            map->size--;
            map->deleted_count++;
            return value;
        }
        
        probe++;
    }
    
    return NULL;
}

int hashmap_contains(hashmap_t *map, tree_id_t key) {
    return hashmap_get(map, key) != NULL;
}

size_t hashmap_size(hashmap_t *map) {
    return map ? map->size : 0;
}

void hashmap_clear(hashmap_t *map) {
    if (!map) {
        return;
    }
    
    for (size_t i = 0; i < map->capacity; i++) {
        map->entries[i].occupied = 0;
        map->entries[i].deleted = 0;
    }
    
    map->size = 0;
    map->deleted_count = 0;
}
