#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
int main(void){
  PCRE2_UCHAR *out=NULL; PCRE2_SIZE outlen=0;
  int rc = pcre2_pattern_convert((PCRE2_SPTR)"a/b", PCRE2_ZERO_TERMINATED,
     PCRE2_CONVERT_POSIX_EXTENDED, &out, &outlen, NULL);
  printf("rc=%d len=%zu out=%p\n", rc, outlen, (void*)out);
  if(rc==0) printf("<%s>\n", (char*)out);
  fflush(stdout);
  if(rc==0) pcre2_converted_pattern_free(out);
  printf("freed ok\n");
  return 0;
}
