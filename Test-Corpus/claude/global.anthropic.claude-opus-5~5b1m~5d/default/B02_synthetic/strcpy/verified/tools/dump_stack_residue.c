#include <stdio.h>
static unsigned char save[0x1400];
int main(void) {
    volatile unsigned char *fp = (volatile unsigned char *)__builtin_frame_address(0);
    /* copy [fp-0x1000, fp+0x400) into BSS without calling any function */
    unsigned long i = 0;
    while (i < 0x1400) { save[i] = fp[(long)i - 0x1000L]; i++; }
    printf("BASE 0x1000\n");
    for (unsigned long j = 0; j < 0x1400; j++) printf("%02x", save[j]);
    printf("\n");
    return 0;
}
