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
    printf("%s=%s\n", name, value ? value : "(null)");
}

static void print_bytes(const char *name, const char *value, size_t size) {
    printf("%s=", name);
    for (size_t i = 0; i < size; ++i) {
        printf("%02x", (unsigned char)value[i]);
    }
    putchar('\n');
}

static void free_os_data(os_data *osd) {
    free(osd->os_name);
    free(osd->os_version);
    free(osd->os_major);
    free(osd->os_minor);
    free(osd->os_codename);
    free(osd->os_platform);
    free(osd->os_build);
    free(osd->os_uname);
    free(osd->os_arch);
}

static void run_parse(parse_uname_string_fn parse, const char *input) {
    size_t input_size = strlen(input) + 1;
    char *mutable_input = strdup(input);
    os_data osd = {0};

    parse(mutable_input, &osd);
    printf("parse=%s\n", input);
    print_value("mutated", mutable_input);
    print_bytes("mutated-bytes", mutable_input, input_size);
    print_value("name", osd.os_name);
    print_value("version", osd.os_version);
    print_value("major", osd.os_major);
    print_value("minor", osd.os_minor);
    print_value("codename", osd.os_codename);
    print_value("platform", osd.os_platform);
    print_value("build", osd.os_build);
    print_value("uname", osd.os_uname);
    print_value("arch", osd.os_arch);
    puts("--");

    free_os_data(&osd);
    free(mutable_input);
}

static void run_arch(get_os_arch_fn get_arch, const char *input) {
    char *mutable_input = strdup(input);
    char *result = get_arch(mutable_input);

    printf("arch(%s)=%s\n", input, result ? result : "(null)");
    free(result);
    free(mutable_input);
}

static void run_regex(
    w_regexec_fn regex,
    const char *pattern,
    const char *input,
    size_t nmatch
) {
    regmatch_t matches[4] = {
        {.rm_so = -7, .rm_eo = -8},
        {.rm_so = -7, .rm_eo = -8},
        {.rm_so = -7, .rm_eo = -8},
        {.rm_so = -7, .rm_eo = -8},
    };
    int result = regex(pattern, input, nmatch, matches);

    printf("regex(%s,%s,%zu)=%d", pattern, input, nmatch, result);
    for (size_t i = 0; i < nmatch; ++i) {
        printf(" [%d,%d]", matches[i].rm_so, matches[i].rm_eo);
    }
    putchar('\n');
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!library) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    get_os_arch_fn get_arch = (get_os_arch_fn)dlsym(library, "get_os_arch");
    w_regexec_fn regex = (w_regexec_fn)dlsym(library, "w_regexec");
    parse_uname_string_fn parse =
        (parse_uname_string_fn)dlsym(library, "parse_uname_string");
    if (!get_arch || !regex || !parse) {
        fprintf(stderr, "dlsym: %s\n", dlerror());
        return 2;
    }

    run_arch(get_arch, "x86_64 amd64");
    run_arch(get_arch, "aarch64 arm64");
    run_arch(get_arch, "unknown");

    run_regex(regex, "^([0-9]+)\\.([a-z]+)$", "123.alpha", 4);
    run_regex(regex, "a(b*)c", "xxabbbczz", 2);
    run_regex(regex, "^z+$", "aaaa", 1);
    printf("regex-null-pattern=%d\n", regex(NULL, "x", 0, NULL));
    printf("regex-null-string=%d\n", regex("x", NULL, 0, NULL));
    printf("regex-invalid=%d\n", regex("[", "x", 0, NULL));

    run_parse(parse, "Windows 11 Pro [Ver: 10.0.22631.3155]");
    run_parse(parse, "Linux x86_64 [Ubuntu: 22.04 (Jammy Jellyfish)]");
    run_parse(parse, "Linux arm64 [FreeBSD]");
    run_parse(parse, "Linux i686 [Solaris|sunos: 11.4]");
    run_parse(parse, "Linux aarch64 kernel");
    run_parse(parse, "Darwin [macOS: 14]");
    parse(NULL, NULL);

    dlclose(library);
    return 0;
}
