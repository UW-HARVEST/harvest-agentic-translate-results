#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef size_t HUF_CElt;
extern size_t HUF_buildCTable_wksp(HUF_CElt*, const unsigned*, unsigned, unsigned, void*, size_t);
extern size_t HUF_compress1X_usingCTable(void*, size_t, const void*, size_t, const HUF_CElt*, int);
extern size_t HUF_compress4X_usingCTable(void*, size_t, const void*, size_t, const HUF_CElt*, int);
extern unsigned HUF_isError(size_t);
int main(int argc, char** argv) {
  int which = atoi(argv[1]);
  size_t cap = (size_t)atoi(argv[2]);
  static unsigned cnt[300];
  static HUF_CElt ct[300];
  static unsigned wksp[8192];
  unsigned char srcb[8]; unsigned char* dst = malloc(1<<20);
  size_t r;
  /* histogram of "rand n=2 mv=3": HIST_count returned 1, mv'=2 -> two distinct symbols */
  memset(cnt,0,sizeof(cnt));
  cnt[0]=1; cnt[2]=1;
  srcb[0]=0; srcb[1]=2;
  r = HUF_buildCTable_wksp(ct, cnt, 2, 12, wksp, sizeof(wksp));
  printf("buildCTable=%zd\n", r);
  memset(dst,0x5A,1<<20);
  if (which==1) r = HUF_compress1X_usingCTable(dst, cap, srcb, 2, ct, 0);
  else          r = HUF_compress4X_usingCTable(dst, cap, srcb, 2, ct, 0);
  printf("compress%dX(cap=%zu)=%zd\n", which, cap, r);
  return 0;
}
