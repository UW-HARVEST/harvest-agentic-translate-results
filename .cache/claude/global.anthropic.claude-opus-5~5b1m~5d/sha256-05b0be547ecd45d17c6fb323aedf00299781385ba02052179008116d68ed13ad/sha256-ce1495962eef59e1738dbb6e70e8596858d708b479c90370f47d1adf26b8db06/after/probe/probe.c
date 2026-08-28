#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/prctl.h>
#include <sys/resource.h>
int main(void){
  void *h = dlopen("./target/debug/libprobe.so", RTLD_NOW);
  if(!h){ printf("dlopen: %s\n", dlerror()); return 1; }
  const char *names[] = {"a_place","b_ptr_write","c_volatile","d_copy","e_read_place","f_read_ptr","g_read_vol","h_read_copy"};
  for(int i=0;i<8;i++){
    void (*f)(void*) = dlsym(h, names[i]);
    if(!f){ printf("%-14s MISSING\n", names[i]); continue; }
    pid_t p = fork();
    if(p==0){ struct rlimit rl={0,0}; setrlimit(RLIMIT_CORE,&rl); prctl(PR_SET_DUMPABLE,0,0,0,0); f(NULL); _exit(0); }
    int st; waitpid(p,&st,0);
    int sig = st & 0x7f;
    printf("%-14s -> %s%d\n", names[i], sig?"signal ":"exit ", sig?sig:(st>>8)&0xff);
  }
  return 0;
}
