#include <stdlib.h>

void c_div(int x, int y, int *quot, int *rem) {
    div_t result = div(x, y);
    *quot = result.quot;
    *rem = result.rem;
}
