#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2_internal.h"
#include "pcre2_compile.h"
#include <stdio.h>
#include <stddef.h>
int main(void) {
#define P(t) printf("size %s = %zu align %zu\n", #t, sizeof(t), _Alignof(t))
  P(pcre2_memctl); P(pcre2_real_general_context); P(pcre2_real_compile_context);
  P(pcre2_real_match_context); P(pcre2_real_convert_context); P(pcre2_real_code);
  P(pcre2_callout_block); P(pcre2_callout_enumerate_block); P(pcre2_substitute_callout_block);
  P(ucd_record); P(ucp_type_table); P(pcre2_serialized_data); P(named_group);
  P(compile_block); P(match_block); P(dfa_match_block); P(class_ranges);
  P(recurse_arguments); P(eclass_op_info); printf("size heapframe_fields = %zu align %zu\n", sizeof(((heapframe*)0)->fields), _Alignof(((heapframe*)0)->fields)); P(pcre2_real_jit_stack);
  printf("offset match_data.ovector = %zu\n", offsetof(pcre2_real_match_data, ovector));
  printf("offset heapframe.eptr = %zu\n", offsetof(heapframe, eptr));
  printf("offset heapframe.ovector = %zu\n", offsetof(heapframe, ovector));
  printf("offset heapframe.fields = %zu\n", offsetof(heapframe, fields));
  printf("align heapframe = %zu\n", (size_t)HEAPFRAME_ALIGNMENT);
  printf("offset code.start_bitmap = %zu\n", offsetof(pcre2_real_code, tables));
  printf("offset code.optimization_flags = %zu\n", offsetof(pcre2_real_code, optimization_flags));
  printf("offset compile_block.classbits = %zu\n", offsetof(compile_block, classbits));
  printf("offset compile_block.char_lists_size = %zu\n", offsetof(compile_block, char_lists_size));
  printf("offset match_block.callout = %zu\n", offsetof(match_block, callout));
  printf("offset dfa_match_block.recursive = %zu\n", offsetof(dfa_match_block, recursive));
  return 0;
}
