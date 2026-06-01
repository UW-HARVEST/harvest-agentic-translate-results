#include "my_regex.h"

int main(void)
{
    {
        RegexToken tokens[256];
        int16_t token_count = 256;
        int e = regex_parse("(a)(b)(c)", tokens, &token_count, 0);
        if (e) return (puts("err"), 1);
        int64_t cap_pos[16], cap_span[16];
        memset(cap_pos, 0xFF, sizeof(cap_pos));
        memset(cap_span, 0xFF, sizeof(cap_span));
        int64_t m = regex_match(tokens, "abc", 0, 16, cap_pos, cap_span);
        printf("(a)(b)(c) on 'abc': len=%ld\n", (long)m);
        for (int i = 0; i < 5; i++) printf("  cap %d: pos=%ld span=%ld\n", i, (long)cap_pos[i], (long)cap_span[i]);
    }
    {
        RegexToken tokens[256];
        int16_t token_count = 256;
        int e = regex_parse("(\\w+)\\s(\\w+)", tokens, &token_count, 0);
        if (e) return (puts("err"), 1);
        int64_t cap_pos[16], cap_span[16];
        memset(cap_pos, 0xFF, sizeof(cap_pos));
        memset(cap_span, 0xFF, sizeof(cap_span));
        int64_t m = regex_match(tokens, "hello world", 0, 16, cap_pos, cap_span);
        printf("(\\w+)\\s(\\w+) on 'hello world': len=%ld\n", (long)m);
        for (int i = 0; i < 4; i++) printf("  cap %d: pos=%ld span=%ld\n", i, (long)cap_pos[i], (long)cap_span[i]);
    }
    {
        RegexToken tokens[256];
        int16_t token_count = 256;
        int e = regex_parse("(a|b)(c|d)", tokens, &token_count, 0);
        if (e) return (puts("err"), 1);
        int64_t cap_pos[16], cap_span[16];
        memset(cap_pos, 0xFF, sizeof(cap_pos));
        memset(cap_span, 0xFF, sizeof(cap_span));
        int64_t m = regex_match(tokens, "ad", 0, 16, cap_pos, cap_span);
        printf("(a|b)(c|d) on 'ad': len=%ld\n", (long)m);
        for (int i = 0; i < 4; i++) printf("  cap %d: pos=%ld span=%ld\n", i, (long)cap_pos[i], (long)cap_span[i]);
    }
    {
        RegexToken tokens[256];
        int16_t token_count = 256;
        int e = regex_parse("(a)+", tokens, &token_count, 0);
        if (e) return (puts("err"), 1);
        int64_t cap_pos[16], cap_span[16];
        memset(cap_pos, 0xFF, sizeof(cap_pos));
        memset(cap_span, 0xFF, sizeof(cap_span));
        int64_t m = regex_match(tokens, "aaaa", 0, 16, cap_pos, cap_span);
        printf("(a)+ on 'aaaa': len=%ld\n", (long)m);
        for (int i = 0; i < 3; i++) printf("  cap %d: pos=%ld span=%ld\n", i, (long)cap_pos[i], (long)cap_span[i]);
    }
    return 0;
}
