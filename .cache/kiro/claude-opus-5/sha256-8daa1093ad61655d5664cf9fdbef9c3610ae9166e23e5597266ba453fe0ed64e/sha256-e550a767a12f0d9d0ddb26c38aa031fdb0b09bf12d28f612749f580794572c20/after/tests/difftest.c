/* Differential test driver: compiled twice, once against the C libjansson.so
   and once against the Rust one; the stdout of both runs must be identical. */
#define _GNU_SOURCE
#include <jansson.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <errno.h>
#include <unistd.h>
#include <stddef.h>
#include <fcntl.h>

/* private / internal symbols the shared object also exports */
extern char *dtoa_r(double dd, int mode, int ndigits, int *decpt, int *sign, char **rve,
                    char *buf, size_t blen);
extern char *dtoa(double dd, int mode, int ndigits, int *decpt, int *sign, char **rve);
extern void freedtoa(char *s);
extern int dtoa_divmax;

extern int utf8_encode(int32_t codepoint, char *buffer, size_t *size);
extern size_t utf8_check_first(char byte);
extern size_t utf8_check_full(const char *buffer, size_t size, int32_t *codepoint);
extern const char *utf8_iterate(const char *buffer, size_t size, int32_t *codepoint);
extern int utf8_check_string(const char *string, size_t length);

typedef struct {
    char *value;
    size_t length;
    size_t size;
} strbuffer_t;
extern int strbuffer_init(strbuffer_t *);
extern void strbuffer_close(strbuffer_t *);
extern void strbuffer_clear(strbuffer_t *);
extern const char *strbuffer_value(const strbuffer_t *);
extern char *strbuffer_steal_value(strbuffer_t *);
extern int strbuffer_append_byte(strbuffer_t *, char);
extern int strbuffer_append_bytes(strbuffer_t *, const char *, size_t);
extern char strbuffer_pop(strbuffer_t *);

extern int jsonp_dtostr(char *buffer, size_t size, double value, int prec);
extern int jsonp_strtod(strbuffer_t *strbuffer, double *out);
extern char *jsonp_strndup(const char *, size_t);
extern void *jsonp_malloc(size_t);
extern void jsonp_free(void *);
extern void *jsonp_realloc(void *, size_t, size_t);
extern json_t *jsonp_stringn_nocheck_own(const char *value, size_t len);
extern void jsonp_error_init(json_error_t *error, const char *source);
extern void jsonp_error_set_source(json_error_t *error, const char *source);
extern void jsonp_error_set(json_error_t *error, int line, int column, size_t position,
                            enum json_error_code code, const char *msg, ...);

struct hashtable_list {
    struct hashtable_list *prev;
    struct hashtable_list *next;
};
typedef struct hashtable {
    size_t size;
    struct hashtable_bucket *buckets;
    size_t order;
    struct hashtable_list list;
    struct hashtable_list ordered_list;
} hashtable_t;
extern int hashtable_init(hashtable_t *);
extern void hashtable_close(hashtable_t *);
extern int hashtable_set(hashtable_t *, const char *, size_t, json_t *);
extern void *hashtable_get(hashtable_t *, const char *, size_t);
extern int hashtable_del(hashtable_t *, const char *, size_t);
extern void hashtable_clear(hashtable_t *);
extern void *hashtable_iter(hashtable_t *);
extern void *hashtable_iter_at(hashtable_t *, const char *, size_t);
extern void *hashtable_iter_next(hashtable_t *, void *);
extern void *hashtable_iter_key(void *);
extern size_t hashtable_iter_key_len(void *);
extern void *hashtable_iter_value(void *);
extern void hashtable_iter_set(void *, json_t *);

extern json_t *do_deep_copy(const json_t *json, hashtable_t *parents);
extern int do_object_update_recursive(json_t *object, json_t *other, hashtable_t *parents);
extern int jsonp_loop_check(hashtable_t *parents, const json_t *json, char *key,
                            size_t key_size, size_t *key_len_out);

#define P(...) printf(__VA_ARGS__)

static void show(const char *tag, json_t *j, size_t flags) {
    char *s = json_dumps(j, flags);
    P("%s|%s\n", tag, s ? s : "(null)");
    free(s);
}

static void show_err(const char *tag, json_error_t *e) {
    /* text[JSON_ERROR_TEXT_LENGTH-1] (the code byte) is only initialised when a
       message was stored, so only report it then. */
    P("%s|line=%d col=%d pos=%d src=%s code=%d text=%s\n", tag, e->line, e->column,
      e->position, e->source, e->text[0] ? (int)json_error_code(e) : -1, e->text);
}

/* ---------------------------------------------------------------- */

static const char *load_inputs[] = {
    "{}",
    "[]",
    "[1,2,3]",
    "{\"a\":1,\"b\":[true,false,null],\"c\":{\"d\":\"e\"}}",
    "  [ 1 , 2 ]  ",
    "[1.5,2.25,1e10,1e-10,1e308,1e-308,-0.0,0.0]",
    "[123456789012345678,-123456789012345678]",
    "[9223372036854775807,-9223372036854775808]",
    "[9223372036854775808]",
    "[-9223372036854775809]",
    "[1e400]",
    "[1e-400]",
    "[\"\\u0041\\u00e9\\u4e2d\\ud83d\\ude00\"]",
    "[\"\\/\\\\\\\"\\b\\f\\n\\r\\t\"]",
    "[\"\\u0000\"]",
    "[\"tab\\there\"]",
    "{\"dup\":1,\"dup\":2}",
    "[1,]",
    "[,1]",
    "{,}",
    "{\"a\"}",
    "{\"a\":}",
    "{\"a\" 1}",
    "[1 2]",
    "[",
    "]",
    "{",
    "}",
    "",
    " ",
    "nul",
    "tru",
    "fals",
    "truex",
    "1",
    "1.0",
    "\"str\"",
    "null",
    "true",
    "false",
    "[01]",
    "[1.]",
    "[.1]",
    "[1e]",
    "[1e+]",
    "[--1]",
    "[+1]",
    "[0x10]",
    "[\"unterminated",
    "[\"bad\\escape\"]",
    "[\"bad\\u12\"]",
    "[\"\\ud834\"]",
    "[\"\\ud834\\u0041\"]",
    "[\"\\udd1e\"]",
    "[\"\x80\"]",
    "[\"\xc3\"]",
    "[\"\xc3\x28\"]",
    "[\"\xed\xa0\x80\"]",
    "[\"\xf5\x80\x80\x80\"]",
    "\x80",
    "[[[[[[[[[[1]]]]]]]]]]",
    "{\"a\":{\"b\":{\"c\":{\"d\":[1,2,{\"e\":null}]}}}}",
    "[1,2,3]garbage",
    "{\"key with \\u00e9\":\"value\"}",
    NULL,
};

static size_t load_flags[] = {
    0,
    JSON_REJECT_DUPLICATES,
    JSON_DISABLE_EOF_CHECK,
    JSON_DECODE_ANY,
    JSON_DECODE_INT_AS_REAL,
    JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_ALLOW_NUL,
    JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
};

static size_t dump_flags[] = {
    0,
    JSON_COMPACT,
    JSON_INDENT(2),
    JSON_INDENT(4) | JSON_SORT_KEYS,
    JSON_ENSURE_ASCII,
    JSON_ESCAPE_SLASH,
    JSON_SORT_KEYS | JSON_COMPACT,
    JSON_ENCODE_ANY,
    JSON_ENCODE_ANY | JSON_INDENT(1),
    JSON_INDENT(31),
    JSON_REAL_PRECISION(1),
    JSON_REAL_PRECISION(4),
    JSON_REAL_PRECISION(17),
    JSON_ENCODE_ANY | JSON_REAL_PRECISION(6),
};

static void test_load_dump(void) {
    P("== load/dump ==\n");
    for (int i = 0; load_inputs[i]; i++) {
        for (size_t f = 0; f < sizeof(load_flags) / sizeof(load_flags[0]); f++) {
            json_error_t err;
            json_t *j = json_loads(load_inputs[i], load_flags[f], &err);
            P("in[%d] lf=%zu -> %s\n", i, load_flags[f], j ? "ok" : "fail");
            show_err("  err", &err);
            if (j) {
                for (size_t d = 0; d < sizeof(dump_flags) / sizeof(dump_flags[0]); d++) {
                    char *s = json_dumps(j, dump_flags[d]);
                    P("  df=%zu|%s\n", dump_flags[d], s ? s : "(null)");
                    free(s);
                }
                /* json_dumpb */
                char buf[512];
                memset(buf, 0, sizeof(buf));
                size_t n = json_dumpb(j, buf, sizeof(buf), JSON_COMPACT);
                P("  dumpb=%zu|%.*s\n", n, (int)(n < sizeof(buf) ? n : sizeof(buf)), buf);
                size_t n2 = json_dumpb(j, buf, 3, JSON_COMPACT);
                P("  dumpb3=%zu\n", n2);
                json_decref(j);
            }
        }
        /* json_loadb with explicit length */
        json_error_t err2;
        json_t *jb = json_loadb(load_inputs[i], strlen(load_inputs[i]), 0, &err2);
        P("in[%d] loadb -> %s\n", i, jb ? "ok" : "fail");
        show_err("  errb", &err2);
        json_decref(jb);
    }
}

/* ---------------------------------------------------------------- */

static double doubles[] = {
    0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 0.2, 0.3, 1.0 / 3.0, 2.0 / 3.0,
    1e-1, 1e-2, 1e-3, 1e-4, 1e-5, 1e-10, 1e-20, 1e-100, 1e-300, 1e-308,
    5e-324, 1e1, 1e2, 1e3, 1e4, 1e15, 1e16, 1e17, 1e20, 1e21,
    1e22, 1e23, 1e100, 1e300, 1.7976931348623157e308, 2.2250738585072014e-308,
    3.141592653589793, 2.718281828459045, 1.4142135623730951,
    123456789.0, 1234567890123456.0, 12345678901234567.0,
    9007199254740992.0, 9007199254740993.0, 4503599627370496.0,
    0.30000000000000004, 9.862818194192001e18, 1.2e-307, 2.5, 3.5, -2.5,
    1e-323, 4.9e-324, 1.5e-323, 8.98846567431158e307, 1.1125369292536007e-308,
    100.0, 1000.0, 0.0001, 0.00001, 1e-7, 123.456, -123.456,
    1e-6, 2e-6, 1234.5678e-30, 7.8e-5, 1e6, 1e7, 999999999999999.0,
    1000000000000000.0, 10000000000000000.0, 1e-4, 9.999999999999999e22,
};

static void test_reals(void) {
    P("== reals ==\n");
    int n = (int)(sizeof(doubles) / sizeof(doubles[0]));
    for (int i = 0; i < n; i++) {
        double d = doubles[i];
        for (int prec = 0; prec <= 17; prec++) {
            char buf[64];
            int r = jsonp_dtostr(buf, sizeof(buf), d, prec);
            P("dtostr[%d,%d]=%d|%s\n", i, prec, r, r >= 0 ? buf : "");
        }
        /* small buffers to exercise the failure paths */
        for (size_t sz = 1; sz <= 26; sz++) {
            char buf[64];
            memset(buf, 0, sizeof buf);
            int r = jsonp_dtostr(buf, sz, d, 0);
            P("dtostr_sz[%d,%zu]=%d|%s\n", i, sz, r, r >= 0 ? buf : "");
        }
        json_t *j = json_real(d);
        if (j) {
            json_t *a = json_array();
            json_array_append_new(a, j);
            for (size_t f = 0; f < sizeof(dump_flags) / sizeof(dump_flags[0]); f++) {
                char *s = json_dumps(a, dump_flags[f]);
                P("real[%d] df=%zu|%s\n", i, dump_flags[f], s ? s : "(null)");
                free(s);
            }
            json_decref(a);
        } else {
            P("real[%d]|null\n", i);
        }
        /* dtoa_r over all modes and digit counts */
        for (int mode = -1; mode <= 10; mode++) {
            for (int nd = -2; nd <= 20; nd++) {
                char db[40];
                int decpt = 0, sgn = 0;
                char *rve = NULL;
                char *r = dtoa_r(d, mode, nd, &decpt, &sgn, &rve, db, sizeof(db));
                P("dtoa_r[%d,%d,%d]=%s decpt=%d sign=%d rvelen=%td\n", i, mode, nd,
                  r ? r : "(null)", decpt, sgn, r && rve ? rve - r : (ptrdiff_t)-1);
            }
        }
        /* dtoa (allocating variant) */
        {
            int decpt = 0, sgn = 0;
            char *rve = NULL;
            char *r = dtoa(d, 0, 0, &decpt, &sgn, &rve);
            P("dtoa[%d]=%s decpt=%d sign=%d\n", i, r ? r : "(null)", decpt, sgn);
            if (r)
                freedtoa(r);
        }
    }
    /* special values */
    double specials[3];
    specials[0] = INFINITY;
    specials[1] = -INFINITY;
    specials[2] = NAN;
    for (int i = 0; i < 3; i++) {
        char db[40];
        int decpt = 0, sgn = 0;
        char *rve = NULL;
        char *r = dtoa_r(specials[i], 0, 0, &decpt, &sgn, &rve, db, sizeof(db));
        P("dtoa_spec[%d]=%s decpt=%d sign=%d\n", i, r ? r : "(null)", decpt, sgn);
        P("json_real_spec[%d]=%d\n", i, json_real(specials[i]) != NULL);
    }
    /* exhaustive-ish bit patterns */
    P("== real bit patterns ==\n");
    uint64_t seed = 88172645463325252ULL;
    for (int i = 0; i < 4000; i++) {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        double d;
        memcpy(&d, &seed, 8);
        if (isnan(d) || isinf(d))
            continue;
        char buf[64];
        int r = jsonp_dtostr(buf, sizeof(buf), d, 0);
        P("bp[%d]=%d|%s\n", i, r, r >= 0 ? buf : "");
        for (int prec = 1; prec <= 17; prec += 4) {
            int r2 = jsonp_dtostr(buf, sizeof(buf), d, prec);
            P("bp[%d,%d]=%d|%s\n", i, prec, r2, r2 >= 0 ? buf : "");
        }
    }
    /* small integers and simple decimals via round trip */
    P("== round trip ==\n");
    for (int i = -1000; i <= 1000; i += 7) {
        double d = i / 8.0;
        char buf[64];
        int r = jsonp_dtostr(buf, sizeof(buf), d, 0);
        P("rt[%d]=%d|%s\n", i, r, buf);
    }
}

/* ---------------------------------------------------------------- */

static void test_strtod(void) {
    P("== jsonp_strtod ==\n");
    static const char *nums[] = {
        "0",      "1",     "-1",      "1.5",     "1e10",   "1e-10",   "1e308",
        "1e309",  "1e-323", "1e-324",  "-1e309",  "0.0001", "123456789012345678901234567890",
        "1.7976931348623157e308", "2.2250738585072014e-308", "4.9406564584124654e-324",
        "0.1", "0.2", "0.3", "3.141592653589793", "9.862818194192001e18",
        "1e-500", "1e500", "9007199254740993", "1e22", "1e23", NULL};
    for (int i = 0; nums[i]; i++) {
        strbuffer_t sb;
        if (strbuffer_init(&sb))
            continue;
        strbuffer_append_bytes(&sb, nums[i], strlen(nums[i]));
        double out = 0;
        int r = jsonp_strtod(&sb, &out);
        char b[64];
        int n = jsonp_dtostr(b, sizeof(b), r ? 0.0 : out, 0);
        P("strtod[%d]=%d|%s (%d)\n", i, r, n >= 0 ? b : "", n);
        strbuffer_close(&sb);
    }
}

/* ---------------------------------------------------------------- */

extern double strtod__unused(const char *s00, char **se);
extern void gethex(const char **sp, void *rvp, int rounding, int sign);

static const char *strtod_cases[] = {
    "", " ", "  \t\n\v\f\r 1", "+", "-", "+x", "-x", "0", "-0", "+0",
    "00000", "0000x", "1", "-1", "+1", "12345", "1.", ".5", "-.5", ".",
    "1.5", "1.25e3", "1e", "1e+", "1e-", "1e5", "1E5", "1e+5", "1e-5",
    "1e0005", "1e00000000005", "1e19999", "1e20000", "1e-19999", "1e-20000",
    "1e999999999999999999999", "0.0000000000000000000000001",
    "123456789012345678901234567890", "1234567890123456789012345678901234567890e-40",
    "0.1", "0.2", "0.3", "0.30000000000000004", "2.2250738585072011e-308",
    "2.2250738585072012e-308", "2.2250738585072014e-308", "1.7976931348623157e308",
    "1.7976931348623159e308", "4.9406564584124654e-324", "2.4703282292062327e-324",
    "2.4703282292062328e-324", "4.9e-324", "5e-324", "1e-323", "3e-324",
    "9007199254740992", "9007199254740993", "9007199254740994",
    "9007199254740992.5", "9007199254740993.5",
    "1e22", "1e23", "9.999999999999999e22", "1e-22", "1e-23",
    "0.000000000000000000001", "1000000000000000000000",
    "inf", "INF", "Inf", "infinity", "INFINITY", "-inf", "-infinity", "infi",
    "nan", "NAN", "NaN", "nan(1234)", "nan(0x1234)", "nan()", "nan(", "nanx",
    "0x1p0", "0x1P0", "0x10", "0X10", "0x1.8p1", "0x.8p1", "0x1p-1",
    "0x1p1000", "0x1p-1000", "0x1p10000", "0x1p-10000", "0xg", "0x", "0x.",
    "0x1.fffffffffffffp1023", "0x1p1024", "0x1p-1074", "0x1p-1075", "0x0p0",
    "0x0.0p0", "0x1.0000000000000000000000001p0",
    "1e308", "1e309", "-1e309", "1e-308", "1e-309",
    "18446744073709551616", "340282366920938463463374607431768211456",
    "0.5000000000000000000000001", "1.0000000000000000000000001",
    "2.00000000000000000000000000000000001",
    "1000000000000000000000000000000000000000000000000000000000000000000e-66",
    "12345678901234567890123456789012345678901234567890",
    "0.99999999999999999999999999999999999",
    "1.000000000000000000000000000000000000000000000000000000000001",
    "10000000000000000000000000000000000000000000e-43",
    "3.141592653589793238462643383279502884197169399375105820974944",
    "1e-4932", "1e4932", "5.0e-324", "1.5e-323",
    "722.0e-1", "0.00000000000000000000000000000000000000000000000000000001e60",
    NULL,
};

static void test_strtod_unused(void) {
    P("== strtod__unused ==\n");
    for (int i = 0; strtod_cases[i]; i++) {
        char *end = NULL;
        errno = 0;
        double v = strtod__unused(strtod_cases[i], &end);
        int en = errno;
        uint64_t bits;
        memcpy(&bits, &v, 8);
        P("st[%d] \"%s\" -> %016llx endoff=%td errno=%d\n", i, strtod_cases[i],
          (unsigned long long)bits, end ? end - strtod_cases[i] : (ptrdiff_t)-1, en);
        /* NULL se */
        errno = 0;
        double v2 = strtod__unused(strtod_cases[i], NULL);
        uint64_t bits2;
        memcpy(&bits2, &v2, 8);
        P("st2[%d]=%016llx errno=%d\n", i, (unsigned long long)bits2, errno);
    }

    /* generated decimal strings covering many exponents and digit counts */
    P("== strtod__unused generated ==\n");
    uint64_t seed = 0x243F6A8885A308D3ULL;
    for (int t = 0; t < 3000; t++) {
        char buf[80];
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        int ndig = 1 + (int)(seed % 25);
        int ex = (int)((seed >> 8) % 700) - 350;
        int neg = (seed >> 20) & 1;
        int pos = 0;
        if (neg)
            buf[pos++] = '-';
        for (int d = 0; d < ndig; d++) {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            int digit = (int)(seed % 10);
            if (d == 0 && digit == 0)
                digit = 1;
            buf[pos++] = '0' + digit;
            if (d == 0 && ndig > 1)
                buf[pos++] = '.';
        }
        pos += snprintf(buf + pos, sizeof(buf) - pos, "e%d", ex);
        buf[pos] = 0;
        char *end = NULL;
        errno = 0;
        double v = strtod__unused(buf, &end);
        uint64_t bits;
        memcpy(&bits, &v, 8);
        P("gen[%d] %s -> %016llx endoff=%td errno=%d\n", t, buf,
          (unsigned long long)bits, end - buf, errno);
    }

    /* long digit strings (exercise bigcomp / the STRTOD_DIGLIM path) */
    P("== strtod__unused long ==\n");
    for (int t = 0; t < 400; t++) {
        char buf[200];
        int pos = 0;
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        int ndig = 40 + (int)(seed % 100);
        for (int d = 0; d < ndig; d++) {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            buf[pos++] = '0' + (int)(seed % 10);
            if (d == 0)
                buf[pos++] = '.';
        }
        int ex = (int)((seed >> 12) % 60) - 30;
        pos += snprintf(buf + pos, sizeof(buf) - pos, "e%d", ex);
        buf[pos] = 0;
        char *end = NULL;
        errno = 0;
        double v = strtod__unused(buf, &end);
        uint64_t bits;
        memcpy(&bits, &v, 8);
        P("lng[%d] %s -> %016llx endoff=%td errno=%d\n", t, buf,
          (unsigned long long)bits, end - buf, errno);
    }

    /* halfway cases: exact binary values plus a trailing digit */
    P("== strtod__unused halfway ==\n");
    for (int e2 = -1080; e2 <= 1020; e2 += 37) {
        for (int m = 0; m < 3; m++) {
            char buf[64];
            snprintf(buf, sizeof(buf), "0x1%sp%d", m == 0 ? "" : (m == 1 ? ".8" : ".4"),
                     e2);
            char *end = NULL;
            errno = 0;
            double v = strtod__unused(buf, &end);
            uint64_t bits;
            memcpy(&bits, &v, 8);
            P("hex[%d,%d] %s -> %016llx endoff=%td errno=%d\n", e2, m, buf,
              (unsigned long long)bits, end - buf, errno);
        }
    }

    /* gethex() called directly with every rounding mode and sign */
    P("== gethex ==\n");
    static const char *hexcases[] = {
        "0x1p0",  "0x1.8p0",  "0x1p-1074", "0x1p-1075", "0x1.fffffffffffff8p1023",
        "0x1p1024", "0x0p0", "0x.8p1", "0x1.0000000000000001p0", "0xzz",
        "0x1p99999999999", "0x1p-99999999999", "0x.p0", "0x1.p3", "0x1p+3",
        "0x123456789abcdef0123456789p0", NULL};
    for (int i = 0; hexcases[i]; i++) {
        for (int rnd = 0; rnd <= 3; rnd++) {
            for (int sg = 0; sg <= 1; sg++) {
                const char *sp = hexcases[i];
                double d = 0;
                errno = 0;
                gethex(&sp, &d, rnd, sg);
                uint64_t bits;
                memcpy(&bits, &d, 8);
                P("gh[%d,%d,%d]=%016llx endoff=%td errno=%d\n", i, rnd, sg,
                  (unsigned long long)bits, sp - hexcases[i], errno);
            }
        }
    }
}

/* ---------------------------------------------------------------- */
/* allocator instrumentation: verifies that the translation performs the
   same allocations (count and size) in the same order, and that the
   out-of-memory paths behave identically.                            */

static long alloc_count, realloc_count, free_count;
static long alloc_bytes;
static long alloc_budget = -1; /* -1 = unlimited */
#define HIST_MAX 256
static long alloc_hist[HIST_MAX + 1];

/* The circular-reference hash table keys its entries with a "%p" rendering of
   a heap pointer, so those pair allocations are 56 + strlen("0x...") + 1 bytes
   and their size depends on the process' heap layout.  Fold them into a single
   bucket so the accounting is reproducible. */
static size_t norm_size(size_t n) {
    if (n >= 56 + 9 + 1 && n <= 56 + 2 + 2 * sizeof(void *) + 1)
        return 56;
    return n;
}

static void alloc_record(size_t n) {
    size_t k = norm_size(n);
    alloc_bytes += (long)k;
    alloc_hist[k < HIST_MAX ? k : HIST_MAX]++;
}

static void *cnt_malloc(size_t n) {
    if (alloc_budget >= 0 && alloc_count >= alloc_budget)
        return NULL;
    alloc_count++;
    alloc_record(n);
    return malloc(n);
}
static void *cnt_realloc(void *p, size_t n) {
    if (alloc_budget >= 0 && alloc_count >= alloc_budget)
        return NULL;
    realloc_count++;
    alloc_record(n);
    return realloc(p, n);
}
static void cnt_free(void *p) {
    if (p)
        free_count++;
    free(p);
}

static void alloc_reset(long budget) {
    alloc_count = realloc_count = free_count = alloc_bytes = 0;
    memset(alloc_hist, 0, sizeof(alloc_hist));
    alloc_budget = budget;
}

static void alloc_report(const char *tag) {
    P("%s malloc=%ld realloc=%ld free=%ld bytes=%ld\n", tag, alloc_count, realloc_count,
      free_count, alloc_bytes);
    for (int i = 0; i <= HIST_MAX; i++)
        if (alloc_hist[i])
            P("  %s size%d=%ld\n", tag, i, alloc_hist[i]);
}

/* A fixed workload touching every allocating code path. */
static void workload(void) {
    json_error_t err;
    json_t *j = json_loads(
        "{\"a\":[1,2.5,\"three\",true,null,{\"n\":[]}],\"b\":\"\\u00e9\\ud83d\\ude00\","
        "\"ccccccccccccccccccccccccccc\":1e-300}",
        0, &err);
    if (j) {
        char *s = json_dumps(j, JSON_INDENT(2) | JSON_SORT_KEYS);
        free(s);
        s = json_dumps(j, JSON_COMPACT);
        free(s);
        json_t *d = json_deep_copy(j);
        json_decref(d);
        json_t *c = json_copy(j);
        json_decref(c);
        json_t *p = json_pack("{s:s,s:[i,f],s:O}", "k", "v", "arr", 1, 2.5, "sub", j);
        if (p) {
            s = json_dumps(p, JSON_SORT_KEYS);
            free(s);
            json_decref(p);
        }
        for (int i = 0; i < 30; i++) {
            char kb[16];
            snprintf(kb, sizeof(kb), "extra%d", i);
            json_object_set_new(j, kb, json_integer(i));
        }
        s = json_dumps(j, JSON_SORT_KEYS);
        free(s);
        json_decref(j);
    }
    json_t *big = json_array();
    for (int i = 0; i < 40; i++)
        json_array_append_new(big, json_real(i * 1.5e-5));
    char *bs = json_dumps(big, 0);
    free(bs);
    json_decref(big);
    json_t *sp = json_sprintf("%s-%d", "fmt", 12);
    json_decref(sp);
}

static void test_alloc(void) {
    P("== allocation parity ==\n");
    json_set_alloc_funcs2(cnt_malloc, cnt_realloc, cnt_free);
    alloc_reset(-1);
    workload();
    alloc_report("parity");

    P("== out of memory paths ==\n");
    for (long budget = 0; budget <= 220; budget++) {
        alloc_reset(budget);
        json_error_t err;
        memset(&err, 0, sizeof(err));
        json_t *j = json_loads("{\"key\":[1,2,\"three\",{\"deep\":1.5}]}", 0, &err);
        P("oom-load[%ld] ok=%d line=%d text=%s\n", budget, j != NULL, err.line, err.text);
        if (j) {
            char *s = json_dumps(j, JSON_SORT_KEYS | JSON_INDENT(2));
            P("  dump=%s\n", s ? s : "(null)");
            free(s);
            json_decref(j);
        }
        alloc_reset(budget);
        memset(&err, 0, sizeof(err));
        json_t *p = json_pack_ex(&err, 0, "{s:s,s:[i,i],s:f}", "a", "bbbb", "c", 1, 2, "d",
                                 1.5);
        P("oom-pack[%ld] ok=%d text=%s\n", budget, p != NULL, err.text);
        json_decref(p);
        alloc_reset(budget);
        json_t *arr = json_array();
        int rc = 0;
        for (int i = 0; i < 30; i++)
            rc |= json_array_append_new(arr, json_integer(i));
        P("oom-arr[%ld] rc=%d size=%zu\n", budget, rc, json_array_size(arr));
        json_decref(arr);
        alloc_reset(budget);
        json_t *o = json_object();
        rc = 0;
        for (int i = 0; i < 30; i++) {
            char kb[16];
            snprintf(kb, sizeof(kb), "k%d", i);
            rc |= json_object_set_new(o, kb, json_integer(i));
        }
        P("oom-obj[%ld] rc=%d size=%zu\n", budget, rc, json_object_size(o));
        json_decref(o);
        alloc_reset(budget);
        char *ds = json_dumps(json_pack("[i,i,i]", 1, 2, 3), 0);
        P("oom-dumps[%ld]=%s\n", budget, ds ? ds : "(null)");
        free(ds);
    }
    alloc_reset(-1);
    json_set_alloc_funcs2(malloc, realloc, free);

    /* json_set_alloc_funcs() disables realloc, exercising the emulation path */
    P("== realloc emulation ==\n");
    json_set_alloc_funcs(cnt_malloc, cnt_free);
    alloc_reset(-1);
    workload();
    alloc_report("emul");
    json_set_alloc_funcs2(malloc, realloc, free);
}

/* ---------------------------------------------------------------- */

static void test_values(void) {
    P("== values ==\n");
    P("version=%s cmp=%d %d %d\n", jansson_version_str(), jansson_version_cmp(2, 15, 0),
      jansson_version_cmp(1, 0, 0), jansson_version_cmp(3, 0, 0));

    json_t *o = json_object();
    P("obj size=%zu\n", json_object_size(o));
    P("set=%d\n", json_object_set_new(o, "a", json_integer(1)));
    P("set=%d\n", json_object_set_new(o, "b", json_string("x")));
    P("set=%d\n", json_object_set_new(o, "c", json_real(1.5)));
    P("set=%d\n", json_object_set_new(o, "d", json_true()));
    P("set=%d\n", json_object_set_new(o, "e", json_null()));
    P("setn=%d\n", json_object_setn_new(o, "fghij", 3, json_integer(9)));
    P("setnc=%d\n", json_object_setn_new_nocheck(o, "kl\xff", 3, json_integer(10)));
    P("set null key=%d\n", json_object_set_new(o, NULL, json_integer(1)));
    P("set bad utf8=%d\n", json_object_set_new(o, "\xff", json_integer(1)));
    show("obj", o, JSON_SORT_KEYS);
    show("obj-ins", o, 0);
    P("size=%zu\n", json_object_size(o));
    P("get a=%s\n", json_object_get(o, "a") ? "y" : "n");
    P("getn fgh=%s\n", json_object_getn(o, "fghij", 3) ? "y" : "n");
    P("del a=%d del z=%d\n", json_object_del(o, "a"), json_object_del(o, "z"));
    P("deln=%d\n", json_object_deln(o, "fghij", 3));
    show("obj2", o, JSON_SORT_KEYS);

    const char *key;
    json_t *val;
    json_object_foreach(o, key, val) {
        P("iter %s -> %d\n", key, json_typeof(val));
    }
    void *it = json_object_iter(o);
    while (it) {
        P("it %s len=%zu type=%d\n", json_object_iter_key(it),
          json_object_iter_key_len(it), json_typeof(json_object_iter_value(it)));
        P("  key_to_iter same=%d\n",
          json_object_key_to_iter(json_object_iter_key(it)) == it);
        it = json_object_iter_next(o, it);
    }
    void *at = json_object_iter_at(o, "b");
    P("iter_at b=%s\n", at ? json_object_iter_key(at) : "(null)");
    P("iter_set=%d\n", json_object_iter_set_new(o, at, json_integer(42)));
    show("obj3", o, JSON_SORT_KEYS);

    json_t *o2 = json_object();
    json_object_set_new(o2, "b", json_integer(100));
    json_object_set_new(o2, "zz", json_integer(200));
    json_t *o3 = json_deep_copy(o);
    P("update=%d\n", json_object_update(o3, o2));
    show("upd", o3, JSON_SORT_KEYS);
    json_t *o4 = json_deep_copy(o);
    P("update_existing=%d\n", json_object_update_existing(o4, o2));
    show("upde", o4, JSON_SORT_KEYS);
    json_t *o5 = json_deep_copy(o);
    P("update_missing=%d\n", json_object_update_missing(o5, o2));
    show("updm", o5, JSON_SORT_KEYS);

    json_t *r1 = json_pack("{s:{s:i,s:i},s:i}", "a", "x", 1, "y", 2, "b", 3);
    json_t *r2 = json_pack("{s:{s:i,s:i},s:i}", "a", "x", 10, "z", 20, "c", 30);
    P("update_recursive=%d\n", json_object_update_recursive(r1, r2));
    show("updr", r1, JSON_SORT_KEYS);
    json_decref(r1);
    json_decref(r2);

    json_t *cp = json_copy(o);
    show("copy", cp, JSON_SORT_KEYS);
    json_t *dcp = json_deep_copy(o);
    show("deepcopy", dcp, JSON_SORT_KEYS);
    P("equal=%d %d\n", json_equal(o, dcp), json_equal(o, o2));
    json_decref(cp);
    json_decref(dcp);
    json_decref(o2);
    json_decref(o3);
    json_decref(o4);
    json_decref(o5);
    P("clear=%d size=%zu\n", json_object_clear(o), json_object_size(o));
    json_decref(o);

    json_t *a = json_array();
    P("arr size=%zu\n", json_array_size(a));
    for (int i = 0; i < 20; i++)
        P("append=%d\n", json_array_append_new(a, json_integer(i)));
    show("arr", a, JSON_COMPACT);
    P("get5=%lld get99null=%d\n", json_integer_value(json_array_get(a, 5)),
      json_array_get(a, 99) == NULL);
    P("set=%d\n", json_array_set_new(a, 3, json_string("three")));
    P("set oob=%d\n", json_array_set_new(a, 300, json_string("x")));
    P("insert=%d\n", json_array_insert_new(a, 2, json_string("ins")));
    P("insert oob=%d\n", json_array_insert_new(a, 300, json_string("x")));
    P("remove=%d\n", json_array_remove(a, 0));
    P("remove oob=%d\n", json_array_remove(a, 300));
    show("arr2", a, JSON_COMPACT);
    json_t *b = json_array();
    json_array_append_new(b, json_string("p"));
    json_array_append_new(b, json_string("q"));
    P("extend=%d\n", json_array_extend(a, b));
    show("arr3", a, JSON_COMPACT);
    size_t idx;
    json_t *v;
    json_array_foreach(a, idx, v) {
        P("af %zu %d\n", idx, json_typeof(v));
    }
    P("clear=%d size=%zu\n", json_array_clear(a), json_array_size(a));
    json_decref(a);
    json_decref(b);

    json_t *s = json_string("hello");
    P("sval=%s slen=%zu\n", json_string_value(s), json_string_length(s));
    {
        int rc = json_string_set(s, "world!");
        P("set=%d %s\n", rc, json_string_value(s));
        rc = json_string_setn(s, "abcdef", 3);
        P("setn=%d %s len=%zu\n", rc, json_string_value(s), json_string_length(s));
    }
    P("setnc=%d len=%zu\n", json_string_setn_nocheck(s, "ab\xff" "cd", 5),
      json_string_length(s));
    P("set bad=%d\n", json_string_set(s, "\xff"));
    json_decref(s);
    P("string null=%d\n", json_string(NULL) == NULL);
    P("stringn bad=%d\n", json_stringn("\xff", 1) == NULL);
    s = json_stringn_nocheck("a\0b", 3);
    P("nocheck len=%zu\n", json_string_length(s));
    json_decref(s);
    s = json_string_nocheck("\xff" "bad");
    P("nocheck2 len=%zu\n", json_string_length(s));
    json_decref(s);

    json_t *i1 = json_integer(1234567890123LL);
    P("ival=%lld set=%d ival=%lld\n", json_integer_value(i1), json_integer_set(i1, -5),
      json_integer_value(i1));
    P("num=%f\n", json_number_value(i1));
    json_decref(i1);
    json_t *rr = json_real(2.5);
    P("rval=%.17g set=%d rval=%.17g\n", json_real_value(rr), json_real_set(rr, -3.25),
      json_real_value(rr));
    P("set nan=%d\n", json_real_set(rr, NAN));
    P("num=%.17g\n", json_number_value(rr));
    json_decref(rr);
    P("true=%d false=%d null=%d\n", json_typeof(json_true()), json_typeof(json_false()),
      json_typeof(json_null()));
    P("boolean=%d %d\n", json_typeof(json_boolean(1)), json_typeof(json_boolean(0)));
    P("num of string=%f\n", json_number_value(json_null()));

    /* circular references */
    json_t *c1 = json_array();
    json_array_append(c1, c1);
    show("circ", c1, 0);
    json_t *cc = json_deep_copy(c1);
    P("deepcopy circ null=%d\n", cc == NULL);
    json_array_clear(c1);
    json_decref(c1);

    json_t *c2 = json_object();
    json_object_set(c2, "self", c2);
    show("circo", c2, 0);
    json_object_clear(c2);
    json_decref(c2);
}

/* ---------------------------------------------------------------- */

static void test_pack_unpack(void) {
    P("== pack/unpack ==\n");
    json_error_t err;
    json_t *j;

    j = json_pack("{s:s,s:i,s:f,s:b,s:n,s:[i,i],s:{s:s}}", "str", "value", "int", 42,
                  "real", 1.5, "bool", 1, "null", "arr", 1, 2, "obj", "k", "v");
    show("pack1", j, JSON_SORT_KEYS);
    json_decref(j);

    j = json_pack_ex(&err, 0, "{s:s}", "a", "b");
    show("pack2", j, 0);
    show_err("pack2err", &err);
    json_decref(j);

    j = json_pack_ex(&err, 0, "{s:s}", "a", NULL);
    P("pack3 null=%d\n", j == NULL);
    show_err("pack3err", &err);

    j = json_pack_ex(&err, 0, "{s:s*}", "a", NULL);
    show("pack4", j, 0);
    show_err("pack4err", &err);
    json_decref(j);

    j = json_pack_ex(&err, 0, "{s:s?}", "a", NULL);
    show("pack5", j, 0);
    show_err("pack5err", &err);
    json_decref(j);

    j = json_pack_ex(&err, 0, "s#", "abcdef", 3);
    show("pack6", j, JSON_ENCODE_ANY);
    json_decref(j);

    j = json_pack_ex(&err, 0, "s%", "abcdef", (size_t)4);
    show("pack7", j, JSON_ENCODE_ANY);
    json_decref(j);

    j = json_pack_ex(&err, 0, "s++", "a", "b", "c");
    show("pack8", j, JSON_ENCODE_ANY);
    json_decref(j);

    j = json_pack_ex(&err, 0, "[s,s]", "x", "y");
    show("pack9", j, 0);
    json_decref(j);

    j = json_pack_ex(&err, 0, "{s:i", "a", 1);
    P("pack10 null=%d\n", j == NULL);
    show_err("pack10err", &err);

    j = json_pack_ex(&err, 0, "");
    P("pack11 null=%d\n", j == NULL);
    show_err("pack11err", &err);

    j = json_pack_ex(&err, 0, "[i]xx", 1);
    P("pack12 null=%d\n", j == NULL);
    show_err("pack12err", &err);

    j = json_pack_ex(&err, 0, "q");
    P("pack13 null=%d\n", j == NULL);
    show_err("pack13err", &err);

    j = json_pack_ex(&err, 0, "{s:o}", "a", json_integer(5));
    show("pack14", j, 0);
    json_decref(j);

    json_t *shared = json_string("shared");
    j = json_pack_ex(&err, 0, "{s:O}", "a", shared);
    show("pack15", j, 0);
    json_decref(j);
    json_decref(shared);

    j = json_pack_ex(&err, 0, "{s:I}", "a", (json_int_t)1234567890123LL);
    show("pack16", j, 0);
    json_decref(j);

    j = json_pack_ex(&err, 0, "f", 1.0 / 0.0);
    P("pack17 null=%d\n", j == NULL);
    show_err("pack17err", &err);

    /* json_sprintf */
    j = json_sprintf("hello %s %d %.3f", "world", 42, 1.5);
    show("sprintf", j, JSON_ENCODE_ANY);
    json_decref(j);
    j = json_sprintf("%s", "");
    show("sprintf2", j, JSON_ENCODE_ANY);
    json_decref(j);
    j = json_sprintf("%c", 0xff);
    P("sprintf3 null=%d\n", j == NULL);

    /* unpack */
    json_t *root =
        json_pack("{s:s,s:i,s:f,s:b,s:n,s:[i,i,i],s:{s:s}}", "s", "v", "i", 7, "f", 2.5,
                  "b", 1, "n", "a", 1, 2, 3, "o", "k", "vv");
    const char *sv = NULL;
    json_int_t iv = 0;
    double fv = 0;
    int bv = 0;
    int i1 = 0, i2 = 0, i3 = 0;
    const char *kv = NULL;
    int r = json_unpack_ex(root, &err, 0, "{s:s,s:I,s:f,s:b,s:n,s:[i,i,i],s:{s:s}}", "s",
                           &sv, "i", &iv, "f", &fv, "b", &bv, "n", "a", &i1, &i2, &i3,
                           "o", "k", &kv);
    P("unpack1=%d sv=%s iv=%lld fv=%.17g bv=%d %d %d %d kv=%s\n", r, sv, iv, fv, bv, i1,
      i2, i3, kv);
    show_err("unpack1err", &err);

    r = json_unpack_ex(root, &err, JSON_STRICT, "{s:s}", "s", &sv);
    P("unpack2=%d\n", r);
    show_err("unpack2err", &err);

    r = json_unpack_ex(root, &err, 0, "{s:s,s:i!}", "s", &sv, "i", &i1);
    P("unpack3=%d\n", r);
    show_err("unpack3err", &err);

    r = json_unpack_ex(root, &err, 0, "{s?:s}", "nothere", &sv);
    P("unpack4=%d\n", r);
    show_err("unpack4err", &err);

    r = json_unpack_ex(root, &err, 0, "{s:s}", "missing", &sv);
    P("unpack5=%d\n", r);
    show_err("unpack5err", &err);

    r = json_unpack_ex(root, &err, JSON_VALIDATE_ONLY, "{s:s,s:I}", "s", "i");
    P("unpack6=%d\n", r);
    show_err("unpack6err", &err);

    size_t slen = 0;
    r = json_unpack_ex(root, &err, 0, "{s:s%}", "s", &sv, &slen);
    P("unpack7=%d slen=%zu\n", r, slen);

    double F = 0;
    r = json_unpack_ex(root, &err, 0, "{s:F}", "i", &F);
    P("unpack8=%d F=%.17g\n", r, F);

    json_t *op = NULL;
    r = json_unpack_ex(root, &err, 0, "{s:o}", "o", &op);
    P("unpack9=%d op=%d\n", r, op ? json_typeof(op) : -1);

    r = json_unpack_ex(root, &err, 0, "{s:i}", "s", &i1);
    P("unpack10=%d\n", r);
    show_err("unpack10err", &err);

    r = json_unpack_ex(root, &err, 0, "[i]");
    P("unpack11=%d\n", r);
    show_err("unpack11err", &err);

    r = json_unpack_ex(NULL, &err, 0, "i");
    P("unpack12=%d\n", r);
    show_err("unpack12err", &err);

    r = json_unpack_ex(root, &err, 0, "");
    P("unpack13=%d\n", r);
    show_err("unpack13err", &err);

    json_t *arr = json_pack("[i,i,i,i]", 1, 2, 3, 4);
    r = json_unpack_ex(arr, &err, 0, "[i,i]", &i1, &i2);
    P("unpack14=%d\n", r);
    r = json_unpack_ex(arr, &err, 0, "[i,i!]", &i1, &i2);
    P("unpack15=%d\n", r);
    show_err("unpack15err", &err);
    r = json_unpack_ex(arr, &err, 0, "[i,i,i,i,i]", &i1, &i2, &i3, &i1, &i2);
    P("unpack16=%d\n", r);
    show_err("unpack16err", &err);
    r = json_unpack_ex(arr, &err, 0, "[i,i,q]", &i1, &i2);
    P("unpack17=%d\n", r);
    show_err("unpack17err", &err);
    r = json_unpack(arr, "[i,i,i,i]", &i1, &i2, &i3, &i1);
    P("unpack18=%d\n", r);
    json_decref(arr);
    json_decref(root);
}

/* ---------------------------------------------------------------- */

static void test_internals(void) {
    P("== internals ==\n");
    /* utf8 */
    for (int cp = -2; cp < 0x110100; cp += 4093) {
        char buf[8];
        size_t sz = 0;
        int r = utf8_encode(cp, buf, &sz);
        P("enc %d=%d sz=%zu", cp, r, r == 0 ? sz : (size_t)0);
        if (r == 0) {
            for (size_t i = 0; i < sz; i++)
                P(" %02x", (unsigned char)buf[i]);
            int32_t back = 0;
            const char *e = utf8_iterate(buf, sz, &back);
            P(" it=%td back=%d", e ? e - buf : (ptrdiff_t)-1, back);
            P(" chk=%d", utf8_check_string(buf, sz));
        }
        P("\n");
    }
    for (int b = 0; b < 256; b++)
        P("first %d=%zu\n", b, utf8_check_first((char)b));
    static const char *seqs[] = {"\x41", "\xc3\xa9", "\xe4\xb8\xad", "\xf0\x9f\x98\x80",
                                 "\xc0\x80", "\xe0\x80\x80", "\xed\xa0\x80",
                                 "\xf4\x90\x80\x80", "\xc3", "\xc3\x28", NULL};
    for (int i = 0; seqs[i]; i++) {
        size_t len = strlen(seqs[i]);
        int32_t cp = 0;
        P("full[%d]=%zu cp=%d chk=%d\n", i, utf8_check_full(seqs[i], len, &cp), cp,
          utf8_check_string(seqs[i], len));
    }

    /* strbuffer */
    strbuffer_t sb;
    P("sbinit=%d\n", strbuffer_init(&sb));
    for (int i = 0; i < 100; i++)
        strbuffer_append_byte(&sb, 'a' + (i % 26));
    P("sblen=%zu sbsize=%zu val=%s\n", sb.length, sb.size, strbuffer_value(&sb));
    P("pop=%c len=%zu\n", strbuffer_pop(&sb), sb.length);
    strbuffer_append_bytes(&sb, "12345", 5);
    P("val=%s len=%zu\n", strbuffer_value(&sb), sb.length);
    strbuffer_clear(&sb);
    P("cleared len=%zu val=[%s]\n", sb.length, strbuffer_value(&sb));
    P("pop empty=%d\n", strbuffer_pop(&sb));
    strbuffer_close(&sb);

    /* hashtable */
    hashtable_t ht;
    P("htinit=%d\n", hashtable_init(&ht));
    char kb[32];
    for (int i = 0; i < 40; i++) {
        snprintf(kb, sizeof(kb), "key%d", i);
        hashtable_set(&ht, kb, strlen(kb), json_integer(i));
    }
    P("htsize=%zu order=%zu\n", ht.size, ht.order);
    for (int i = 0; i < 40; i++) {
        snprintf(kb, sizeof(kb), "key%d", i);
        json_t *g = hashtable_get(&ht, kb, strlen(kb));
        P("htget %s=%lld\n", kb, g ? json_integer_value(g) : -1);
    }
    void *hi = hashtable_iter(&ht);
    int cnt = 0;
    while (hi) {
        P("htiter %s len=%zu val=%lld\n", (char *)hashtable_iter_key(hi),
          hashtable_iter_key_len(hi),
          json_integer_value((json_t *)hashtable_iter_value(hi)));
        hi = hashtable_iter_next(&ht, hi);
        cnt++;
    }
    P("htcount=%d\n", cnt);
    void *ha = hashtable_iter_at(&ht, "key5", 4);
    P("htat=%s\n", ha ? (char *)hashtable_iter_key(ha) : "(null)");
    hashtable_iter_set(ha, json_integer(999));
    P("after set=%lld\n", json_integer_value((json_t *)hashtable_get(&ht, "key5", 4)));
    for (int i = 0; i < 20; i++) {
        snprintf(kb, sizeof(kb), "key%d", i);
        P("htdel %s=%d\n", kb, hashtable_del(&ht, kb, strlen(kb)));
    }
    P("htsize=%zu\n", ht.size);
    hashtable_clear(&ht);
    P("cleared htsize=%zu\n", ht.size);
    hashtable_close(&ht);

    /* jsonp_* helpers */
    char *dup = jsonp_strndup("abcdefgh", 4);
    P("strndup=%s\n", dup);
    jsonp_free(dup);
    void *m = jsonp_malloc(0);
    P("malloc0=%d\n", m == NULL);
    m = jsonp_malloc(16);
    memset(m, 'z', 16);
    m = jsonp_realloc(m, 16, 32);
    P("realloc=%d\n", m != NULL);
    jsonp_free(m);
    jsonp_free(NULL);

    json_t *own = jsonp_stringn_nocheck_own(jsonp_strndup("owned", 5), 5);
    P("own=%s\n", json_string_value(own));
    json_decref(own);

    json_error_t e;
    jsonp_error_init(&e, "src");
    show_err("err1", &e);
    jsonp_error_set(&e, 1, 2, 3, json_error_invalid_syntax, "msg %d %s", 7, "x");
    show_err("err2", &e);
    jsonp_error_set(&e, 9, 9, 9, json_error_unknown, "second");
    show_err("err3", &e);
    jsonp_error_init(&e, NULL);
    show_err("err4", &e);
    char longsrc[200];
    memset(longsrc, 'L', sizeof(longsrc) - 1);
    longsrc[sizeof(longsrc) - 1] = 0;
    jsonp_error_init(&e, longsrc);
    show_err("err5", &e);
    jsonp_error_set_source(&e, "shorter");
    show_err("err6", &e);
    char longmsg[400];
    memset(longmsg, 'M', sizeof(longmsg) - 1);
    longmsg[sizeof(longmsg) - 1] = 0;
    jsonp_error_init(&e, "s");
    jsonp_error_set(&e, 1, 1, 1, json_error_unknown, "%s", longmsg);
    show_err("err7", &e);

    /* loop check + deep copy internals */
    hashtable_t parents;
    hashtable_init(&parents);
    json_t *node = json_integer(1);
    char lk[32];
    size_t lkl = 0;
    P("loopchk1=%d\n", jsonp_loop_check(&parents, node, lk, sizeof(lk), &lkl));
    P("loopchk2=%d\n", jsonp_loop_check(&parents, node, lk, sizeof(lk), &lkl));
    hashtable_close(&parents);
    hashtable_init(&parents);
    json_t *dc = do_deep_copy(node, &parents);
    P("do_deep_copy=%lld\n", json_integer_value(dc));
    json_decref(dc);
    hashtable_close(&parents);
    json_decref(node);

    hashtable_init(&parents);
    json_t *ra = json_pack("{s:{s:i}}", "a", "b", 1);
    json_t *rb = json_pack("{s:{s:i}}", "a", "c", 2);
    P("do_object_update_recursive=%d\n", do_object_update_recursive(ra, rb, &parents));
    show("dour", ra, JSON_SORT_KEYS);
    hashtable_close(&parents);
    json_decref(ra);
    json_decref(rb);

    /* alloc funcs */
    json_malloc_t mf;
    json_realloc_t rf;
    json_free_t ff;
    json_get_alloc_funcs(&mf, &ff);
    P("alloc funcs libc=%d %d\n", mf == malloc, ff == free);
    json_get_alloc_funcs2(&mf, &rf, &ff);
    P("alloc funcs2 libc=%d %d %d\n", mf == malloc, rf == realloc, ff == free);
    json_set_alloc_funcs(malloc, free);
    json_get_alloc_funcs2(&mf, &rf, &ff);
    P("after set: %d %d %d\n", mf == malloc, rf == NULL, ff == free);
    json_t *tmp = json_pack("[i,s]", 1, "x");
    show("with custom alloc", tmp, 0);
    json_decref(tmp);
    json_set_alloc_funcs2(malloc, realloc, free);
    json_get_alloc_funcs2(&mf, &rf, &ff);
    P("after set2: %d %d %d\n", mf == malloc, rf == realloc, ff == free);
    json_get_alloc_funcs(NULL, NULL);
    json_get_alloc_funcs2(NULL, NULL, NULL);
    P("divmax=%d\n", dtoa_divmax);
}

/* ---------------------------------------------------------------- */

static size_t cb_pos;
static const char *cb_data;
static size_t load_cb(void *buffer, size_t buflen, void *data) {
    (void)data;
    size_t n = strlen(cb_data) - cb_pos;
    if (n > buflen)
        n = buflen;
    if (n > 7)
        n = 7;
    memcpy(buffer, cb_data + cb_pos, n);
    cb_pos += n;
    return n;
}

static int dump_cb(const char *buffer, size_t size, void *data) {
    (void)data;
    P("[cb %.*s]", (int)size, buffer);
    return 0;
}

static int dump_cb_fail(const char *buffer, size_t size, void *data) {
    (void)buffer;
    (void)size;
    (void)data;
    return -1;
}

static void test_io(void) {
    P("== io ==\n");
    json_error_t err;

    cb_data = "{\"callback\":[1,2,3],\"more\":\"data here\"}";
    cb_pos = 0;
    json_t *j = json_load_callback(load_cb, NULL, 0, &err);
    show("loadcb", j, JSON_SORT_KEYS);
    show_err("loadcberr", &err);

    P("dumpcb=%d\n", json_dump_callback(j, dump_cb, NULL, JSON_COMPACT));
    P("\n");
    P("dumpcbfail=%d\n", json_dump_callback(j, dump_cb_fail, NULL, JSON_COMPACT));
    P("dumpcbnull=%d\n", json_dump_callback(NULL, dump_cb, NULL, JSON_COMPACT));
    P("dumpcbany=%d\n",
      json_dump_callback(json_integer(1), dump_cb, NULL, JSON_ENCODE_ANY));
    P("\n");

    j = json_load_callback(NULL, NULL, 0, &err);
    P("loadcbnull null=%d\n", j == NULL);
    show_err("loadcbnullerr", &err);

    /* files */
    const char *path = "/tmp/jansson_difftest.json";
    json_t *doc = json_pack("{s:[i,i,i],s:s,s:f}", "arr", 1, 2, 3, "str", "text", "real",
                            1.25);
    P("dumpfile=%d\n", json_dump_file(doc, path, JSON_INDENT(2) | JSON_SORT_KEYS));
    FILE *f = fopen(path, "r");
    char content[1024];
    size_t n = fread(content, 1, sizeof(content) - 1, f);
    content[n] = 0;
    fclose(f);
    P("filecontent=[%s]\n", content);
    json_t *ld = json_load_file(path, 0, &err);
    show("loadfile", ld, JSON_SORT_KEYS);
    show_err("loadfileerr", &err);
    json_decref(ld);

    ld = json_load_file("/tmp/definitely_missing_file_xyz.json", 0, &err);
    P("loadmissing null=%d\n", ld == NULL);
    P("loadmissingerr|line=%d col=%d pos=%d code=%d text=%s\n", err.line, err.column,
      err.position, (int)json_error_code(&err), err.text);
    ld = json_load_file(NULL, 0, &err);
    P("loadnullpath null=%d\n", ld == NULL);
    show_err("loadnullpatherr", &err);

    f = fopen(path, "r");
    ld = json_loadf(f, 0, &err);
    show("loadf", ld, JSON_SORT_KEYS);
    json_decref(ld);
    fclose(f);
    ld = json_loadf(NULL, 0, &err);
    P("loadfnull null=%d\n", ld == NULL);
    show_err("loadfnullerr", &err);

    int fd = open(path, 0 /*O_RDONLY*/);
    ld = json_loadfd(fd, 0, &err);
    show("loadfd", ld, JSON_SORT_KEYS);
    json_decref(ld);
    close(fd);
    ld = json_loadfd(-1, 0, &err);
    P("loadfdneg null=%d\n", ld == NULL);
    show_err("loadfdnegerr", &err);

    f = fopen("/tmp/jansson_difftest_out.json", "w");
    P("dumpf=%d\n", json_dumpf(doc, f, JSON_COMPACT));
    fclose(f);
    f = fopen("/tmp/jansson_difftest_out.json", "r");
    n = fread(content, 1, sizeof(content) - 1, f);
    content[n] = 0;
    fclose(f);
    P("dumpfcontent=[%s]\n", content);

    fd = open("/tmp/jansson_difftest_fd.json", 577 /*O_WRONLY|O_CREAT|O_TRUNC*/, 0644);
    P("dumpfd=%d\n", json_dumpfd(doc, fd, JSON_COMPACT));
    close(fd);
    f = fopen("/tmp/jansson_difftest_fd.json", "r");
    n = fread(content, 1, sizeof(content) - 1, f);
    content[n] = 0;
    fclose(f);
    P("dumpfdcontent=[%s]\n", content);

    P("dumpfile bad path=%d\n", json_dump_file(doc, "/nonexistent_dir_xyz/x.json", 0));
    json_decref(doc);
    json_decref(j);
    unlink(path);
    unlink("/tmp/jansson_difftest_out.json");
    unlink("/tmp/jansson_difftest_fd.json");
}

/* ---------------------------------------------------------------- */

static void test_big(void) {
    P("== big ==\n");
    /* a large document to exercise rehashing / array growth / deep nesting */
    json_t *root = json_object();
    for (int i = 0; i < 300; i++) {
        char k[32];
        snprintf(k, sizeof(k), "k%03d", i);
        json_t *arr = json_array();
        for (int m = 0; m < 5; m++)
            json_array_append_new(arr, json_integer(i * 5 + m));
        json_object_set_new(root, k, arr);
    }
    char *s = json_dumps(root, JSON_SORT_KEYS | JSON_COMPACT);
    P("biglen=%zu\n", strlen(s));
    unsigned long h = 5381;
    for (char *p = s; *p; p++)
        h = h * 33 + (unsigned char)*p;
    P("bighash=%lu\n", h);
    json_error_t err;
    json_t *rt = json_loads(s, 0, &err);
    char *s2 = json_dumps(rt, JSON_SORT_KEYS | JSON_COMPACT);
    P("roundtrip equal=%d\n", strcmp(s, s2) == 0);
    P("json_equal=%d\n", json_equal(root, rt));
    free(s);
    free(s2);
    json_decref(rt);
    json_decref(root);

    /* deep nesting near the parser limit */
    for (int depth = 2045; depth <= 2050; depth++) {
        char *buf = malloc(2 * depth + 3);
        for (int i = 0; i < depth; i++)
            buf[i] = '[';
        for (int i = 0; i < depth; i++)
            buf[depth + i] = ']';
        buf[2 * depth] = 0;
        json_t *j = json_loads(buf, 0, &err);
        P("depth %d=%s\n", depth, j ? "ok" : "fail");
        show_err("  deptherr", &err);
        json_decref(j);
        free(buf);
    }

    /* dump with indentation at depth */
    json_t *nested = json_integer(1);
    for (int i = 0; i < 40; i++) {
        json_t *a = json_array();
        json_array_append_new(a, nested);
        nested = a;
    }
    char *ds = json_dumps(nested, JSON_INDENT(4));
    P("nestedlen=%zu\n", ds ? strlen(ds) : 0);
    h = 5381;
    for (char *p = ds; p && *p; p++)
        h = h * 33 + (unsigned char)*p;
    P("nestedhash=%lu\n", h);
    free(ds);
    json_decref(nested);

    /* long strings */
    char *ls = malloc(70000);
    for (int i = 0; i < 69999; i++)
        ls[i] = 'a' + (i % 26);
    ls[69999] = 0;
    json_t *str = json_string(ls);
    json_t *wrap = json_array();
    json_array_append_new(wrap, str);
    char *out = json_dumps(wrap, 0);
    P("longstrlen=%zu\n", out ? strlen(out) : 0);
    free(out);
    json_decref(wrap);
    free(ls);
}

int main(void) {
    json_object_seed(0x12345678);
    test_load_dump();
    test_reals();
    test_strtod();
    test_strtod_unused();
    test_values();
    test_pack_unpack();
    test_internals();
    test_io();
    test_big();
    test_alloc();
    P("== done ==\n");
    return 0;
}
