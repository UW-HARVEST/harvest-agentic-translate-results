/*
 * probe.c -- a second driver over the SAME library sources in c_src/src.
 *
 * c_src/src/main.c never reaches most of the branches in tree.c / hashmap.c:
 * three of the five reachable error messages, the "(empty tree)" path,
 * hashmap_clear, NULL data, strncpy truncation, tombstone reuse, and the
 * length clamps in tree_find_path are all unexercised. This program takes a
 * scenario name on argv[1] and drives those paths so the Rust translation can
 * be diffed against them.
 *
 * This file lives OUTSIDE c_src and does not modify it; it is compiled
 * together with c_src/src/hashmap.c and c_src/src/tree.c.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tree.h"

/* ---------- shared dump helpers (mirrored exactly in the Rust probe) ---------- */

static void dump_map(hashmap_t *map, const char *label) {
    printf("%s: size=%zu capacity=%zu deleted=%zu\n",
           label, map->size, map->capacity, map->deleted_count);
    for (size_t i = 0; i < map->capacity; i++) {
        if (map->entries[i].occupied) {
            printf("  slot %zu key=%lu occupied=%d deleted=%d value=%s\n",
                   i, map->entries[i].key, map->entries[i].occupied,
                   map->entries[i].deleted,
                   map->entries[i].value ? "set" : "null");
        }
    }
}

static void dump_node(tree_t *tree, tree_id_t id) {
    tree_node_t *n = tree_get_node(tree, id);
    if (!n) {
        printf("node %lu: (null)\n", id);
        return;
    }
    printf("node %lu: parent=%lu child_count=%d data='%s' children=[",
           n->id, n->parent_id, n->child_count, n->data);
    for (int i = 0; i < n->child_count; i++) {
        if (i) printf(",");
        printf("%lu", n->child_ids[i]);
    }
    printf("]\n");
}

static void dump_tree(tree_t *tree, const char *label) {
    printf("%s: size=%zu has_root=%d root_id=%lu\n",
           label, tree_size(tree), tree->has_root, tree->root_id);
    tree_print(tree);
}

static void dump_path(tree_t *tree, tree_id_t id, int max_length,
                      tree_id_t *path, int path_cap) {
    for (int i = 0; i < path_cap; i++) path[i] = 0;
    int len = tree_find_path(tree, id, path, max_length);
    printf("find_path(id=%lu, max=%d) = %d path=[", id, max_length, len);
    for (int i = 0; i < len; i++) {
        if (i) printf(",");
        printf("%lu", path[i]);
    }
    printf("]\n");
}

static void dump_queries(tree_t *tree, tree_id_t id) {
    printf("id=%lu contains=%d depth=%d height=%d descendants=%d\n",
           id, tree_contains(tree, id), tree_get_depth(tree, id),
           tree_get_height(tree, id), tree_count_descendants(tree, id));
}

/* data containing bytes that are not valid UTF-8, plus printf metacharacters */
static const char weird_data[] = {
    (char)0xff, (char)0xfe, (char)0x80, ' ', 'c', 'a', 'f',
    (char)0xc3, (char)0xa9, ' ', '%', 's', ' ', '%', 'd', ' ', '%', '%', '\0'
};

/* ---------- scenarios ---------- */

static void sc_empty_print(void) {
    tree_t *tree = tree_create();
    /* tree_print with has_root == 0 -> "(empty tree)" */
    dump_tree(tree, "fresh");
    dump_queries(tree, 0);
    dump_queries(tree, 1);
    tree_delete(tree);
}

static void sc_null_data(void) {
    tree_t *tree = tree_create();
    /* data == NULL -> node->data[0] = '\0' */
    printf("add(1,0,NULL) = %d\n", tree_add_node(tree, 1, 0, NULL));
    printf("add(2,1,NULL) = %d\n", tree_add_node(tree, 2, 1, NULL));
    dump_tree(tree, "null-data");
    dump_node(tree, 1);
    dump_node(tree, 2);
    tree_delete(tree);
}

static void sc_parent_missing(void) {
    tree_t *tree = tree_create();
    printf("add(1,0,root) = %d\n", tree_add_node(tree, 1, 0, "root"));
    /* parent lookup fails -> "Error: Parent node %lu not found" */
    printf("add(2,99,orphan) = %d\n", tree_add_node(tree, 2, 99, "orphan"));
    printf("add(3,0,parent-zero) = %d\n", tree_add_node(tree, 3, 0, "parent-zero"));
    dump_tree(tree, "after-failures");
    tree_delete(tree);
}

static void sc_remove_missing(void) {
    tree_t *tree = tree_create();
    /* tree_remove_node on an empty tree and on a missing id */
    printf("remove(1) on empty = %d\n", tree_remove_node(tree, 1));
    printf("add(1,0,root) = %d\n", tree_add_node(tree, 1, 0, "root"));
    printf("remove(42) = %d\n", tree_remove_node(tree, 42));
    dump_tree(tree, "after-failed-removes");
    tree_delete(tree);
}

static void sc_queries_missing(void) {
    tree_t *tree = tree_create();
    tree_add_node(tree, 1, 0, "root");
    tree_add_node(tree, 2, 1, "child");
    /* every query against an absent id returns -1 */
    dump_queries(tree, 999);
    tree_id_t path[64];
    dump_path(tree, 999, 64, path, 64);
    dump_queries(tree, 1);
    dump_queries(tree, 2);
    tree_delete(tree);
}

static void sc_find_path_clamp(void) {
    tree_t *tree = tree_create();
    for (tree_id_t i = 1; i <= 5; i++) {
        tree_add_node(tree, i, i - 1, "chain");
    }
    tree_id_t path[64];
    /* length > max_length is truncated AFTER the path is built, so the
     * reversal reads from the middle of temp_path */
    dump_path(tree, 5, 64, path, 64);
    dump_path(tree, 5, 5, path, 64);
    dump_path(tree, 5, 3, path, 64);
    dump_path(tree, 5, 1, path, 64);
    dump_path(tree, 5, 0, path, 64);
    dump_path(tree, 5, -1, path, 64);
    dump_path(tree, 1, 0, path, 64);
    tree_delete(tree);
}

static void sc_find_path_deep(void) {
    tree_t *tree = tree_create();
    /* deeper than the 1000-entry temp_path, so the loop bound stops it
     * before the root is reached */
    for (tree_id_t i = 1; i <= 1200; i++) {
        tree_add_node(tree, i, i - 1, "deep");
    }
    printf("size=%zu depth(1200)=%d\n", tree_size(tree), tree_get_depth(tree, 1200));
    tree_id_t *path = malloc(sizeof(tree_id_t) * 2000);
    int len = tree_find_path(tree, 1200, path, 2000);
    printf("len=%d first=%lu second=%lu last=%lu\n",
           len, path[0], path[1], path[len - 1]);
    free(path);
    tree_delete(tree);
}

static void sc_data_trunc(void) {
    tree_t *tree = tree_create();
    char buf[600];

    /* exactly MAX_DATA_LENGTH-1 = 255 bytes: fills the buffer, no NUL from strncpy */
    memset(buf, 'a', 255);
    buf[255] = '\0';
    tree_add_node(tree, 1, 0, buf);

    /* 254 bytes: strncpy zero-pads */
    memset(buf, 'b', 254);
    buf[254] = '\0';
    tree_add_node(tree, 2, 1, buf);

    /* 300 bytes: truncated to 255 */
    memset(buf, 'c', 300);
    buf[300] = '\0';
    tree_add_node(tree, 3, 1, buf);

    /* empty string */
    tree_add_node(tree, 4, 1, "");

    /* non-UTF-8 bytes and printf metacharacters */
    tree_add_node(tree, 5, 1, weird_data);

    for (tree_id_t i = 1; i <= 5; i++) {
        tree_node_t *n = tree_get_node(tree, i);
        printf("node %lu strlen=%zu\n", i, strlen(n->data));
    }
    tree_print(tree);
    tree_delete(tree);
}

static void sc_hashmap_reuse(void) {
    hashmap_t *map = hashmap_create();
    int vals[8];
    for (int i = 0; i < 8; i++) vals[i] = i;

    for (tree_id_t k = 0; k < 8; k++) hashmap_put(map, k, &vals[k]);
    dump_map(map, "after-8-puts");

    /* independent keys, but sequence them explicitly so the probe does not
     * depend on printf argument evaluation order */
    const char *r3 = hashmap_remove(map, 3) ? "set" : "null";
    const char *r5 = hashmap_remove(map, 5) ? "set" : "null";
    const char *r99 = hashmap_remove(map, 99) ? "set" : "null";
    printf("remove(3)=%s remove(5)=%s remove(99)=%s\n", r3, r5, r99);
    dump_map(map, "after-removes");

    /* re-inserting reuses a deleted slot and decrements deleted_count */
    hashmap_put(map, 3, &vals[3]);
    dump_map(map, "after-reinsert");

    /* updating an existing key must not change size */
    hashmap_put(map, 3, &vals[7]);
    printf("contains(3)=%d contains(5)=%d size=%zu\n",
           hashmap_contains(map, 3), hashmap_contains(map, 5), hashmap_size(map));
    dump_map(map, "after-update");
    hashmap_destroy(map);
}

static void sc_hashmap_null_value(void) {
    hashmap_t *map = hashmap_create();
    /* a stored NULL value makes hashmap_contains report absent even though the
     * slot is occupied, and hashmap_remove returns NULL while still shrinking */
    printf("put(7,NULL)=%d\n", hashmap_put(map, 7, NULL));
    printf("size=%zu contains(7)=%d get(7)=%s\n",
           hashmap_size(map), hashmap_contains(map, 7),
           hashmap_get(map, 7) ? "set" : "null");
    dump_map(map, "null-value");
    printf("remove(7)=%s\n", hashmap_remove(map, 7) ? "set" : "null");
    dump_map(map, "after-remove");
    hashmap_destroy(map);
}

static void sc_hashmap_clear(void) {
    hashmap_t *map = hashmap_create();
    int vals[20];
    for (int i = 0; i < 20; i++) vals[i] = i;
    for (tree_id_t k = 0; k < 20; k++) hashmap_put(map, k, &vals[k]);
    hashmap_remove(map, 4);
    dump_map(map, "before-clear");
    /* hashmap_clear is never called by the C driver */
    hashmap_clear(map);
    dump_map(map, "after-clear");
    printf("contains(0)=%d size=%zu\n", hashmap_contains(map, 0), hashmap_size(map));
    /* reuse after clear */
    hashmap_put(map, 0, &vals[0]);
    dump_map(map, "after-clear-put");
    hashmap_destroy(map);
}

static void sc_hashmap_resize(void) {
    hashmap_t *map = hashmap_create();
    static int vals[300];
    for (int i = 0; i < 300; i++) vals[i] = i;
    /* cross several resize thresholds, with tombstones counting toward load */
    for (tree_id_t k = 0; k < 300; k++) {
        hashmap_put(map, k * 7 + 1, &vals[k]);
        if (k % 3 == 0) hashmap_remove(map, k * 7 + 1);
    }
    dump_map(map, "resized");
    int found = 0;
    for (tree_id_t k = 0; k < 300; k++) {
        if (hashmap_contains(map, k * 7 + 1)) found++;
    }
    printf("found=%d size=%zu capacity=%zu deleted=%zu\n",
           found, map->size, map->capacity, map->deleted_count);
    hashmap_destroy(map);
}

static void sc_big_ids(void) {
    tree_t *tree = tree_create();
    /* %lu formatting at the extremes of uint64_t */
    printf("add(max,0)=%d\n",
           tree_add_node(tree, 18446744073709551615ULL, 0, "max"));
    printf("add(0,max)=%d\n",
           tree_add_node(tree, 0, 18446744073709551615ULL, "zero"));
    printf("add(9223372036854775808,max)=%d\n",
           tree_add_node(tree, 9223372036854775808ULL,
                         18446744073709551615ULL, "high-bit"));
    dump_tree(tree, "big-ids");
    dump_node(tree, 18446744073709551615ULL);
    dump_queries(tree, 18446744073709551615ULL);
    dump_queries(tree, 0);
    tree_id_t path[64];
    dump_path(tree, 9223372036854775808ULL, 64, path, 64);
    /* duplicate at an extreme id -> "already exists" with %lu */
    printf("dup(max)=%d\n", tree_add_node(tree, 18446744073709551615ULL, 0, "dup"));
    /* missing parent at an extreme id */
    printf("orphan=%d\n", tree_add_node(tree, 5, 12345678901234567890ULL, "orphan"));
    printf("remove(missing-max)=%d\n", tree_remove_node(tree, 18446744073709551614ULL));
    tree_delete(tree);
}

static void sc_zero_root(void) {
    tree_t *tree = tree_create();
    /* root_id == 0 is indistinguishable from "no parent" */
    printf("add(0,0,root)=%d\n", tree_add_node(tree, 0, 0, "root"));
    printf("add(1,0,child)=%d\n", tree_add_node(tree, 1, 0, "child"));
    printf("add(2,1,grand)=%d\n", tree_add_node(tree, 2, 1, "grand"));
    dump_tree(tree, "zero-root");
    dump_queries(tree, 0);
    dump_queries(tree, 1);
    dump_queries(tree, 2);
    tree_id_t path[64];
    dump_path(tree, 2, 64, path, 64);
    tree_delete(tree);
}

static void sc_remove_root_readd(void) {
    tree_t *tree = tree_create();
    tree_add_node(tree, 1, 0, "root");
    tree_add_node(tree, 2, 1, "child");
    tree_add_node(tree, 3, 2, "grand");
    dump_tree(tree, "before");
    printf("remove(1)=%d\n", tree_remove_node(tree, 1));
    dump_tree(tree, "after-remove-root");
    dump_map(tree->node_map, "map-after-remove-root");
    /* has_root == 0, so the next add becomes the root and its parent_id is
     * forced to 0 even though 99 was requested */
    printf("add(7,99,newroot)=%d\n", tree_add_node(tree, 7, 99, "newroot"));
    printf("add(8,7,newchild)=%d\n", tree_add_node(tree, 8, 7, "newchild"));
    /* re-adding a previously removed id must succeed */
    printf("add(1,7,readded)=%d\n", tree_add_node(tree, 1, 7, "readded"));
    dump_tree(tree, "after-readd");
    dump_node(tree, 7);
    dump_map(tree->node_map, "map-after-readd");
    tree_delete(tree);
}

static void sc_child_shift(void) {
    tree_t *tree = tree_create();
    tree_add_node(tree, 1, 0, "root");
    for (tree_id_t i = 2; i <= 6; i++) tree_add_node(tree, i, 1, "child");
    dump_node(tree, 1);
    /* remove first, middle and last child to exercise the shifting loop */
    printf("remove(2)=%d\n", tree_remove_node(tree, 2));
    dump_node(tree, 1);
    printf("remove(4)=%d\n", tree_remove_node(tree, 4));
    dump_node(tree, 1);
    printf("remove(6)=%d\n", tree_remove_node(tree, 6));
    dump_node(tree, 1);
    dump_tree(tree, "after-shifts");
    /* refill after the shifts */
    printf("add(9,1)=%d\n", tree_add_node(tree, 9, 1, "new"));
    dump_node(tree, 1);
    dump_tree(tree, "refilled");
    tree_delete(tree);
}

static void sc_max_children(void) {
    tree_t *tree = tree_create();
    tree_add_node(tree, 1, 0, "root");
    for (tree_id_t i = 0; i < MAX_CHILDREN; i++) {
        tree_add_node(tree, i + 2, 1, "child");
    }
    /* one past the limit -> "Error: Parent has maximum children" */
    printf("overflow=%d\n", tree_add_node(tree, 1000, 1, "overflow"));
    printf("size=%zu height=%d descendants=%d\n",
           tree_size(tree), tree_get_height(tree, 1),
           tree_count_descendants(tree, 1));
    dump_node(tree, 1);
    /* freeing a slot lets exactly one more in */
    printf("remove(2)=%d\n", tree_remove_node(tree, 2));
    printf("refill=%d\n", tree_add_node(tree, 1000, 1, "refill"));
    printf("overflow2=%d\n", tree_add_node(tree, 1001, 1, "overflow2"));
    dump_node(tree, 1);
    tree_delete(tree);
}

static void sc_subtree_removal(void) {
    tree_t *tree = tree_create();
    /* a wide, deep tree, removed from the middle */
    tree_add_node(tree, 1, 0, "root");
    tree_id_t next = 2;
    for (int a = 0; a < 3; a++) {
        tree_id_t branch = next++;
        tree_add_node(tree, branch, 1, "branch");
        for (int b = 0; b < 3; b++) {
            tree_id_t leaf = next++;
            tree_add_node(tree, leaf, branch, "leaf");
            tree_add_node(tree, next++, leaf, "twig");
        }
    }
    dump_tree(tree, "built");
    printf("descendants(1)=%d height(1)=%d\n",
           tree_count_descendants(tree, 1), tree_get_height(tree, 1));
    printf("remove(2)=%d\n", tree_remove_node(tree, 2));
    dump_tree(tree, "after-remove-branch");
    dump_map(tree->node_map, "map");
    for (tree_id_t i = 1; i <= next; i++) {
        printf("contains(%lu)=%d\n", i, tree_contains(tree, i));
    }
    tree_delete(tree);
}

static void sc_dup_and_reinsert(void) {
    tree_t *tree = tree_create();
    /* duplicate root id, then duplicates deeper down */
    printf("add(1,0,a)=%d\n", tree_add_node(tree, 1, 0, "a"));
    printf("add(1,0,b)=%d\n", tree_add_node(tree, 1, 0, "b"));
    printf("add(1,1,c)=%d\n", tree_add_node(tree, 1, 1, "c"));
    printf("add(2,1,d)=%d\n", tree_add_node(tree, 2, 1, "d"));
    printf("add(2,2,e)=%d\n", tree_add_node(tree, 2, 2, "e"));
    dump_tree(tree, "dups");
    dump_node(tree, 1);
    dump_node(tree, 2);
    tree_delete(tree);
}

static void sc_interleaved_output(void) {
    /* stdout is block-buffered when redirected while stderr is unbuffered, so
     * the two streams land out of order in a combined redirect */
    tree_t *tree = tree_create();
    printf("stdout line 1\n");
    tree_add_node(tree, 1, 0, "root");
    tree_add_node(tree, 1, 0, "dup");        /* -> stderr */
    printf("stdout line 2\n");
    tree_remove_node(tree, 77);              /* -> stderr */
    printf("stdout line 3\n");
    fflush(stdout);                          /* explicit flush point */
    tree_add_node(tree, 2, 88, "orphan");    /* -> stderr, after the flush */
    printf("stdout line 4\n");
    tree_delete(tree);
}

static void sc_deep_recursion(void) {
    tree_t *tree = tree_create();
    /* tree_get_height, tree_count_descendants and tree_remove_subtree are all
     * recursive; the C driver only ever goes 5 deep */
    for (tree_id_t i = 1; i <= 5000; i++) {
        tree_add_node(tree, i, i - 1, "chain");
    }
    printf("size=%zu height=%d descendants=%d depth(5000)=%d\n",
           tree_size(tree), tree_get_height(tree, 1),
           tree_count_descendants(tree, 1), tree_get_depth(tree, 5000));
    printf("height(2500)=%d descendants(2500)=%d\n",
           tree_get_height(tree, 2500), tree_count_descendants(tree, 2500));
    /* recursive removal of the whole chain from the middle */
    printf("remove(2500)=%d\n", tree_remove_node(tree, 2500));
    printf("size=%zu contains(2499)=%d contains(2500)=%d contains(5000)=%d\n",
           tree_size(tree), tree_contains(tree, 2499),
           tree_contains(tree, 2500), tree_contains(tree, 5000));
    printf("height=%d descendants=%d\n",
           tree_get_height(tree, 1), tree_count_descendants(tree, 1));
    /* remove the root, unwinding what is left.
     * NOTE: the mutating call is sequenced into a local first. C leaves the
     * evaluation order of printf arguments unspecified (gcc/x86-64 evaluates
     * right-to-left), so mixing tree_remove_node() with tree_size() in one
     * printf would read the size BEFORE the removal, which no translation
     * could be expected to reproduce. */
    int rc = tree_remove_node(tree, 1);
    size_t sz = tree_size(tree);
    int hr = tree->has_root;
    printf("remove(1)=%d size=%zu has_root=%d\n", rc, sz, hr);
    tree_delete(tree);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: probe <scenario>\n");
        return 2;
    }
    const char *s = argv[1];

    if (!strcmp(s, "empty_print"))        sc_empty_print();
    else if (!strcmp(s, "null_data"))          sc_null_data();
    else if (!strcmp(s, "parent_missing"))     sc_parent_missing();
    else if (!strcmp(s, "remove_missing"))     sc_remove_missing();
    else if (!strcmp(s, "queries_missing"))    sc_queries_missing();
    else if (!strcmp(s, "find_path_clamp"))    sc_find_path_clamp();
    else if (!strcmp(s, "find_path_deep"))     sc_find_path_deep();
    else if (!strcmp(s, "data_trunc"))         sc_data_trunc();
    else if (!strcmp(s, "hashmap_reuse"))      sc_hashmap_reuse();
    else if (!strcmp(s, "hashmap_null_value")) sc_hashmap_null_value();
    else if (!strcmp(s, "hashmap_clear"))      sc_hashmap_clear();
    else if (!strcmp(s, "hashmap_resize"))     sc_hashmap_resize();
    else if (!strcmp(s, "big_ids"))            sc_big_ids();
    else if (!strcmp(s, "zero_root"))          sc_zero_root();
    else if (!strcmp(s, "remove_root_readd"))  sc_remove_root_readd();
    else if (!strcmp(s, "child_shift"))        sc_child_shift();
    else if (!strcmp(s, "max_children"))       sc_max_children();
    else if (!strcmp(s, "subtree_removal"))    sc_subtree_removal();
    else if (!strcmp(s, "dup_and_reinsert"))   sc_dup_and_reinsert();
    else if (!strcmp(s, "interleaved"))        sc_interleaved_output();
    else if (!strcmp(s, "deep_recursion"))     sc_deep_recursion();
    else {
        fprintf(stderr, "unknown scenario: %s\n", s);
        return 3;
    }

    return 0;
}
