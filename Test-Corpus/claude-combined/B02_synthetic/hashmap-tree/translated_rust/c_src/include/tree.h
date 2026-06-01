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
// tree.h
#ifndef TREE_H
#define TREE_H

#include "hashmap.h"

#define MAX_CHILDREN 32
#define MAX_DATA_LENGTH 256

typedef struct tree_node {
    tree_id_t id;
    tree_id_t parent_id;
    tree_id_t child_ids[MAX_CHILDREN];
    int child_count;
    char data[MAX_DATA_LENGTH];
} tree_node_t;

typedef struct {
    hashmap_t *node_map;
    tree_id_t root_id;
    int has_root;
    size_t node_count;
} tree_t;

// Create a new tree
tree_t* tree_create(void);

// Delete entire tree and free all nodes
void tree_delete(tree_t *tree);

// Add a node to the tree
int tree_add_node(tree_t *tree, tree_id_t id, tree_id_t parent_id, const char *data);

// Remove a node and all its descendants
int tree_remove_node(tree_t *tree, tree_id_t id);

// Get node by ID
tree_node_t* tree_get_node(tree_t *tree, tree_id_t id);

// Check if node exists
int tree_contains(tree_t *tree, tree_id_t id);

// Get tree size
size_t tree_size(tree_t *tree);

// Print tree structure
void tree_print(tree_t *tree);

// Get depth of a node
int tree_get_depth(tree_t *tree, tree_id_t id);

// Get height of tree from a node
int tree_get_height(tree_t *tree, tree_id_t id);

// Count descendants of a node
int tree_count_descendants(tree_t *tree, tree_id_t id);

// Find path from root to node
int tree_find_path(tree_t *tree, tree_id_t id, tree_id_t *path, int max_length);

#endif // TREE_H
