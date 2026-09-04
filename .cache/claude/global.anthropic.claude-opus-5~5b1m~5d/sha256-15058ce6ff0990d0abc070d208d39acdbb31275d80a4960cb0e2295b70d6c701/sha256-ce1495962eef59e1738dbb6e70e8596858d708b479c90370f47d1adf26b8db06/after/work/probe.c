#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int LZ4_compress_HC(const char*, char*, int, int, int);
int main(void){
    char dst[64];
    char src[1];
    for (int lvl=9; lvl<=12; lvl++){
      for (int n=0; n<=2; n++){
        printf("lvl=%d n=%d ...", lvl, n); fflush(stdout);
        int r = LZ4_compress_HC(src, dst, n, 64, lvl);
        printf(" = %d\n", r); fflush(stdout);
      }
    }
    return 0;
}
