/*
 * Auxiliary differential probe.
 *
 * This file is NOT part of c_src/ and does not modify it: it only #includes the
 * public headers and is linked against the unmodified c_src/src/*.c.
 *
 * c_src/src/main.c exercises only the happy paths of tree.c/hashmap.c. Every
 * early `return`, NULL check and bound check that main.c never reaches is
 * driven from here instead, one scenario per process invocation, so the Rust
 * translation of those branches can be compared byte for byte.
 *
 * The scenario list is mirrored exactly by translation/src/bin/probe.rs.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "tree.h"

/* ------------------------------------------------------------------ helpers */

static unsigned long long lcg_state;

static void lcg_seed(unsigned long long s) { lcg_state = s; }

static unsigned long long lcg_next(void) {
    lcg_state = lcg_state * 6364136223846793005ULL + 1442695040888963407ULL;
    /* Shift away the low-quality low bits but keep 48 usable ones. */
    return lcg_state >> 16;
}

/* Dump the full internal state of a hashmap whose values point into `base`.
 * Printing slot-by-slot makes probe order, tombstones and capacity growth
 * observable, which is where an open-addressing translation is most likely to
 * drift. */
static void dump_int_map(hashmap_t *map, int *base) {
    printf("map size=%zu capacity=%zu deleted=%zu\n", map->size, map->capacity,
           map->deleted_count);
    for (size_t i = 0; i < map->capacity; i++) {
        hashmap_entry_t *e = &map->entries[i];
        if (!e->occupied) {
            printf("  [%zu] empty\n", i);
        } else if (e->deleted) {
            printf("  [%zu] key=%lu tombstone\n", i, e->key);
        } else {
            printf("  [%zu] key=%lu val=%ld\n", i, e->key,
                   (long)((int *)e->value - base));
        }
    }
}

/* Dump every live node in map-slot order, then the tree's own printout. */
static void dump_tree(tree_t *t) {
    printf("tree count=%zu root=%lu has_root=%d map_size=%zu map_cap=%zu "
           "map_deleted=%zu\n",
           t->node_count, t->root_id, t->has_root, t->node_map->size,
           t->node_map->capacity, t->node_map->deleted_count);
    for (size_t i = 0; i < t->node_map->capacity; i++) {
        hashmap_entry_t *e = &t->node_map->entries[i];
        if (!e->occupied || e->deleted) {
            continue;
        }
        tree_node_t *n = (tree_node_t *)e->value;
        printf("  slot=%zu id=%lu parent=%lu nchild=%d data='%s' children=[", i,
               n->id, n->parent_id, n->child_count, n->data);
        for (int j = 0; j < n->child_count; j++) {
            printf("%s%lu", j ? "," : "", n->child_ids[j]);
        }
        printf("]\n");
    }
    printf("print:\n");
    tree_print(t);
}

static void show_path(tree_t *t, tree_id_t id, tree_id_t *buf, int cap,
                      int max_length) {
    int n = tree_find_path(t, id, buf, max_length);
    printf("find_path(id=%lu, max=%d) = %d [", id, max_length, n);
    for (int i = 0; i < n && i < cap; i++) {
        printf("%s%lu", i ? "," : "", buf[i]);
    }
    printf("]\n");
}

static void show_queries(tree_t *t, tree_id_t id) {
    printf("id=%lu contains=%d depth=%d height=%d descendants=%d\n", id,
           tree_contains(t, id), tree_get_depth(t, id), tree_get_height(t, id),
           tree_count_descendants(t, id));
}

static char *repeat(char c, size_t n) {
    char *s = malloc(n + 1);
    memset(s, c, n);
    s[n] = '\0';
    return s;
}

/* ---------------------------------------------------------------- scenarios */

/* tree_print's `!tree->has_root` branch. */
static void sc_empty_print(void) {
    tree_t *t = tree_create();
    printf("size=%zu has_root=%d root_id=%lu\n", tree_size(t), t->has_root,
           t->root_id);
    tree_print(t);
    dump_tree(t);
    tree_delete(t);
}

/* tree_add_node's `data == NULL` branch. */
static void sc_null_data(void) {
    tree_t *t = tree_create();
    printf("add=%d\n", tree_add_node(t, 7, 0, NULL));
    printf("add_child=%d\n", tree_add_node(t, 8, 7, NULL));
    dump_tree(t);
    tree_delete(t);
}

/* strncpy(dst, src, MAX_DATA_LENGTH - 1) truncation boundaries. */
static void sc_data_lengths(void) {
    size_t lens[] = {0, 1, 254, 255, 256, 300, 1024};
    for (size_t k = 0; k < sizeof(lens) / sizeof(lens[0]); k++) {
        tree_t *t = tree_create();
        char *s = repeat('A' + (int)(k % 26), lens[k]);
        printf("len=%zu add=%d\n", lens[k], tree_add_node(t, 1, 0, s));
        tree_node_t *n = tree_get_node(t, 1);
        printf("  strlen=%zu data='%s'\n", strlen(n->data), n->data);
        printf("  last=%d byte254=%d\n", (int)(unsigned char)n->data[255],
               (int)(unsigned char)n->data[254]);
        tree_print(t);
        free(s);
        tree_delete(t);
    }
}

/* tree_add_node: parent lookup failure. */
static void sc_parent_missing(void) {
    tree_t *t = tree_create();
    printf("root=%d\n", tree_add_node(t, 1, 0, "root"));
    printf("orphan=%d\n", tree_add_node(t, 2, 99, "orphan"));
    printf("orphan_zero=%d\n", tree_add_node(t, 3, 0, "parent-zero"));
    printf("size=%zu\n", tree_size(t));
    dump_tree(t);
    tree_delete(t);
}

/* tree_add_node: duplicate id, including a duplicate of the root. */
static void sc_duplicate_ids(void) {
    tree_t *t = tree_create();
    printf("root=%d\n", tree_add_node(t, 1, 0, "root"));
    printf("dup_root=%d\n", tree_add_node(t, 1, 0, "again"));
    printf("child=%d\n", tree_add_node(t, 2, 1, "child"));
    printf("dup_child=%d\n", tree_add_node(t, 2, 1, "again"));
    printf("dup_child_other_parent=%d\n", tree_add_node(t, 2, 2, "again"));
    printf("size=%zu\n", tree_size(t));
    dump_tree(t);
    tree_delete(t);
}

/* tree_remove_node: node not found, on an empty tree and a populated one. */
static void sc_remove_missing(void) {
    tree_t *t = tree_create();
    printf("empty_remove=%d\n", tree_remove_node(t, 1));
    printf("root=%d\n", tree_add_node(t, 1, 0, "root"));
    printf("missing_remove=%d\n", tree_remove_node(t, 42));
    printf("zero_remove=%d\n", tree_remove_node(t, 0));
    printf("root_remove=%d\n", tree_remove_node(t, 1));
    printf("again=%d\n", tree_remove_node(t, 1));
    dump_tree(t);
    tree_delete(t);
}

/* Every query function's `node == NULL` early return. */
static void sc_queries_missing(void) {
    tree_t *t = tree_create();
    tree_id_t buf[16];
    show_queries(t, 1);
    show_path(t, 1, buf, 16, 16);
    printf("root=%d\n", tree_add_node(t, 1, 0, "root"));
    show_queries(t, 1);
    show_queries(t, 2);
    show_queries(t, 0);
    show_path(t, 2, buf, 16, 16);
    show_path(t, 0, buf, 16, 16);
    tree_delete(t);
}

/* tree_find_path: max_length clamping, including 0 and negative. */
static void sc_path_bounds(void) {
    tree_t *t = tree_create();
    tree_add_node(t, 1, 0, "a");
    tree_add_node(t, 2, 1, "b");
    tree_add_node(t, 3, 2, "c");
    tree_add_node(t, 4, 3, "d");
    tree_id_t buf[16];
    for (int max = -2; max <= 6; max++) {
        memset(buf, 0, sizeof(buf));
        show_path(t, 4, buf, 16, max);
        printf("  buf=[%lu,%lu,%lu,%lu,%lu]\n", buf[0], buf[1], buf[2], buf[3],
               buf[4]);
    }
    show_path(t, 1, buf, 16, 1);
    tree_delete(t);
}

/* Root removal empties the tree, and the next add becomes the new root with a
 * forced parent_id of 0. */
static void sc_remove_root_then_add(void) {
    tree_t *t = tree_create();
    tree_add_node(t, 10, 0, "root");
    tree_add_node(t, 11, 10, "child");
    tree_add_node(t, 12, 11, "grandchild");
    dump_tree(t);
    printf("remove_root=%d\n", tree_remove_node(t, 10));
    dump_tree(t);
    printf("readd=%d\n", tree_add_node(t, 20, 999, "new-root"));
    dump_tree(t);
    printf("readd_child=%d\n", tree_add_node(t, 21, 20, "new-child"));
    show_queries(t, 20);
    show_queries(t, 21);
    dump_tree(t);
    tree_delete(t);
}

/* MAX_CHILDREN boundary, then freeing a slot and refilling it. */
static void sc_max_children(void) {
    tree_t *t = tree_create();
    tree_add_node(t, 1, 0, "root");
    for (int i = 0; i < MAX_CHILDREN; i++) {
        int rc = tree_add_node(t, (tree_id_t)(i + 2), 1, "child");
        if (rc != 0) {
            printf("unexpected failure at %d\n", i);
        }
    }
    size_t before = tree_size(t);
    int overflow = tree_add_node(t, MAX_CHILDREN + 2, 1, "overflow");
    printf("count=%zu overflow=%d\n", before, overflow);
    printf("overflow2=%d\n", tree_add_node(t, MAX_CHILDREN + 3, 1, "overflow"));
    printf("remove_first=%d\n", tree_remove_node(t, 2));
    printf("refill=%d\n", tree_add_node(t, MAX_CHILDREN + 2, 1, "refill"));
    printf("overflow3=%d\n", tree_add_node(t, MAX_CHILDREN + 4, 1, "overflow"));
    tree_node_t *root = tree_get_node(t, 1);
    printf("root_children=%d\n", root->child_count);
    for (int i = 0; i < root->child_count; i++) {
        printf("%s%lu", i ? "," : "  ", root->child_ids[i]);
    }
    printf("\n");
    printf("height=%d descendants=%d\n", tree_get_height(t, 1),
           tree_count_descendants(t, 1));
    tree_delete(t);
}

/* The child-list shift in tree_remove_node, from each position. */
static void sc_remove_child_positions(void) {
    const char *labels[] = {"first", "middle", "last"};
    tree_id_t victims[] = {2, 4, 7};
    for (int k = 0; k < 3; k++) {
        tree_t *t = tree_create();
        tree_add_node(t, 1, 0, "root");
        for (tree_id_t i = 2; i <= 7; i++) {
            tree_add_node(t, i, 1, "child");
        }
        printf("remove %s (%lu) = %d\n", labels[k], victims[k],
               tree_remove_node(t, victims[k]));
        tree_node_t *root = tree_get_node(t, 1);
        printf("  nchild=%d [", root->child_count);
        for (int i = 0; i < root->child_count; i++) {
            printf("%s%lu", i ? "," : "", root->child_ids[i]);
        }
        printf("] stale_slot=%lu\n", root->child_ids[root->child_count]);
        dump_tree(t);
        tree_delete(t);
    }
}

/* Cascading subtree removal, and removal of a node whose parent link is stale
 * because the parent was removed first. */
static void sc_subtree_cascade(void) {
    tree_t *t = tree_create();
    tree_add_node(t, 1, 0, "root");
    tree_add_node(t, 2, 1, "a");
    tree_add_node(t, 3, 2, "aa");
    tree_add_node(t, 4, 3, "aaa");
    tree_add_node(t, 5, 2, "ab");
    tree_add_node(t, 6, 1, "b");
    dump_tree(t);
    printf("remove_2=%d\n", tree_remove_node(t, 2));
    dump_tree(t);
    printf("remove_3=%d\n", tree_remove_node(t, 3));
    printf("remove_6=%d\n", tree_remove_node(t, 6));
    dump_tree(t);
    printf("remove_1=%d\n", tree_remove_node(t, 1));
    dump_tree(t);
    tree_delete(t);
}

/* Node id 0 as the root: tree_get_depth's loop condition compares against
 * root_id, which is also the "no root" sentinel value. */
static void sc_id_zero(void) {
    tree_t *t = tree_create();
    printf("root_zero=%d\n", tree_add_node(t, 0, 0, "zero-root"));
    printf("child=%d\n", tree_add_node(t, 1, 0, "child"));
    printf("grand=%d\n", tree_add_node(t, 2, 1, "grand"));
    show_queries(t, 0);
    show_queries(t, 1);
    show_queries(t, 2);
    tree_id_t buf[8];
    show_path(t, 2, buf, 8, 8);
    show_path(t, 0, buf, 8, 8);
    dump_tree(t);
    printf("remove_root=%d\n", tree_remove_node(t, 0));
    dump_tree(t);
    tree_delete(t);
}

/* Extreme key values: FNV-1a over the raw bytes plus %lu formatting. */
static void sc_big_ids(void) {
    tree_id_t ids[] = {0ULL,
                       1ULL,
                       255ULL,
                       256ULL,
                       65535ULL,
                       4294967295ULL,
                       4294967296ULL,
                       9223372036854775807ULL,
                       9223372036854775808ULL,
                       18446744073709551615ULL,
                       0x0102030405060708ULL,
                       0x00000000000000FFULL};
    size_t n = sizeof(ids) / sizeof(ids[0]);
    tree_t *t = tree_create();
    printf("root=%d\n", tree_add_node(t, ids[0], 0, "root"));
    for (size_t i = 1; i < n; i++) {
        printf("add %lu = %d\n", ids[i], tree_add_node(t, ids[i], ids[0], "n"));
    }
    dump_tree(t);
    for (size_t i = 0; i < n; i++) {
        show_queries(t, ids[i]);
    }
    /* Duplicate of the largest key hits the "already exists" path. */
    printf("dup_max=%d\n", tree_add_node(t, 18446744073709551615ULL, ids[0], "d"));
    tree_delete(t);
}

/* Chain longer than tree_find_path's fixed 1000-entry scratch buffer. */
static void sc_deep_chain(void) {
    tree_t *t = tree_create();
    const tree_id_t depth = 1100;
    printf("root=%d\n", tree_add_node(t, 1, 0, "n1"));
    for (tree_id_t i = 2; i <= depth; i++) {
        char buf[32];
        snprintf(buf, sizeof(buf), "n%lu", i);
        if (tree_add_node(t, i, i - 1, buf) != 0) {
            printf("add failed at %lu\n", i);
            break;
        }
    }
    printf("size=%zu\n", tree_size(t));
    printf("depth_last=%d height_root=%d descendants_root=%d\n",
           tree_get_depth(t, depth), tree_get_height(t, 1),
           tree_count_descendants(t, 1));
    tree_id_t *buf = malloc(sizeof(tree_id_t) * 2048);
    int n = tree_find_path(t, depth, buf, 2048);
    printf("path_len=%d first=%lu last=%lu\n", n, buf[0], buf[n - 1]);
    n = tree_find_path(t, 500, buf, 2048);
    printf("path500_len=%d first=%lu last=%lu\n", n, buf[0], buf[n - 1]);
    free(buf);
    printf("remove_mid=%d\n", tree_remove_node(t, 550));
    printf("size=%zu height_root=%d\n", tree_size(t), tree_get_height(t, 1));
    printf("depth_last=%d\n", tree_get_depth(t, depth));
    tree_delete(t);
}

/* hashmap_clear: only the flags are reset, keys and values are left behind. */
static void sc_clear_map(void) {
    hashmap_t *map = hashmap_create();
    int values[32];
    for (int i = 0; i < 12; i++) {
        values[i] = i;
        hashmap_put(map, (tree_id_t)i, &values[i]);
    }
    hashmap_remove(map, 3);
    dump_int_map(map, values);
    hashmap_clear(map);
    printf("after clear: size=%zu contains5=%d has5=%d\n", hashmap_size(map),
           hashmap_contains(map, 5), hashmap_get(map, 5) != NULL);
    dump_int_map(map, values);
    hashmap_put(map, 5, &values[7]);
    dump_int_map(map, values);
    hashmap_destroy(map);
}

/* hashmap tombstone reuse: hashmap_put takes the first tombstone it sees
 * without checking whether the key already lives further along the probe
 * chain, so the same key can end up stored twice. */
static void sc_tombstones(void) {
    hashmap_t *map = hashmap_create();
    int values[64];
    for (int i = 0; i < 64; i++) {
        values[i] = i;
    }
    /* Insert a block of keys, then delete and reinsert in an order that walks
     * the probe chains over tombstones. */
    for (tree_id_t k = 0; k < 10; k++) {
        hashmap_put(map, k, &values[k]);
    }
    dump_int_map(map, values);
    for (tree_id_t k = 0; k < 10; k += 2) {
        void *v = hashmap_remove(map, k);
        printf("remove %lu -> %ld\n", k, v ? (long)((int *)v - values) : -1);
    }
    dump_int_map(map, values);
    for (tree_id_t k = 0; k < 10; k++) {
        /* Sequenced deliberately: mixing a mutating call with reads of the map
         * inside one printf would depend on the compiler's argument evaluation
         * order, which C leaves unspecified. */
        int rc = hashmap_put(map, k, &values[k + 20]);
        size_t sz = hashmap_size(map);
        size_t del = map->deleted_count;
        printf("put %lu -> %d (size=%zu deleted=%zu)\n", k, rc, sz, del);
    }
    dump_int_map(map, values);
    for (tree_id_t k = 0; k < 10; k++) {
        void *v = hashmap_get(map, k);
        printf("get %lu -> %ld contains=%d\n", k,
               v ? (long)((int *)v - values) : -1, hashmap_contains(map, k));
    }
    /* Remove everything twice: the second pass must report NULL. */
    for (tree_id_t k = 0; k < 10; k++) {
        void *a = hashmap_remove(map, k);
        void *b = hashmap_remove(map, k);
        printf("double remove %lu -> %ld / %ld\n", k,
               a ? (long)((int *)a - values) : -1,
               b ? (long)((int *)b - values) : -1);
    }
    dump_int_map(map, values);
    hashmap_destroy(map);
}

/* Growth: should_resize is checked on every put, including updates. */
/* Which slot a key lands in when it is the only key in a fresh map, i.e. its
 * hash modulo the initial capacity. Derived empirically because hash_function is
 * static inside hashmap.c. */
static size_t home_slot(tree_id_t key, int *values) {
    hashmap_t *m = hashmap_create();
    hashmap_put(m, key, &values[0]);
    size_t slot = 0;
    for (size_t i = 0; i < m->capacity; i++) {
        if (m->entries[i].occupied) {
            slot = i;
            break;
        }
    }
    hashmap_destroy(m);
    return slot;
}

/* Linear probing over a real collision, plus the consequence of hashmap_put
 * claiming the first tombstone it sees without checking whether the key already
 * lives further along the chain: the key ends up stored twice. */
static void sc_collision_probing(void) {
    static int values[64];
    for (int i = 0; i < 64; i++) {
        values[i] = i;
    }

    size_t homes[400];
    for (tree_id_t k = 0; k < 400; k++) {
        homes[k] = home_slot(k, values);
    }
    tree_id_t a = 0, b = 0;
    int found = 0;
    for (tree_id_t x = 0; x < 400 && !found; x++) {
        for (tree_id_t y = x + 1; y < 400; y++) {
            if (homes[x] == homes[y]) {
                a = x;
                b = y;
                found = 1;
                break;
            }
        }
    }
    printf("found=%d a=%lu b=%lu home=%zu\n", found, a, b, homes[a]);

    hashmap_t *map = hashmap_create();
    int pa = hashmap_put(map, a, &values[1]);
    int pb = hashmap_put(map, b, &values[2]);
    printf("put a=%d put b=%d\n", pa, pb);
    dump_int_map(map, values);

    /* Deleting the key that owns the home slot leaves a tombstone the lookup for
     * `b` has to walk past. */
    void *ra = hashmap_remove(map, a);
    printf("remove a -> %ld\n", ra ? (long)((int *)ra - values) : -1);
    void *gb = hashmap_get(map, b);
    printf("get b past tombstone -> %ld contains=%d size=%zu\n",
           gb ? (long)((int *)gb - values) : -1, hashmap_contains(map, b),
           hashmap_size(map));
    dump_int_map(map, values);

    /* Re-putting `b` now claims the tombstone, duplicating the key. */
    int rc = hashmap_put(map, b, &values[3]);
    size_t sz = hashmap_size(map);
    printf("put b again -> %d size=%zu\n", rc, sz);
    dump_int_map(map, values);
    void *g2 = hashmap_get(map, b);
    printf("get b -> %ld\n", g2 ? (long)((int *)g2 - values) : -1);

    /* Each remove peels off one of the two entries. */
    void *r1 = hashmap_remove(map, b);
    printf("remove b -> %ld then get -> ", r1 ? (long)((int *)r1 - values) : -1);
    void *g3 = hashmap_get(map, b);
    printf("%ld size=%zu\n", g3 ? (long)((int *)g3 - values) : -1,
           hashmap_size(map));
    dump_int_map(map, values);
    void *r2 = hashmap_remove(map, b);
    printf("remove b -> %ld then get -> ", r2 ? (long)((int *)r2 - values) : -1);
    void *g4 = hashmap_get(map, b);
    printf("%ld size=%zu\n", g4 ? (long)((int *)g4 - values) : -1,
           hashmap_size(map));
    dump_int_map(map, values);

    /* Filling past the load factor with tombstones present forces a rehash that
     * has to cope with the duplicate. */
    for (tree_id_t k = 0; k < 40; k++) {
        hashmap_put(map, 1000 + k, &values[k % 64]);
    }
    printf("after fill size=%zu capacity=%zu deleted=%zu\n", hashmap_size(map),
           map->capacity, map->deleted_count);
    printf("get a -> %d get b -> %d\n", hashmap_contains(map, a),
           hashmap_contains(map, b));
    dump_int_map(map, values);
    hashmap_destroy(map);
}

static void sc_resize_map(void) {
    hashmap_t *map = hashmap_create();
    static int values[400];
    for (int i = 0; i < 400; i++) {
        values[i] = i;
    }
    for (int i = 0; i < 200; i++) {
        int rc = hashmap_put(map, (tree_id_t)i, &values[i]);
        if (rc != 0 || map->capacity != 16) {
            printf("i=%d rc=%d size=%zu capacity=%zu\n", i, rc,
                   hashmap_size(map), map->capacity);
        }
        if (i % 25 == 0) {
            printf("i=%d size=%zu capacity=%zu deleted=%zu\n", i,
                   hashmap_size(map), map->capacity, map->deleted_count);
        }
    }
    dump_int_map(map, values);
    /* Deleting a lot inflates deleted_count until a put triggers a rehash that
     * drops the tombstones. */
    for (int i = 0; i < 200; i += 2) {
        hashmap_remove(map, (tree_id_t)i);
    }
    printf("after deletes size=%zu capacity=%zu deleted=%zu\n",
           hashmap_size(map), map->capacity, map->deleted_count);
    for (int i = 200; i < 400; i++) {
        hashmap_put(map, (tree_id_t)i, &values[i]);
    }
    printf("after refill size=%zu capacity=%zu deleted=%zu\n",
           hashmap_size(map), map->capacity, map->deleted_count);
    dump_int_map(map, values);
    /* Re-putting existing keys must not grow the map. */
    for (int i = 200; i < 400; i++) {
        hashmap_put(map, (tree_id_t)i, &values[i - 200]);
    }
    printf("after updates size=%zu capacity=%zu deleted=%zu\n",
           hashmap_size(map), map->capacity, map->deleted_count);
    hashmap_destroy(map);
}

/* A long pseudo-random operation mix over the hashmap. */
static void sc_stress_map(void) {
    lcg_seed(0x0123456789ABCDEFULL);
    hashmap_t *map = hashmap_create();
    static int values[512];
    for (int i = 0; i < 512; i++) {
        values[i] = i;
    }
    for (int step = 0; step < 4000; step++) {
        unsigned long long r = lcg_next();
        int op = (int)(r % 4);
        tree_id_t key = (tree_id_t)((r >> 8) % 96);
        int vidx = (int)((r >> 24) % 512);
        if (op <= 1) {
            printf("%d put %lu %d -> %d\n", step, key, vidx,
                   hashmap_put(map, key, &values[vidx]));
        } else if (op == 2) {
            void *v = hashmap_remove(map, key);
            printf("%d rm %lu -> %ld\n", step, key,
                   v ? (long)((int *)v - values) : -1);
        } else {
            void *v = hashmap_get(map, key);
            printf("%d get %lu -> %ld c=%d s=%zu\n", step, key,
                   v ? (long)((int *)v - values) : -1, hashmap_contains(map, key),
                   hashmap_size(map));
        }
        if (step % 1000 == 0) {
            dump_int_map(map, values);
        }
    }
    dump_int_map(map, values);
    hashmap_destroy(map);
}

/* A long pseudo-random operation mix over the tree, mixing valid and invalid
 * ids so the error paths fire repeatedly. */
static void sc_stress_tree(void) {
    lcg_seed(0xFEEDFACECAFEBEEFULL);
    tree_t *t = tree_create();
    printf("root=%d\n", tree_add_node(t, 1, 0, "root"));
    tree_id_t next_id = 2;
    tree_id_t buf[64];
    for (int step = 0; step < 1500; step++) {
        unsigned long long r = lcg_next();
        int op = (int)(r % 8);
        if (op <= 3) {
            tree_id_t id = ((r >> 3) % 5 == 0) ? (tree_id_t)((r >> 16) % next_id)
                                               : next_id;
            tree_id_t parent = (tree_id_t)((r >> 8) % (next_id + 3));
            char data[32];
            snprintf(data, sizeof(data), "n%lu", id);
            int rc = tree_add_node(t, id, parent, data);
            size_t sz = tree_size(t);
            printf("%d add id=%lu p=%lu -> %d size=%zu\n", step, id, parent, rc,
                   sz);
            next_id++;
        } else if (op == 4 || op == 5) {
            tree_id_t id = (tree_id_t)((r >> 8) % (next_id + 3));
            int rc = tree_remove_node(t, id);
            size_t sz = tree_size(t);
            int hr = t->has_root;
            tree_id_t root = t->root_id;
            printf("%d rm id=%lu -> %d size=%zu has_root=%d root=%lu\n", step, id,
                   rc, sz, hr, root);
        } else {
            tree_id_t id = (tree_id_t)((r >> 8) % (next_id + 3));
            show_queries(t, id);
            show_path(t, id, buf, 64, (int)((r >> 20) % 8));
        }
        if (step % 250 == 0) {
            dump_tree(t);
        }
    }
    dump_tree(t);
    tree_delete(t);
}

/* ---------------------------------------------------------------- dispatch */

struct scenario {
    const char *name;
    void (*run)(void);
};

static const struct scenario scenarios[] = {
    {"empty_print", sc_empty_print},
    {"null_data", sc_null_data},
    {"data_lengths", sc_data_lengths},
    {"parent_missing", sc_parent_missing},
    {"duplicate_ids", sc_duplicate_ids},
    {"remove_missing", sc_remove_missing},
    {"queries_missing", sc_queries_missing},
    {"path_bounds", sc_path_bounds},
    {"remove_root_then_add", sc_remove_root_then_add},
    {"max_children", sc_max_children},
    {"remove_child_positions", sc_remove_child_positions},
    {"subtree_cascade", sc_subtree_cascade},
    {"id_zero", sc_id_zero},
    {"big_ids", sc_big_ids},
    {"deep_chain", sc_deep_chain},
    {"clear_map", sc_clear_map},
    {"tombstones", sc_tombstones},
    {"collision_probing", sc_collision_probing},
    {"resize_map", sc_resize_map},
    {"stress_map", sc_stress_map},
    {"stress_tree", sc_stress_tree},
};

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: probe <scenario>\n");
        return 2;
    }
    size_t n = sizeof(scenarios) / sizeof(scenarios[0]);
    for (size_t i = 0; i < n; i++) {
        if (strcmp(argv[1], scenarios[i].name) == 0) {
            scenarios[i].run();
            return 0;
        }
    }
    fprintf(stderr, "unknown scenario: %s\n", argv[1]);
    return 3;
}
