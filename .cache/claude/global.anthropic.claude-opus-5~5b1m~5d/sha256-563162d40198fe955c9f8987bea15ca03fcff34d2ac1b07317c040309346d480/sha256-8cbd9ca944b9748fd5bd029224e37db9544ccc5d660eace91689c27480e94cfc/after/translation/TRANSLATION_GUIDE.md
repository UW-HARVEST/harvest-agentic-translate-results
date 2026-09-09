# libpng → Rust translation conventions (READ FULLY BEFORE WRITING CODE)

You are translating **one contiguous line range of one C file** from
`../c_src/src/` into **one Rust module file** in `src/`.
The crate is a `cdylib` that must export the *exact* public ABI of the C
library and produce **byte-identical** output. Treat this as a
**mechanical, line-by-line transliteration**. Do NOT redesign, do NOT
"improve", do NOT fix bugs, do NOT reorder checks, do NOT change messages.

## 0. Ground rules

* Never modify anything under `c_src/`.
* Only write the single `src/<your-module>.rs` file you are assigned.
  Do not touch `lib.rs`, `types.rs`, `pngstruct.rs`, `pngpriv.rs`,
  `shared.rs`, `zlib.rs`, `cabi.rs`, or any other module.
* Translate **every** function in your assigned range, in the same order.
* Keep the original C comments (they document subtle behaviour).
* Keep C identifier names verbatim (`png_do_expand`, `iout`, `dp`, ...).
* The build config is `c_src/include/pnglibconf.h`. **Every** `#ifdef`
  in that file that is `#define`d is ON. In particular ON:
  READ, WRITE, SEQUENTIAL_READ, PROGRESSIVE_READ, SETJMP, STDIO,
  CONSOLE_IO, USER_MEM, USER_LIMITS, SET_USER_LIMITS, BENIGN_ERRORS,
  BENIGN_READ_ERRORS, WARNINGS, ERROR_TEXT, FLOATING_POINT,
  FLOATING_ARITHMETIC, FIXED_POINT, ALIGNED_MEMORY, POINTER_INDEXING,
  READ/WRITE_TRANSFORMS, all chunk types incl. cICP/cLLI/mDCV/eXIf,
  SIMPLIFIED_READ/WRITE (+BGR/AFIRST/STDIO), MNG_FEATURES,
  READ_COMPOSITE_NODIV, 16BIT, EASY_ACCESS, INCH_CONVERSIONS, IO_STATE,
  INFO_IMAGE, TEXT, TIME_RFC1123, CONVERT_tIME, COLORSPACE, GAMMA,
  SET_OPTION, HANDLE_AS_UNKNOWN, STORE/SAVE/SET/READ/WRITE_UNKNOWN_CHUNKS,
  USER_CHUNKS, READ_USER_CHUNKS, CHECK_FOR_INVALID_INDEX,
  GET_PALETTE_MAX, BUILD_GRAYSCALE_PALETTE, WRITE_FILTER,
  WRITE_WEIGHTED_FILTER, WRITE_FLUSH, WRITE_OPTIMIZE_CMF,
  WRITE_CUSTOMIZE_COMPRESSION, WRITE_CUSTOMIZE_ZTXT_COMPRESSION,
  READ_QUANTIZE, READ_EXPAND, READ_EXPAND_16, READ_BACKGROUND,
  READ_ALPHA_MODE, READ_RGB_TO_GRAY, READ_GRAY_TO_RGB, READ_SHIFT,
  READ_PACK, READ_PACKSWAP, READ_SWAP, READ_SWAP_ALPHA, READ_INVERT,
  READ_INVERT_ALPHA, READ_FILLER, READ_STRIP_ALPHA,
  READ_STRIP_16_TO_8, READ_SCALE_16_TO_8, READ_INTERLACING,
  WRITE_INTERLACING, READ_USER_TRANSFORM, WRITE_USER_TRANSFORM,
  USER_TRANSFORM_PTR, USER_TRANSFORM_INFO, READ/WRITE_INT_FUNCTIONS,
  SAVE_INT_32, READ_OPT_PLTE, READ_COMPRESSED_TEXT,
  WRITE_COMPRESSED_TEXT, READ_ANCILLARY_CHUNKS, WRITE_ANCILLARY_CHUNKS.

  OFF (skip that code entirely, do not translate the `#else`/`#ifndef`
  branch's *alternative* unless it is the active one):
  `PNG_BENIGN_WRITE_ERRORS_SUPPORTED`, `PNG_ERROR_NUMBERS_SUPPORTED`,
  `PNG_DISABLE_ADLER32_CHECK_SUPPORTED`, `PNG_MAX_MALLOC_64K`,
  `PNG_SMALL_SIZE_T`, `PNG_FIXED_POINT_MACRO_SUPPORTED`,
  `PNG_USE_READ_MACROS` (i.e. real functions are compiled),
  `PNG_ARM_NEON_*`, `PNG_MIPS_*`, `PNG_POWERPC_*`, `PNG_INTEL_SSE_*`,
  `PNG_LOONGARCH_*`, `PNG_RISCV_*`, `PNG_FILTER_OPTIMIZATIONS`,
  `PNG_ARM_NEON_IMPLEMENTATION`, `PNG_RISCV_RVV_IMPLEMENTATION`,
  `PNG_READ_BIG_ENDIAN_SUPPORTED`.

  Version tests: `PNG_LIBPNG_VER` is `10659` so `PNG_LIBPNG_VER < 10700`
  is TRUE. `ZLIB_VERNUM` is `0x12b0` so `ZLIB_VERNUM >= 0x1240`,
  `>= 0x1260`, `>= 0x1290` are all TRUE and `< 0x1260` is FALSE.
  `PNG_RELEASE_BUILD` is FALSE (`PNG_LIBPNG_BUILD_BASE_TYPE` is BETA).
  `PNG_sRGB_PROFILE_CHECKS` is 2 (`>= 0` is TRUE, `> 1` is TRUE).
  `PNG_USE_COMPILE_TIME_MASKS` is 1.
  `INT_MAX` = `c_int::MAX`, `PNG_SIZE_MAX` = `usize::MAX`.

## 1. Module preamble

Start the file with exactly:

```rust
//! <c file name>.c lines <A>-<B>: <short description>
use crate::prelude::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};
```

`crate::prelude::*` already gives you: all libpng types
(`png_structrp`, `png_bytep`, `png_uint_32`, `png_color_16`, ...), every
`PNG_*`/`png_*` constant from png.h and pngpriv.h, `png_struct` and
`png_info` field access, the zlib bindings and constants, the
`memcpy/memset/memcmp/strlen/strcmp` shims, and **every function of
every other translated module**. Do not re-declare anything from it.

## 2. Function signatures

* A C function that is **not** `static` becomes:

  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C-unwind" fn png_foo(a: png_structrp, b: c_int) -> c_int { ... }
  ```

  `extern "C-unwind"` (not `"C"`) — this is required everywhere so that the
  panic used internally in place of `longjmp` can propagate.

* A C function that **is** `static` becomes a plain, but still `pub`, item
  (it must be `pub` because the C file is split over several Rust modules):

  ```rust
  pub unsafe fn png_bar(a: png_structrp) -> c_int { ... }
  ```

  A `static` C function used as a *callback* (i.e. its address is taken and
  stored in a `png_rw_ptr` / `png_error_ptr` / etc.) must instead be
  `pub unsafe extern "C-unwind" fn` **without** `#[unsafe(no_mangle)]`.

* Type mapping:

  | C | Rust |
  |---|---|
  | `png_structp`, `png_structrp` | `png_structrp` (`*mut png_struct`) |
  | `png_const_structp`, `png_const_structrp` | `png_const_structrp` (`*const png_struct`) |
  | `png_infop`, `png_inforp` | `png_inforp` (`*mut png_info`) |
  | `png_const_infop/rp` | `png_const_inforp` |
  | `int` | `c_int` |
  | `unsigned int` | `c_uint` |
  | `long` | `c_long` |
  | `double` | `c_double` (or `f64`) |
  | `float` | `f32` |
  | `size_t`, `png_alloc_size_t` | `usize` |
  | `ptrdiff_t` | `isize` |
  | `char*` / `png_charp` | `png_charp` (`*mut c_char`) |
  | `const char*` / `png_const_charp` | `png_const_charp` |
  | `void*` / `png_voidp` | `png_voidp` |
  | `png_uint_32` | `png_uint_32` (`u32`) |
  | `png_byte` | `png_byte` (`u8`) |
  | `uInt` (zlib) | `uInt` (`c_uint`) |
  | `uLong` (zlib) | `uLong` (`c_ulong`) |
  | `FILE *` | `*mut c_void` |
  | `char out[29]` param | `*mut c_char` |
  | `png_warning_parameters p` param | `*mut [c_char; PNG_WARNING_PARAMETER_SIZE]` |
  | function pointer typedefs | the `Option<unsafe extern "C-unwind" fn ...>` aliases in the prelude |

* Functions marked `PNG_NORETURN` (`png_error`, `png_chunk_error`,
  `png_longjmp`, `png_fixed_error`) already have return type `!` in this
  crate.  (`png_safe_error` returns `()` because it must be storable in a
  `png_error_ptr`, but it still never returns.) So `png_error(pp, msg);` diverges — code after it
  is dead, exactly as in C. Don't add `return`s that C doesn't have; if the
  compiler complains about unreachable code the crate has
  `#![allow(unreachable_code)]`.

## 3. Body translation

* Wrap nothing: the whole `fn` is already `unsafe`, so use raw pointer
  dereferences directly. `png_ptr->width` → `(*png_ptr).width`.
* `foo->bar` → `(*foo).bar`; `a[i]` on a pointer → `*a.add(i)`.
* **Pointer arithmetic**: `p + n` → `p.add(n)`, `p - n` → `p.sub(n)`,
  `++p` → `p = p.add(1)`, `*p++ = v` → `*p = v; p = p.add(1)`,
  `*--p = v` → `p = p.sub(1); *p = v`.
  If an index can be negative use `p.offset(i as isize)`.
  Taking the address of a struct field: `&png_ptr->background` →
  `core::ptr::addr_of_mut!((*png_ptr).background)` (or `&mut (*p).f` when a
  reference is fine).
* **C integers wrap and never panic.** The release profile has
  `overflow-checks = false`, but be defensive in obviously-wrapping code:
  use `wrapping_add/sub/mul/neg`, and for `x - y` on unsigned values that
  the C code allows to go negative use `wrapping_sub`.
  Never use `as` casts that could panic — `as` never panics in Rust, so
  plain `as` is the right translation of every C cast.
  Float → int C casts truncate: `x as i32` in Rust also truncates. OK.
  For `(png_byte)x` use `x as png_byte`; for `(int)(char)x` use
  `x as i8 as c_int`.
* **Right shifts** of signed values are arithmetic in both languages: OK.
  Shifts by >= bit width are UB in C; write the same expression, it will
  not be reached.
* **Division by a variable** which C guarantees non-zero: write it plainly.
* `sizeof x` → `core::mem::size_of_val(&x)`;
  `sizeof (T)` → `core::mem::size_of::<T>()`;
  `sizeof arr` for `T arr[N]` → `N * size_of::<T>()` (usually just `N` for
  `char`/`png_byte` arrays — read the C carefully).
* `memcpy(d, s, n)` → `memcpy(d as *mut u8, s as *const u8, n)`.
  `memset(d, v, n)` → `memset(d as *mut u8, v as u8, n)`.
  `memcmp(a, b, n)` → `memcmp(a as *const u8, b as *const u8, n)`.
  (Use `memmove` if the C used `memmove`.)
* **String literals**: `"foo"` as a `png_const_charp` argument →
  `c"foo".as_ptr()`. A C string literal split over lines with implicit
  concatenation must be joined into one `c"..."`.
  A `static const char x[] = "abc";` becomes
  `pub static x: [c_char; 4] = [b'a' as c_char, ...];` — or, more simply,
  `pub const x: &[u8] = b"abc\0";` and use `x.as_ptr() as *const c_char`.
  Prefer whichever keeps the code readable; the bytes must match.
* **Local arrays**: `char buf[32];` → `let mut buf: [c_char; 32] = [0; 32];`
  then `buf.as_mut_ptr()` where C used `buf`.
  `png_byte buf[4];` → `let mut buf: [png_byte; 4] = [0; 4];`.
  C leaves them uninitialised; zeroing is safe and behaviourally identical
  for this code base (it always writes before reading).
* **`static` file-scope C variables** → `pub static NAME: [T; N] = [...]`
  (immutable) — keep the C name.
* **Structs declared locally in the C file**: the shared ones
  (`png_image_read_control`, `png_image_write_control`,
  `compression_state`) are already in `crate::shared` (in the prelude) —
  use them, do not redefine. Any other local `typedef struct` that only
  your range uses, define at the top of your file with `#[repr(C)]`.
* **`for (;;)` / `goto`**: use `loop {}` + `break`/`continue` with labels.
  A forward `goto err;` is best modelled by a labelled block:
  ```rust
  'err: {
      ...
      if cond { break 'err; }
      ...
      return x;
  }
  /* err: */
  ...
  ```
  Keep the control flow *identical*.
* **`switch`**: use `match` with `_ =>` for `default`. C fall-through must be
  reproduced explicitly (duplicate the code or restructure with `if`).
  Beware: several libpng constants have the *same value*
  (`PNG_NUMBER_FORMAT_u == PNG_NUMBER_FORMAT_d == 1`); if two match arms
  would collide, use an `if/else if` chain instead.
* **Comma expressions / assignment-in-condition**: hoist into statements,
  preserving evaluation order exactly.
* `png_debug*(...)` → delete (they are no-ops in this build).
* `PNG_UNUSED(x)` → delete (or `let _ = x;`).
* `png_voidcast(T, v)` / `png_constcast(T, v)` / `png_aligncast(T, v)` →
  `v as T`.
* `png_float(pp, fixed, s)` → `png_float_of(fixed)` (prelude).
* `png_fixed(pp, fp, s)` / `png_fixed_ITU(...)` are real functions here
  (the macro form is disabled) — just call them.
* `PNG_ROWBYTES(pd, w)` → `PNG_ROWBYTES(pd as u32, w as png_uint_32)`.
* `png_composite(c, fg, alpha, bg)` → `c = png_composite(fg as u16, alpha as u16, bg as u16)`
  (note: it *returns* the value here instead of assigning through a macro
  argument); likewise `png_composite_16(c, fg, alpha, bg)` →
  `c = png_composite_16(fg as u32, alpha as u32, bg as u32)`.
* `png_has_chunk(pp, cHNK)` → `png_file_has_chunk!` does not exist; write
  `((*png_ptr).chunks & png_chunk_flag_from_index(PNG_INDEX_cHNK)) != 0`.
  `png_file_add_chunk(pp, i)` → `(*png_ptr).chunks |= png_chunk_flag_from_index(i);`
* `PNG_STRING_FROM_CHUNK(s, c)`, `PNG_CSTRING_FROM_CHUNK(s, c)`,
  `PNG_CHUNK_FROM_STRING(s)` are `unsafe fn`s in the prelude taking
  `*mut png_byte` / `*const png_byte`.
* `png_handle_result_code` values are the `c_int` constants
  `handled_error`, `handled_discarded`, `handled_saved`, `handled_ok`;
  functions returning that enum return `c_int`.
* Math from `<math.h>`: `floor(x)` → `x.floor()`, `pow(a,b)` → `a.powf(b)`,
  `log(x)` → `x.ln()`, `log10(x)` → `x.log10()`, `exp(x)` → `x.exp()`,
  `ceil(x)` → `x.ceil()`, `fabs(x)` → `x.abs()`,
  `frexp(v, &e)` → use the helper `png_frexp(v)` returning `(f64, c_int)`
  which you may define locally if you need it (mimic C exactly:
  `m in [0.5,1)`, `v == m * 2^e`, and `frexp(0) == (0, 0)`),
  `modf(v, &i)` → `let i = v.trunc(); let frac = v - i;`.
  `abs(i)` on `c_int` → `i.abs()` (use `wrapping_abs` if `i` may be
  `INT_MIN`).
  `DBL_MIN`/`DBL_MAX`/`DBL_DIG` → `f64::MIN_POSITIVE` / `f64::MAX` / `15`.
* zlib: `deflateInit2(&pp->zstream, l, m, w, mem, s)` →
  `deflateInit2(&mut (*png_ptr).zstream, l, m, w, mem, s)`;
  `inflateInit2(&pp->zstream, w)` → `inflateInit2(&mut (*png_ptr).zstream, w)`.
  `PNGZ_MSG_CAST(s)` → `s` (as `*const c_char`); assigning a literal to
  `zstream.msg` is `(*png_ptr).zstream.msg = c"...".as_ptr();`.
  `PNGZ_INPUT_CAST(b)` → `b as *const u8`.
  `PNG_INFLATE(pp, flush)` → `png_zlib_inflate(png_ptr, flush)`.
  `Z_NULL` → `core::ptr::null_mut()` / `0` / `None` depending on context.
* stdio: use `crate::cabi::{fopen, fclose, fread, fwrite, fflush, ferror,
  fprintf, malloc, free, abort, gmtime, tm, time_t, stderr_ptr}`.
  `fopen(name, "rb")` → `crate::cabi::fopen(name, c"rb".as_ptr())`.
* `setjmp`/`longjmp` inside libpng: **only** `png_create_png_struct`,
  `png_free_jmpbuf` and `png_safe_execute` do this and they are already
  translated (`png_free_jmpbuf`/`png_safe_execute` in `pngerror.rs`). If
  your range contains one, use `std::panic::catch_unwind` +
  `std::panic::AssertUnwindSafe` and set
  `(*png_ptr).longjmp_fn = Some(png_internal_longjmp)` with a dummy
  non-NULL `jmp_buf_ptr`, mirroring `png_free_jmpbuf` in `src/pngerror.rs`.
* `png_ptr->jmp_buf_local` has Rust type `jmp_buf` (an opaque
  `[u64; 25]` newtype); `&png_ptr->jmp_buf_local` →
  `core::ptr::addr_of_mut!((*png_ptr).jmp_buf_local)`.
* `png_control` bitfields `for_write` / `owned_file` are accessed via
  `(*ctrl).for_write()` / `(*ctrl).set_for_write(true)` etc.
* Zero-initialising a `png_struct` / `png_info` / `png_control`:
  `png_struct::default()` (equals `memset(...,0,...)`).

## 4. Things that must be bit-exact

* Every literal, every error/warning message string (byte for byte,
  including capitalisation and spaces).
* The order of validation and of `png_error`/`png_warning`/
  `png_benign_error`/`png_chunk_report` calls.
* All arithmetic, including intermediate types. When C promotes to `int`
  before an operation, cast to `c_int` in Rust too. E.g.
  `(png_byte)(a + b)` with `png_byte a, b` is
  `((a as c_int) + (b as c_int)) as png_byte`.
* Loop bounds and off-by-one behaviour.

## 5. Finish

Run, from the `translation/` directory:

```
cargo build --release 2>&1 | tail -60
```

Errors in *other* modules (not yet written) are expected and are fine;
**errors in your own file are not** — fix them. Iterate until your file is
free of errors. Report at the end: the module path you wrote, the number of
functions translated, and any C construct you were unsure about.
