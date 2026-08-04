#include <stdlib.h>

typedef int cJSON_bool;

#ifdef true
#undef true
#endif
#define true ((cJSON_bool)1)

#ifdef false
#undef false
#endif
#define false ((cJSON_bool)0)

#define INT_MIN   (-__INT_MAX__  -1)
#define INT_MAX   __INT_MAX__

#define cJSON_Number (1 << 3)

typedef struct
{
    const unsigned char *content;
    size_t length;
    size_t offset;
    size_t depth; /* How deeply nested (in arrays/objects) is the input at the current offset. */
} parse_buffer;

typedef struct {
    /* The type of the item, as above. */
    int type;
    /* writing to valueint is DEPRECATED, use cJSON_SetNumberValue instead */
    int valueint;
    /* The item's number, if type==cJSON_Number */
    double valuedouble;
} cJSON;

/* Parse the input text to generate a number, and populate the result into item. */
cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
