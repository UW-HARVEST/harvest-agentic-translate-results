#include "cJSON.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

extern cJSON *cJSON_Duplicate_rec(const cJSON *item, size_t depth, cJSON_bool recurse);
extern int driver(const char *strings[7], int numbers[3][3], int ids[4], struct record fields[2]);

static size_t allocations;
static size_t frees;

static void *counting_malloc(size_t size)
{
    allocations++;
    return malloc(size);
}

static void counting_free(void *pointer)
{
    frees++;
    free(pointer);
}

static void print_result(const char *label, cJSON *item)
{
    char *formatted = cJSON_Print(item);
    char *compact = cJSON_PrintUnformatted(item);
    char *buffered = cJSON_PrintBuffered(item, 1, 0);
    char preallocated[2048];
    int status = cJSON_PrintPreallocated(item, preallocated, (int)sizeof(preallocated), 1);

    printf("%s|F=%s|U=%s|B=%s|P=%d:%s\n",
           label,
           formatted ? formatted : "<null>",
           compact ? compact : "<null>",
           buffered ? buffered : "<null>",
           status,
           status ? preallocated : "<failed>");
    cJSON_free(formatted);
    cJSON_free(compact);
    cJSON_free(buffered);
}

static void parse_cases(void)
{
    static const char *cases[] = {
        "null",
        "false",
        "true",
        "0",
        "-12",
        "1.25",
        "1e100",
        "\"a\\n\\t\\\\\\\"\\/\\b\\f\\r\"",
        "\"\\u0041\\u00df\\u6771\\ud834\\udd1e\"",
        "[]",
        "{}",
        "[1, true, null, \"x\", {\"A\":2}]",
        "{\"alpha\":1,\"beta\":[2,3],\"nested\":{\"x\":\"y\"}}"
    };
    size_t index;

    for (index = 0; index < sizeof(cases) / sizeof(cases[0]); index++)
    {
        cJSON *item = cJSON_Parse(cases[index]);
        const char *string_value = cJSON_GetStringValue(item);
        double number_value = cJSON_GetNumberValue(item);
        printf("parse[%lu]|ok=%d|size=%d|str=%s|num=%.17g|pred=%d%d%d%d%d%d%d%d%d%d\n",
               (unsigned long)index,
               item != NULL,
               cJSON_GetArraySize(item),
               string_value ? string_value : "<null>",
               number_value,
               cJSON_IsInvalid(item),
               cJSON_IsFalse(item),
               cJSON_IsTrue(item),
               cJSON_IsBool(item),
               cJSON_IsNull(item),
               cJSON_IsNumber(item),
               cJSON_IsString(item),
               cJSON_IsArray(item),
               cJSON_IsObject(item),
               cJSON_IsRaw(item));
        print_result("parsed", item);
        cJSON_Delete(item);
    }

    {
        static const char with_trailing[] = " [1,2] tail";
        const char *end = NULL;
        cJSON *item = cJSON_ParseWithOpts(with_trailing, &end, 0);
        printf("opts-loose|ok=%d|end=%ld\n", item != NULL, (long)(end - with_trailing));
        cJSON_Delete(item);
        item = cJSON_ParseWithOpts(with_trailing, &end, 1);
        printf("opts-strict|ok=%d|end=%ld|error=%ld\n",
               item != NULL,
               (long)(end - with_trailing),
               (long)(cJSON_GetErrorPtr() - with_trailing));
    }

    {
        static const char bounded[] = {'[', '4', '2', ']', 'x', 'y'};
        const char *end = NULL;
        cJSON *item = cJSON_ParseWithLength(bounded, 4);
        print_result("bounded", item);
        cJSON_Delete(item);
        item = cJSON_ParseWithLengthOpts(bounded, sizeof(bounded), &end, 1);
        printf("length-opts|ok=%d|end=%ld\n", item != NULL, (long)(end - bounded));
    }

    {
        static const char bom[] = "\xEF\xBB\xBF{\"bom\":true}";
        cJSON *item = cJSON_Parse(bom);
        print_result("bom", item);
        cJSON_Delete(item);
    }

    {
        static const char *invalid[] = {
            "",
            "[",
            "[1,]",
            "{\"a\"}",
            "\"\\uD800x\"",
            "tru",
            "{]"
        };
        for (index = 0; index < sizeof(invalid) / sizeof(invalid[0]); index++)
        {
            const char *end = NULL;
            cJSON *item = cJSON_ParseWithOpts(invalid[index], &end, 1);
            printf("invalid[%lu]|ok=%d|end=%ld|error=%ld\n",
                   (unsigned long)index,
                   item != NULL,
                   (long)(end - invalid[index]),
                   (long)(cJSON_GetErrorPtr() - invalid[index]));
            cJSON_Delete(item);
        }
    }
}

static void constructor_cases(void)
{
    int integers[] = {-2, 0, 7};
    float floats[] = {1.5f, -0.25f, 1000.0f};
    double doubles[] = {0.1, 1.0 / 0.0, -2.75};
    const char *strings[] = {"red", "green", "blue"};
    cJSON *array;
    cJSON *object;
    cJSON *item;
    cJSON *detached;
    cJSON *duplicate;
    char overlap[] = "abcdef";

    print_result("null", cJSON_CreateNull());
    item = cJSON_CreateTrue();
    print_result("true", item);
    cJSON_Delete(item);
    item = cJSON_CreateFalse();
    print_result("false", item);
    cJSON_Delete(item);
    item = cJSON_CreateBool(7);
    print_result("bool", item);
    cJSON_Delete(item);
    item = cJSON_CreateNumber(2147483648.0);
    printf("set-number|before=%d|return=%.17g\n", item->valueint, cJSON_SetNumberHelper(item, -4.5));
    print_result("number", item);
    cJSON_Delete(item);
    item = cJSON_CreateString("abcdef");
    printf("set-string|same=%s|overlap=%p\n",
           cJSON_SetValuestring(item, "xy"),
           (void *)cJSON_SetValuestring(item, item->valuestring + 1));
    print_result("string", item);
    cJSON_Delete(item);
    item = cJSON_CreateString(overlap);
    printf("set-string-long=%s\n", cJSON_SetValuestring(item, "a much longer string"));
    cJSON_Delete(item);
    item = cJSON_CreateRaw("{\"raw\":1}");
    print_result("raw", item);
    cJSON_Delete(item);

    array = cJSON_CreateArray();
    cJSON_AddItemToArray(array, cJSON_CreateIntArray(integers, 3));
    cJSON_AddItemToArray(array, cJSON_CreateFloatArray(floats, 3));
    cJSON_AddItemToArray(array, cJSON_CreateDoubleArray(doubles, 3));
    cJSON_AddItemToArray(array, cJSON_CreateStringArray(strings, 3));
    cJSON_InsertItemInArray(array, 1, cJSON_CreateString("inserted"));
    cJSON_ReplaceItemInArray(array, 2, cJSON_CreateString("replaced"));
    print_result("arrays", array);
    printf("array-index|neg=%p|item=%s\n",
           (void *)cJSON_GetArrayItem(array, -1),
           cJSON_GetStringValue(cJSON_GetArrayItem(array, 1)));
    detached = cJSON_DetachItemFromArray(array, 1);
    print_result("detached-array", detached);
    cJSON_Delete(detached);
    cJSON_DeleteItemFromArray(array, 0);
    print_result("array-after-delete", array);

    object = cJSON_CreateObject();
    cJSON_AddNullToObject(object, "null");
    cJSON_AddTrueToObject(object, "true");
    cJSON_AddFalseToObject(object, "false");
    cJSON_AddBoolToObject(object, "bool", 0);
    cJSON_AddNumberToObject(object, "number", 3.125);
    cJSON_AddStringToObject(object, "string", "value");
    cJSON_AddRawToObject(object, "raw", "[9]");
    cJSON_AddObjectToObject(object, "object");
    cJSON_AddArrayToObject(object, "array");
    item = cJSON_CreateString("constant-key");
    cJSON_AddItemToObjectCS(object, "Const", item);
    printf("lookup|casefold=%s|casesens=%p|has=%d\n",
           cJSON_GetStringValue(cJSON_GetObjectItem(object, "STRING")),
           (void *)cJSON_GetObjectItemCaseSensitive(object, "STRING"),
           cJSON_HasObjectItem(object, "number"));
    cJSON_ReplaceItemInObject(object, "string", cJSON_CreateString("new"));
    cJSON_ReplaceItemInObjectCaseSensitive(object, "number", cJSON_CreateNumber(4));
    print_result("object", object);

    detached = cJSON_DetachItemFromObject(object, "TRUE");
    cJSON_Delete(detached);
    detached = cJSON_DetachItemFromObjectCaseSensitive(object, "false");
    cJSON_Delete(detached);
    cJSON_DeleteItemFromObject(object, "BOOL");
    cJSON_DeleteItemFromObjectCaseSensitive(object, "raw");
    print_result("object-after-delete", object);

    duplicate = cJSON_Duplicate(object, 1);
    printf("duplicate|equal=%d|same=%d\n",
           cJSON_Compare(object, duplicate, 1),
           object == duplicate);
    cJSON_Delete(duplicate);
    duplicate = cJSON_Duplicate_rec(object, 0, 1);
    printf("duplicate-rec|equal=%d\n", cJSON_Compare(object, duplicate, 0));
    cJSON_Delete(duplicate);

    item = cJSON_CreateStringReference("referenced");
    cJSON_AddItemReferenceToArray(array, item);
    cJSON_AddItemReferenceToObject(object, "reference", item);
    print_result("references-array", array);
    print_result("references-object", object);
    cJSON_Delete(item);

    {
        cJSON *array_reference = cJSON_CreateArrayReference(array->child);
        cJSON *object_reference = cJSON_CreateObjectReference(object->child);
        print_result("array-reference", array_reference);
        print_result("object-reference", object_reference);
        cJSON_Delete(array_reference);
        cJSON_Delete(object_reference);
    }

    item = cJSON_GetArrayItem(array, 0);
    detached = cJSON_DetachItemViaPointer(array, item);
    cJSON_Delete(detached);
    item = cJSON_CreateString("replacement-pointer");
    cJSON_ReplaceItemViaPointer(array, cJSON_GetArrayItem(array, 0), item);
    print_result("pointer-mutations", array);

    cJSON_Delete(array);
    cJSON_Delete(object);
}

static void minify_and_hooks(void)
{
    char json[] = " { /* x */ \"a\" : 1, // y\n \"b\" : \"x y\\\"z\" } ";
    cJSON_Hooks hooks;
    cJSON *item;
    char *printed;

    cJSON_Minify(json);
    printf("minify=%s\n", json);

    allocations = 0;
    frees = 0;
    hooks.malloc_fn = counting_malloc;
    hooks.free_fn = counting_free;
    cJSON_InitHooks(&hooks);
    item = cJSON_Parse("{\"hook\":[1,2,3]}");
    printed = cJSON_PrintUnformatted(item);
    printf("hooks|json=%s|alloc-before-free=%lu|free-before-delete=%lu\n",
           printed,
           (unsigned long)allocations,
           (unsigned long)frees);
    cJSON_free(printed);
    cJSON_Delete(item);
    printf("hooks|alloc=%lu|free=%lu\n",
           (unsigned long)allocations,
           (unsigned long)frees);
    cJSON_InitHooks(NULL);

    printed = cJSON_malloc(4);
    memcpy(printed, "ok", 3);
    printf("malloc=%s\n", printed);
    cJSON_free(printed);
}

static void driver_case(void)
{
    const char *strings[7] = {
        "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"
    };
    int numbers[3][3] = {
        {0, 1, 2},
        {3, 4, 5},
        {6, 7, 8}
    };
    int ids[4] = {116, 943, 234, 38793};
    struct record fields[2] = {
        {"zip", 37.7668, -122.3959, "", "SAN FRANCISCO", "CA", "94107", "US"},
        {"zip", 37.371991, -122.026020, "", "SUNNYVALE", "CA", "94085", "US"}
    };
    printf("driver-return=%d\n", driver(strings, numbers, ids, fields));
}

int main(void)
{
    printf("version=%s\n", cJSON_Version());
    parse_cases();
    constructor_cases();
    minify_and_hooks();
    driver_case();
    return 0;
}
