#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
int main(int argc,char**argv){
  for(int i=1;i<argc;i++){
    int ec; PCRE2_SIZE eo;
    pcre2_code *re=pcre2_compile((PCRE2_SPTR)argv[i],PCRE2_ZERO_TERMINATED,PCRE2_UTF|PCRE2_UCP,&ec,&eo,NULL);
    if(!re){printf("<%s> FAIL %d\n",argv[i],ec);continue;}
    uint8_t *b=NULL; PCRE2_SIZE bl=0;
    int rc=pcre2_serialize_encode((const pcre2_code**)&re,1,&b,&bl,NULL);
    printf("<%s> rc=%d len=%zu\n",argv[i],rc,bl);
    if(rc>0){for(size_t k=0;k<bl;k++){printf("%02x",b[k]); if((k%32)==31)printf("\n");}printf("\n");
      pcre2_serialize_free(b);}
    pcre2_code_free(re);
  }
  return 0;
}
