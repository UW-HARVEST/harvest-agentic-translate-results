#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
int main(void){
  const char *pats[]={"[\\p{L}x]","[\\p{L}]","[\\p{L}\\p{N}]","[\\x{100}a]","[\\p{Lu}a]",NULL};
  const char *subs[]={"a","1","x","A",NULL};
  for(int i=0;pats[i];i++){
    int ec; PCRE2_SIZE eo;
    pcre2_code *re=pcre2_compile((PCRE2_SPTR)pats[i],PCRE2_ZERO_TERMINATED,PCRE2_UTF|PCRE2_UCP,&ec,&eo,NULL);
    printf("<%s>",pats[i]); fflush(stdout);
    if(!re){printf(" FAIL %d\n",ec);continue;}
    pcre2_match_data *md=pcre2_match_data_create_from_pattern(re,NULL);
    for(int j=0;subs[j];j++) printf(" %s=%d",subs[j],
      pcre2_match(re,(PCRE2_SPTR)subs[j],strlen(subs[j]),0,0,md,NULL));
    printf("\n");fflush(stdout);
    pcre2_match_data_free(md);pcre2_code_free(re);
  }
  return 0;
}
