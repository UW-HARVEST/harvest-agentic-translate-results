#include <stdio.h>
struct ConfigFlags { unsigned int verbose:1, debug:1, optimize:1, cache_enabled:1, log_level:3, reserved:1; };
struct ProcessState { struct ConfigFlags flags; int base_value; int multiplier; char operation; };
int main(void){
  printf("C  ConfigFlags  size=%zu align=%zu\n", sizeof(struct ConfigFlags), _Alignof(struct ConfigFlags));
  printf("C  ProcessState size=%zu align=%zu\n", sizeof(struct ProcessState), _Alignof(struct ProcessState));
  printf("C  offsets: flags=%zu base=%zu mult=%zu op=%zu\n",
    __builtin_offsetof(struct ProcessState,flags), __builtin_offsetof(struct ProcessState,base_value),
    __builtin_offsetof(struct ProcessState,multiplier), __builtin_offsetof(struct ProcessState,operation));
  return 0; }
