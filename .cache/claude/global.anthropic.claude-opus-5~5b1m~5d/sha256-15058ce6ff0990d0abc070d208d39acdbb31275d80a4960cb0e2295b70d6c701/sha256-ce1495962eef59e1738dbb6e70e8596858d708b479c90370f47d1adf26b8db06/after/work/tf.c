#include <stdio.h>
int main(void){ FILE* f = tmpfile(); printf("tmpfile=%p\n", (void*)f); if(f) fclose(f); return 0; }
