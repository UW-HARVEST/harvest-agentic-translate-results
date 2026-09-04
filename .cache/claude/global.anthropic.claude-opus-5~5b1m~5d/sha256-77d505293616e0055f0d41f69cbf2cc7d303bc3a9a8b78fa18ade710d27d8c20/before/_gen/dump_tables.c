/* Dumps the PCRE2 static data tables as Rust source, so that the Rust
   translation contains byte-identical data. Compiled with the same flags as
   the C library. */

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2_internal.h"

#include "pcre2_tables.c"
#include "pcre2_ucd.c"
#include "pcre2_chartables.c"

#include <stdio.h>

#define ARRLEN(a) (sizeof(a)/sizeof((a)[0]))

static FILE *out;

static void hdr(const char *what)
{
  fprintf(out, "// Auto-generated from %s by _gen/dump_tables.c. Do not edit.\n", what);
  fprintf(out, "#![allow(dead_code, non_upper_case_globals)]\n\n");
}

static void dump_u8(const char *name, const uint8_t *a, size_t n)
{
  fprintf(out, "#[unsafe(no_mangle)]\npub static %s: [u8; %zu] = [\n", name, n);
  for (size_t i = 0; i < n; i++)
    fprintf(out, "%s0x%02x,%s", (i % 12 == 0) ? "  " : " ", (unsigned)a[i],
            (i % 12 == 11) ? "\n" : "");
  fprintf(out, "\n];\n\n");
}

static void dump_u16(const char *name, const uint16_t *a, size_t n)
{
  fprintf(out, "#[unsafe(no_mangle)]\npub static %s: [u16; %zu] = [\n", name, n);
  for (size_t i = 0; i < n; i++)
    fprintf(out, "%s0x%04x,%s", (i % 10 == 0) ? "  " : " ", (unsigned)a[i],
            (i % 10 == 9) ? "\n" : "");
  fprintf(out, "\n];\n\n");
}

static void dump_u32(const char *name, const uint32_t *a, size_t n)
{
  fprintf(out, "#[unsafe(no_mangle)]\npub static %s: [u32; %zu] = [\n", name, n);
  for (size_t i = 0; i < n; i++)
    fprintf(out, "%s0x%08x,%s", (i % 8 == 0) ? "  " : " ", (unsigned)a[i],
            (i % 8 == 7) ? "\n" : "");
  fprintf(out, "\n];\n\n");
}

static void dump_i32(const char *name, const int *a, size_t n)
{
  fprintf(out, "#[unsafe(no_mangle)]\npub static %s: [i32; %zu] = [\n", name, n);
  for (size_t i = 0; i < n; i++)
    fprintf(out, "%s%d,%s", (i % 8 == 0) ? "  " : " ", a[i], (i % 8 == 7) ? "\n" : "");
  fprintf(out, "\n];\n\n");
}

int main(void)
{
  /* ---------------- chartables.rs ---------------- */
  out = fopen("../translation/src/chartables.rs", "w");
  hdr("pcre2_chartables.c");
  dump_u8("_pcre2_default_tables_8", PRIV(default_tables), TABLES_LENGTH);
  fclose(out);

  /* ---------------- tables.rs ---------------- */
  out = fopen("../translation/src/tables.rs", "w");
  hdr("pcre2_tables.c and pcre2_ucptables_inc.h");
  fprintf(out, "use crate::types::ucp_type_table;\n\n");
  dump_u8("_pcre2_OP_lengths_8", PRIV(OP_lengths), OP_TABLE_LENGTH);
  dump_u32("_pcre2_hspace_list_8", PRIV(hspace_list), ARRLEN(PRIV(hspace_list)));
  dump_u32("_pcre2_vspace_list_8", PRIV(vspace_list), ARRLEN(PRIV(vspace_list)));
  dump_u32("_pcre2_callout_start_delims_8", PRIV(callout_start_delims),
           ARRLEN(PRIV(callout_start_delims)));
  dump_u32("_pcre2_callout_end_delims_8", PRIV(callout_end_delims),
           ARRLEN(PRIV(callout_end_delims)));
  dump_i32("_pcre2_utf8_table1", PRIV(utf8_table1), ARRLEN(PRIV(utf8_table1)));
  fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_utf8_table1_size: u32 = %u;\n\n",
          PRIV(utf8_table1_size));
  dump_i32("_pcre2_utf8_table2", PRIV(utf8_table2), ARRLEN(PRIV(utf8_table2)));
  dump_i32("_pcre2_utf8_table3", PRIV(utf8_table3), ARRLEN(PRIV(utf8_table3)));
  dump_u8("_pcre2_utf8_table4", PRIV(utf8_table4), ARRLEN(PRIV(utf8_table4)));
  dump_u32("_pcre2_ucp_gentype_8", PRIV(ucp_gentype), ARRLEN(PRIV(ucp_gentype)));
  dump_u32("_pcre2_ucp_gbtable_8", PRIV(ucp_gbtable), ARRLEN(PRIV(ucp_gbtable)));

  /* utt_names is a single large string of names, with embedded NULs. */
  {
    size_t n = sizeof(PRIV(utt_names));
    fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_utt_names_8: [u8; %zu] = [\n", n);
    for (size_t i = 0; i < n; i++)
      fprintf(out, "%s0x%02x,%s", (i % 12 == 0) ? "  " : " ",
              (unsigned)(unsigned char)PRIV(utt_names)[i], (i % 12 == 11) ? "\n" : "");
    fprintf(out, "\n];\n\n");
  }

  {
    size_t n = ARRLEN(PRIV(utt));
    fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_utt_8: [ucp_type_table; %zu] = [\n", n);
    for (size_t i = 0; i < n; i++)
      fprintf(out, "  ucp_type_table { name_offset: %u, type_: %u, value: %u },\n",
              (unsigned)PRIV(utt)[i].name_offset, (unsigned)PRIV(utt)[i].type,
              (unsigned)PRIV(utt)[i].value);
    fprintf(out, "];\n\n");
    fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_utt_size_8: usize = %zu;\n\n",
            PRIV(utt_size));
  }
  fclose(out);

  /* ---------------- ucd.rs ---------------- */
  out = fopen("../translation/src/ucd.rs", "w");
  hdr("pcre2_ucd.c");
  fprintf(out, "use crate::types::ucd_record;\nuse core::ffi::c_char;\n\n");
  fprintf(out, "#[repr(transparent)]\npub struct CStrPtr(pub *const c_char);\n");
  fprintf(out, "unsafe impl Sync for CStrPtr {}\n\n");
  fprintf(out, "static UNICODE_VERSION_STRING: [u8; %zu] = *b\"%s\\0\";\n",
          strlen(PRIV(unicode_version)) + 1, PRIV(unicode_version));
  fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_unicode_version_8: CStrPtr =\n"
               "    CStrPtr(UNICODE_VERSION_STRING.as_ptr() as *const c_char);\n\n");
  dump_u32("_pcre2_ucd_caseless_sets_8", PRIV(ucd_caseless_sets),
           ARRLEN(PRIV(ucd_caseless_sets)));
  fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_ucd_turkish_dotted_i_caseset_8: u32 = %u;\n\n",
          PRIV(ucd_turkish_dotted_i_caseset));
  dump_u32("_pcre2_ucd_nocase_ranges_8", PRIV(ucd_nocase_ranges),
           ARRLEN(PRIV(ucd_nocase_ranges)));
  fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_ucd_nocase_ranges_size_8: u32 = %u;\n\n",
          PRIV(ucd_nocase_ranges_size));
  dump_u32("_pcre2_ucd_digit_sets_8", PRIV(ucd_digit_sets), ARRLEN(PRIV(ucd_digit_sets)));
  dump_u32("_pcre2_ucd_script_sets_8", PRIV(ucd_script_sets), ARRLEN(PRIV(ucd_script_sets)));
  dump_u32("_pcre2_ucd_boolprop_sets_8", PRIV(ucd_boolprop_sets),
           ARRLEN(PRIV(ucd_boolprop_sets)));
  {
    size_t n = ARRLEN(PRIV(ucd_records));
    fprintf(out, "#[unsafe(no_mangle)]\npub static _pcre2_ucd_records_8: [ucd_record; %zu] = [\n", n);
    for (size_t i = 0; i < n; i++)
      fprintf(out, "  ucd_record { script: %u, chartype: %u, gbprop: %u, caseset: %u,"
                   " other_case: %d, scriptx_bidiclass: %u, bprops: %u },\n",
              (unsigned)PRIV(ucd_records)[i].script,
              (unsigned)PRIV(ucd_records)[i].chartype,
              (unsigned)PRIV(ucd_records)[i].gbprop,
              (unsigned)PRIV(ucd_records)[i].caseset,
              (int)PRIV(ucd_records)[i].other_case,
              (unsigned)PRIV(ucd_records)[i].scriptx_bidiclass,
              (unsigned)PRIV(ucd_records)[i].bprops);
    fprintf(out, "];\n\n");
  }
  dump_u16("_pcre2_ucd_stage1_8", PRIV(ucd_stage1), ARRLEN(PRIV(ucd_stage1)));
  dump_u16("_pcre2_ucd_stage2_8", PRIV(ucd_stage2), ARRLEN(PRIV(ucd_stage2)));
  fclose(out);

  printf("ok: TABLES_LENGTH=%d OP_TABLE_LENGTH=%d ucd_records=%zu stage1=%zu stage2=%zu utt=%zu\n",
         (int)TABLES_LENGTH, (int)OP_TABLE_LENGTH, ARRLEN(PRIV(ucd_records)),
         ARRLEN(PRIV(ucd_stage1)), ARRLEN(PRIV(ucd_stage2)), ARRLEN(PRIV(utt)));
  return 0;
}
