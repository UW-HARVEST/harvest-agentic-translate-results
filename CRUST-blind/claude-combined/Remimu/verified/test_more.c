#include "my_regex.h"

void run(const char* pat, const char* txt) {
    RegexToken tokens[256];
    int16_t token_count = 256;
    int e = regex_parse(pat, tokens, &token_count, 0);
    if (e) { printf("'%s': ERR\n", pat); return; }
    int64_t r = regex_match(tokens, txt, 0, 0, 0, 0);
    printf("'%s' on '%s': %ld\n", pat, txt, (long)r);
}

int main(void) {
    run("a{3}", "aaaa");
    run("a{3}", "aa");
    run("a{2,4}", "aaaaaa");
    run("a{2,4}?", "aaaaaa");
    run("a*", "aaaa");
    run("a*?", "aaaa");
    run("a+", "");
    run("a+", "a");
    run("a*", "");
    run("[a-z]+", "abcXYZ");
    run("[A-Z]+", "abcXYZ");
    run("[^a-z]+", "abcXYZ");
    run("ab|cd", "abef");
    run("ab|cd", "cdef");
    run("ab|cd", "xyz");
    run("(ab)*", "ababab");
    run("(ab)*?", "ababab");
    run("(ab)+", "");
    run("(.*),(.*),(.*)", "a,b,c");
    run("\\d+", "12345");
    run("\\D+", "abcde");
    run("\\W+", "!!!");
    run("\\S+", "hello");
    run("a.b", "a\nb");
    run("a.b", "axb");
    run("\\\\", "\\");
    run("\\.", ".");
    return 0;
}
