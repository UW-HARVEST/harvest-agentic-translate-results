#include <stdio.h>
#include <math.h>
#include "src/quadtree.h"

int main(void) {
    // Bounds tests
    quadtree_bounds_t *b = quadtree_bounds_new();
    printf("bounds_new: nw=(%.2f,%.2f) se=(%.2f,%.2f) w=%.2f h=%.2f\n",
           b->nw->x, b->nw->y, b->se->x, b->se->y, b->width, b->height);
    quadtree_bounds_extend(b, 5.0, 5.0);
    printf("after extend(5,5): nw=(%.2f,%.2f) se=(%.2f,%.2f) w=%.2f h=%.2f\n",
           b->nw->x, b->nw->y, b->se->x, b->se->y, b->width, b->height);
    quadtree_bounds_extend(b, 10.0, 10.0);
    printf("after extend(10,10): nw=(%.2f,%.2f) se=(%.2f,%.2f) w=%.2f h=%.2f\n",
           b->nw->x, b->nw->y, b->se->x, b->se->y, b->width, b->height);
    quadtree_bounds_extend(b, -5.0, -5.0);
    printf("after extend(-5,-5): nw=(%.2f,%.2f) se=(%.2f,%.2f) w=%.2f h=%.2f\n",
           b->nw->x, b->nw->y, b->se->x, b->se->y, b->width, b->height);
    quadtree_bounds_free(b);

    // Node tests
    quadtree_node_t *n = quadtree_node_new();
    printf("new node: isleaf=%d isempty=%d ispointer=%d\n",
           quadtree_node_isleaf(n), quadtree_node_isempty(n), quadtree_node_ispointer(n));
    free(n);

    quadtree_node_t *nb = quadtree_node_with_bounds(1, 1, 10, 10);
    printf("node_with_bounds(1,1,10,10): nw=(%.2f,%.2f) se=(%.2f,%.2f) w=%.2f h=%.2f\n",
           nb->bounds->nw->x, nb->bounds->nw->y, nb->bounds->se->x, nb->bounds->se->y,
           nb->bounds->width, nb->bounds->height);
    quadtree_bounds_free(nb->bounds);
    free(nb);

    // Tree
    int val = 10;
    quadtree_t *tree = quadtree_new(1, 1, 10, 10);
    printf("tree root bounds: nw=(%.2f,%.2f) se=(%.2f,%.2f)\n",
           tree->root->bounds->nw->x, tree->root->bounds->nw->y,
           tree->root->bounds->se->x, tree->root->bounds->se->y);
    printf("tree length: %u\n", tree->length);

    printf("insert(0,0)=%d\n", quadtree_insert(tree, 0, 0, &val));
    printf("insert(110,110)=%d\n", quadtree_insert(tree, 110.0, 110.0, &val));
    printf("insert(8,2)=%d  length=%u\n", quadtree_insert(tree, 8.0, 2.0, &val), tree->length);
    printf("root.point.x=%.2f y=%.2f\n", tree->root->point->x, tree->root->point->y);
    printf("insert(0,1)=%d\n", quadtree_insert(tree, 0.0, 1.0, &val));
    printf("insert(2,3)=%d  length=%u\n", quadtree_insert(tree, 2.0, 3.0, &val), tree->length);
    printf("root.point=%p\n", (void *)tree->root->point);
    printf("insert(2,3)=%d  length=%u\n", quadtree_insert(tree, 2.0, 3.0, &val), tree->length);
    printf("insert(3,1.1)=%d  length=%u\n", quadtree_insert(tree, 3.0, 1.1, &val), tree->length);
    quadtree_point_t *found = quadtree_search(tree, 3.0, 1.1);
    printf("search(3,1.1): x=%.2f y=%.2f\n", found->x, found->y);

    // Search miss
    quadtree_point_t *miss = quadtree_search(tree, 99.0, 99.0);
    printf("search miss at (99,99): %p\n", (void *)miss);

    // Verify quadrants populated after split
    printf("root nw exists: %d, ne exists: %d, sw exists: %d, se exists: %d\n",
           tree->root->nw != NULL, tree->root->ne != NULL,
           tree->root->sw != NULL, tree->root->se != NULL);
    // Check which quadrant has (8, 2)
    if (tree->root->se && tree->root->se->point) {
        printf("root.se.point = (%.2f,%.2f)\n", tree->root->se->point->x, tree->root->se->point->y);
    }
    printf("root.ne is leaf? %d empty? %d\n",
           quadtree_node_isleaf(tree->root->ne), quadtree_node_isempty(tree->root->ne));

    quadtree_free(tree);

    // Point
    quadtree_point_t *p = quadtree_point_new(5, 6);
    printf("point: x=%.2f y=%.2f\n", p->x, p->y);
    quadtree_point_free(p);

    return 0;
}
