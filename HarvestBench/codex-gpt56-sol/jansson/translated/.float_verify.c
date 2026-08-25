#include "jansson.h"
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    static const double values[] = {
        0.0, -0.0, 1.0, -1.0, 0.1, 0.0001, 0.00001, 1e-6, 1e-7,
        1e15, 1e16, 1e17, 1e20, 1e23, 1.2345678901234567,
        2.2250738585072014e-308, 4.9406564584124654e-324,
        1.7976931348623157e308, 9007199254740991.0,
        9007199254740992.0, 2.5000000000000000, 2.5000000000000004
    };
    static const int precisions[] = {0, 1, 2, 6, 10, 17};
    for (size_t i = 0; i < sizeof(values) / sizeof(values[0]); i++) {
        json_t *value = json_real(values[i]);
        for (size_t j = 0; j < sizeof(precisions) / sizeof(precisions[0]); j++) {
            size_t flags = JSON_ENCODE_ANY;
            if (precisions[j])
                flags |= JSON_REAL_PRECISION(precisions[j]);
            char *text = json_dumps(value, flags);
            printf("%zu/%d=%s\n", i, precisions[j], text ? text : "<null>");
            free(text);
        }
        json_decref(value);
    }
    return 0;
}
