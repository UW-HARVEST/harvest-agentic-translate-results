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
// tree.c
#include "tree.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

tree_t* tree_create(void) {
    tree_t *tree = malloc(sizeof(tree_t));
    if (!tree) {
        return NULL;
    }
    
    tree->node_map = hashmap_create();
    if (!tree->node_map) {
        free(tree);
        return NULL;
    }
    
    tree->root_id = 0;
    tree->has_root = 0;
    tree->node_count = 0;
    
    return tree;
}

static void tree_free_node(tree_node_t *node) {
    if (node) {
        free(node);
    }
}

void tree_delete(tree_t *tree) {
    if (!tree) {
        return;
    }
    
    // Free all nodes in the hashmap
    for (size_t i = 0; i < tree->node_map->capacity; i++) {
        if (tree->node_map->entries[i].occupied && 
            !tree->node_map->entries[i].deleted) {
            tree_free_node((tree_node_t *)tree->node_map->entries[i].value);
        }
    }
    
    hashmap_destroy(tree->node_map);
    free(tree);
}

int tree_add_node(tree_t *tree, tree_id_t id, tree_id_t parent_id, const char *data) {
    if (!tree) {
        return -1;
    }
    
    // Check if node already exists
    if (tree_contains(tree, id)) {
        fprintf(stderr, "Error: Node with ID %lu already exists\n", id);
        return -1;
    }
    
    // Allocate new node
    tree_node_t *node = malloc(sizeof(tree_node_t));
    if (!node) {
        fprintf(stderr, "Error: Failed to allocate node\n");
        return -1;
    }
    
    node->id = id;
    node->parent_id = parent_id;
    node->child_count = 0;
    
    if (data) {
        strncpy(node->data, data, MAX_DATA_LENGTH - 1);
        node->data[MAX_DATA_LENGTH - 1] = '\0';
    } else {
        node->data[0] = '\0';
    }
    
    // If this is the first node, make it the root
    if (!tree->has_root) {
        tree->root_id = id;
        tree->has_root = 1;
        node->parent_id = 0; // Root has no parent
    } else {
        // Find parent and add this node as a child
        tree_node_t *parent = tree_get_node(tree, parent_id);
        if (!parent) {
            fprintf(stderr, "Error: Parent node %lu not found\n", parent_id);
            free(node);
            return -1;
        }
        
        if (parent->child_count >= MAX_CHILDREN) {
            fprintf(stderr, "Error: Parent has maximum children\n");
            free(node);
            return -1;
        }
        
        parent->child_ids[parent->child_count++] = id;
    }
    
    // Add to hashmap
    if (hashmap_put(tree->node_map, id, node) != 0) {
        fprintf(stderr, "Error: Failed to add node to hashmap\n");
        free(node);
        return -1;
    }
    
    tree->node_count++;
    return 0;
}

static int tree_remove_subtree(tree_t *tree, tree_id_t id) {
    tree_node_t *node = tree_get_node(tree, id);
    if (!node) {
        return -1;
    }
    
    // Recursively remove all children first
    for (int i = 0; i < node->child_count; i++) {
        tree_remove_subtree(tree, node->child_ids[i]);
    }
    
    // Remove this node from hashmap
    tree_node_t *removed = (tree_node_t *)hashmap_remove(tree->node_map, id);
    if (removed) {
        tree_free_node(removed);
        tree->node_count--;
    }
    
    return 0;
}

int tree_remove_node(tree_t *tree, tree_id_t id) {
    if (!tree) {
        return -1;
    }
    
    tree_node_t *node = tree_get_node(tree, id);
    if (!node) {
        fprintf(stderr, "Error: Node %lu not found\n", id);
        return -1;
    }
    
    // If removing root, tree becomes empty
    if (id == tree->root_id) {
        tree_remove_subtree(tree, id);
        tree->has_root = 0;
        tree->root_id = 0;
        return 0;
    }
    
    // Remove from parent's child list
    tree_node_t *parent = tree_get_node(tree, node->parent_id);
    if (parent) {
        for (int i = 0; i < parent->child_count; i++) {
            if (parent->child_ids[i] == id) {
                // Shift remaining children
                for (int j = i; j < parent->child_count - 1; j++) {
                    parent->child_ids[j] = parent->child_ids[j + 1];
                }
                parent->child_count--;
                break;
            }
        }
    }
    
    // Remove this node and all descendants
    tree_remove_subtree(tree, id);
    
    return 0;
}

tree_node_t* tree_get_node(tree_t *tree, tree_id_t id) {
    if (!tree) {
        return NULL;
    }
    
    return (tree_node_t *)hashmap_get(tree->node_map, id);
}

int tree_contains(tree_t *tree, tree_id_t id) {
    return tree_get_node(tree, id) != NULL;
}

size_t tree_size(tree_t *tree) {
    return tree ? tree->node_count : 0;
}

static void tree_print_helper(tree_t *tree, tree_id_t id, int depth) {
    tree_node_t *node = tree_get_node(tree, id);
    if (!node) {
        return;
    }
    
    // Print indentation
    for (int i = 0; i < depth; i++) {
        printf("  ");
    }
    
    printf("[%lu] %s\n", node->id, node->data);
    
    // Print children
    for (int i = 0; i < node->child_count; i++) {
        tree_print_helper(tree, node->child_ids[i], depth + 1);
    }
}

void tree_print(tree_t *tree) {
    if (!tree || !tree->has_root) {
        printf("(empty tree)\n");
        return;
    }
    
    tree_print_helper(tree, tree->root_id, 0);
}

int tree_get_depth(tree_t *tree, tree_id_t id) {
    if (!tree || !tree_contains(tree, id)) {
        return -1;
    }
    
    int depth = 0;
    tree_id_t current_id = id;
    
    while (current_id != tree->root_id) {
        tree_node_t *node = tree_get_node(tree, current_id);
        if (!node) {
            return -1;
        }
        current_id = node->parent_id;
        depth++;
    }
    
    return depth;
}

int tree_get_height(tree_t *tree, tree_id_t id) {
    tree_node_t *node = tree_get_node(tree, id);
    if (!node) {
        return -1;
    }
    
    if (node->child_count == 0) {
        return 0;
    }
    
    int max_height = 0;
    for (int i = 0; i < node->child_count; i++) {
        int child_height = tree_get_height(tree, node->child_ids[i]);
        if (child_height > max_height) {
            max_height = child_height;
        }
    }
    
    return max_height + 1;
}

int tree_count_descendants(tree_t *tree, tree_id_t id) {
    tree_node_t *node = tree_get_node(tree, id);
    if (!node) {
        return -1;
    }
    
    int count = 0;
    for (int i = 0; i < node->child_count; i++) {
        count++; // Count the child
        count += tree_count_descendants(tree, node->child_ids[i]);
    }
    
    return count;
}

int tree_find_path(tree_t *tree, tree_id_t id, tree_id_t *path, int max_length) {
    if (!tree || !path || !tree_contains(tree, id)) {
        return -1;
    }
    
    // Build path from node to root, then reverse
    tree_id_t temp_path[1000];
    int length = 0;
    tree_id_t current_id = id;
    
    while (length < 1000) {
        temp_path[length++] = current_id;
        
        if (current_id == tree->root_id) {
            break;
        }
        
        tree_node_t *node = tree_get_node(tree, current_id);
        if (!node) {
            return -1;
        }
        current_id = node->parent_id;
    }
    
    // Reverse into output path
    if (length > max_length) {
        length = max_length;
    }
    
    for (int i = 0; i < length; i++) {
        path[i] = temp_path[length - 1 - i];
    }
    
    return length;
}
