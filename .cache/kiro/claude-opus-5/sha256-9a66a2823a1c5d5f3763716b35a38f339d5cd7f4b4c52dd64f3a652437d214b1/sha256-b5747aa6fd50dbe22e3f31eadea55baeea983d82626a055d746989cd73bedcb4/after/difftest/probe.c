#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
int main(int argc,char**argv){
  for(int i=1;i<argc;i++){
    int ec; PCRE2_SIZE eo;
    pcre2_code *re=pcre2_compile((PCRE2_SPTR)argv[i],PCRE2_ZERO_TERMINATED,0,&ec,&eo,NULL);
    if(re==NULL){PCRE2_UCHAR b[256];pcre2_get_error_message(ec,b,256);
      printf("<%s> FAIL %d off=%zu %s\n",argv[i],ec,eo,(char*)b);}
    else{printf("<%s> OK\n",argv[i]);pcre2_code_free(re);}
  }
  return 0;
}
