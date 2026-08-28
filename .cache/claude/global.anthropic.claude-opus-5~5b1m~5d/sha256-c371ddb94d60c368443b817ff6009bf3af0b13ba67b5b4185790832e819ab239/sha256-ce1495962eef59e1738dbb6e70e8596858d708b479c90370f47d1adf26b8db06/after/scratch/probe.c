#define _GNU_SOURCE
#include <stdio.h>
#include <stddef.h>
#include <stdint.h>
#include <dlfcn.h>
int main(void){
    printf("sizeof(wchar_t)=%zu signed=%d\n", sizeof(wchar_t), (int)((wchar_t)-1 < 0));
    void *h = dlopen("$HARVEST_WORKDIR/c_src/build/libharvest-work-4UINGg.so", RTLD_NOW|RTLD_LOCAL);
    if(!h){printf("dlopen fail %s\n", dlerror());return 1;}
    int (*f)(wchar_t*, size_t, const wchar_t*) = dlsym(h, "wcscat");
    Dl_info info; dladdr((void*)f, &info);
    printf("resolved from: %s\n", info.dli_fname);
    wchar_t buf[8]; for(int i=0;i<8;i++) buf[i]=0x41424344;
    buf[0]=0;
    wchar_t src[2]={'x',0};
    int r = f(buf, SIZE_MAX, src);
    printf("SIZE_MAX case ret=%d buf0=%d buf1=%x\n", r, (int)buf[0], (unsigned)buf[1]);
    for(int i=0;i<8;i++) buf[i]=0x41424344; buf[0]=0;
    r = f(buf, (size_t)1<<40, src);
    printf("1<<40 case ret=%d buf0=%d buf1=%x buf2=%x\n", r, (int)buf[0], (unsigned)buf[1], (unsigned)buf[2]);
    for(int i=0;i<8;i++) buf[i]=0x41424344; buf[0]=0;
    r = f(buf, SIZE_MAX/4, src);
    printf("SIZE_MAX/4 ret=%d buf0=%x buf1=%x\n", r, (unsigned)buf[0], (unsigned)buf[1]);
    return 0;
}
