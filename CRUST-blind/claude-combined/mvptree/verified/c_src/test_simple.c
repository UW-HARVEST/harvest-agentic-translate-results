/* Custom C test for gathering expected values */
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <math.h>
#include <assert.h>
#include "mvptree.h"

static unsigned long long nbcalcs = 0;

float point_l1_distance(MVPDP *pointA, MVPDP *pointB){
    if (!pointA || !pointB) return -1.0f;
    unsigned int i;
    unsigned int sum = 0;
    uint8_t *data1 = (uint8_t*)pointA->data;
    uint8_t *data2 = (uint8_t*)pointB->data;
    for (i = 0; i < pointA->datalen; i++){
        int d1 = (int)data1[i];
        int d2 = (int)data2[i];
        int diff = abs(d1 - d2);
        sum += diff;
    }
    nbcalcs++;
    return (float)sum/(float)pointA->datalen;
}

MVPDP *make_point(const char *id, uint8_t *data, unsigned int datalen) {
    MVPDP *dp = dp_alloc(BYTEARRAY);
    dp->id = strdup(id);
    dp->data = malloc(datalen);
    memcpy(dp->data, data, datalen);
    dp->datalen = datalen;
    return dp;
}

int main() {
    // Simple distance test
    uint8_t a[] = {0, 0, 0, 0, 0};
    uint8_t b[] = {1, 1, 1, 1, 1};
    MVPDP *pa = make_point("A", a, 5);
    MVPDP *pb = make_point("B", b, 5);
    float d = point_l1_distance(pa, pb);
    printf("L1(A,B)=%f\n", d);

    // Build a small tree
    MVPTree *tree = mvptree_alloc(NULL, point_l1_distance, 2, 5, 25);
    printf("tree.bf=%d, tree.pl=%d, tree.lc=%d\n", tree->branchfactor, tree->pathlength, tree->leafcap);

    // Add a few points
    int N = 5;
    MVPDP **pts = malloc(N * sizeof(MVPDP*));
    for (int i = 0; i < N; i++) {
        char name[16];
        snprintf(name, 16, "pt%d", i);
        uint8_t data[5] = {(uint8_t)i, (uint8_t)i, (uint8_t)i, (uint8_t)i, (uint8_t)i};
        pts[i] = make_point(name, data, 5);
    }
    MVPError err = mvptree_add(tree, pts, N);
    printf("add err=%d\n", err);

    // Retrieve
    uint8_t qdata[5] = {2, 2, 2, 2, 2};
    MVPDP *target = make_point("Q", qdata, 5);
    unsigned int nbresults = 0;
    MVPDP **results = mvptree_retrieve(tree, target, 10, 1.0, &nbresults, &err);
    printf("retrieve err=%d, nbresults=%u\n", err, nbresults);
    for (unsigned int i = 0; i < nbresults; i++) {
        printf("  [%u] %s\n", i, results[i]->id);
    }
    free(results);

    // Retrieve with radius 0
    nbresults = 0;
    results = mvptree_retrieve(tree, target, 10, 0.0, &nbresults, &err);
    printf("retrieve r=0 err=%d, nbresults=%u\n", err, nbresults);
    free(results);

    // Retrieve with radius 5
    nbresults = 0;
    results = mvptree_retrieve(tree, target, 10, 5.0, &nbresults, &err);
    printf("retrieve r=5 err=%d, nbresults=%u\n", err, nbresults);
    for (unsigned int i = 0; i < nbresults; i++) {
        printf("  r5[%u] %s\n", i, results[i]->id);
    }
    free(results);

    // Retrieve identical
    uint8_t qdata2[5] = {3, 3, 3, 3, 3};
    MVPDP *target2 = make_point("Q2", qdata2, 5);
    nbresults = 0;
    results = mvptree_retrieve(tree, target2, 10, 0.0, &nbresults, &err);
    printf("retrieve exact pt3 r=0 err=%d, nbresults=%u\n", err, nbresults);
    for (unsigned int i = 0; i < nbresults; i++) {
        printf("  exact[%u] %s\n", i, results[i]->id);
    }
    free(results);

    // Larger tree to trigger internal nodes
    MVPTree *tree2 = mvptree_alloc(NULL, point_l1_distance, 2, 5, 5);
    int M = 20;
    MVPDP **pts2 = malloc(M * sizeof(MVPDP*));
    for (int i = 0; i < M; i++) {
        char name[16];
        snprintf(name, 16, "p%d", i);
        uint8_t data[5] = {(uint8_t)(i*5), (uint8_t)(i*3), (uint8_t)(i*7), (uint8_t)i, (uint8_t)(i*2)};
        pts2[i] = make_point(name, data, 5);
    }
    err = mvptree_add(tree2, pts2, M);
    printf("tree2 add err=%d\n", err);

    uint8_t qdata3[5] = {25, 15, 35, 5, 10};
    MVPDP *target3 = make_point("Q3", qdata3, 5);
    nbresults = 0;
    results = mvptree_retrieve(tree2, target3, 100, 100.0, &nbresults, &err);
    printf("tree2 retrieve r=100 err=%d, nbresults=%u\n", err, nbresults);
    for (unsigned int i = 0; i < nbresults; i++) {
        printf("  t2[%u] %s\n", i, results[i]->id);
    }
    free(results);

    // Test error strings
    printf("err 0: %s\n", mvp_errstr(MVP_SUCCESS));
    printf("err 1: %s\n", mvp_errstr(MVP_ARGERR));
    printf("err 24: %s\n", mvp_errstr(MVP_UNRECOGNIZED));

    return 0;
}
