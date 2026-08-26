/* Dumps the raw bytes of the exported data tables of the C libpcre2.so so that
   they can be mechanically transcribed into Rust. */
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct ent { const char *name; unsigned long size; int is_ptr_to_string; };

static struct ent ents[] = {
  {"_pcre2_OP_lengths_8", 0xad, 0},
  {"_pcre2_callout_end_delims_8", 0x24, 0},
  {"_pcre2_callout_start_delims_8", 0x24, 0},
  {"_pcre2_default_tables_8", 0x440, 0},
  {"_pcre2_hspace_list_8", 0x50, 0},
  {"_pcre2_vspace_list_8", 0x20, 0},
  {"_pcre2_posix_class_maps8", 0xa8, 0},
  {"_pcre2_ucd_boolprop_sets_8", 0x5f8, 0},
  {"_pcre2_ucd_caseless_sets_8", 0x1d8, 0},
  {"_pcre2_ucd_digit_sets_8", 0x138, 0},
  {"_pcre2_ucd_nocase_ranges_8", 0x150, 0},
  {"_pcre2_ucd_nocase_ranges_size_8", 4, 0},
  {"_pcre2_ucd_records_8", 0x4944, 0},
  {"_pcre2_ucd_script_sets_8", 0x770, 0},
  {"_pcre2_ucd_stage1_8", 0x4400, 0},
  {"_pcre2_ucd_stage2_8", 0x13a00, 0},
  {"_pcre2_ucd_turkish_dotted_i_caseset_8", 4, 0},
  {"_pcre2_ucp_gbtable_8", 0x3c, 0},
  {"_pcre2_ucp_gentype_8", 0x78, 0},
  {"_pcre2_unicode_version_8", 8, 1},
  {"_pcre2_utf8_table1", 0x18, 0},
  {"_pcre2_utf8_table1_size", 4, 0},
  {"_pcre2_utf8_table2", 0x18, 0},
  {"_pcre2_utf8_table3", 0x18, 0},
  {"_pcre2_utf8_table4", 0x40, 0},
  {"_pcre2_utt_8", 0xc24, 0},
  {"_pcre2_utt_names_8", 0xefa, 0},
  {"_pcre2_utt_size_8", 8, 0},
  {NULL, 0, 0}
};

int main(int argc, char **argv)
{
if (argc < 3) { fprintf(stderr, "usage: %s libpcre2.so outdir\n", argv[0]); return 1; }
void *h = dlopen(argv[1], RTLD_NOW);
if (h == NULL) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
for (struct ent *e = ents; e->name != NULL; e++)
  {
  void *p = dlsym(h, e->name);
  if (p == NULL) { fprintf(stderr, "missing symbol %s\n", e->name); return 1; }
  char path[1024];
  snprintf(path, sizeof(path), "%s/%s.bin", argv[2], e->name);
  FILE *f = fopen(path, "wb");
  if (f == NULL) { perror("fopen"); return 1; }
  if (e->is_ptr_to_string)
    {
    const char *s = *(const char **)p;
    fwrite(s, 1, strlen(s) + 1, f);
    }
  else fwrite(p, 1, e->size, f);
  fclose(f);
  printf("%s %lu\n", e->name, e->size);
  }
return 0;
}
