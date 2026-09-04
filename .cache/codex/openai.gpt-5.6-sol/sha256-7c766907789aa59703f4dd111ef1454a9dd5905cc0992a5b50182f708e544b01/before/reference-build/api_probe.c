#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../c_src/cJSON.h"

struct record
{
    const char *precision;
    double lat;
    double lon;
    const char *address;
    const char *city;
    const char *state;
    const char *zip;
    const char *country;
};

extern int driver(
    const char *strings[7],
    int numbers[3][3],
    int ids[4],
    struct record fields[2]);

static int hook_allocations;
static int hook_frees;

static void *counting_malloc(size_t size)
{
    hook_allocations++;
    return malloc(size);
}

static void counting_free(void *pointer)
{
    hook_frees++;
    free(pointer);
}

static void print_result(const char *label, cJSON *item)
{
    char *formatted = cJSON_Print(item);
    char *compact = cJSON_PrintUnformatted(item);
    int needed = formatted == NULL ? 0 : (int)strlen(formatted) + 5;
    char *preallocated = needed == 0 ? NULL : malloc((size_t)needed);
    int preallocated_ok = preallocated == NULL ? 0 :
        cJSON_PrintPreallocated(item, preallocated, needed, 1);

    printf("%s|F=", label);
    if (formatted != NULL) {
        fwrite(formatted, 1, strlen(formatted), stdout);
    } else {
        fputs("<null>", stdout);
    }
    fputs("|U=", stdout);
    if (compact != NULL) {
        fwrite(compact, 1, strlen(compact), stdout);
    } else {
        fputs("<null>", stdout);
    }
    printf("|P=%d:", preallocated_ok);
    if (preallocated_ok) {
        fwrite(preallocated, 1, strlen(preallocated), stdout);
    }
    fputc('\n', stdout);

    cJSON_free(formatted);
    cJSON_free(compact);
    free(preallocated);
}

static void parse_cases(void)
{
    static const char *cases[] = {
        "null", "true", "false", "0", "-0", "1.5", "1e+20", "1e-20",
        "9007199254740991", "\"plain\"", "\"a\\\\b\\n\\t\\\"c\"",
        "\"\\u0041\\u00df\\u6771\\ud834\\udd1e\"", "\"x\\u0000y\"",
        "[]", "[1,true,null,\"x\",{\"k\":2}]", "{}", "{\"a\":1,\"B\":[2,3]}",
        " \t\r\n [ 1 , 2 ] ", "\xEF\xBB\xBF{\"bom\":true}",
        "null trailing", "", "[", "[1,", "{\"a\"", "{\"a\":}", "\"\\uD800\"",
        "\"\\q\"", "01", "1e", "tru", "/*x*/1"
    };
    size_t i;

    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        const char *end = NULL;
        const char *input = cases[i];
        cJSON *item = cJSON_ParseWithLengthOpts(
            input, strlen(input) + 1, &end, 1);
        printf("PARSE%02lu|ok=%d|end=%ld|err=%ld",
            (unsigned long)i, item != NULL,
            end == NULL ? -1L : (long)(end - input),
            cJSON_GetErrorPtr() == NULL ? -1L :
                (long)(cJSON_GetErrorPtr() - input));
        if (item != NULL) {
            char *compact = cJSON_PrintUnformatted(item);
            printf("|json=%s", compact == NULL ? "<null>" : compact);
            cJSON_free(compact);
        }
        fputc('\n', stdout);
        cJSON_Delete(item);
    }
}

static void construction_cases(void)
{
    int ints[] = { -2, 0, 7, 2147483647 };
    float floats[] = { 1.25f, -0.5f, 1000000.0f };
    double doubles[] = { 1.0 / 3.0, -0.0, 1e100 };
    const char *strings[] = { "one", "t\"wo", "" };
    cJSON *root = cJSON_CreateObject();
    cJSON *array;
    cJSON *detached;
    cJSON *duplicate;
    cJSON *string;
    char minified[] = " /*a*/ { \"x\" : [ 1, 2 ], // b\n \"s\":\"a b\" } ";

    cJSON_AddNullToObject(root, "null");
    cJSON_AddTrueToObject(root, "true");
    cJSON_AddFalseToObject(root, "false");
    cJSON_AddBoolToObject(root, "bool", 9);
    cJSON_AddNumberToObject(root, "number", 1.0 / 3.0);
    cJSON_AddRawToObject(root, "raw", "[9,8]");
    string = cJSON_AddStringToObject(root, "string", "long value");
    cJSON_SetValuestring(string, "short");
    cJSON_SetValuestring(string, "a much longer replacement");

    array = cJSON_AddArrayToObject(root, "array");
    cJSON_AddItemToArray(array, cJSON_CreateString("a"));
    cJSON_AddItemToArray(array, cJSON_CreateString("c"));
    cJSON_InsertItemInArray(array, 1, cJSON_CreateString("b"));
    cJSON_ReplaceItemInArray(array, 2, cJSON_CreateString("C"));
    detached = cJSON_DetachItemFromArray(array, 0);
    cJSON_Delete(detached);
    cJSON_AddItemReferenceToArray(array, cJSON_GetArrayItem(array, 0));

    cJSON_AddItemToObject(root, "ints", cJSON_CreateIntArray(ints, 4));
    cJSON_AddItemToObject(root, "floats", cJSON_CreateFloatArray(floats, 3));
    cJSON_AddItemToObject(root, "doubles", cJSON_CreateDoubleArray(doubles, 3));
    cJSON_AddItemToObject(root, "strings", cJSON_CreateStringArray(strings, 3));

    print_result("TREE", root);
    duplicate = cJSON_Duplicate(root, 1);
    printf("COMPARE|same=%d|duplicate=%d\n",
        cJSON_Compare(root, root, 1), cJSON_Compare(root, duplicate, 1));
    cJSON_ReplaceItemInObjectCaseSensitive(
        duplicate, "number", cJSON_CreateNumber(4.0));
    printf("COMPARE2|value=%d\n", cJSON_Compare(root, duplicate, 1));
    print_result("DUP", duplicate);

    cJSON_Minify(minified);
    printf("MINIFY|%s\n", minified);
    printf("TYPES|%d%d%d%d%d%d%d%d%d%d|size=%d|has=%d|num=%.17g|str=%s\n",
        cJSON_IsInvalid(root), cJSON_IsFalse(root), cJSON_IsTrue(root),
        cJSON_IsBool(root), cJSON_IsNull(root), cJSON_IsNumber(root),
        cJSON_IsString(root), cJSON_IsArray(root), cJSON_IsObject(root),
        cJSON_IsRaw(root), cJSON_GetArraySize(array),
        cJSON_HasObjectItem(root, "NuLl"),
        cJSON_GetNumberValue(cJSON_GetObjectItem(root, "number")),
        cJSON_GetStringValue(cJSON_GetObjectItem(root, "string")));

    cJSON_Delete(duplicate);
    cJSON_Delete(root);
}

static void hook_case(void)
{
    cJSON_Hooks hooks;
    cJSON *item;
    char *printed;

    hook_allocations = 0;
    hook_frees = 0;
    hooks.malloc_fn = counting_malloc;
    hooks.free_fn = counting_free;
    cJSON_InitHooks(&hooks);
    item = cJSON_Parse("{\"hook\":[1,2,3]}");
    printed = cJSON_PrintUnformatted(item);
    printf("HOOK|json=%s|alloc=%d|free_before=%d",
        printed, hook_allocations, hook_frees);
    cJSON_free(printed);
    cJSON_Delete(item);
    printf("|free_after=%d\n", hook_frees);
    cJSON_InitHooks(NULL);
}

static void driver_case(void)
{
    const char *strings[7] = {
        "Sunday", "Monday", "Tuesday", "Wednesday",
        "Thursday", "Friday", "Saturday"
    };
    int numbers[3][3] = {
        { 0, -1, 2 },
        { 3, 4, 5 },
        { 6, 7, 8 }
    };
    int ids[4] = { 116, 943, 234, 38793 };
    struct record fields[2] = {
        {
            "zip", 37.7668, -122.3959, "", "SAN FRANCISCO",
            "CA", "94107", "US"
        },
        {
            "zip", 37.371991, -122.026020, "", "SUNNYVALE",
            "CA", "94085", "US"
        }
    };

    printf("DRIVER|return=%d\n", driver(strings, numbers, ids, fields));
}

int main(void)
{
    printf("VERSION|%s\n", cJSON_Version());
    parse_cases();
    construction_cases();
    hook_case();
    driver_case();
    return 0;
}
