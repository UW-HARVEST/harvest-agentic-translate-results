#define PCRE2_CODE_UNIT_WIDTH 8
#include "../../c_src/include/pcre2.h"

#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define LOAD(name) \
  __typeof__(&name) f_##name __attribute__((unused)) = \
    (__typeof__(&name))load_symbol(handle, #name)

static void *load_symbol(void *handle, const char *name)
{
  void *symbol = dlsym(handle, name);
  if (symbol == NULL)
    {
    fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
    exit(2);
    }
  return symbol;
}

static uint64_t hash_bytes(const uint8_t *bytes, size_t length)
{
  uint64_t hash = UINT64_C(1469598103934665603);
  size_t index;
  for (index = 0; index < length; index++)
    {
    hash ^= bytes[index];
    hash *= UINT64_C(1099511628211);
    }
  return hash;
}

static void *test_malloc(size_t size, void *memory_data)
{
  (void)memory_data;
  return malloc(size);
}

static void test_free(void *pointer, void *memory_data)
{
  (void)memory_data;
  free(pointer);
}

static void print_match(
  const char *label,
  pcre2_code_8 *code,
  const uint8_t *subject,
  size_t subject_length,
  uint32_t options,
  pcre2_match_data_8 *match_data,
  __typeof__(&pcre2_match_8) match_function,
  __typeof__(&pcre2_get_ovector_pointer_8) ovector_function)
{
  int result = match_function(
    code, subject, subject_length, 0, options, match_data, NULL);
  printf("%s rc=%d", label, result);
  if (result >= 0)
    {
    PCRE2_SIZE *ovector = ovector_function(match_data);
    int index;
    for (index = 0; index < result * 2; index++)
      printf(" %zu", ovector[index]);
    }
  putchar('\n');
}

int main(int argc, char **argv)
{
  void *handle;
  int selector;
  int error_code;
  PCRE2_SIZE error_offset;
  pcre2_code_8 *code;
  pcre2_match_data_8 *match_data;

  if (argc != 2)
    {
    fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
    return 2;
    }

  handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
  if (handle == NULL)
    {
    fprintf(stderr, "dlopen failed: %s\n", dlerror());
    return 2;
    }

  LOAD(pcre2_config_8);
  LOAD(pcre2_get_error_message_8);
  LOAD(pcre2_compile_8);
  LOAD(pcre2_code_free_8);
  LOAD(pcre2_code_copy_8);
  LOAD(pcre2_code_copy_with_tables_8);
  LOAD(pcre2_pattern_info_8);
  LOAD(pcre2_callout_enumerate_8);
  LOAD(pcre2_match_data_create_8);
  LOAD(pcre2_match_data_create_from_pattern_8);
  LOAD(pcre2_match_data_free_8);
  LOAD(pcre2_match_8);
  LOAD(pcre2_jit_match_8);
  LOAD(pcre2_dfa_match_8);
  LOAD(pcre2_get_mark_8);
  LOAD(pcre2_get_ovector_pointer_8);
  LOAD(pcre2_get_ovector_count_8);
  LOAD(pcre2_get_match_data_size_8);
  LOAD(pcre2_get_match_data_heapframes_size_8);
  LOAD(pcre2_get_startchar_8);
  LOAD(pcre2_next_match_8);
  LOAD(pcre2_substring_copy_bynumber_8);
  LOAD(pcre2_substring_copy_byname_8);
  LOAD(pcre2_substring_get_byname_8);
  LOAD(pcre2_substring_get_bynumber_8);
  LOAD(pcre2_substring_length_byname_8);
  LOAD(pcre2_substring_length_bynumber_8);
  LOAD(pcre2_substring_nametable_scan_8);
  LOAD(pcre2_substring_number_from_name_8);
  LOAD(pcre2_substring_free_8);
  LOAD(pcre2_substring_list_get_8);
  LOAD(pcre2_substring_list_free_8);
  LOAD(pcre2_substitute_8);
  LOAD(pcre2_pattern_convert_8);
  LOAD(pcre2_converted_pattern_free_8);
  LOAD(pcre2_serialize_encode_8);
  LOAD(pcre2_serialize_decode_8);
  LOAD(pcre2_serialize_get_number_of_codes_8);
  LOAD(pcre2_serialize_free_8);
  LOAD(pcre2_general_context_create_8);
  LOAD(pcre2_general_context_copy_8);
  LOAD(pcre2_general_context_free_8);
  LOAD(pcre2_compile_context_create_8);
  LOAD(pcre2_compile_context_copy_8);
  LOAD(pcre2_compile_context_free_8);
  LOAD(pcre2_set_bsr_8);
  LOAD(pcre2_set_character_tables_8);
  LOAD(pcre2_set_newline_8);
  LOAD(pcre2_set_max_pattern_length_8);
  LOAD(pcre2_set_max_pattern_compiled_length_8);
  LOAD(pcre2_set_max_varlookbehind_8);
  LOAD(pcre2_set_parens_nest_limit_8);
  LOAD(pcre2_set_compile_extra_options_8);
  LOAD(pcre2_set_compile_recursion_guard_8);
  LOAD(pcre2_set_optimize_8);
  LOAD(pcre2_match_context_create_8);
  LOAD(pcre2_match_context_copy_8);
  LOAD(pcre2_match_context_free_8);
  LOAD(pcre2_set_depth_limit_8);
  LOAD(pcre2_set_heap_limit_8);
  LOAD(pcre2_set_match_limit_8);
  LOAD(pcre2_set_offset_limit_8);
  LOAD(pcre2_set_recursion_limit_8);
  LOAD(pcre2_set_recursion_memory_management_8);
  LOAD(pcre2_set_callout_8);
  LOAD(pcre2_set_substitute_callout_8);
  LOAD(pcre2_set_substitute_case_callout_8);
  LOAD(pcre2_convert_context_create_8);
  LOAD(pcre2_convert_context_copy_8);
  LOAD(pcre2_convert_context_free_8);
  LOAD(pcre2_set_glob_escape_8);
  LOAD(pcre2_set_glob_separator_8);
  LOAD(pcre2_jit_compile_8);
  LOAD(pcre2_jit_free_unused_memory_8);
  LOAD(pcre2_jit_stack_create_8);
  LOAD(pcre2_jit_stack_assign_8);
  LOAD(pcre2_jit_stack_free_8);
  LOAD(pcre2_maketables_8);
  LOAD(pcre2_maketables_free_8);

  for (selector = 0; selector <= 16; selector++)
    {
    if (selector == PCRE2_CONFIG_JITTARGET ||
        selector == PCRE2_CONFIG_UNICODE_VERSION ||
        selector == PCRE2_CONFIG_VERSION)
      {
      int length = f_pcre2_config_8((uint32_t)selector, NULL);
      uint8_t buffer[128] = { 0 };
      int result = length > 0
        ? f_pcre2_config_8((uint32_t)selector, buffer)
        : length;
      printf("config[%d] len=%d rc=%d text=%s\n",
        selector, length, result, result >= 0 ? (char *)buffer : "-");
      }
    else
      {
      uint32_t value = UINT32_C(0xdeadbeef);
      int result = f_pcre2_config_8((uint32_t)selector, &value);
      printf("config[%d] rc=%d value=%u\n", selector, result, value);
      }
    }
  printf("config[999] rc=%d\n", f_pcre2_config_8(999, NULL));

  for (error_code = -67; error_code <= 220; error_code++)
    {
    uint8_t message[256] = { 0 };
    int result = f_pcre2_get_error_message_8(error_code, message, sizeof(message));
    printf("error[%d] rc=%d hash=%016llx\n", error_code, result,
      (unsigned long long)hash_bytes(message, result > 0 ? (size_t)result : 0));
    }

  {
    static const uint8_t bad_pattern[] = "(?<x>a";
    code = f_pcre2_compile_8(
      bad_pattern, sizeof(bad_pattern) - 1, 0, &error_code, &error_offset, NULL);
    printf("bad_compile null=%d error=%d offset=%zu\n",
      code == NULL, error_code, error_offset);
  }

  {
    static const uint8_t pattern[] =
      "(?<word>\\p{L}+)-(\\d+)|(?i:abc)(*MARK:hit)";
    code = f_pcre2_compile_8(
      pattern, sizeof(pattern) - 1, PCRE2_UTF | PCRE2_UCP,
      &error_code, &error_offset, NULL);
    printf("compile null=%d error=%d offset=%zu\n",
      code == NULL, error_code, error_offset);
  }
  if (code == NULL) return 3;

  {
    static const uint32_t selectors[] = {
      PCRE2_INFO_ALLOPTIONS, PCRE2_INFO_ARGOPTIONS, PCRE2_INFO_BACKREFMAX,
      PCRE2_INFO_BSR, PCRE2_INFO_CAPTURECOUNT, PCRE2_INFO_FIRSTCODEUNIT,
      PCRE2_INFO_FIRSTCODETYPE, PCRE2_INFO_HASCRORLF, PCRE2_INFO_JCHANGED,
      PCRE2_INFO_JITSIZE, PCRE2_INFO_LASTCODEUNIT, PCRE2_INFO_LASTCODETYPE,
      PCRE2_INFO_MATCHEMPTY, PCRE2_INFO_MATCHLIMIT, PCRE2_INFO_MAXLOOKBEHIND,
      PCRE2_INFO_MINLENGTH, PCRE2_INFO_NAMECOUNT, PCRE2_INFO_NAMEENTRYSIZE,
      PCRE2_INFO_NEWLINE, PCRE2_INFO_DEPTHLIMIT, PCRE2_INFO_SIZE,
      PCRE2_INFO_HASBACKSLASHC, PCRE2_INFO_FRAMESIZE, PCRE2_INFO_HEAPLIMIT,
      PCRE2_INFO_EXTRAOPTIONS
    };
    size_t index;
    for (index = 0; index < sizeof(selectors) / sizeof(selectors[0]); index++)
      {
      uint64_t value = 0;
      int result = f_pcre2_pattern_info_8(code, selectors[index], &value);
      printf("info[%u] rc=%d value=%llu\n", selectors[index], result,
        (unsigned long long)value);
      }
  }

  match_data = f_pcre2_match_data_create_from_pattern_8(code, NULL);
  printf("match_data null=%d size=%zu heap=%zu count=%u\n",
    match_data == NULL,
    f_pcre2_get_match_data_size_8(match_data),
    f_pcre2_get_match_data_heapframes_size_8(match_data),
    f_pcre2_get_ovector_count_8(match_data));

  {
    static const uint8_t subject1[] = "prefix caf\xc3\xa9-123 suffix";
    static const uint8_t subject2[] = "zzAbCyy";
    static const uint8_t invalid_utf[] = { 0xc3, 0x28 };
    print_match("match1", code, subject1, sizeof(subject1) - 1, 0, match_data,
      f_pcre2_match_8, f_pcre2_get_ovector_pointer_8);
    printf("startchar=%zu\n", f_pcre2_get_startchar_8(match_data));
    print_match("match2", code, subject2, sizeof(subject2) - 1, 0, match_data,
      f_pcre2_match_8, f_pcre2_get_ovector_pointer_8);
    print_match("invalid_utf", code, invalid_utf, sizeof(invalid_utf), 0, match_data,
      f_pcre2_match_8, f_pcre2_get_ovector_pointer_8);
  }

  {
    uint8_t output[64] = { 0 };
    PCRE2_SIZE output_length = sizeof(output);
    int result = f_pcre2_substring_copy_bynumber_8(
      match_data, 0, output, &output_length);
    printf("substring0 rc=%d len=%zu text=%.*s\n",
      result, output_length, (int)output_length, output);
    output_length = sizeof(output);
    result = f_pcre2_substring_copy_byname_8(
      match_data, (const uint8_t *)"word", output, &output_length);
    printf("substring_word rc=%d len=%zu number=%d\n",
      result, output_length,
      f_pcre2_substring_number_from_name_8(code, (const uint8_t *)"word"));
  }

  {
    int workspace[128] = { 0 };
    static const uint8_t subject[] = "abc";
    int result = f_pcre2_dfa_match_8(
      code, subject, sizeof(subject) - 1, 0, 0, match_data, NULL,
      workspace, sizeof(workspace) / sizeof(workspace[0]));
    PCRE2_SIZE *ovector = f_pcre2_get_ovector_pointer_8(match_data);
    printf("dfa rc=%d first=%zu,%zu\n", result, ovector[0], ovector[1]);
  }

  {
    PCRE2_SIZE start_offset = 0;
    uint32_t options = 0;
    int result = f_pcre2_next_match_8(match_data, &start_offset, &options);
    printf("next rc=%d start=%zu options=%u\n", result, start_offset, options);
  }

  {
    static const uint8_t pattern[] = "(?<item>[a-z]+)";
    static const uint8_t subject[] = "one 22 two";
    static const uint8_t replacement[] = "<${item}:$0>";
    uint8_t output[128] = { 0 };
    PCRE2_SIZE output_length = sizeof(output);
    pcre2_code_8 *sub_code = f_pcre2_compile_8(
      pattern, sizeof(pattern) - 1, 0, &error_code, &error_offset, NULL);
    int result = f_pcre2_substitute_8(
      sub_code, subject, sizeof(subject) - 1, 0, PCRE2_SUBSTITUTE_GLOBAL,
      NULL, NULL, replacement, sizeof(replacement) - 1, output, &output_length);
    printf("substitute rc=%d len=%zu text=%.*s\n",
      result, output_length, (int)output_length, output);
    f_pcre2_code_free_8(sub_code);
  }

  {
    static const uint8_t glob[] = "src/**/[a-z]?\\*.c";
    uint8_t *converted = NULL;
    PCRE2_SIZE converted_length = 0;
    int result = f_pcre2_pattern_convert_8(
      glob, sizeof(glob) - 1, PCRE2_CONVERT_GLOB,
      &converted, &converted_length, NULL);
    printf("convert rc=%d len=%zu text=%.*s\n",
      result, converted_length, (int)converted_length,
      converted == NULL ? (uint8_t *)"" : converted);
    f_pcre2_converted_pattern_free_8(converted);
  }

  {
    const pcre2_code_8 *codes[1] = { code };
    uint8_t *serialized = NULL;
    PCRE2_SIZE serialized_size = 0;
    pcre2_code_8 *decoded = NULL;
    int32_t result = f_pcre2_serialize_encode_8(
      codes, 1, &serialized, &serialized_size, NULL);
    printf("serialize rc=%d size=%zu count=%d hash=%016llx\n",
      result, serialized_size,
      f_pcre2_serialize_get_number_of_codes_8(serialized),
      (unsigned long long)hash_bytes(serialized, serialized_size));
    result = f_pcre2_serialize_decode_8(&decoded, 1, serialized, NULL);
    printf("deserialize rc=%d null=%d\n", result, decoded == NULL);
    print_match("decoded_match", decoded, (const uint8_t *)"ABC", 3, 0,
      match_data, f_pcre2_match_8, f_pcre2_get_ovector_pointer_8);
    f_pcre2_code_free_8(decoded);
    f_pcre2_serialize_free_8(serialized);
  }

  {
    pcre2_code_8 *copy = f_pcre2_code_copy_8(code);
    pcre2_code_8 *copy_tables = f_pcre2_code_copy_with_tables_8(code);
    printf("copies null=%d,%d\n", copy == NULL, copy_tables == NULL);
    f_pcre2_code_free_8(copy);
    f_pcre2_code_free_8(copy_tables);
  }

  {
    pcre2_general_context_8 *general =
      f_pcre2_general_context_create_8(test_malloc, test_free, NULL);
    pcre2_general_context_8 *general_copy =
      f_pcre2_general_context_copy_8(general);
    pcre2_compile_context_8 *compile_context =
      f_pcre2_compile_context_create_8(general);
    pcre2_compile_context_8 *compile_copy;
    pcre2_match_context_8 *match_context =
      f_pcre2_match_context_create_8(general);
    pcre2_match_context_8 *match_copy;
    pcre2_convert_context_8 *convert_context =
      f_pcre2_convert_context_create_8(general);
    pcre2_convert_context_8 *convert_copy;
    printf("compile_setters %d %d %d %d %d %d %d %d\n",
      f_pcre2_set_bsr_8(compile_context, PCRE2_BSR_ANYCRLF),
      f_pcre2_set_newline_8(compile_context, PCRE2_NEWLINE_ANYCRLF),
      f_pcre2_set_max_pattern_length_8(compile_context, 12345),
      f_pcre2_set_max_pattern_compiled_length_8(compile_context, 54321),
      f_pcre2_set_max_varlookbehind_8(compile_context, 99),
      f_pcre2_set_parens_nest_limit_8(compile_context, 77),
      f_pcre2_set_compile_extra_options_8(compile_context, 0),
      f_pcre2_set_optimize_8(compile_context, 0));
    printf("match_setters %d %d %d %d %d\n",
      f_pcre2_set_depth_limit_8(match_context, 10),
      f_pcre2_set_heap_limit_8(match_context, 20),
      f_pcre2_set_match_limit_8(match_context, 30),
      f_pcre2_set_offset_limit_8(match_context, 40),
      f_pcre2_set_recursion_limit_8(match_context, 50));
    printf("convert_setters %d %d\n",
      f_pcre2_set_glob_escape_8(convert_context, '\\'),
      f_pcre2_set_glob_separator_8(convert_context, '/'));
    compile_copy = f_pcre2_compile_context_copy_8(compile_context);
    match_copy = f_pcre2_match_context_copy_8(match_context);
    convert_copy = f_pcre2_convert_context_copy_8(convert_context);
    printf("contexts null=%d%d%d%d%d%d%d\n",
      general == NULL, general_copy == NULL, compile_copy == NULL,
      match_copy == NULL, convert_copy == NULL,
      compile_context == NULL, match_context == NULL);
    f_pcre2_compile_context_free_8(compile_copy);
    f_pcre2_compile_context_free_8(compile_context);
    f_pcre2_match_context_free_8(match_copy);
    f_pcre2_match_context_free_8(match_context);
    f_pcre2_convert_context_free_8(convert_copy);
    f_pcre2_convert_context_free_8(convert_context);
    f_pcre2_general_context_free_8(general_copy);
    f_pcre2_general_context_free_8(general);
  }

  {
    const uint8_t *tables = f_pcre2_maketables_8(NULL);
    printf("tables hash=%016llx\n",
      (unsigned long long)hash_bytes(tables, 1088));
    f_pcre2_maketables_free_8(NULL, tables);
  }

  printf("jit_compile=%d\n", f_pcre2_jit_compile_8(code, PCRE2_JIT_COMPLETE));
  f_pcre2_match_data_free_8(match_data);
  f_pcre2_code_free_8(code);
  dlclose(handle);
  return 0;
}
