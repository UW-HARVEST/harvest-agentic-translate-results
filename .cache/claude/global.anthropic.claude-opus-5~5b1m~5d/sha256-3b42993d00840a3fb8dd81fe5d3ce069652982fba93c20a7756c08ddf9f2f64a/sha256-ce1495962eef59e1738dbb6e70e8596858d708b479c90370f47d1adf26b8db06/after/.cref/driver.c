#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct CF { unsigned int v:1, d:1, o:1, c:1, l:3, r:1; };

extern int envy(int,int,int,int);
extern int parse_env_numeric(const char*, int);
extern void init_config_from_env(struct CF*);
extern int perform_operation(int,int,struct CF*);
extern int apply_bit_operations(int,struct CF*);

int main(void) {
    static const int vals[] = {0,1,-1,2,3,7,-7,10,100,-100,255,-256,1000,-1000,
                               65535,-65536,123456789,-123456789,
                               2147483647,-2147483648,4,-4,5,-5,64,-64,15,-15};
    const int n = (int)(sizeof(vals)/sizeof(vals[0]));

    /* envy over a grid of inputs */
    for (int a = 0; a < n; a++)
      for (int b = 0; b < n; b++) {
        int c = vals[(a+b) % n], d = vals[(a*3+b*7) % n];
        printf("envy(%d,%d,%d,%d)=%d\n", vals[a], vals[b], c, d,
               envy(vals[a], vals[b], c, d));
      }

    /* direct calls into the other exported entry points */
    for (unsigned raw = 0; raw < 256u; raw++) {
        for (int i = 0; i < n; i++) {
            union { struct CF f; unsigned char b[4]; } u;
            memset(&u, 0, sizeof(u));
            u.b[0] = (unsigned char)raw;
            printf("po(%d,%d,%u)=%d\n", vals[i], vals[(i+1)%n], raw,
                   perform_operation(vals[i], vals[(i+1)%n], &u.f));
            printf("abo(%d,%u)=%d\n", vals[i], raw,
                   apply_bit_operations(vals[i], &u.f));
            printf("  raw-after=%02x%02x%02x%02x\n",
                   u.b[0], u.b[1], u.b[2], u.b[3]);
        }
    }

    /* init_config_from_env writes only byte 0; check it leaves the rest alone */
    for (unsigned fill = 0; fill < 256u; fill += 37u) {
        union { struct CF f; unsigned char b[4]; } u;
        memset(&u, (int)fill, sizeof(u));
        init_config_from_env(&u.f);
        printf("icfe(fill=%u)=%02x%02x%02x%02x v=%u d=%u o=%u c=%u l=%u r=%u\n",
               fill, u.b[0], u.b[1], u.b[2], u.b[3],
               u.f.v, u.f.d, u.f.o, u.f.c, u.f.l, u.f.r);
    }

    /* parse_env_numeric against a variety of names */
    static const char* names[] = {
        "PROG_BASE_OFFSET", "PROG_MULTIPLIER", "PROG_VERBOSE", "PROG_DEBUG",
        "PROG_OPTIMIZE", "PROG_X_COMMA", "PROG_X_SEMI", "PROG_X_BOTH",
        "PROG_X_JUNK", "PROG_X_EMPTY", "PROG_X_BIG", "PROG_X_NEG",
        "PROG_X_SPACE", "PROG_X_HEX", "PROG_X_ABSENT_XYZ", "PATH"
    };
    for (int i = 0; i < (int)(sizeof(names)/sizeof(names[0])); i++) {
        printf("pen(%s,%d)=%d\n", names[i], -12345,
               parse_env_numeric(names[i], -12345));
        printf("pen(%s,%d)=%d\n", names[i], 0,
               parse_env_numeric(names[i], 0));
    }
    fflush(NULL);
    return 0;
}
