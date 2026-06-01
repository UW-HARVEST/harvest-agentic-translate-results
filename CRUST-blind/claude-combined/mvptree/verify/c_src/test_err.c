#include <stdio.h>
#include "mvptree.h"
int main() {
    for (int i = 0; i <= 24; i++) {
        printf("%d: %s\n", i, mvp_errstr((MVPError)i));
    }
    return 0;
}
