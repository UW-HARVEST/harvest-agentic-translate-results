# libpng C -> Rust translation conventions

Read this file **completely** before translating anything.  Every translated
file must follow these rules exactly so that the pieces fit together.

## Goal

A *mechanical*, behaviour-identical transliteration of libpng 1.6.59 C code into
`unsafe` Rust.  **Do not** refactor, "improve", fix bugs, reorder checks, or
change arithmetic.  The resulting shared library must produce byte-identical
output to the C library.

## Where code goes

Each C file `c_src/src/<x>.c` maps to a Rust module.  Large C files are split
into "part" files which are `include!`d into the module, so **all parts of one
C file share a single Rust module scope**:

| C file        | Rust module file  | part files                 |
|---------------|-------------------|----------------------------|
| png.c         | `src/png_c.rs`    | `src/gen/png_c_pNN.rs`     |
| pngerror.c    | `src/pngerror.rs` | (already done)             |
| pngget.c      | `src/pngget.rs`   | `src/gen/pngget_pNN.rs`    |
| ...           | ...               | ...                        |

**A part file must contain only items (functions, `static`s, `const`s, `struct`s).
It must NOT contain `use` statements, `mod` statements, or inner attributes**
(`#![...]`); the enclosing module file already has `use crate::*;` and the crate
root has all the `#![allow(...)]` lint attributes.

Everything from `src/pngtypes.rs`, `src/util.rs`, `src/ffi.rs` and every other
module is in scope automatically via `use crate::*;`.

## Function signatures

* Every function that is **not** `static` in C becomes:

  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C" fn png_foo(a: png_structrp, b: c_int) -> png_uint_32 {
  ```

  The `#[unsafe(no_mangle)]` attribute is mandatory and the symbol name must be
  exactly the C name.

* Every function that **is** `static` in C becomes a module-private Rust fn:

  ```rust
  unsafe fn png_foo(a: png_structrp) -> c_int {
  ```

  *Exception*: a `static` C function whose address is taken (stored in a
  function pointer, e.g. the row filter functions or `png_image_read_*`
  callbacks) must be `unsafe extern "C" fn` (no `#[no_mangle]`, not `pub`):

  ```rust
  unsafe extern "C" fn png_read_filter_row_sub(
      row_info: png_row_infop, row: png_bytep, prev_row: png_const_bytep) {
  ```

* Use the libpng type aliases (`png_structrp`, `png_bytep`, `png_uint_32`,
  `png_const_charp`, `c_int`, `usize`, ...) exactly as they appear in the C
  prototype.  `size_t` -> `usize`, `int` -> `c_int`, `unsigned int` -> `c_uint`,
  `double` -> `f64`, `float` -> `f32`, `char` -> `c_char`, `png_byte` ->
  `png_byte`.

* `png_const_structrp`, `png_const_structp`, `png_const_inforp`,
  `png_const_infop` are all aliases for **mutable** pointers
  (`*mut png_struct` / `*mut png_info`), so `png_constcast(png_structrp, png_ptr)`
  is simply `png_ptr`.

* Since the fn is `unsafe fn`, the whole body is an implicit unsafe block
  (crate edition is 2021).  Do **not** wrap things in `unsafe { }`.

## Pointers

* `NULL` -> `core::ptr::null_mut()` for `*mut T`, `core::ptr::null()` for
  `*const T`.  Comparisons: `p != core::ptr::null_mut()`, or `p.is_null()` /
  `!p.is_null()` — either is fine, be consistent within a function.
* `p[i]` -> `*p.add(i)` when `i` is `usize`, or `*p.offset(i as isize)` when `i`
  may be signed.  For **writes**: `*p.add(i) = v;`
* `*p++` (read then advance) ->
  ```rust
  let v = *sp; sp = sp.add(1);
  ```
  `*--p` (advance then read/write) ->
  ```rust
  sp = sp.sub(1); let v = *sp;
  ```
* `p + n` -> `p.add(n)` / `p.offset(n as isize)`; `p - n` -> `p.sub(n)`.
* Pointer difference `p - q` -> `p.offset_from(q)` (gives `isize`).
* Casting between pointer types is a plain `as`:
  `data as *mut c_void`, `p as *const c_char`, `q as png_bytep`.
* `png_voidcast(type, value)`, `png_aligncast`, `png_aligncastconst` -> `value as <type>`.
* Taking the address of a struct field: `core::ptr::addr_of_mut!((*png_ptr).zstream)`
  (or `&mut (*png_ptr).zstream` — prefer `addr_of_mut!` for fields of a `*mut`).

## Struct field access

`png_ptr->mode` -> `(*png_ptr).mode`.  `info_ptr->text[i].key` ->
`(*(*info_ptr).text.add(i)).key`.

## Function pointer fields and parameters

All C function-pointer types are `Option<unsafe extern "C" fn(..)>` in Rust
(see `src/pngtypes.rs`).  Therefore:

| C                                | Rust                                     |
|----------------------------------|------------------------------------------|
| `png_ptr->error_fn != NULL`      | `(*png_ptr).error_fn.is_some()`           |
| `png_ptr->error_fn == NULL`      | `(*png_ptr).error_fn.is_none()`           |
| `png_ptr->error_fn = NULL`       | `(*png_ptr).error_fn = None`               |
| `png_ptr->error_fn = my_fn`      | `(*png_ptr).error_fn = Some(my_fn)`        |
| `(*(png_ptr->error_fn))(a, b)`   | `((*png_ptr).error_fn.unwrap())(a, b)`     |
| parameter `png_error_ptr fn`     | `fn_: png_error_ptr` (already an `Option`) |

`png_ptr->read_filter[i]` has type `png_read_filter_fn`
(`Option<unsafe extern "C" fn(png_row_infop, png_bytep, png_const_bytep)>`).

## String literals

C string literals become NUL-terminated byte strings cast to a pointer:

```rust
png_error(png_ptr, b"Out of memory\0".as_ptr() as png_const_charp);
```

Adjacent C literals `"a" "b"` are one Rust literal `b"ab\0"`.
A `char` literal `'x'` is `b'x' as c_char` (or just the numeric value).

## Control flow

* `for (i = 0; i < n; i++) { body }` becomes
  ```rust
  let mut i = 0; // or the C type
  while i < n {
      body
      i += 1;
  }
  ```
  **If `body` contains `continue`, you must place the increment before every
  `continue`**, or restructure with a labelled block:
  ```rust
  while i < n {
      'cont: {
          ... break 'cont; ...   // replaces `continue`
      }
      i += 1;
  }
  ```
* `do { body } while (c);` -> `loop { body; if !(c) { break; } }`
* `while (c) body` -> `while c { body }`
* `switch (x) { case A: ...; break; case B: ... }` -> `match x { A => {...}, B => {...}, _ => {} }`
  Consts can be used as match patterns.  **Fall-through** must be made explicit
  by duplicating the shared code into each arm (see `png_format_number` in
  `src/pngerror.rs` for an example).  A `switch` with no `default` needs
  `_ => {}`.
* `goto label` where `label` is *forward* and at the end of an enclosing block:
  wrap the region in a labelled block and use `break 'label;`.
* Ternary `c ? a : b` -> `if c { a } else { b }`.
* `if (x)` where `x` is an integer -> `if x != 0`; `if (!x)` -> `if x == 0`.
  For pointers -> `if !x.is_null()` / `if x.is_null()`.
* Comma expressions and assignments-in-expressions must be rewritten as
  statements.

## Arithmetic

* The crate is built with `overflow-checks = false`, so `+ - *` wrap like C's
  unsigned arithmetic.  Still, prefer `wrapping_add` / `wrapping_sub` /
  `wrapping_mul` / `wrapping_neg` when the C code is *known* to rely on
  wrap-around, and always use them for `png_uint_32`/`png_uint_16` expressions
  that could overflow.  **Never** introduce a panic path.
* Watch out for C integer promotion: `png_byte a, b; a - b` is computed in
  `int`.  Translate as `(a as c_int) - (b as c_int)`.
* Shifts: C `x << n` on a `png_byte` promotes to int.  Use
  `((x as c_int) << n)` then cast back.
* `(png_byte)x` -> `x as png_byte` etc.  Rust `as` on integers truncates just
  like C.
* Division/modulo by a value that could be 0 must be reproduced verbatim (the C
  code never does this on a valid path).
* `abs(x)` -> `abs(x)` (helper in `util.rs`, takes/returns `c_int`).

## Available helpers (from `src/util.rs`)

`memcpy(dst, src, n)`, `memmove`, `memset(dst, v, n)`, `memcmp(a, b, n)` all
take `*mut c_void` / `*const c_void`, so add `as *mut c_void` /
`as *const c_void` casts.  `strlen(s: *const c_char) -> usize`, `strcmp`,
`strncmp`.

Macros translated as functions (same names unless noted):

* `PNG_ROWBYTES(pixel_bits, width)` — both args `usize`, returns `usize`.
* `PNG_TRAILBITS`, `PNG_PADBITS`, `PNG_DIV65535`, `PNG_DIV257`,
  `PNG_OUT_OF_RANGE`, `PNG_COLOR_DIST`, `png_float`.
* **`png_get_uint_32(buf)` / `png_get_uint_16(buf)` / `png_get_int_32(buf)` are
  MACROS inside the library.  Inside translated code call
  `PNG_get_uint_32(buf)`, `PNG_get_uint_16(buf)`, `PNG_get_int_32(buf)`**
  (the identically-named exported functions live in `pngrutil.rs`).
* `PNG_CHUNK_FROM_STRING(s)`, `PNG_STRING_FROM_CHUNK(s, c)`,
  `PNG_CSTRING_FROM_CHUNK(s, c)`, `PNG_CHUNK_ANCILLARY(c)`,
  `PNG_CHUNK_CRITICAL(c)` (returns `bool`), `PNG_CHUNK_PRIVATE(c)`,
  `PNG_CHUNK_RESERVED(c)`, `PNG_CHUNK_SAFE_TO_COPY(c)`,
  `PNG_CHUNK_NAME_VALID(c)` (returns `bool`).
* `png_chunk_flag_from_index(i)`, `png_file_has_chunk(png_ptr, i)` (`bool`),
  `png_file_add_chunk(png_ptr, i)`.  `png_has_chunk(png_ptr, cHNK)` ->
  `png_file_has_chunk(png_ptr, PNG_INDEX_cHNK)`.
* `png_chunk_max(png_ptr)`.
* `PNG_PASS_START_ROW/START_COL/ROW_OFFSET/COL_OFFSET/ROW_SHIFT/COL_SHIFT(pass:
  c_int) -> c_int`, `PNG_PASS_ROWS/COLS(v: png_uint_32, pass: c_int)`,
  `PNG_ROW_FROM_PASS_ROW`, `PNG_COL_FROM_PASS_COL`, `PNG_PASS_MASK(pass, off)`,
  `PNG_ROW_IN_INTERLACE_PASS(y, pass)`, `PNG_COL_IN_INTERLACE_PASS(x, pass)`.
* `png_composite(fg, alpha, bg) -> png_byte` and
  `png_composite_16(fg, alpha, bg) -> png_uint_16` **return** the composite
  instead of assigning it, so
  `png_composite(*dp, *sp, alpha, bg)` becomes `*dp = png_composite(...)`.
  Arguments must be cast to `png_uint_16` / `png_uint_32` respectively.
* `PNG_sRGB_FROM_LINEAR(linear)` is in `png_c.rs`; call it as a normal fn.
* `PNG_IMAGE_*` size macros take `fmt: png_uint_32` or `image: *const png_image`
  — see `src/util.rs`.
* `PNG_ABORT()` -> `PNG_ABORT()`.
* `PNG_UNUSED(x)`, `png_debug*(...)` -> delete the line entirely.

## Local arrays / structs

```c
png_byte buf[13];
```
```rust
let mut buf: [png_byte; 13] = [0; 13];
```
Pass as `buf.as_mut_ptr()` / `buf.as_ptr()`.  `sizeof buf` -> the constant size.

```c
png_color_16 my_background;
```
```rust
let mut my_background: png_color_16 = Default::default(); // == {0}
```
`png_row_info`, `png_color`, `png_color_8`, `png_color_16`, `png_sPLT_entry`,
`png_time`, `png_xy`, `png_XYZ` all implement `Default` and `Copy`.
`png_text`, `png_unknown_chunk`, `png_sPLT_t` are `Copy` but not `Default`
(they contain pointers) — build them field by field or with a struct literal.

Struct assignment `*a = *b;` -> `*a = *b;` works for `Copy` structs;
for `png_struct` / `png_info` use `core::ptr::read` / `core::ptr::write` or
`memcpy`.

C `static const` tables inside a function stay `static`/`const` items — put
them **outside** the function in Rust (module-private), e.g.
```rust
static row_mask: [[[png_uint_32; 6]; 3]; 2] = [ ... ];
```
Note C's `png_uint_32 row_mask[2][3][6]` is `[[[png_uint_32; 6]; 3]; 2]` in Rust
(dimensions reversed in the type, same indexing order `row_mask[a][b][c]`).

## zlib

`png_ptr->zstream` is a `z_stream` value.  `&png_ptr->zstream` ->
`core::ptr::addr_of_mut!((*png_ptr).zstream)`.  The zlib functions
(`inflate`, `deflate`, `inflateReset`, `inflateReset2`, `deflateEnd`,
`deflateBound`, `deflateReset`, `crc32`, ...), the `Z_*` constants,
`uInt`, `uLong`, `ZLIB_IO_MAX`, `MAX_WBITS`, `MAX_MEM_LEVEL`,
`deflateInit2(strm, level, method, wbits, memlevel, strategy)` and
`inflateInit2(strm, wbits)` are all available.

`PNG_INFLATE(pp, flush)` -> `png_zlib_inflate(pp, flush)`.

`ZLIB_VERNUM` is `0x12b0` and `PNG_ZLIB_VERNUM` is `0`, so translate
`#if ZLIB_VERNUM >= 0x1240` branches as *taken* and
`#if PNG_ZLIB_VERNUM >= ...` / `#if PNG_ZLIB_VERNUM != 0` branches as *not taken*
unless the condition also holds for 0.

`PNGZ_MSG_CAST(s)` -> `s`; `PNGZ_INPUT_CAST(b)` -> `b as *const Bytef`.

## Preprocessor conditionals

The build configuration is `c_src/include/pnglibconf.h`.  **Essentially every
`PNG_*_SUPPORTED` macro is defined**, so translate the `#ifdef` branch and drop
the `#else`.  The ones that are **NOT** defined (so translate the `#else`
branch / omit the code):

```
PNG_ARM_NEON_API_SUPPORTED      PNG_ARM_NEON_CHECK_SUPPORTED
PNG_BENIGN_WRITE_ERRORS_SUPPORTED
PNG_DISABLE_ADLER32_CHECK_SUPPORTED   (=> PNG_IGNORE_ADLER32 undefined)
PNG_ERROR_NUMBERS_SUPPORTED
PNG_MIPS_*  PNG_POWERPC_*  PNG_RISCV_*  PNG_LOONGARCH_*  PNG_INTEL_SSE_*
PNG_USER_CONFIG  PNG_MAX_MALLOC_64K  PNG_SMALL_SIZE_T
PNG_FIXED_POINT_MACRO_SUPPORTED  PNG_DEBUG  PNG_PREFIX
PNG_READ_BIG_ENDIAN_SUPPORTED
PNG_ARM_NEON_INTRINSICS_AVAILABLE  PNG_USE_ABS  PNG_NO_MEMZERO
PNG_WRITE_FLUSH_AFTER_IEND_SUPPORTED  PNG_BUILTIN_BSWAP16_SUPPORTED
PNG_STRING_COPYRIGHT  __COVERITY__
```

**Careful**: `PNG_ARM_NEON_IMPLEMENTATION` and `PNG_RISCV_RVV_IMPLEMENTATION` *are*
`#define`d by pngpriv.h (to `0`).  Blocks guarded by `#if PNG_ARM_NEON_IMPLEMENTATION == 1`
are therefore **out**, but a block guarded by `#if defined(PNG_ARM_NEON_IMPLEMENTATION)`
is **in** — this is why `png_struct::riffled_palette` exists in the C struct and
why `png_read_destroy` frees it.

`PNG_ARM_NEON_OPT`, `PNG_MIPS_*_OPT`, `PNG_POWERPC_VSX_OPT`,
`PNG_INTEL_SSE_IMPLEMENTATION`, `PNG_LOONGARCH_LSX_IMPLEMENTATION`,
`PNG_RISCV_RVV_IMPLEMENTATION`, `PNG_MIPS_MSA_IMPLEMENTATION` are all `0`.
`PNG_FILTER_OPTIMIZATIONS` is **not** defined.
`PNG_LIBPNG_VER` is `10659` (so `#if PNG_LIBPNG_VER < 10700` is taken).
`PNG_RELEASE_BUILD` is **false** (`PNG_LIBPNG_BUILD_BASE_TYPE` == BETA == 2 <
RC == 3), so `#if PNG_RELEASE_BUILD` branches are **not** taken.
`PNG_USE_COMPILE_TIME_MASKS` is `1`.
`PNG_sRGB_PROFILE_CHECKS` is `2`.
`PNG_USE_READ_MACROS` **is** defined.

## Diverging functions

`png_error`, `png_chunk_error`, `png_longjmp`, `png_fixed_error` and
`png_safe_error` are declared `-> !` in Rust.  Code after a call to them is
unreachable; that is fine (the `unreachable_code` lint is allowed).  If a
function must return a value and every path ends in `png_error`, no `return` is
needed.

`png_benign_error`, `png_chunk_benign_error`, `png_app_error` return `()`.

## Floating point

`pow`, `floor`, `ceil`, `fabs`, `log`, `exp`, `modf`, `frexp` are declared in
`src/ffi.rs` as the C library functions — call them directly (do **not** use
Rust's `f64::powf` etc., to guarantee identical results).  `DBL_DIG`,
`DBL_MIN`, `DBL_MAX` are available.

## Checklist before finishing

1. Only the functions in your assigned line range, in the same order.
2. Every non-`static` C function has `#[unsafe(no_mangle)] pub unsafe extern "C" fn`
   with the exact C symbol name.
3. No `use`/`mod`/`#![..]` lines in a part file.
4. No `unsafe { }` blocks needed (bodies of `unsafe fn` are implicitly unsafe).
5. Comments from the C source are worth keeping for traceability.

---

## Your workflow (sub-agent instructions)

Project dir (`cd` here for all commands):
`$HARVEST_WORKDIR/translated_rust`

1. Read this whole file.
2. Read `src/pngerror.rs`, `src/pngmem.rs`, `src/gen/png_c_p01.rs` — finished
   reference translations.  Match their style exactly.
3. Read `src/pngtypes.rs` and `src/util.rs` so you use the correct struct field
   names, type aliases and helper names.
4. Read the module header `src/<MOD>.rs` for your C file: it already declares the
   file-scope constants / types / data tables.  **Do NOT redefine them.**
5. Read your assigned C line range with the `Read` tool (`offset`/`limit`) and
   write the translation of **every function whose definition begins in that
   range** into your assigned `src/gen/<MOD>_pNN.rs`, overwriting the placeholder.
   Nothing else goes in that file.
6. Verify:
   ```
   cd $HARVEST_WORKDIR/translated_rust \
     && cargo build --release --target-dir target/<your unique dir> 2>&1 \
        | grep -B3 -A10 "<MOD>_pNN.rs" | head -250
   ```
   Fix every error located in **your** file, except `E0425 cannot find
   function/value <png_...>` for libpng functions that live in other,
   not-yet-translated part files — those are expected.  Repeat until only such
   expected errors remain.  Do not edit any file other than your own part file.
7. Report the list of functions translated and `wc -l` of your file.


---

## Validation

`png_struct` (1232 bytes), `png_info` (352 bytes) and every one of their
149 + 71 field offsets were verified byte-for-byte against the C headers, along
with `z_stream`, `jmp_buf`, `png_image`, `png_control`, `png_text`,
`png_unknown_chunk`, `png_sPLT_t`, `png_color_16`, `png_row_info`, `png_time`
and `png_compression_buffer`.

`nm -D` on the Rust `libpng.so` exports exactly the same 384 symbols as the C
`libpng.so`, with the same symbol types (381 `T`, 3 `R`).

Six differential harnesses (~9.5k lines of traced output: every colour type x bit
depth x interlace, all filters/compression levels/strategies, every read
transform and combination, the high-level `png_read_png`/`png_write_png` API,
the simplified read/write API in every format, progressive reading at many chunk
sizes, custom allocators, user chunk/transform/status callbacks, CRC actions,
MNG intrapixel, ICC profiles, ancillary-chunk data fuzzing with repaired CRCs,
truncated and bit-flipped input) produce **byte-identical** output.

### Known residual difference

libpng's `png_handle_iCCP` leaves `png_ptr->zstream.next_in` pointing into its
own dead stack frame (the local `char keyword[81]`) when it rejects a profile
mid-decompression.  If the application then calls `png_read_end()` *without*
reading any rows, `png_read_IDAT_data()` inflates from that dangling pointer.
The bytes it sees are whatever the callee frames happen to leave there, so the
resulting zlib message ("incorrect data check" vs. "invalid distance too far
back") is undefined: the C library itself returns different answers depending
only on the caller's stack depth.  This affects 8 of ~9500 traced output lines
and nothing else; control flow and all data are identical.
