# PCRE2 C -> Rust translation guide (READ THIS FIRST)

We are transliterating the PCRE2 10.48 C library in `c_src/` into a Rust cdylib in
`src/`. The build is **8-bit code unit width**, `SUPPORT_UNICODE` defined,
`SUPPORT_JIT` **not** defined, `EBCDIC` **not** defined, `PCRE2_DEBUG` **not**
defined, `LINK_SIZE == 2`, `IMM2_SIZE == 2`, `HAVE_CONFIG_H` defined.

**GOAL: byte-identical behaviour.** This is a *literal transliteration*, not a
rewrite. Do NOT restructure algorithms, do NOT fix bugs, do NOT reorder error
checks. Keep the same variable names as the C where possible so the code can be
diffed against the C by a human.

## Hard rules

1. Use raw pointers and `unsafe`, mirroring the C 1:1.
   - `PCRE2_SPTR` -> `*const u8`, `PCRE2_UCHAR *` -> `*mut u8`
   - `PCRE2_SIZE` -> `usize`, `BOOL` -> `c_int` (with `TRUE`/`FALSE` consts)
   - `int` -> `c_int`, `unsigned int` -> `c_uint`/`u32`, `uint32_t` -> `u32`, etc.
2. Every function that the C exports (i.e. is not `static`) must be
   `#[unsafe(no_mangle)] pub unsafe extern "C" fn <FINAL_LINKER_NAME>(...)`.
   The final linker name has the `_8` suffix applied by the `PCRE2_SUFFIX` macro.
   Check `nm` names in `SYMBOLS.txt`. Note the irregular ones without an
   underscore before the 8, e.g. `_pcre2_compile_get_hash_from_name8`,
   `_pcre2_posix_class_maps8`.
3. `static` C functions become private `unsafe fn` in the module.
4. Arithmetic: the crate is built with `overflow-checks = false`, so `+`/`-`/`*`
   wrap like C. Prefer `wrapping_add`/`wrapping_sub` where the C intent is
   clearly modular, but plain operators are acceptable. NEVER use checked math
   that changes behaviour.
5. Pointer arithmetic:
   - `p + n` -> `p.add(n)` (or `p.offset(n as isize)` if `n` may be negative)
   - `p - q` -> `p.offset_from(q)` (gives `isize`); when the C stores this in a
     `PCRE2_SIZE` use `p.offset_from(q) as usize`.
   - `*p++` -> `{ let v = *p; p = p.add(1); v }`
   - `*++p` -> `{ p = p.add(1); *p }`
   - `*p--` / `*--p` similarly.
   - Comparing pointers with `<`, `>=` etc. works directly on raw pointers.
6. `goto` must be translated with Rust control flow. Preferred idioms:
   - forward `goto LABEL;` where LABEL is at the end of a region:
     `'label: { ... break 'label; ... }` then the code after the block.
   - backward goto (loop): `'label: loop { ... continue 'label; ... break; }`
   - When a C function has a big `for(;;)` with `goto` targets inside a switch,
     model it exactly with labelled blocks/loops. Keep the structure.
7. `memcpy`/`memmove`/`memset`/`memchr`/`memcmp`/`malloc`/`free`/`strlen` are
   declared in `crate::internal` as `extern "C"` bindings to libc; call them
   exactly as the C does (`memcpy(dst as *mut c_void, src as *const c_void, n)`).
8. File-scope C `static const` tables become `static` items in the Rust module
   (private). Do not add `#[unsafe(no_mangle)]` to them.
9. Do not add extra bounds checks or `assert!`. `PCRE2_ASSERT`,
   `PCRE2_DEBUG_UNREACHABLE`, `PCRE2_UNREACHABLE` are all no-ops in this build;
   translate them to nothing (or a comment).
10. `PCRE2_FALLTHROUGH` is nothing. C `switch` fallthrough must be replicated
    explicitly (duplicate the code or restructure with an inner labelled block).
11. Anything inside `#ifdef SUPPORT_JIT`, `#ifdef PCRE2_DEBUG`,
    `#ifdef EBCDIC`, `#ifdef SUPPORT_VALGRIND`, `#if PCRE2_CODE_UNIT_WIDTH != 8`
    is **omitted**. Anything inside `#ifdef SUPPORT_UNICODE`,
    `#ifdef SUPPORT_WIDE_CHARS`, `#ifdef MAYBE_UTF_MULTI`,
    `#if PCRE2_CODE_UNIT_WIDTH == 8` is **included**.

## What the foundation already provides

`src/internal.rs` (`use crate::internal::*;`) has:

- All types/structs, laid out to match C byte-for-byte (verified with
  `offset_of!` tests): `pcre2_memctl`, `pcre2_real_general_context`,
  `pcre2_real_compile_context`, `pcre2_real_match_context`,
  `pcre2_real_convert_context`, `pcre2_real_code`, `pcre2_real_match_data`,
  `heapframe` (+ `hf_fields` union and its `hf_*` member structs),
  `match_block`, `dfa_match_block`, `compile_block`, `class_bits_storage`
  (union with `.classbits: [u8;32]` / `.classwords: [u32;8]`),
  `pcre2_callout_block`, `pcre2_callout_enumerate_block`,
  `pcre2_substitute_callout_block`, `ucd_record`, `ucp_type_table`,
  `named_group`, `open_capitem`, `compile_data`, `class_ranges`,
  `recurse_arguments`, `eclass_op_info`, `recurse_check`,
  `parsed_recurse_check`, `recurse_cache`, `branch_chain`,
  `dfa_recursion_info`, `pcre2_serialized_data`, `pcre2_real_jit_stack`.
- Function-pointer typedefs: `MallocFn`, `FreeFn`, `CalloutFn`,
  `SubstCalloutFn`, `SubstCaseCalloutFn`, `StackGuardFn` (all `Option<unsafe
  extern "C" fn ...>` so that `NULL` is `None`).
- All `OP_*` opcode constants (as `u32`), `ESC_*` (as `c_int`), `PT_*`, `XCL_*`,
  `ECL_*`, `CHAR_*` (as `u32`), `cbit_*`, `ctype_*`, `*_offset`,
  `PCRE2_MODE8`.., `NLTYPE_*`, `MAGIC_NUMBER`, `TABLES_LENGTH`,
  `START_FRAMES_SIZE`, `DFA_START_RWS_SIZE`, `IMM2_SIZE`, `LINK_SIZE`,
  `MAX_PATTERN_SIZE`, `MAX_MARK`, `MAX_UTF_SINGLE_CU`, `MAX_UTF_CODE_POINT`,
  `MAX_NON_UTF_CHAR`, `NOTACHAR`, `LOOKBEHIND_MAX`, `REQ_CU_MAX`,
  `ECLASS_NEST_LIMIT`, `HEAPFRAME_ALIGNMENT`, `RREF_ANY`, `REFI_FLAG_*`,
  `PCRE2_OPTIM_*`, config values (`HEAP_LIMIT`, `MATCH_LIMIT`,
  `MATCH_LIMIT_DEPTH`, `MAX_NAME_COUNT`, `MAX_NAME_SIZE`, `MAX_VARLOOKBEHIND`,
  `NEWLINE_DEFAULT`, `PARENS_NEST_LIMIT`, `BSR_DEFAULT`).
- Inline helpers replacing C macros:
  - `GET(a,n) -> u32`, `PUT(a,n,d)`, `GET2(a,n) -> u32`, `PUT2(a,n,d)`
    (all `unsafe`, take `*const u8`/`*mut u8`)
  - `CU2BYTES(x)`, `BYTES2CU(x)` (identity in 8-bit)
  - `HASUTF8EXTRALEN(c)`, `HAS_EXTRALEN(c)`, `NOT_FIRSTCU(c)`,
    `GET_EXTRALEN(c)`, `MAX_255(c)`, `CHMAX_255(c)`, `TABLE_GET(c,table,def)`
  - `getutf8(c, eptr) -> u32` — the `GETUTF8` macro: `eptr` points at the
    *leading* byte, `c` is that leading byte's value.
  - `utf8_extra(c) -> usize` — how many extra bytes `getutf8` consumed
    (use for `GETUTF8LEN`: `len += utf8_extra(c)`).
  - `getutf8inc(c, eptr) -> (u32, *const u8)` — the `GETUTF8INC` macro:
    `c` is the already-consumed leading byte, `eptr` points just past it;
    returns the char and the advanced pointer.
  - `PUTCHAR(utf: bool, c, p) -> usize`
  - UCD: `GET_UCD(ch) -> &'static ucd_record`, `UCD_CHARTYPE`, `UCD_SCRIPT`,
    `UCD_CATEGORY`, `UCD_GRAPHBREAK`, `UCD_CASESET`, `UCD_OTHERCASE`,
    `UCD_SCRIPTX`, `UCD_BPROPS`, `UCD_BIDICLASS`, `UCD_SCRIPTX_PROP`,
    `UCD_BIDICLASS_PROP`, `UCD_BPROPS_PROP`, `UCD_ANY_I`, `UCD_DOTTED_I`,
    `UCD_FOLD_I_TURKISH`, `UCD_BLOCK_SIZE`, `UCD_SCRIPTX_MASK`,
    `UCD_BIDICLASS_SHIFT`, `UCD_BPROPS_MASK`
  - `MAPBIT(map,n)`, `script_set_bit(offset,n) -> bool`,
    `boolprop_set_bit(offset,n) -> bool`, `SETBIT(a,b)`,
    `CLIST_ALIGN_TO(base,align)`
- `SyncPtr(*const c_char)` wrapper for exported pointer statics.

Common convenience for GETCHAR-family macros (write these inline in your module,
they are only a couple of lines):

```rust
// GETCHAR(c, eptr):        let mut c = *eptr as u32; if c >= 0xc0 { c = getutf8(c, eptr); }
// GETCHARTEST(c, eptr):    let mut c = *eptr as u32; if utf && c >= 0xc0 { c = getutf8(c, eptr); }
// GETCHARINC(c, eptr):     let mut c = *eptr as u32; eptr = eptr.add(1);
//                          if c >= 0xc0 { let r = getutf8inc(c, eptr); c = r.0; eptr = r.1; }
// GETCHARINCTEST(c, eptr): same but `if utf && c >= 0xc0`
// GETCHARLEN(c, eptr, len): let mut c = *eptr as u32;
//                          if c >= 0xc0 { len += utf8_extra(c); c = getutf8(c, eptr); }
// GETCHARLENTEST: same but `if utf && c >= 0xc0`
// BACKCHAR(eptr):          while (*eptr & 0xc0) == 0x80 { eptr = eptr.sub(1); }
// FORWARDCHAR(eptr):       while (*eptr & 0xc0) == 0x80 { eptr = eptr.add(1); }
// FORWARDCHARTEST(eptr,end): while eptr < end && (*eptr & 0xc0) == 0x80 { eptr = eptr.add(1); }
// ACROSSCHAR(cond, eptr, act): while (cond) && (*eptr & 0xc0) == 0x80 { act }
```

## Data tables (already generated, do not re-derive)

`crate::tables`:
`_pcre2_OP_lengths_8: [u8;173]`, `_pcre2_default_tables_8: [u8;1088]`,
`_pcre2_hspace_list_8: [u32;_]`, `_pcre2_vspace_list_8: [u32;_]`,
`_pcre2_callout_start_delims_8: [u32;_]`, `_pcre2_callout_end_delims_8: [u32;_]`,
`_pcre2_ucp_gentype_8: [u32;32]`, `_pcre2_ucp_gbtable_8: [u32;16]`,
`_pcre2_utf8_table1: [c_int;6]`, `_pcre2_utf8_table1_size: c_uint`,
`_pcre2_utf8_table2: [c_int;6]`, `_pcre2_utf8_table3: [c_int;6]`,
`_pcre2_utf8_table4: [u8;64]`, `_pcre2_posix_class_maps8: [c_int;42]`

`crate::ucd_data`:
`_pcre2_ucd_caseless_sets_8: [u32;_]`, `_pcre2_ucd_turkish_dotted_i_caseset_8: u32`,
`_pcre2_ucd_nocase_ranges_8: [u32;_]`, `_pcre2_ucd_nocase_ranges_size_8: u32`,
`_pcre2_ucd_digit_sets_8: [u32;_]`, `_pcre2_ucd_script_sets_8: [u32;_]`,
`_pcre2_ucd_boolprop_sets_8: [u32;_]`, `_pcre2_ucd_records_8: [ucd_record;_]`,
`_pcre2_ucd_stage1_8: [u16;_]`, `_pcre2_ucd_stage2_8: [u16;_]`,
`_pcre2_utt_names_8: [u8;_]`, `_pcre2_utt_8: [ucp_type_table;518]`,
`_pcre2_utt_size_8: usize`, `_pcre2_unicode_version_8: SyncPtr`

`crate::ucp`: every `ucp_*` constant as `u32` (`ucp_L`, `ucp_Lu`, `ucp_Zs`,
`ucp_gbCR`, `ucp_bidiAL`, `ucp_Latin`, `ucp_Script_Count`, `ucp_Bprop_Count`,
`ucd_script_sets_item_size`, `ucd_boolprop_sets_item_size`, ...).
Note the field of `ucp_type_table` named `type` in C is `type_` in Rust.

`crate::pcre2_pub`: every public `PCRE2_*` constant. Option/flag values are
`u32`; `PCRE2_ERROR_*` are `c_int`; `PCRE2_ZERO_TERMINATED`/`PCRE2_UNSET` are
`usize`.

`crate::compile_h`: `META_*` constants (`u32`), `META_CODE()`, `META_DATA()`,
`META_DIFF()`, `SIZEOFFSET` (== 2), `CLASS_IS_ECLASS`, `MAX_UCHAR_VALUE`,
`GET_MAX_CHAR_VALUE(utf)`, `PC_DIGIT/GRAPH/PRINT/PUNCT/XDIGIT`,
`NAMED_GROUP_HASH_MASK`, `NAMED_GROUP_IS_DUPNAME`, `ERR(n)` (== 100 + n).
For `ERRnn` in the C, write `ERR(nn)`.

`PUTOFFSET`/`GETOFFSET`/`GETPLUSOFFSET`/`READPLUSOFFSET`/`SKIPOFFSET` use the
64-bit variants (SIZEOFFSET == 2):
```rust
// PUTOFFSET(s, p):      *p = (s >> 32) as u32; p = p.add(1); *p = (s & 0xffffffff) as u32; p = p.add(1);
// GETOFFSET(s, p):      s = ((*p.add(0) as usize) << 32) | (*p.add(1) as usize); p = p.add(2);
// GETPLUSOFFSET(s, p):  s = ((*p.add(1) as usize) << 32) | (*p.add(2) as usize); p = p.add(2);
// READPLUSOFFSET(s, p): s = ((*p.add(1) as usize) << 32) | (*p.add(2) as usize);
// SKIPOFFSET(p):        p = p.add(2);
```

## Module map (Rust module <- C file)

| Rust module        | C file                   |
|--------------------|--------------------------|
| `chkdint`          | pcre2_chkdint.c          |
| `ord2utf`          | pcre2_ord2utf.c          |
| `string_utils`     | pcre2_string_utils.c     |
| `newline`          | pcre2_newline.c          |
| `maketables`       | pcre2_maketables.c       |
| `match_data`       | pcre2_match_data.c       |
| `match_next`       | pcre2_match_next.c       |
| `extuni`           | pcre2_extuni.c           |
| `find_bracket`     | pcre2_find_bracket.c     |
| `config`           | pcre2_config.c           |
| `context`          | pcre2_context.c          |
| `error`            | pcre2_error.c            |
| `serialize`        | pcre2_serialize.c        |
| `valid_utf`        | pcre2_valid_utf.c        |
| `script_run`       | pcre2_script_run.c       |
| `pattern_info`     | pcre2_pattern_info.c     |
| `substring`        | pcre2_substring.c        |
| `xclass`           | pcre2_xclass.c           |
| `compile_cgroup`   | pcre2_compile_cgroup.c   |
| `jit_compile`      | pcre2_jit_compile.c      |
| `convert`          | pcre2_convert.c          |
| `auto_possess`     | pcre2_auto_possess.c     |
| `substitute`       | pcre2_substitute.c       |
| `study`            | pcre2_study.c            |
| `compile_class`    | pcre2_compile_class.c    |
| `dfa_match`        | pcre2_dfa_match.c        |
| `matcher`          | pcre2_match.c            |
| `compile`          | pcre2_compile.c          |

Cross-module calls use the full path, e.g.
`crate::string_utils::_pcre2_strlen_8(p)`,
`crate::newline::_pcre2_is_newline_8(...)`,
`crate::context::_pcre2_memctl_malloc_8(size, &mut memctl)`.

## Module preamble to use

```rust
use crate::compile_h::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};
```
(drop unused ones; warnings are silenced crate-wide anyway)

## Verifying

`cargo build --release` must succeed with no errors. Then
`nm -D --defined-only target/release/libpcre2.so` must contain the symbols your
module owns.
