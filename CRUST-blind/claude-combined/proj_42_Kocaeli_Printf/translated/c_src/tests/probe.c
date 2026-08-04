#include "../src/ft_printf.h"
#include <stdio.h>
#include <stdlib.h>

static void measure(const char *label, int (*fn)(int), int n) {
    int len = 0;
    int r = fn(n);
    printf("\n%s: ret=? len=? -- expected len: \n", label);
}

int main(void) {
    int len;

    len = 0; printf("writechar('A')=%d ", writechar('A', &len)); printf("len=%d\n", len);
    len = 0; printf("writestring(\"Hello\")=%d ", writestring("Hello", &len)); printf("len=%d\n", len);
    len = 0; printf("writestring(\"\")=%d ", writestring("", &len)); printf("len=%d\n", len);
    len = 0; printf("writestring(NULL)=%d ", writestring(NULL, &len)); printf("len=%d\n", len);
    len = 0; printf("writestring(\"abc\\n\\t\")=%d ", writestring("abc\n\t", &len)); printf("len=%d\n", len);

    len = 0; printf("writeint(42)=%d ", writeint(42, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(-42)=%d ", writeint(-42, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(0)=%d ", writeint(0, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(-2147483648)=%d ", writeint(-2147483648, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(2147483647)=%d ", writeint(2147483647, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(1)=%d ", writeint(1, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(-1)=%d ", writeint(-1, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(10)=%d ", writeint(10, &len)); printf("len=%d\n", len);
    len = 0; printf("writeint(99999)=%d ", writeint(99999, &len)); printf("len=%d\n", len);

    len = 0; printf("writeuint(42)=%d ", writeuint(42, &len)); printf("len=%d\n", len);
    len = 0; printf("writeuint(0)=%d ", writeuint(0, &len)); printf("len=%d\n", len);
    len = 0; printf("writeuint(4294967295)=%d ", writeuint(4294967295UL, &len)); printf("len=%d\n", len);
    len = 0; printf("writeuint(100)=%d ", writeuint(100, &len)); printf("len=%d\n", len);

    len = 0; printf("writehex(42,x)=%d ", writehex(42, 'x', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(42,X)=%d ", writehex(42, 'X', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(0,x)=%d ", writehex(0, 'x', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(0,X)=%d ", writehex(0, 'X', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(255,x)=%d ", writehex(255, 'x', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(255,X)=%d ", writehex(255, 'X', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(4096,x)=%d ", writehex(4096, 'x', &len)); printf("len=%d\n", len);
    len = 0; printf("writehex(0xdeadbeef,x)=%d ", writehex(0xdeadbeefUL, 'x', &len)); printf("len=%d\n", len);

    len = 0; printf("writepoint(0x1234)=%d ", writepoint((void*)0x1234, &len)); printf("len=%d\n", len);
    len = 0; printf("writepoint(NULL)=%d ", writepoint(NULL, &len)); printf("len=%d\n", len);
    len = 0; printf("writepoint(0xff)=%d ", writepoint((void*)0xff, &len)); printf("len=%d\n", len);

    return 0;
}
