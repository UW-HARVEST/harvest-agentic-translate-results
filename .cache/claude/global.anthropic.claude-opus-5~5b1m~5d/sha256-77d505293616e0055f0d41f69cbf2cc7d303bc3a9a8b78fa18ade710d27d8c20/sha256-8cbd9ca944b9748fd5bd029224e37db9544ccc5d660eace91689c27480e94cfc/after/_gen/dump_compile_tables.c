/* Dump the static tables of pcre2_compile.c as Rust source. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2_compile.c"

#include <stdio.h>

#define ARRLEN(a) (sizeof(a)/sizeof((a)[0]))

static FILE *out;

static void bytes(const char *name, const unsigned char *a, size_t n)
{
  fprintf(out, "pub static %s: [u8; %zu] = [\n", name, n);
  for (size_t i = 0; i < n; i++)
    fprintf(out, "%s0x%02x,%s", (i % 12 == 0) ? "  " : " ", (unsigned)a[i],
            (i % 12 == 11) ? "\n" : "");
  fprintf(out, "\n];\n\n");
}

int main(void)
{
  out = fopen("../translation/src/compile_tables.rs", "w");
  fprintf(out, "// Auto-generated from the static tables of pcre2_compile.c by\n"
               "// _gen/dump_compile_tables.c. Do not edit.\n"
               "#![allow(dead_code, non_upper_case_globals)]\n\n");

  fprintf(out, "/* Constants from the top of pcre2_compile.c */\n");
  fprintf(out, "pub const MAX_REPEAT_COUNT: u32 = %uu32;\n", MAX_REPEAT_COUNT);
  fprintf(out, "pub const REPEAT_UNLIMITED: u32 = %uu32;\n", REPEAT_UNLIMITED);
  fprintf(out, "pub const COMPILE_WORK_SIZE: usize = %d;\n", (int)COMPILE_WORK_SIZE);
  fprintf(out, "pub const C16_WORK_SIZE: usize = %d;\n", (int)C16_WORK_SIZE);
  fprintf(out, "pub const WORK_SIZE_SAFETY_MARGIN: usize = %d;\n", (int)WORK_SIZE_SAFETY_MARGIN);
  fprintf(out, "pub const NAMED_GROUP_LIST_SIZE: usize = %d;\n", (int)NAMED_GROUP_LIST_SIZE);
  fprintf(out, "pub const PARSED_PATTERN_DEFAULT_SIZE: usize = %d;\n",
          (int)PARSED_PATTERN_DEFAULT_SIZE);
  fprintf(out, "pub const OFLOW_MAX: i32 = %d;\n", (int)OFLOW_MAX);
  fprintf(out, "pub const REQ_UNSET: u32 = 0x%08xu32;\n", REQ_UNSET);
  fprintf(out, "pub const REQ_NONE: u32 = 0x%08xu32;\n", REQ_NONE);
  fprintf(out, "pub const REQ_CASELESS: u32 = 0x%08xu32;\n", REQ_CASELESS);
  fprintf(out, "pub const REQ_VARY: u32 = 0x%08xu32;\n", REQ_VARY);
  fprintf(out, "pub const GI_SET_FIXED_LENGTH: u32 = 0x%08xu32;\n", GI_SET_FIXED_LENGTH);
  fprintf(out, "pub const GI_NOT_FIXED_LENGTH: u32 = 0x%08xu32;\n", GI_NOT_FIXED_LENGTH);
  fprintf(out, "pub const GI_FIXED_LENGTH_MASK: u32 = 0x%08xu32;\n", GI_FIXED_LENGTH_MASK);
  fprintf(out, "pub const ESCAPES_FIRST: u32 = %d;\n", (int)ESCAPES_FIRST);
  fprintf(out, "pub const ESCAPES_LAST: u32 = %d;\n", (int)ESCAPES_LAST);
  fprintf(out, "pub const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = 0x%08xu32;\n",
          PUBLIC_LITERAL_COMPILE_OPTIONS);
  fprintf(out, "pub const PUBLIC_COMPILE_OPTIONS: u32 = 0x%08xu32;\n", PUBLIC_COMPILE_OPTIONS);
  fprintf(out, "pub const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = 0x%08xu32;\n",
          PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS);
  fprintf(out, "pub const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = 0x%08xu32;\n",
          PUBLIC_COMPILE_EXTRA_OPTIONS);
  fprintf(out, "pub const PSKIP_ALT: u32 = %d;\n", PSKIP_ALT);
  fprintf(out, "pub const PSKIP_CLASS: u32 = %d;\n", PSKIP_CLASS);
  fprintf(out, "pub const PSKIP_KET: u32 = %d;\n", PSKIP_KET);
  fprintf(out, "pub const PSO_OPT: u32 = %d;\n", PSO_OPT);
  fprintf(out, "pub const PSO_XOPT: u32 = %d;\n", PSO_XOPT);
  fprintf(out, "pub const PSO_FLG: u32 = %d;\n", PSO_FLG);
  fprintf(out, "pub const PSO_NL: u32 = %d;\n", PSO_NL);
  fprintf(out, "pub const PSO_BSR: u32 = %d;\n", PSO_BSR);
  fprintf(out, "pub const PSO_LIMH: u32 = %d;\n", PSO_LIMH);
  fprintf(out, "pub const PSO_LIMM: u32 = %d;\n", PSO_LIMM);
  fprintf(out, "pub const PSO_LIMD: u32 = %d;\n", PSO_LIMD);
  fprintf(out, "pub const PSO_OPTMZ: u32 = %d;\n\n", PSO_OPTMZ);

  bytes("meta_extra_lengths", meta_extra_lengths, ARRLEN(meta_extra_lengths));
  bytes("xdigitab", xdigitab, ARRLEN(xdigitab));

  fprintf(out, "pub static escapes: [i16; %zu] = [\n", ARRLEN(escapes));
  for (size_t i = 0; i < ARRLEN(escapes); i++)
    fprintf(out, "%s%d,%s", (i % 10 == 0) ? "  " : " ", (int)escapes[i],
            (i % 10 == 9) ? "\n" : "");
  fprintf(out, "\n];\n\n");

  bytes("verbnames", (const unsigned char *)verbnames, sizeof(verbnames));
  fprintf(out, "#[repr(C)]\npub struct verbitem { pub len: u32, pub meta: u32, pub has_arg: i32 }\n");
  fprintf(out, "pub static verbs: [verbitem; %zu] = [\n", ARRLEN(verbs));
  for (size_t i = 0; i < ARRLEN(verbs); i++)
    fprintf(out, "  verbitem { len: %u, meta: 0x%08x, has_arg: %d },\n",
            verbs[i].len, verbs[i].meta, verbs[i].has_arg);
  fprintf(out, "];\npub const verbcount: i32 = %d;\n\n", verbcount);

  fprintf(out, "pub static verbops: [u32; %zu] = [\n", ARRLEN(verbops));
  for (size_t i = 0; i < ARRLEN(verbops); i++) fprintf(out, "  %u,\n", verbops[i]);
  fprintf(out, "];\n\n");

  bytes("alasnames", (const unsigned char *)alasnames, sizeof(alasnames));
  fprintf(out, "#[repr(C)]\npub struct alasitem { pub len: u32, pub meta: u32 }\n");
  fprintf(out, "pub static alasmeta: [alasitem; %zu] = [\n", ARRLEN(alasmeta));
  for (size_t i = 0; i < ARRLEN(alasmeta); i++)
    fprintf(out, "  alasitem { len: %u, meta: 0x%08x },\n", alasmeta[i].len, alasmeta[i].meta);
  fprintf(out, "];\npub const alascount: i32 = %d;\n\n", alascount);

  fprintf(out, "pub static chartypeoffset: [u32; %zu] = [", ARRLEN(chartypeoffset));
  for (size_t i = 0; i < ARRLEN(chartypeoffset); i++) fprintf(out, " %u,", chartypeoffset[i]);
  fprintf(out, " ];\n\n");

  bytes("posix_names", (const unsigned char *)posix_names, sizeof(posix_names));
  bytes("posix_name_lengths", posix_name_lengths, ARRLEN(posix_name_lengths));

  fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_posix_class_maps8: [i32; %zu] = [\n",
          ARRLEN(PRIV(posix_class_maps)));
  for (size_t i = 0; i < ARRLEN(PRIV(posix_class_maps)); i++)
    fprintf(out, "%s%d,%s", (i % 3 == 0) ? "  " : " ", PRIV(posix_class_maps)[i],
            (i % 3 == 2) ? "\n" : "");
  fprintf(out, "];\n\n");

  fprintf(out, "pub static posix_substitutes: [i32; %zu] = [\n", ARRLEN(posix_substitutes));
  for (size_t i = 0; i < ARRLEN(posix_substitutes); i++)
    fprintf(out, "%s%d,%s", (i % 2 == 0) ? "  " : " ", posix_substitutes[i],
            (i % 2 == 1) ? "\n" : "");
  fprintf(out, "];\n\n");

  /* pso_list: names are pointers to string literals; emit each name as its own
     byte array (NUL-terminated) and build the table from their addresses. */
  fprintf(out, "#[repr(C)]\npub struct pso { pub name: *const u8, pub length: u16,\n"
               "    pub type_: u16, pub value: u32 }\nunsafe impl Sync for pso {}\n\n");
  for (size_t i = 0; i < ARRLEN(pso_list); i++)
    {
    size_t n = strlen(pso_list[i].name);
    fprintf(out, "static PSO_NAME_%zu: [u8; %zu] = [", i, n + 1);
    for (size_t j = 0; j < n; j++)
      fprintf(out, "0x%02x,", (unsigned)(unsigned char)pso_list[i].name[j]);
    fprintf(out, "0x00];  // \"%s\"\n", pso_list[i].name);
    }
  fprintf(out, "\npub static pso_list: [pso; %zu] = [\n", ARRLEN(pso_list));
  for (size_t i = 0; i < ARRLEN(pso_list); i++)
    fprintf(out, "  pso { name: PSO_NAME_%zu.as_ptr(), length: %u, type_: %u, value: 0x%08x },\n",
            i, pso_list[i].length, pso_list[i].type, pso_list[i].value);
  fprintf(out, "];\npub const PSO_LIST_COUNT: usize = %zu;\n\n", ARRLEN(pso_list));

  bytes("opcode_possessify", opcode_possessify, ARRLEN(opcode_possessify));

  fclose(out);
  printf("ok escapes=%zu meta_extra=%zu opcode_poss=%zu pso=%zu verbnames=%zu alasnames=%zu\n",
         ARRLEN(escapes), ARRLEN(meta_extra_lengths), ARRLEN(opcode_possessify),
         ARRLEN(pso_list), sizeof(verbnames), sizeof(alasnames));
  return 0;
}
