#include "my_regex.h"

int main(void)
{
    RegexToken tokens[1024];
    int16_t token_count = 1024;
    int e = regex_parse(".", tokens, &token_count, 1);
    if (e) return (puts("regex has error"), 1);

    // test newline
    int64_t r = regex_match(tokens, "\n", 0, 0, 0, 0);
    printf("dot vs \\n with flag: %ld\n", (long)r);

    // test \r
    r = regex_match(tokens, "\r", 0, 0, 0, 0);
    printf("dot vs \\r with flag: %ld\n", (long)r);

    // test 'a'
    r = regex_match(tokens, "a", 0, 0, 0, 0);
    printf("dot vs 'a' with flag: %ld\n", (long)r);

    return 0;
}
