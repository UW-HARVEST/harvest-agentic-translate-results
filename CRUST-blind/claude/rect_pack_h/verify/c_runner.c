#define RECT_PACK_H_IMPL
#include "c_src/rect_pack.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Read input from stdin: max_w max_h paging n_rects
// then n_rects lines of: id w h
// Output: result, then for each rect: id x y packed page (in sorted order)
int main(int argc, char* argv[]) {
    int max_w, max_h, paging_int, n;
    if (scanf("%d %d %d %d", &max_w, &max_h, &paging_int, &n) != 4) return 1;
    bool paging = paging_int != 0;

    rect_r* rects = (rect_r*)malloc(sizeof(rect_r) * n);
    for (int i = 0; i < n; i++) {
        scanf("%d %d %d", &rects[i].id, &rects[i].w, &rects[i].h);
    }

    bool ok = rect_pack(max_w, max_h, paging, rects, n);
    printf("RESULT %d\n", ok ? 1 : 0);
    for (int i = 0; i < n; i++) {
        printf("%d %d %d %d %d\n", rects[i].id, rects[i].info.x, rects[i].info.y,
               rects[i].info.packed ? 1 : 0, rects[i].info.page);
    }
    free(rects);
    return 0;
}
