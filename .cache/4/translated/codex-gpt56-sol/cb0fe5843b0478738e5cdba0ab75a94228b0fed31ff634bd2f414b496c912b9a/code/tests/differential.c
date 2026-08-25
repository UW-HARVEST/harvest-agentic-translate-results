#define _GNU_SOURCE

#include <dlfcn.h>
#include <regex.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct os_data {
    char *os_name;
    char *os_version;
    char *os_major;
    char *os_minor;
    char *os_codename;
    char *os_platform;
    char *os_build;
    char *os_uname;
    char *os_arch;
} os_data;

typedef char *(*get_os_arch_fn)(char *);
typedef int (*w_regexec_fn)(const char *, const char *, size_t, regmatch_t *);
typedef void (*parse_uname_string_fn)(char *, os_data *);

static void print_value(const char *name, const char *value) {
    size_t i;

    printf("%s=", name);
    if (!value) {
        puts("<null>");
        return;
    }

    for (i = 0; value[i]; ++i) {
        printf("%02x", (unsigned char)value[i]);
    }
    putchar('\n');
}

static void free_data(os_data *data) {
    free(data->os_name);
    free(data->os_version);
    free(data->os_major);
    free(data->os_minor);
    free(data->os_codename);
    free(data->os_platform);
    free(data->os_build);
    free(data->os_uname);
    free(data->os_arch);
}

static void test_arch(get_os_arch_fn get_os_arch) {
    char *cases[] = {
        "",
        "Linux host x86_64",
        "i686 and i386",
        "SunOS i86pc",
        "AIX powerpc",
        "Linux aarch64",
        "Linux arm64",
        "unknown",
    };
    size_t i;

    puts("[arch]");
    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        char *result = get_os_arch(cases[i]);
        printf("%zu:", i);
        print_value("result", result);
        free(result);
    }
}

static void test_regex(w_regexec_fn w_regexec) {
    struct regex_case {
        const char *pattern;
        const char *string;
        size_t nmatch;
    } cases[] = {
        {"^([0-9]+)\\.*", "12.34", 4},
        {"^[0-9]+\\.([0-9]+)\\.*", "12.34.56", 4},
        {"(foo)?bar", "bar", 4},
        {"a+", "xxaaayy", 1},
        {"a+", "bbb", 4},
        {"[", "anything", 4},
        {"^$", "", 0},
    };
    size_t i;

    puts("[regex]");
    printf("null-pattern=%d\n", w_regexec(NULL, "text", 0, NULL));
    printf("null-string=%d\n", w_regexec("text", NULL, 0, NULL));
    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        regmatch_t matches[4] = {
            {.rm_so = 101, .rm_eo = 102},
            {.rm_so = 103, .rm_eo = 104},
            {.rm_so = 105, .rm_eo = 106},
            {.rm_so = 107, .rm_eo = 108},
        };
        int result = w_regexec(
            cases[i].pattern,
            cases[i].string,
            cases[i].nmatch,
            cases[i].nmatch ? matches : NULL
        );
        size_t j;

        printf("%zu:result=%d", i, result);
        for (j = 0; j < 4; ++j) {
            printf(",%d:%d", matches[j].rm_so, matches[j].rm_eo);
        }
        putchar('\n');
    }
}

static void test_parse(parse_uname_string_fn parse_uname_string) {
    const char *cases[] = {
        "Linux host x86_64",
        "Unknown operating system",
        "Microsoft Windows 10 Pro [Ver: 10.0.19045.4046]",
        "Microsoft Windows [Ver: 6.1]",
        "Linux x86_64 [Ubuntu: 22.04 (Jammy Jellyfish)]",
        "Linux aarch64 [Ubuntu|linux: 24.04 (Noble)]",
        "FreeBSD amd64 [FreeBSD|freebsd]",
        "Darwin arm64 [macOS: 14.5]",
        "prefix [Name]",
    };
    size_t i;

    puts("[parse]");
    parse_uname_string(NULL, NULL);
    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        char *input = strdup(cases[i]);
        os_data data = {0};

        parse_uname_string(input, &data);
        printf("case=%zu\n", i);
        print_value("input", input);
        print_value("os_name", data.os_name);
        print_value("os_version", data.os_version);
        print_value("os_major", data.os_major);
        print_value("os_minor", data.os_minor);
        print_value("os_codename", data.os_codename);
        print_value("os_platform", data.os_platform);
        print_value("os_build", data.os_build);
        print_value("os_uname", data.os_uname);
        print_value("os_arch", data.os_arch);

        free_data(&data);
        free(input);
    }
}

int main(int argc, char **argv) {
    void *library;
    get_os_arch_fn get_os_arch;
    w_regexec_fn w_regexec;
    parse_uname_string_fn parse_uname_string;

    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    get_os_arch = (get_os_arch_fn)dlsym(library, "get_os_arch");
    w_regexec = (w_regexec_fn)dlsym(library, "w_regexec");
    parse_uname_string = (parse_uname_string_fn)dlsym(library, "parse_uname_string");
    if (!get_os_arch || !w_regexec || !parse_uname_string) {
        fputs("missing symbol\n", stderr);
        return 2;
    }

    test_arch(get_os_arch);
    test_regex(w_regexec);
    test_parse(parse_uname_string);
    dlclose(library);
    return 0;
}
