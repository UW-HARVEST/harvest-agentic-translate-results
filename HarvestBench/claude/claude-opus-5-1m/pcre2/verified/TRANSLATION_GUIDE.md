# PCRE2 C -> Rust translation guide (READ THIS FIRST)

We are translating the **entire** PCRE2 8-bit library in `c_src/` into a Rust
`cdylib` that exports the same ABI and produces **byte-identical** behaviour.

Build config that is already baked in (do not re-derive it):
`PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE` defined, `LINK_SIZE == 2`,
`IMM2_SIZE == 2`, **no** `SUPPORT_JIT`, no EBCDIC, no `PCRE2_DEBUG`,
`HAVE_BUILTIN_MUL_OVERFLOW` undefined, `HAVE_ATTRIBUTE_UNINITIALIZED` undefined.
So: only translate the code that the C preprocessor keeps for that configuration.
Code inside `#ifdef SUPPORT_JIT`, `#ifdef EBCDIC`, `#ifdef PCRE2_DEBUG`,
`#if PCRE2_CODE_UNIT_WIDTH == 16/32`, `#ifdef DEBUG_*` etc. must be **omitted**.
Code inside `#ifdef SUPPORT_UNICODE`, `#ifdef SUPPORT_WIDE_CHARS`,
`#if PCRE2_CODE_UNIT_WIDTH == 8`, `#ifndef SUPPORT_JIT`, `#ifndef EBCDIC` must be
**kept**.

## Golden rules

1. **Transliterate, do not redesign.** Keep the same control flow, the same order
   of tests, the same variable names, the same helper functions (as private Rust
   `unsafe fn`s), the same table contents. Reproduce C bugs exactly.
2. **No panics ever.** A panic aborts the process. Never use `.unwrap()` on
   `Option<fn>` unless C already dereferenced the pointer unconditionally; never
   index a Rust array/slice with a value that could be out of range - read through
   raw pointers instead (`*TABLE.as_ptr().add(i)`).
3. Everything is `unsafe`: write `unsafe fn` / `unsafe extern "C" fn` and use raw
   pointers exactly like the C code (`.add(n)`, `.offset(n)`, `.sub(n)`,
   `ptr1.offset_from(ptr2)` for pointer differences).
4. Integer types must match C: `int` -> `c_int` (= i32), `unsigned int` -> `c_uint`
   (u32), `uint32_t` -> u32, `PCRE2_SIZE`/`size_t` -> `usize`, `BOOL` -> `BOOL`
   (= i32, values `TRUE`/`FALSE`). Arithmetic wraps (release profile has
   `overflow-checks = false`).
5. Do **not** add `mod` statements: `src/lib.rs` already declares every module.
   Each module starts with:
   ```rust
   use crate::internal::*;
   ```
   (macros are exported at the crate root and are used as `GET!(...)`, `PUT!(...)`).

## What is already available (read `src/internal.rs` + `src/macros.rs`)

* Types: `PCRE2_UCHAR` (u8), `PCRE2_SPTR` (`*const u8`), `PCRE2_SIZE` (usize),
  `BOOL` (i32), `TRUE`, `FALSE`, `PCRE2_UNSET`, `PCRE2_ZERO_TERMINATED`.
* All `#[repr(C)]` structures: `pcre2_memctl`, `pcre2_real_code`,
  `pcre2_real_match_data`, `pcre2_real_compile_context`, `pcre2_real_match_context`,
  `pcre2_real_convert_context`, `pcre2_real_general_context`, `compile_block`,
  `match_block`, `dfa_match_block`, `heapframe` (+ its `fields` union types
  `hf_char_repeat`, ...), `named_group`, `branch_chain`, `open_capitem`,
  `class_ranges`, `recurse_arguments`, `compile_data`, `class_bits_storage`,
  `ucd_record`, `ucp_type_table`, `pcre2_callout_block`,
  `pcre2_callout_enumerate_block`, `pcre2_substitute_callout_block`,
  `eclass_op_info`, `dfa_recursion_info`, `recurse_check`, `parsed_recurse_check`,
  `recurse_cache`, `pcre2_serialized_data`, `pcre2_real_jit_stack`.
* All constants: `PCRE2_*` (public + private flags), `OP_*` (u32), `ESC_*` (i32),
  `META_*` (u32), `ERR1..ERR120` (i32), `PT_*`, `XCL_*`, `ECL_*`, `cbit_*`,
  `ctype_*`, `lcc_offset`, `fcc_offset`, `cbits_offset`, `ctypes_offset`,
  `TABLES_LENGTH`, `CHAR_*` (u32, e.g. `CHAR_a`), `ucp_*` (u32),
  `MAGIC_NUMBER`, `LINK_SIZE`, `IMM2_SIZE`, `MAX_*`, `NLTYPE_*`, ...
* Data tables (already transcribed): `_pcre2_OP_lengths_8`, `_pcre2_hspace_list_8`,
  `_pcre2_vspace_list_8`, `_pcre2_callout_start_delims_8`,
  `_pcre2_callout_end_delims_8`, `_pcre2_utf8_table1..4`, `_pcre2_utf8_table1_size`,
  `_pcre2_ucp_gentype_8`, `_pcre2_ucp_gbtable_8`, `_pcre2_utt_8`,
  `_pcre2_utt_names_8`, `_pcre2_utt_size_8`, `_pcre2_default_tables_8`,
  `_pcre2_ucd_*`, `_pcre2_unicode_version_8`.
  In C these are referred to as `PRIV(name)`, e.g. `PRIV(OP_lengths)[x]` becomes
  `_pcre2_OP_lengths_8[x as usize]`.
* libc: `malloc`, `free`, `memcpy`, `memmove`, `memset`, `memcmp`, `memchr`,
  `strlen`, `strcmp`, `isspace`, `isdigit`, ... `tolower`, `toupper`
  (declared `extern "C"`, call them exactly as C does).
* **Every** library function (public `pcre2_xxx_8` and private `_pcre2_xxx_8`) is
  declared in an `extern "C"` block in `internal.rs`, so you can call any of them
  by its linker name without knowing which module defines it. In C these calls
  look like `PRIV(strlen)(x)` or `pcre2_match(...)`; write `_pcre2_strlen_8(x)` /
  `pcre2_match_8(...)`.
* UCD accessors are inline fns: `GET_UCD(c)`, `UCD_CHARTYPE(c)`, `UCD_SCRIPT(c)`,
  `UCD_CATEGORY(c)`, `UCD_GRAPHBREAK(c)`, `UCD_CASESET(c)`, `UCD_OTHERCASE(c)`,
  `UCD_SCRIPTX(c)`, `UCD_BPROPS(c)`, `UCD_BIDICLASS(c)`, `UCD_SCRIPTX_PROP(p)`,
  `UCD_BIDICLASS_PROP(p)`, `UCD_BPROPS_PROP(p)`, `UCD_ANY_I(c)`, `UCD_DOTTED_I(c)`,
  `UCD_FOLD_I_TURKISH(c)`.
* Macros (in `src/macros.rs`, called with `!`): `GET!`, `PUT!`, `GET2!`, `PUT2!`,
  `PUTINC!`, `PUT2INC!`, `GETCHAR!`, `GETCHARINC!`, `GETCHARLEN!`, `GETUTF8!`,
  `GETUTF8INC!`, `GETUTF8LEN!`, `BACKCHAR!`, `FORWARDCHAR!`, `FORWARDCHARTEST!`,
  `ACROSSCHAR!`, `MAPBIT!`, `MAPSET!`, `SETBIT!`, `TABLE_GET!`, `MAX_255!`,
  `CHMAX_255!`, `META_CODE!`, `META_DATA!`, `META_DIFF!`, `PUTOFFSET!`,
  `GETOFFSET!`, `GETPLUSOFFSET!`, `READPLUSOFFSET!`, `SKIPOFFSET!`, `CU2BYTES!`,
  `BYTES2CU!`, `CLIST_ALIGN_TO!`, `GET_MAX_CHAR_VALUE!`, `SELECT_VALUE8!`,
  `HAS_EXTRALEN!`, `GET_EXTRALEN!`, `NOT_FIRSTCU!`, `HASUTF8EXTRALEN!`, `PUTCHAR!`,
  `IS_NEWLINE!`, `WAS_NEWLINE!`.
  **Differences from C**: the macros whose C version implicitly used the local
  variable `utf` take it as an extra last argument:
  `GETCHARTEST!(c, eptr, utf)`, `GETCHARINCTEST!(c, eptr, utf)`,
  `GETCHARLENTEST!(c, eptr, len, utf)`, `PUTCHAR!(c, p, utf)`,
  `GET_MAX_CHAR_VALUE!(utf)`.
  `IS_NEWLINE!(p, blk, psend, utf)` / `WAS_NEWLINE!(p, blk, psstart, utf)` take the
  NLBLOCK pointer and the PSEND/PSSTART expression explicitly, e.g. the C
  `IS_NEWLINE(ptr)` inside pcre2_match.c (NLBLOCK = mb, PSEND = end_subject)
  becomes `IS_NEWLINE!(ptr, mb, (*mb).end_subject, utf)`.

## Translation patterns

| C | Rust |
|---|---|
| `PCRE2_SPTR p = ...; p++` | `let mut p: PCRE2_SPTR = ...; p = p.add(1);` |
| `*p++` | `{ let t = *p; p = p.add(1); t }` |
| `p[3]` | `*p.add(3)` |
| `p - q` (pointers) | `p.offset_from(q)` (i64) - cast as needed |
| `if (x)` where x is int | `if x != 0` |
| `if (ptr == NULL)` | `if ptr.is_null()` |
| `NULL` | `std::ptr::null_mut()` / `std::ptr::null()` |
| `(void)x;` | omit |
| `sizeof(T)` | `size_of::<T>()` |
| `offsetof(S, f)` | `offset_of!(S, f)` |
| `switch (x) { case A: ... }` | `match x { A => {...}, _ => {} }` - but if the C code falls through or uses `break` to leave the switch inside a loop, use `if/else if` chains or a labeled block; **never** change semantics |
| `for (;;) { ... }` | `loop { ... }` |
| `do {...} while (c)` | `loop { ... if !(c) { break } }` |
| `goto LABEL;` forward | wrap the region in `'label: { ... break 'label; ... }` |
| `goto LABEL;` backward | `'label: loop { ... continue 'label; ... break; }` |
| `x = TRUE` (BOOL) | `x = TRUE` |
| struct field `->` | `(*ptr).field` |
| callbacks (`Option<fn>`) | `if (*mcontext).callout.is_some() { ((*mcontext).callout.unwrap())(args) }` |
| char literal `'a'` | `CHAR_a` (u32) - cast with `as u8` when storing a code unit |
| `"literal"` | `b"literal\0"` (keep the NUL when C relies on it) |

Comparisons between a code unit (u8) and a `CHAR_*` constant (u32): write
`*p as u32 == CHAR_a` or `*p == CHAR_a as u8` - pick one and be consistent.

Local `static const` tables inside a C file become private module-level statics:
`static tablename: [u8; N] = [...];` (exact same values, same order).

## Exported symbols

For every function that the C library exports (the `nm -D` list), the Rust
definition must be:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(...) -> *mut pcre2_real_code { ... }
```

The linker name must match **exactly** what the C macros produce (e.g.
`_pcre2_compile_add_name_to_table8` has no underscore before the `8`, but
`_pcre2_compile_class_nested_8` does). Your task description gives you the exact
names to define. Private (`static`) C functions must NOT be `no_mangle` and must
NOT be `pub`.

## Style / hygiene

* Keep the C comments (they explain the tricky bits) - copy the important ones.
* Do not reformat the logic into "nicer" Rust: no iterators, no `Option<>`
  wrapping, no bounds-checked slices in hot paths.
* Do not use any external crate. `std` is available.
* Do not modify `c_src/`, `Cargo.toml`, `src/lib.rs`, `src/internal.rs`,
  `src/macros.rs`, or any file that is not yours.
* When you need a helper that C got from a macro that is not listed above,
  write it as a private `#[inline] unsafe fn` in your module.
