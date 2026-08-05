# libpng → Rust translation conventions (READ FIRST)

You are translating ONE C source file from `c_src/src/<name>.c` into
`src/<name>.rs` of a Rust cdylib that reproduces libpng 1.6.59's public ABI
**byte-for-byte**. The foundation is already written; you only write the
module file.

## Ground rules
- Do **NOT** modify anything under `c_src/`. Do not touch other `src/*.rs`
  files, `Cargo.toml`, `build.rs`, `lib.rs`, or the shared modules
  (`cffi.rs`, `consts.rs`, `helpers.rs`, `pstruct.rs`, `ptypes.rs`).
- Preserve behaviour EXACTLY, including bugs, order of error checks, integer
  overflow/wrapping, and rounding. Do not "improve" anything.
- The whole library is built with the full feature set enabled
  (`pnglibconf.h`). Every `#ifdef` in the C for a feature listed as
  `#define ..._SUPPORTED` there is ACTIVE; `#undef`/commented ones are OFF
  (e.g. ARM/NEON/MIPS/SSE/POWERPC/RISCV are OFF; ERROR_NUMBERS OFF;
  BENIGN_WRITE_ERRORS OFF). Translate only the active branches.

## File skeleton
```rust
//! Translation of <name>.c
use crate::prelude::*;
```
The prelude gives you: all png types (`png_structrp`, `png_bytep`, …), all
constants (`PNG_INFO_*`, `png_IDAT`, `PNG_FLAG_*`, …), zlib + libc FFI
(`malloc`, `memcpy`, `memset`, `fread`, `deflate`, `inflate`, `crc32`,
`z_stream`, `Z_OK`, …), helper fns (`png_rowbytes`, `png_div257`,
`png_cstring_from_chunk`, `png_chunk_from_string`, …), and **every other
libpng function from every module** (cross-module calls are ordinary Rust
calls — just call `png_malloc(png_ptr, n)` etc.).

## Exporting functions
Every function that the C file defines with `PNGAPI`/`PNGCBAPI` (exported) OR
that is a `PNG_INTERNAL_FUNCTION` (used across files) MUST be:
```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_foo(png_ptr: png_structrp, ...) -> RetType { ... }
```
Use the EXACT C signature. Map C types to prelude aliases:
`png_structrp`→`png_structrp`, `png_bytep`→`png_bytep`,
`png_const_charp`→`png_const_charp`, `int`→`c_int`, `unsigned int`→`c_uint`,
`size_t`→`size_t`, `png_uint_32`→`png_uint_32`, `double`→`f64`,
`voidpf`→`voidpf`, `uInt`→`uInt`. A C function pointer typedef like
`png_rw_ptr` is `Option<unsafe extern "C" fn(...)>` — call via
`(f.unwrap())(args)` after `f.is_some()`.

File-`static` (private) C functions become module-private `unsafe fn` (NOT
`no_mangle`, NOT `pub` required). File-`static` const data becomes `static`.

## Field access
`png_ptr->field` → `(*png_ptr).field`. All struct fields exist with the same
names in `pstruct.rs` (`png_struct_def`, `png_info_def`). Public struct field
names match `ptypes.rs`. Callback fields are `Option<fn>`; set with
`Some(func)` / `None`, test with `.is_some()`, assign a null the C code sets
via `= None`.

## Strings
C string literals → `c"..."` C-string literals: `c"IHDR".as_ptr()`. Pass
message literals to `png_error(png_ptr, c"msg".as_ptr())`.

## Common macros already provided (call as functions)
- `PNG_ROWBYTES(pixel_bits,width)` → `png_rowbytes(pixel_bits, width)`
- `PNG_DIV257(v)` → `png_div257(v)`, `PNG_DIV65535` → `png_div65535`
- `PNG_CSTRING_FROM_CHUNK(s,c)` → `png_cstring_from_chunk(s, c)`
- `PNG_CHUNK_FROM_STRING(s)` → `png_chunk_from_string(s)`
- `PNG_STRING_FROM_CHUNK(s,c)` → `png_string_from_chunk(s, c)`
- chunk id constants: `png_IHDR`, `png_IDAT`, … (u32 values)
- `PNG_CHUNK_CRITICAL(c)` → `png_chunk_critical(c)` (returns u32 0/1), etc.
- `png_debug*` macros → drop them (they are no-ops in this build).
- `PNG_UNUSED(x)` → drop it (or `let _ = x;`).
- `png_voidcast(type, v)` / `png_constcast(type, v)` / `png_aligncast` →
  just `v as TargetType` (a Rust cast).
- `png_sizeof(x)` → `core::mem::size_of::<X>()`.

## zlib
`png_ptr->zstream` is a real `z_stream` (system zlib). Call `deflate`,
`inflate`, `deflateReset`, etc. from the prelude. `deflateInit2(&mut zs, ...)`
and `inflateInit2(&mut zs, wbits)` helper wrappers exist. Return codes:
`Z_OK`, `Z_STREAM_END`, `Z_BUF_ERROR`, flush values `Z_NO_FLUSH`,
`Z_FINISH`, etc. `crc32(crc, ptr, len)` available.

## Integer semantics
Use `wrapping_add`/`wrapping_sub`/`wrapping_mul`/`wrapping_shl` or `as`
truncation to mirror C wraparound. Right shifts on unsigned are logical.
Casts between int widths use `as`. Be careful to match C promotion where the
result matters.

## Do NOT
- Do not add `mod` declarations. Do not create helper files.
- Do not call into the C library. Everything is pure Rust + zlib/libc FFI.

## Verify before finishing
Your file must define **every** exported symbol the C object exports. The
list for your file is in `/tmp/symmap/<name>.txt`. After writing, mentally
check each symbol there has a matching `#[unsafe(no_mangle)] pub unsafe extern
"C" fn`.
