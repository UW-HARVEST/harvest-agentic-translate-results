# C→Rust translation conventions (PCRE2, 8-bit mode)

You are translating **one C file (or one range of one C file)** of PCRE2 into Rust.
The Rust crate lives in `translation/` (`cargo build --release` from inside it).
**Never modify anything in `c_src/`.** Never edit another module's file unless your
task says so.

The build configuration that must be assumed everywhere:

* `PCRE2_CODE_UNIT_WIDTH == 8` → `PCRE2_UCHAR = u8`, `PCRE2_SPTR = *const u8`
* `LINK_SIZE == 2`, `IMM2_SIZE == 2`
* `SUPPORT_UNICODE` **defined** (so `SUPPORT_WIDE_CHARS` and `MAYBE_UTF_MULTI` are defined)
* `SUPPORT_JIT` **not** defined, `EBCDIC` **not** defined, `PCRE2_DEBUG` **not** defined,
  `HAVE_BUILTIN_MUL_OVERFLOW`/`HAVE_BUILTIN_UNREACHABLE`/`HAVE_ATTRIBUTE_UNINITIALIZED` **not** defined
* `PCRE2_ASSERT(x)`, `PCRE2_UNREACHABLE()`, `PCRE2_DEBUG_UNREACHABLE()` are **no-ops** – drop them
  (or write `/* PCRE2_ASSERT */`). Never insert a `panic!`/`unreachable!()` for them.
* Any code inside `#ifdef PCRE2_DEBUG`, `#ifdef SUPPORT_JIT`, `#ifdef EBCDIC`,
  `#if PCRE2_CODE_UNIT_WIDTH != 8`, `#ifdef SUPPORT_VALGRIND`, `#ifdef PCRE2_PCRE2TEST`
  is **not** compiled: skip it. Code in `#ifdef SUPPORT_UNICODE`, `#ifdef SUPPORT_WIDE_CHARS`,
  `#if PCRE2_CODE_UNIT_WIDTH == 8` **is** compiled.

## Fidelity rules (most important)

1. Translate **literally**, statement by statement, in the same order. Preserve the exact
   order of error checks, the exact error codes and the exact arithmetic.
2. **Do not fix bugs**, do not "improve", do not reorder, do not add bounds checks,
   do not add early returns.
3. Integer arithmetic must behave like C: use the same widths (`i32` for `int`,
   `u32` for `uint32_t`, `usize` for `size_t`/`PCRE2_SIZE`) and insert explicit
   `as` casts exactly where C's implicit conversions happen. The release profile has
   `overflow-checks = false`, so `+`/`-`/`*` wrap like C. Where C relies on wrapping of
   *unsigned* values, plain operators are fine; use `wrapping_*` if a value can overflow
   in a way that would trip a debug assertion.
4. No `panic!`, `unwrap()` on `Option<T>` other than function pointers, `assert!`,
   `expect`, or slice indexing that can go out of bounds. Use raw pointers.
5. Keep the C control flow. `goto` is emulated (see below).

## Naming and layout

* Your module file is `translation/src/<module>.rs`; it already contains stub definitions
  for every **exported** symbol with the exact `#[unsafe(no_mangle)] pub unsafe extern "C" fn
  <linker_name>(...)` signature. **Replace the stub body; never change the name or the
  signature.**
* C file-`static` functions become `pub(crate) unsafe fn <same_name>(...)` (same name as in C).
  Make them `pub(crate)` (not private) so that other chunks of the same C file can call them.
* C file-`static` const tables become `static <NAME>: [T; N] = [...];` (no `#[no_mangle]`).
* C file-`static` mutable state: there is none that matters; if you find one, use
  `static mut` + `unsafe`.
* Add `use crate::consts::*; use crate::types::*; use crate::macros::*;` (already in the stub)
  plus whatever `use crate::<other_module>::<fn>;` you need. All cross-module (i.e. `PRIV(...)`)
  functions live in other modules under their **linker name**, e.g.
  `crate::string_utils::_pcre2_strcmp_c8_8`, `crate::newline::_pcre2_is_newline_8`,
  `crate::tables::_pcre2_OP_lengths_8`, `crate::ucd::_pcre2_ucd_records_8`.

## Type mapping

| C | Rust |
|---|---|
| `BOOL` (`int`), `TRUE`, `FALSE` | `BOOL` (= `i32`), `TRUE`, `FALSE` (from `consts`) |
| `int`, `int32_t` | `i32` |
| `unsigned int`, `uint32_t` | `u32` |
| `uint8_t`, `PCRE2_UCHAR` | `u8` |
| `uint16_t` | `u16` |
| `size_t`, `PCRE2_SIZE` | `usize` |
| `int64_t` / `INT64_OR_DOUBLE` | `i64` |
| `PCRE2_SPTR` | `*const u8` |
| `PCRE2_UCHAR *` | `*mut u8` |
| `void *` | `*mut core::ffi::c_void` |
| `const char *` | `*const core::ffi::c_char` |
| struct types (`compile_block`, `heapframe`, ...) | identically named `#[repr(C)]` structs in `crate::types` |

`BOOL`-returning functions return `1`/`0`; write `if cond { 1 } else { 0 }` or
`(cond) as BOOL`. Test them with `!= 0`.

## Pointers

* `p++` → `p = p.add(1)`; `*p++` → `{ let t = *p; p = p.add(1); t }`.
* `p[i]` → `*p.add(i as usize)`; when `i` may be negative use `*p.offset(i as isize)`.
* `p - 1` where the result may be before the start of the object → `p.wrapping_sub(1)`
  (also `wrapping_add`/`wrapping_offset`). Prefer these in loops that decrement past
  the start, which C does routinely.
* Pointer difference `a - b` → `a.offset_from(b)` (an `isize`) or
  `(a as usize - b as usize)`; pick whatever keeps the C types (`PCRE2_SIZE`, `int`, ...).
* Casting a `*const u8` to `*mut u8`: `p as *mut u8`. Casting between struct pointers:
  `p as *mut heapframe`, `(p as *mut u8).add(n) as *mut heapframe`, etc.
* `memcpy(d,s,n)` → `core::ptr::copy_nonoverlapping(s as *const u8, d as *mut u8, n)`
  (`memmove` → `core::ptr::copy`, `memset(d,c,n)` → `core::ptr::write_bytes(d as *mut u8, c as u8, n)`).
  `memcmp` → compare bytes in a loop or `crate::support::memcmp`-style helper you write locally.
* `strlen`/`strcmp` on `const char*`: write a tiny local loop.

## Memory allocation

`code->memctl.malloc(size, code->memctl.memory_data)` becomes

```rust
((*code).memctl.malloc.unwrap())(size, (*code).memctl.memory_data)
```

and `free` likewise. (`malloc`/`free` fields are `Option<unsafe extern "C" fn ...>` and are
never null in practice.) The default allocators are
`crate::context::default_malloc` / `crate::context::default_free`.

## Macros

`crate::macros` provides the C macros; they are exported crate-wide (`#[macro_use]`),
so call them as `GET!(...)`, `PUT!(...)`, etc. Differences from C:

* `GET!(a, n)`, `GET2!(a, n)` → `u32`; `PUT!(a, n, d)`, `PUT2!(a, n, d)`, `PUTINC!(a,n,d)`,
  `PUT2INC!(a,n,d)` (the latter two assign to `a`, so `a` must be a mutable place).
* `GETCHARINC!(c, eptr)`, `GETCHAR!(c, eptr)`, `GETCHARLEN!(c, eptr, len)` – same as C.
* The `...TEST` variants take `utf` explicitly:
  `GETCHARINCTEST!(c, eptr, utf)`, `GETCHARTEST!(c, eptr, utf)`, `GETCHARLENTEST!(c, eptr, len, utf)`.
* `PUTCHAR!(c, p, utf)` returns `u32`.
* `TABLE_GET!(c, table, default)`, `MAX_255!(c)` (always 1), `CHMAX_255!(c)`,
  `HAS_EXTRALEN!(c)`, `GET_EXTRALEN!(c)`, `NOT_FIRSTCU!(c)`, `SETBIT!(a,b)`,
  `MAPBIT!(map,n)`, `MAPSET!(map,n)`, `BACKCHAR!(p)`, `FORWARDCHAR!(p)`,
  `FORWARDCHARTEST!(p,end)`, `ACROSSCHAR!(cond, p, stmt)`.
* UCD: `GET_UCD!(ch)` (→ `*const ucd_record`), `UCD_CHARTYPE!`, `UCD_SCRIPT!`,
  `UCD_CATEGORY!`, `UCD_GRAPHBREAK!`, `UCD_CASESET!`, `UCD_OTHERCASE!`, `UCD_SCRIPTX!`,
  `UCD_BPROPS!`, `UCD_BIDICLASS!`, `UCD_SCRIPTX_PROP!`, `UCD_BIDICLASS_PROP!`,
  `UCD_BPROPS_PROP!`, `UCD_ANY_I!`, `UCD_DOTTED_I!`, `UCD_FOLD_I_TURKISH!`.
* Parsed-pattern offsets: `PUTOFFSET!(s,p)`, `GETOFFSET!(s,p)`, `GETPLUSOFFSET!(s,p)`,
  `READPLUSOFFSET!(s,p)`, `SKIPOFFSET!(p)`, `SIZEOFFSET` (= 2),
  `META_CODE!(x)`, `META_DATA!(x)`, `META_DIFF!(x,y)`.
* `IS_NEWLINE(p)` / `WAS_NEWLINE(p)`: use the helpers
  `is_newline_block(p, nltype, &mut nllen, nl_ptr, psend, utf)` and
  `was_newline_block(p, nltype, &mut nllen, nl_ptr, psstart, utf)` from `crate::macros`,
  passing the fields of your module's NLBLOCK (e.g. `(*mb).nltype`, `&mut (*mb).nllen`,
  `(*mb).nl.as_ptr()`, `(*mb).end_subject`). It is fine to define a local
  `macro_rules! IS_NEWLINE` wrapper at the top of your module.
* `CHAR_xxx` constants: there is no table of them; write the literal ASCII value the C
  compiler would use, e.g. `CHAR_A` → `b'A'` / `0x41`, `CHAR_LEFT_CURLY_BRACKET` → `b'{'`.
  Keep the C name in a comment if it helps. Character literals in comparisons: cast so the
  types line up, e.g. `if c == b'a' as u32`.
* Local convenience `macro_rules!` are welcome for repetitive C macros defined inside the
  file you are translating (e.g. `ADD_ACTIVE`, `RMATCH`, `CHECKMEMCPY`, ...). Define them
  at the top of your module.

## Emulating `goto`

Rust has no `goto`. Use these patterns, in order of preference:

1. **Forward jump to the end of a block** → labeled block + `break 'label`.
   ```rust
   'skip: {  ...  if cond { break 'skip; }  ... }
   ```
2. **Backward jump to the top of a loop** → `loop { ... continue 'lab; }`.
3. **A label jumped to from several places, in the middle of a function** → put the
   label's code in a `pub(crate) unsafe fn` if the state it uses is small, **or** use a
   state machine:
   ```rust
   const L_TOP: u32 = 0; const L_FOO: u32 = 1; ...
   let mut state = L_TOP;
   'sm: loop { match state {
       L_TOP => { ... state = L_FOO; continue 'sm; }
       L_FOO => { ... }
       _ => {}
   } break; }
   ```
   Keep all the variables the states share declared before the loop.
4. `goto` out of a nested loop → labeled `break 'outer`.

Whatever you choose, the executed sequence of statements must match C exactly.
Add a comment `/* goto LABEL */` at each jump site.

## Switch statements

C `switch` falls through; Rust `match` does not. Translate fall-through explicitly
(duplicate the code, or use a labeled block, or `|` for empty-body case lists).
`switch` on a code unit: `match c as u32 { OP_CHAR => {...}, _ => {} }` — remember that the
`OP_*`, `META_*`, `ESC_*`, `PT_*`, `ucp_*` constants in `crate::consts` are `u32`, so cast
the scrutinee (`*code as u32`) rather than the constants.

## Verifying your work

```
cd translation && cargo build --release 2>&1 | grep -E "^(error|warning: unused)" | head -40
cargo build --release 2>&1 | grep -A8 "src/<yourfile>.rs" | head -80
```

Your file must produce **no errors**. Errors coming from *other* files are not yours
(other modules are still stubs). Do not silence errors by deleting code, and do not
leave `todo!()`/`unimplemented!()` behind.
