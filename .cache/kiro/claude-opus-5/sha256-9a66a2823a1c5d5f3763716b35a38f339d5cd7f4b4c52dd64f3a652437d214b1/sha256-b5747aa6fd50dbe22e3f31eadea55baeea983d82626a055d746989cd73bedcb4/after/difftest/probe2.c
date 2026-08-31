#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"
#include <stdio.h>
#include <string.h>
int main(int argc,char**argv){
  const char *pats[]={"[[:word:]]","[[:word:]-]","[a[:digit:]z]","[[:digit:]a]","[a[:digit:]]",
    "[[:alpha:]x]","[x[:alpha:]]","[[:word:]a]","[[:^word:]a]","[[:graph:]a]","[[:print:]a]",
    "[[:punct:]a]","[[:xdigit:]a]","[[:space:]a]","[[:upper:]a]",NULL};
  const char *subs[]={"a","1","-","z","x","A"," ",NULL};
  for(int i=0;pats[i];i++){ printf("try <%s>\n",pats[i]); fflush(stdout);
    int ec; PCRE2_SIZE eo;
    pcre2_code *re=pcre2_compile((PCRE2_SPTR)pats[i],PCRE2_ZERO_TERMINATED,
        PCRE2_UTF|PCRE2_UCP,&ec,&eo,NULL);
    if(!re){printf("<%s> FAIL %d\n",pats[i],ec);fflush(stdout);continue;}
    pcre2_match_data *md=pcre2_match_data_create_from_pattern(re,NULL);
    printf("<%s>",pats[i]);
    for(int j=0;subs[j];j++){
      int rc=pcre2_match(re,(PCRE2_SPTR)subs[j],strlen(subs[j]),0,0,md,NULL);
      printf(" %s=%d",subs[j],rc);
    }
    printf("\n");
    pcre2_match_data_free(md);pcre2_code_free(re);
  }
  return 0;
}
