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
#include <assert.h>
#include "tree.h"

#define TEST_PASS printf("✓ PASS: %s\n", __func__)
#define TEST_FAIL printf("✗ FAIL: %s at line %d\n", __func__, __LINE__)

void test_hashmap_basic(void) {
    printf("\n=== Testing Hashmap Basic Operations ===\n");
    
    hashmap_t *map = hashmap_create();
    assert(map != NULL);
    assert(hashmap_size(map) == 0);
    
    // Test put and get
    int val1 = 42, val2 = 100, val3 = 200;
    assert(hashmap_put(map, 1, &val1) == 0);
    assert(hashmap_put(map, 2, &val2) == 0);
    assert(hashmap_put(map, 3, &val3) == 0);
    assert(hashmap_size(map) == 3);
    
    assert(*(int *)hashmap_get(map, 1) == 42);
    assert(*(int *)hashmap_get(map, 2) == 100);
    assert(*(int *)hashmap_get(map, 3) == 200);
    
    // Test update
    int val4 = 500;
    assert(hashmap_put(map, 1, &val4) == 0);
    assert(hashmap_size(map) == 3);
    assert(*(int *)hashmap_get(map, 1) == 500);
    
    // Test remove
    void *removed = hashmap_remove(map, 2);
    assert(removed == &val2);
    assert(hashmap_size(map) == 2);
    assert(hashmap_get(map, 2) == NULL);
    
    // Test contains
    assert(hashmap_contains(map, 1) == 1);
    assert(hashmap_contains(map, 2) == 0);
    assert(hashmap_contains(map, 3) == 1);
    
    hashmap_destroy(map);
    TEST_PASS;
}

void test_hashmap_collisions(void) {
    printf("\n=== Testing Hashmap Collisions ===\n");
    
    hashmap_t *map = hashmap_create();
    
    // Add many items to force collisions
    int values[100];
    for (int i = 0; i < 100; i++) {
        values[i] = i * 10;
        assert(hashmap_put(map, i, &values[i]) == 0);
    }
    
    assert(hashmap_size(map) == 100);
    
    // Verify all values
    for (int i = 0; i < 100; i++) {
        int *val = (int *)hashmap_get(map, i);
        assert(val != NULL);
        assert(*val == i * 10);
    }
    
    hashmap_destroy(map);
    TEST_PASS;
}

void test_tree_creation(void) {
    printf("\n=== Testing Tree Creation ===\n");
    
    tree_t *tree = tree_create();
    assert(tree != NULL);
    assert(tree_size(tree) == 0);
    assert(tree->has_root == 0);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_add_root(void) {
    printf("\n=== Testing Tree Add Root ===\n");
    
    tree_t *tree = tree_create();
    
    // Add root node
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_size(tree) == 1);
    assert(tree->has_root == 1);
    assert(tree->root_id == 1);
    
    tree_node_t *root = tree_get_node(tree, 1);
    assert(root != NULL);
    assert(root->id == 1);
    assert(strcmp(root->data, "root") == 0);
    assert(root->child_count == 0);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_add_children(void) {
    printf("\n=== Testing Tree Add Children ===\n");
    
    tree_t *tree = tree_create();
    
    // Build tree structure
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 1, "child2") == 0);
    assert(tree_add_node(tree, 4, 1, "child3") == 0);
    
    assert(tree_size(tree) == 4);
    
    tree_node_t *root = tree_get_node(tree, 1);
    assert(root->child_count == 3);
    assert(root->child_ids[0] == 2);
    assert(root->child_ids[1] == 3);
    assert(root->child_ids[2] == 4);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_deep_hierarchy(void) {
    printf("\n=== Testing Tree Deep Hierarchy ===\n");
    
    tree_t *tree = tree_create();
    
    // Build deep tree
    assert(tree_add_node(tree, 1, 0, "level0") == 0);
    assert(tree_add_node(tree, 2, 1, "level1") == 0);
    assert(tree_add_node(tree, 3, 2, "level2") == 0);
    assert(tree_add_node(tree, 4, 3, "level3") == 0);
    assert(tree_add_node(tree, 5, 4, "level4") == 0);
    
    assert(tree_size(tree) == 5);
    
    assert(tree_get_depth(tree, 1) == 0);
    assert(tree_get_depth(tree, 2) == 1);
    assert(tree_get_depth(tree, 3) == 2);
    assert(tree_get_depth(tree, 4) == 3);
    assert(tree_get_depth(tree, 5) == 4);
    
    assert(tree_get_height(tree, 1) == 4);
    assert(tree_get_height(tree, 2) == 3);
    assert(tree_get_height(tree, 5) == 0);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_remove_leaf(void) {
    printf("\n=== Testing Tree Remove Leaf ===\n");
    
    tree_t *tree = tree_create();
    
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 1, "child2") == 0);
    
    assert(tree_size(tree) == 3);
    
    // Remove leaf
    assert(tree_remove_node(tree, 3) == 0);
    assert(tree_size(tree) == 2);
    assert(tree_contains(tree, 3) == 0);
    
    tree_node_t *root = tree_get_node(tree, 1);
    assert(root->child_count == 1);
    assert(root->child_ids[0] == 2);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_remove_subtree(void) {
    printf("\n=== Testing Tree Remove Subtree ===\n");
    
    tree_t *tree = tree_create();
    
    // Build tree with subtree
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 2, "grandchild1") == 0);
    assert(tree_add_node(tree, 4, 2, "grandchild2") == 0);
    assert(tree_add_node(tree, 5, 1, "child2") == 0);
    
    assert(tree_size(tree) == 5);
    
    // Remove node 2 and its children
    assert(tree_remove_node(tree, 2) == 0);
    assert(tree_size(tree) == 2);
    assert(tree_contains(tree, 2) == 0);
    assert(tree_contains(tree, 3) == 0);
    assert(tree_contains(tree, 4) == 0);
    assert(tree_contains(tree, 1) == 1);
    assert(tree_contains(tree, 5) == 1);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_remove_root(void) {
    printf("\n=== Testing Tree Remove Root ===\n");
    
    tree_t *tree = tree_create();
    
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 1, "child2") == 0);
    
    assert(tree_size(tree) == 3);
    
    // Remove root
    assert(tree_remove_node(tree, 1) == 0);
    assert(tree_size(tree) == 0);
    assert(tree->has_root == 0);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_count_descendants(void) {
    printf("\n=== Testing Tree Count Descendants ===\n");
    
    tree_t *tree = tree_create();
    
    /*
     * Build tree:
     *        1
     *       / \
     *      2   5
     *     / \
     *    3   4
     */
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 2, "grandchild1") == 0);
    assert(tree_add_node(tree, 4, 2, "grandchild2") == 0);
    assert(tree_add_node(tree, 5, 1, "child2") == 0);
    
    assert(tree_count_descendants(tree, 1) == 4);
    assert(tree_count_descendants(tree, 2) == 2);
    assert(tree_count_descendants(tree, 3) == 0);
    assert(tree_count_descendants(tree, 5) == 0);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_find_path(void) {
    printf("\n=== Testing Tree Find Path ===\n");
    
    tree_t *tree = tree_create();
    
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child") == 0);
    assert(tree_add_node(tree, 3, 2, "grandchild") == 0);
    
    tree_id_t path[10];
    int length;
    
    length = tree_find_path(tree, 3, path, 10);
    assert(length == 3);
    assert(path[0] == 1);
    assert(path[1] == 2);
    assert(path[2] == 3);
    
    length = tree_find_path(tree, 1, path, 10);
    assert(length == 1);
    assert(path[0] == 1);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_duplicate_id(void) {
    printf("\n=== Testing Tree Duplicate ID ===\n");
    
    tree_t *tree = tree_create();
    
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child") == 0);
    
    // Try to add duplicate
    assert(tree_add_node(tree, 2, 1, "duplicate") != 0);
    assert(tree_size(tree) == 2);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_max_children(void) {
    printf("\n=== Testing Tree Max Children ===\n");
    
    tree_t *tree = tree_create();
    
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    
    // Add MAX_CHILDREN children
    for (int i = 0; i < MAX_CHILDREN; i++) {
        assert(tree_add_node(tree, i + 2, 1, "child") == 0);
    }
    
    // Try to add one more (should fail)
    assert(tree_add_node(tree, MAX_CHILDREN + 2, 1, "overflow") != 0);
    assert(tree_size(tree) == MAX_CHILDREN + 1);
    
    tree_delete(tree);
    TEST_PASS;
}

void test_tree_complex_structure(void) {
    printf("\n=== Testing Tree Complex Structure ===\n");
    
    tree_t *tree = tree_create();
    
    /*
     * Build complex tree:
     *           1
     *        /  |  \
     *       2   3   4
     *      /|   |   |\
     *     5 6   7   8 9
     *           |
     *          10
     */
    assert(tree_add_node(tree, 1, 0, "root") == 0);
    assert(tree_add_node(tree, 2, 1, "child1") == 0);
    assert(tree_add_node(tree, 3, 1, "child2") == 0);
    assert(tree_add_node(tree, 4, 1, "child3") == 0);
    assert(tree_add_node(tree, 5, 2, "gc1") == 0);
    assert(tree_add_node(tree, 6, 2, "gc2") == 0);
    assert(tree_add_node(tree, 7, 3, "gc3") == 0);
    assert(tree_add_node(tree, 8, 4, "gc4") == 0);
    assert(tree_add_node(tree, 9, 4, "gc5") == 0);
    assert(tree_add_node(tree, 10, 7, "ggc1") == 0);
    
    assert(tree_size(tree) == 10);
    assert(tree_get_height(tree, 1) == 3);
    assert(tree_count_descendants(tree, 1) == 9);
    assert(tree_count_descendants(tree, 2) == 2);
    assert(tree_count_descendants(tree, 7) == 1);
    
    tree_print(tree);
    
    tree_delete(tree);
    TEST_PASS;
}

int main(void) {
    printf("╔════════════════════════════════════════╗\n");
    printf("║  TREE WITH HASHMAP ID MAPPING TESTS   ║\n");
    printf("╚════════════════════════════════════════╝\n");
    
    // Hashmap tests
    test_hashmap_basic();
    test_hashmap_collisions();
    
    // Tree creation tests
    test_tree_creation();
    test_tree_add_root();
    test_tree_add_children();
    
    // Tree structure tests
    test_tree_deep_hierarchy();
    test_tree_complex_structure();
    
    // Tree removal tests
    test_tree_remove_leaf();
    test_tree_remove_subtree();
    test_tree_remove_root();
    
    // Tree query tests
    test_tree_count_descendants();
    test_tree_find_path();
    
    // Error handling tests
    test_tree_duplicate_id();
    test_tree_max_children();
    
    printf("\n");
    printf("========================================\n");
    printf("  All tests passed successfully!\n");
    printf("========================================\n");
    
    return 0;
}
