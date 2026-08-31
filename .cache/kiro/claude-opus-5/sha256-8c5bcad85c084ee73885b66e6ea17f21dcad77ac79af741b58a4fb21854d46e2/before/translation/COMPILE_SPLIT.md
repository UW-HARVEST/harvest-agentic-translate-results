# Ownership map for the translation of `c_src/src/pcre2_compile.c`

`pcre2_compile.c` is ~11350 lines, so it is split across several Rust modules.
Read this together with `CONVENTIONS.md`.

**Every item you translate that another module needs must be declared
`pub(crate)`** (functions, statics, structs, constants). Items only used inside
your own module stay private. Call items owned by another module by full path,
e.g. `crate::compile_tables::check_escape(...)`.

Approximate C line ranges are given as a guide; follow function boundaries, not
line numbers.

## `compile_tables.rs` -- lines ~60..3050

Owns all the file-scope tables and the "small helper" statics:

* `meta_extra_lengths`, `xdigitab` (non-EBCDIC version), `escapes` +
  `ESCAPES_FIRST`/`ESCAPES_LAST`, `verbnames`/`verbs`/`verbops` + the `verbitem`
  struct, `alasnames`/`alasmeta` + the `alasitem` struct, `chartypeoffset`,
  `posix_names`, `posix_name_lengths`, `posix_substitutes`, `pso_list` + the `pso`
  struct and its `PSO_*` constants, `opcode_possessify`
* `IS_DIGIT` / `IS_XDIGIT` (as `#[inline] pub(crate) fn`), `MAX_REPEAT_COUNT`,
  `REPEAT_UNLIMITED`, `META_*`-adjacent local constants defined in this C file,
  and any other `#define` local to `pcre2_compile.c`
* functions: `read_number`, `read_repeat_counts`, `PRIV(check_escape)` (exported
  as `_pcre2_check_escape_8`, plus `pub(crate) unsafe fn check_escape`),
  `get_ucp`, `check_posix_syntax`, `check_posix_name`, `read_name`,
  `parse_capture_list`, `manage_callouts`, `handle_escdsw`, `max_parsed_pattern`

Note: `PRIV(posix_class_maps)` is already translated in `compile_class.rs` as
`crate::compile_class::POSIX_CLASS_MAPS`; do not duplicate it.
`show_parsed` is `#ifdef PCRE2_DEBUG` only -- skip it entirely.

## `compile_parse.rs` -- `parse_regex` (lines ~3050..5960)

Owns `pub(crate) unsafe fn parse_regex` and any helper that exists only for it.
Uses `crate::compile_tables::*`.

## `compile_branch.rs` -- lines ~5960..8895

Owns `first_significant_code`, `compile_branch`, `compile_regex` (all
`pub(crate)`), plus their local statics.

## `compile_scan.rs` -- lines ~8896..10600

Owns `is_anchored`, `is_startline`, `find_recurse`, `find_firstassertedcu`,
`parsed_skip`, `get_grouplength`, `get_branchlength`, `set_lookbehind_lengths`,
`check_lookbehinds` (all `pub(crate)`).

## `compile.rs` -- lines ~1130..1210 and ~10600..end

Owns the public API: `pcre2_code_copy_8`, `pcre2_code_copy_with_tables_8`,
`pcre2_code_free_8`, `pcre2_compile_8`, and the static `pcre2_compile` helpers
that live at the end of the file.

## Already-translated modules you can call

* `crate::internal` -- types, structs, constants, GET/PUT/GETCHAR helpers, UCD
* `crate::compile_internal` -- `pcre2_compile.h`: `META_*`, `ERR*`,
  `eclass_op_info`, `setbit`, `putoffset`/`getoffset`/`getplusoffset`/
  `readplusoffset`/`skipoffset`/`SIZEOFFSET`, `meta_code`/`meta_data`/`meta_diff`,
  `PC_*`, `NAMED_GROUP_*`, `CLASS_IS_ECLASS`, `get_max_char_value`
* `crate::opcodes` -- `OP_*`, `OP_LENGTHS`, `ESC_*`
* `crate::chars` -- `CHAR_*` (u32), `STR_*`/`STRING_*` (`&[u8]`)
* `crate::ucp`, `crate::ucd`, `crate::ucptables` (`UTT`, `UTT_NAMES`, `UTT_SIZE`)
* `crate::compile_class` -- `POSIX_CLASS_MAPS`, `update_classbits`,
  `compile_class_nested`, `compile_class_not_nested`
* `crate::compile_cgroup` -- `get_hash_from_name`, `find_named_group`,
  `add_name_to_table`, `find_dupname_details`, `parse_scan_substr_args`,
  `parse_recurse_args`
* `crate::auto_possess::auto_possessify`, `crate::study::study`
* `crate::find_bracket::find_bracket`, `crate::xclass::{xclass, eclass}`
* `crate::string_utils` -- `strcmp`, `strcmp_c8`, `strncmp`, `strncmp_c8`,
  `strlen_`, `strcpy_c8`
* `crate::ord2utf::ord2utf`, `crate::valid_utf::valid_utf`,
  `crate::newline::{is_newline, was_newline}`, `crate::chkdint::ckd_smul`
* `crate::jit` -- non-JIT stubs
