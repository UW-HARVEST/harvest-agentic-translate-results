#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
int main(int argc,char**argv){
  uint32_t copt=(uint32_t)strtoul(argv[1],NULL,0);
  uint32_t xopt=(uint32_t)strtoul(argv[2],NULL,0);
  for(int i=3;i<argc;i++){
    int ec; PCRE2_SIZE eo;
    pcre2_compile_context *cc=pcre2_compile_context_create(NULL);
    pcre2_set_compile_extra_options(cc,xopt);
    pcre2_code *re=pcre2_compile((PCRE2_SPTR)argv[i],PCRE2_ZERO_TERMINATED,copt,&ec,&eo,cc);
    if(!re){printf("<%s> FAIL %d off=%zu\n",argv[i],ec,eo);pcre2_compile_context_free(cc);continue;}
    PCRE2_SIZE sz=0; pcre2_pattern_info(re,PCRE2_INFO_SIZE,&sz);
    printf("<%s> OK size=%zu\n",argv[i],sz);
    pcre2_code_free(re);pcre2_compile_context_free(cc);
  }
  return 0;
}
