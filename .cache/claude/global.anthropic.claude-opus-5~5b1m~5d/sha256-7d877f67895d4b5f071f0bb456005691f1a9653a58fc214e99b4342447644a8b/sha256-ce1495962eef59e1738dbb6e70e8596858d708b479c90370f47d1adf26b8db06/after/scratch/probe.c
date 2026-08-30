#include <stdio.h>
typedef struct { float x, y; } c2v;
extern float c2GJK(const void*, unsigned, const void*, const void*, unsigned, const void*,
                   c2v*, c2v*, int, int*, void*);
int main(void){
  float circ[8] = {1,2,3,0,0,0,0,0};
  c2v a,b; int it=0;
  float d = c2GJK(circ, 7u, 0, circ, 9u, 0, &a, &b, 1, &it, 0);
  printf("dist=%g a=(%g,%g) b=(%g,%g) it=%d\n", d, a.x,a.y,b.x,b.y,it);
  return 0;
}
