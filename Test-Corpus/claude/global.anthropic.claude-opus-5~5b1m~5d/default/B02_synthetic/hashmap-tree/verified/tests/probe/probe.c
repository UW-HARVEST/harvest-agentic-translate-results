/*
 * Branch-coverage probe, C side.
 *
 * This file is NOT part of c_src/ and does not modify it. It is a second
 * driver that links the *pristine* c_src/src/tree.c and c_src/src/hashmap.c so
 * that the branches main.c never reaches (bad parent, NULL data, empty-tree
 * print, path truncation, deleted-slot reuse, hashmap_clear, ...) can be
 * exercised and diffed against the Rust translation.
 *
 * The Rust counterpart is probe.rs, which mirrors this file statement for
 * statement so that identical text is expected on stdout and stderr.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tree.h"

/* Values the hashmap entries point at. C stores &VALS[i]; the Rust probe
 * stores the index i and looks it up in the same table. */
#define NVALS 64
static int VALS[NVALS];

static void init_vals(void) {
    for (int i = 0; i < NVALS; i++) {
        VALS[i] = i * 7 + 1;
    }
}

/* ---------------- hashmap helpers ---------------- */

static void hm_state(hashmap_t *m, const char *tag) {
    printf("%s: size=%zu cap=%zu del=%zu\n", tag, m->size, m->capacity,
           m->deleted_count);
}

static void hm_get(hashmap_t *m, tree_id_t k) {
    void *v = hashmap_get(m, k);
    if (v) {
        printf("  get(%lu)=%d contains=%d\n", k, *(int *)v,
               hashmap_contains(m, k));
    } else {
        printf("  get(%lu)=(null) contains=%d\n", k, hashmap_contains(m, k));
    }
}

static void hm_put(hashmap_t *m, tree_id_t k, int vi) {
    int rc = hashmap_put(m, k, vi < 0 ? NULL : (void *)&VALS[vi]);
    printf("  put(%lu, %s)=%d size=%zu cap=%zu del=%zu\n", k,
           vi < 0 ? "NULL" : "val", rc, m->size, m->capacity, m->deleted_count);
}

static void hm_remove(hashmap_t *m, tree_id_t k) {
    void *v = hashmap_remove(m, k);
    if (v) {
        printf("  remove(%lu)=%d size=%zu del=%zu\n", k, *(int *)v, m->size,
               m->deleted_count);
    } else {
        printf("  remove(%lu)=(null) size=%zu del=%zu\n", k, m->size,
               m->deleted_count);
    }
}

/* ---------------- tree helpers ---------------- */

static void t_state(tree_t *t, const char *tag) {
    printf("%s: size=%zu has_root=%d root_id=%lu\n", tag, tree_size(t),
           t->has_root, t->root_id);
}

static void t_node(tree_t *t, tree_id_t id) {
    tree_node_t *n = tree_get_node(t, id);
    if (!n) {
        printf("  node(%lu)=(null)\n", id);
        return;
    }
    printf("  node(%lu): parent=%lu nchild=%d [", n->id, n->parent_id,
           n->child_count);
    for (int i = 0; i < n->child_count; i++) {
        printf("%s%lu", i ? "," : "", n->child_ids[i]);
    }
    printf("] datalen=%zu data=\"%s\"\n", strlen(n->data), n->data);
}

static void t_query(tree_t *t, tree_id_t id) {
    printf("  q(%lu): contains=%d depth=%d height=%d desc=%d\n", id,
           tree_contains(t, id), tree_get_depth(t, id), tree_get_height(t, id),
           tree_count_descendants(t, id));
}

static void t_path(tree_t *t, tree_id_t id, int max_len) {
    tree_id_t path[64];
    for (int i = 0; i < 64; i++) {
        path[i] = 0;
    }
    int n = tree_find_path(t, id, path, max_len);
    printf("  path(%lu, max=%d)=%d [", id, max_len, n);
    for (int i = 0; i < n && i < 64; i++) {
        printf("%s%lu", i ? "," : "", path[i]);
    }
    printf("]\n");
}

static void t_add(tree_t *t, tree_id_t id, tree_id_t parent, const char *data) {
    int rc = tree_add_node(t, id, parent, data);
    printf("  add(id=%lu,parent=%lu,data=%s)=%d size=%zu\n", id, parent,
           data ? "str" : "NULL", rc, tree_size(t));
}

static void t_remove(tree_t *t, tree_id_t id) {
    int rc = tree_remove_node(t, id);
    printf("  remove(%lu)=%d size=%zu has_root=%d root_id=%lu\n", id, rc,
           tree_size(t), t->has_root, t->root_id);
}

/* ================= sections ================= */

static void sec_hashmap_growth(void) {
    printf("\n### hashmap growth ###\n");
    hashmap_t *m = hashmap_create();
    hm_state(m, "fresh");

    /* Insert enough keys to cross the 0.75 load factor twice. */
    for (int i = 0; i < 31; i++) {
        hm_put(m, (tree_id_t)i, i);
    }
    hm_state(m, "after 31 puts");

    for (int i = 0; i < 34; i++) {
        hm_get(m, (tree_id_t)i);
    }

    /* Update an existing key: hits the "update existing" branch. */
    hm_put(m, 3, 20);
    hm_get(m, 3);
    hm_state(m, "after update");

    hashmap_destroy(m);
}

static void sec_hashmap_deletion(void) {
    printf("\n### hashmap deletion / reuse / clear ###\n");
    hashmap_t *m = hashmap_create();

    for (int i = 0; i < 10; i++) {
        hm_put(m, (tree_id_t)i, i);
    }
    hm_state(m, "10 keys");

    /* Remove present keys, then a missing one. */
    hm_remove(m, 0);
    hm_remove(m, 4);
    hm_remove(m, 9);
    hm_remove(m, 9);   /* already deleted */
    hm_remove(m, 777); /* never inserted */
    hm_state(m, "after removes");

    for (int i = 0; i < 11; i++) {
        hm_get(m, (tree_id_t)i);
    }

    /* Re-insert removed keys: hits the "reuse deleted slot" branch. */
    hm_put(m, 4, 11);
    hm_put(m, 0, 12);
    hm_put(m, 9, 13);
    hm_state(m, "after reinsert");
    for (int i = 0; i < 10; i++) {
        hm_get(m, (tree_id_t)i);
    }

    /* Grow while deleted entries exist, so should_resize() counts tombstones
     * and hashmap_resize() drops them. */
    hm_remove(m, 1);
    hm_remove(m, 2);
    hm_remove(m, 3);
    for (int i = 10; i < 25; i++) {
        hm_put(m, (tree_id_t)i, i % NVALS);
    }
    hm_state(m, "after growth with tombstones");
    for (int i = 0; i < 25; i++) {
        hm_get(m, (tree_id_t)i);
    }

    /* NULL value: the key occupies a slot and counts toward size, yet
     * hashmap_get returns NULL so hashmap_contains reports 0. */
    hm_put(m, 100, -1);
    hm_get(m, 100);
    hm_state(m, "after NULL value");
    hm_remove(m, 100); /* returns NULL even though the key was found */
    hm_state(m, "after removing NULL value");

    /* Extreme keys exercise the hash and the %lu formatting. */
    hm_put(m, 0, 1);
    hm_put(m, 18446744073709551615UL, 2);
    hm_put(m, 9223372036854775808UL, 3);
    hm_get(m, 18446744073709551615UL);
    hm_get(m, 9223372036854775808UL);
    hm_state(m, "after extreme keys");

    hashmap_clear(m);
    hm_state(m, "after clear");
    for (int i = 0; i < 5; i++) {
        hm_get(m, (tree_id_t)i);
    }
    hm_get(m, 18446744073709551615UL);
    hm_put(m, 5, 5);
    hm_get(m, 5);
    hm_state(m, "after put post-clear");

    hashmap_destroy(m);
}

static void sec_tree_empty(void) {
    printf("\n### tree: empty-state branches ###\n");
    tree_t *t = tree_create();
    t_state(t, "fresh");
    tree_print(t); /* "(empty tree)" */
    t_node(t, 1);
    t_query(t, 1);
    t_query(t, 0);
    t_path(t, 1, 10);
    t_path(t, 0, 10);
    t_remove(t, 1); /* "Error: Node 1 not found" */
    t_remove(t, 0);
    tree_delete(t);
}

static void sec_tree_add_paths(void) {
    printf("\n### tree: add-node validation order ###\n");
    tree_t *t = tree_create();

    /* First node becomes the root even though parent 12345 does not exist;
     * its parent_id is forced to 0. */
    t_add(t, 10, 12345, NULL); /* NULL data branch */
    t_state(t, "after NULL-data root");
    t_node(t, 10);
    tree_print(t);

    /* Parent not found. */
    t_add(t, 11, 99, "orphan"); /* "Error: Parent node 99 not found" */
    t_state(t, "after bad parent");
    t_node(t, 11);

    /* Duplicate id is checked *before* the parent lookup, so a duplicate with
     * a bogus parent reports the duplicate. */
    t_add(t, 10, 99, "dup-bad-parent");

    /* Self-parent: id is not present yet, so the parent lookup fails. */
    t_add(t, 12, 12, "self");

    /* Long data is truncated by strncpy to MAX_DATA_LENGTH-1 bytes. */
    char long_data[400];
    for (int i = 0; i < 399; i++) {
        long_data[i] = (char)('a' + (i % 26));
    }
    long_data[399] = '\0';
    t_add(t, 13, 10, long_data);
    t_node(t, 13);

    /* Exactly 255 and exactly 256 bytes. */
    char d255[256];
    memset(d255, 'x', 255);
    d255[255] = '\0';
    t_add(t, 14, 10, d255);
    t_node(t, 14);

    char d256[257];
    memset(d256, 'y', 256);
    d256[256] = '\0';
    t_add(t, 15, 10, d256);
    t_node(t, 15);

    /* Empty string data. */
    t_add(t, 16, 10, "");
    t_node(t, 16);

    tree_print(t);
    t_state(t, "final");
    tree_delete(t);
}

static void sec_tree_max_children(void) {
    printf("\n### tree: MAX_CHILDREN boundary ###\n");
    tree_t *t = tree_create();
    t_add(t, 1, 0, "root");
    for (int i = 0; i < MAX_CHILDREN; i++) {
        int rc = tree_add_node(t, (tree_id_t)(i + 2), 1, "c");
        if (rc != 0) {
            printf("  unexpected failure at child %d\n", i);
        }
    }
    t_state(t, "root full");
    t_add(t, 100, 1, "overflow"); /* "Error: Parent has maximum children" */

    /* A duplicate id on a full parent still reports the duplicate first. */
    t_add(t, 2, 1, "dup-on-full");

    t_node(t, 1);
    t_query(t, 1);

    /* Freeing a slot lets one more child in. */
    t_remove(t, 17);
    t_node(t, 1);
    t_add(t, 100, 1, "now-fits");
    t_node(t, 1);

    tree_delete(t);
}

static void sec_tree_child_removal(void) {
    printf("\n### tree: child-list shifting ###\n");
    tree_t *t = tree_create();
    t_add(t, 1, 0, "root");
    for (int i = 2; i <= 7; i++) {
        t_add(t, (tree_id_t)i, 1, "c");
    }
    t_node(t, 1);

    t_remove(t, 4); /* middle */
    t_node(t, 1);
    t_remove(t, 2); /* first */
    t_node(t, 1);
    t_remove(t, 7); /* last */
    t_node(t, 1);
    t_remove(t, 4); /* already gone: "Error: Node 4 not found" */
    t_node(t, 1);

    tree_print(t);
    tree_delete(t);
}

static void sec_tree_subtree_and_root(void) {
    printf("\n### tree: subtree removal, root removal, re-add ###\n");
    tree_t *t = tree_create();
    t_add(t, 1, 0, "root");
    t_add(t, 2, 1, "a");
    t_add(t, 3, 2, "aa");
    t_add(t, 4, 3, "aaa");
    t_add(t, 5, 1, "b");
    t_add(t, 6, 5, "bb");
    tree_print(t);
    t_query(t, 1);
    t_query(t, 2);

    t_remove(t, 2); /* removes 2,3,4 */
    t_state(t, "after subtree removal");
    for (tree_id_t id = 1; id <= 6; id++) {
        printf("  contains(%lu)=%d\n", id, tree_contains(t, id));
    }
    tree_print(t);

    t_remove(t, 1); /* root */
    t_state(t, "after root removal");
    tree_print(t); /* "(empty tree)" */
    t_query(t, 1);

    /* Re-add into a hashmap that is now full of tombstones. */
    t_add(t, 20, 0, "new-root");
    t_add(t, 21, 20, "new-child");
    t_add(t, 3, 21, "recycled-id");
    t_state(t, "after re-add");
    tree_print(t);
    t_query(t, 20);
    t_path(t, 3, 10);

    tree_delete(t);
}

static void sec_tree_zero_and_max_ids(void) {
    printf("\n### tree: id 0 and id UINT64_MAX ###\n");
    tree_t *t = tree_create();
    t_add(t, 0, 0, "zero-root");
    t_state(t, "zero root");
    t_add(t, 18446744073709551615UL, 0, "max-child");
    t_add(t, 1, 18446744073709551615UL, "deep");
    tree_print(t);
    t_query(t, 0);
    t_query(t, 18446744073709551615UL);
    t_query(t, 1);
    t_path(t, 1, 10);
    t_node(t, 18446744073709551615UL);
    t_remove(t, 18446744073709551615UL);
    t_state(t, "after removing max id");
    tree_print(t);
    tree_delete(t);
}

static void sec_tree_deep_chain(void) {
    printf("\n### tree: deep chain, path truncation, 1000-entry cap ###\n");
    tree_t *t = tree_create();
    char buf[32];

    t_add(t, 2000, 0, "chain-root");
    for (int i = 1; i < 1010; i++) {
        snprintf(buf, sizeof(buf), "n%d", i);
        if (tree_add_node(t, (tree_id_t)(2000 + i), (tree_id_t)(2000 + i - 1),
                          buf) != 0) {
            printf("  chain add failed at %d\n", i);
            break;
        }
    }
    t_state(t, "chain built");

    printf("  depth(2000)=%d\n", tree_get_depth(t, 2000));
    printf("  depth(2500)=%d\n", tree_get_depth(t, 2500));
    printf("  depth(3009)=%d\n", tree_get_depth(t, 3009));
    printf("  height(2000)=%d\n", tree_get_height(t, 2000));
    printf("  height(3009)=%d\n", tree_get_height(t, 3009));
    printf("  desc(2000)=%d\n", tree_count_descendants(t, 2000));
    printf("  desc(3000)=%d\n", tree_count_descendants(t, 3000));

    /* max_length shorter than the path: the C code keeps the *last*
     * max_length entries of the root-to-node walk. */
    t_path(t, 2005, 10);
    t_path(t, 2005, 3);
    t_path(t, 2005, 1);
    t_path(t, 2005, 0);
    /* Node 3009 is 1010 deep, so the temp_path[1000] loop cap is hit before
     * the root is reached. */
    t_path(t, 3009, 64);
    t_path(t, 3009, 5);
    t_path(t, 2000, 64);
    t_path(t, 12345, 10); /* not in the tree */

    tree_delete(t);
}

static void sec_tree_wide_and_print(void) {
    printf("\n### tree: wide fan-out printing ###\n");
    tree_t *t = tree_create();
    t_add(t, 1, 0, "root");
    for (int i = 0; i < 5; i++) {
        t_add(t, (tree_id_t)(10 + i), 1, "mid");
        for (int j = 0; j < 3; j++) {
            t_add(t, (tree_id_t)(100 + i * 10 + j), (tree_id_t)(10 + i), "leaf");
        }
    }
    tree_print(t);
    t_query(t, 1);
    for (int i = 0; i < 5; i++) {
        t_query(t, (tree_id_t)(10 + i));
    }
    t_path(t, 123, 10);
    tree_delete(t);
}

int main(void) {
    init_vals();
    printf("=== BRANCH PROBE ===\n");
    sec_hashmap_growth();
    sec_hashmap_deletion();
    sec_tree_empty();
    sec_tree_add_paths();
    sec_tree_max_children();
    sec_tree_child_removal();
    sec_tree_subtree_and_root();
    sec_tree_zero_and_max_ids();
    sec_tree_deep_chain();
    sec_tree_wide_and_print();
    printf("=== PROBE DONE ===\n");
    return 0;
}
