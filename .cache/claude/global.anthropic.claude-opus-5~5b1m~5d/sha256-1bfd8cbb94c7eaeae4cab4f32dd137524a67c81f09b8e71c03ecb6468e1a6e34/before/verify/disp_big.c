/* Exercises the clock-gated ZDICT_trainBuffer_legacy DISPLAYUPDATE path with a
 * corpus large enough that training exceeds the 0.3s refresh rate. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#define ZDICT_STATIC_LINKING_ONLY
#include "zdict.h"
static unsigned long long st = 88172645463325252ULL;
static unsigned r32(void){ st^=st<<13; st^=st>>7; st^=st<<17; return (unsigned)(st>>32); }
#define NB 4096
#define SS 2048
int main(void){
    static unsigned char buf[NB*SS];
    static size_t sz[NB];
    static unsigned char dict[16384];
    static const char* w[]={"the ","quick ","brown ","fox ","jumps ","over ","lazy ","dog ",
                            "zstandard ","compression ","library ","test ","data "};
    size_t i=0,k; ZDICT_legacy_params_t p; size_t r;
    while(i<sizeof(buf)){ const char*s=w[r32()%13]; size_t l=strlen(s);
        if(i+l>sizeof(buf))break; memcpy(buf+i,s,l); i+=l; }
    while(i<sizeof(buf)) buf[i++]=' ';
    for(k=0;k<NB;k++) sz[k]=SS;
    memset(&p,0,sizeof(p)); p.zParams.notificationLevel=2; p.zParams.compressionLevel=3;
    r = ZDICT_trainFromBuffer_legacy(dict,sizeof(dict),buf,sz,NB,p);
    fflush(stderr);
    printf("legacy=%zu err=%d\n", r, (int)ZDICT_isError(r));
    return 0;
}
