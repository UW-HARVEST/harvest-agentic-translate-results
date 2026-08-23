# Translation conventions (READ FULLY BEFORE WRITING CODE)

We are translating libpng 1.6.59 (C) to Rust, function by function, into a
`cdylib` that must export the same ABI and produce **byte-identical** results.
This is a *transliteration*, not a redesign: same control flow, same order of
checks, same messages, same arithmetic, same bugs.

## What to translate

* Translate from `active/<file>.c`.  Those files are the C sources **after the
  C preprocessor resolved every `#if/#ifdef`**, so everything you see there is
  compiled into the library and must be translated.  (`c_src/src/<file>.c` is
  the original if you want the surrounding context; never modify `c_src/`.)
* Comments: keep the important ones, they explain the logic.
* Ignore/drop: `png_debug*(...)` calls (no-ops), `PNG_UNUSED(x)`,
  `PNG_ARG_UNUSED`.

## Where to write

Write **only** the file you are told to write, in `src/`.  Never touch
`src/lib.rs`, `src/ctypes.rs`, `src/pngtypes.rs`, `src/pngconsts.rs`,
`Cargo.toml`, `build.rs` or anything in `c_src/`.

Every file starts with:

```rust
use crate::*;
```

`crate::*` re-exports **all** types, constants, macros-as-functions and **all
functions of all other modules**, so you can call any libpng function directly
by its C name (`png_error(png_ptr, cstr!("..."))`).

## Function declarations

* A non-static C function (i.e. one declared in `png.h`/`pngpriv.h`) becomes

  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C" fn png_foo(png_ptr: png_structrp, ...) -> ...
  ```

  **The signature must be copied verbatim from `SIGNATURES.md`** (that file
  lists the exact required Rust signature of all 381 exported functions).
  Do not invent signatures, do not rename parameters into something else if the
  file says otherwise, and never omit `#[unsafe(no_mangle)]`.
* A `static` C function becomes a *private* (not `pub`) function:

  ```rust
  unsafe fn png_do_something(row_info: png_row_infop, row: png_bytep) { ... }
  ```

  Do **not** put `#[no_mangle]` or `pub` on it, and do not make it `extern "C"`
  unless its address is taken and stored in a C function pointer (in that case
  use `unsafe extern "C" fn` so it is ABI compatible).
* File scope `static`/`static const` data becomes `static` items; if the symbol
  is one of `png_sRGB_table`, `png_sRGB_base`, `png_sRGB_delta` it must be
  `#[unsafe(no_mangle)] pub static ...` (they are exported data symbols),
  otherwise a private `static`.
* Bodies of `unsafe fn` are implicitly unsafe (edition 2021), so no `unsafe {}`
  blocks are needed inside.
* `png_error`, `png_chunk_error`, `png_fixed_error` and `png_longjmp` are
  declared `-> !` (they never return).  Anything else returns what
  `SIGNATURES.md` says.

## Types

Use the aliases from `src/ctypes.rs` / `src/pngtypes.rs` (all in scope through
`use crate::*;`):

| C | Rust |
|---|---|
| `int`, `unsigned int` | `c_int`, `c_uint` |
| `size_t` | `usize` |
| `double` | `f64` |
| `png_byte`, `png_uint_16`, `png_uint_32`, `png_int_32`, `png_fixed_point`, `png_alloc_size_t` | same names |
| `png_structrp`, `png_const_structrp`, `png_structp`, `png_const_structp` | same names (`*mut`/`*const png_struct`) |
| `png_inforp`, `png_const_inforp`, `png_infop`, `png_infopp` | same names |
| `png_bytep`, `png_const_bytep`, `png_charp`, `png_const_charp`, ... | same names |
| `FILE *` | `*mut FILE` (alias `png_FILE_p`) |
| `z_stream`, `uInt`, `uLong`, `voidpf` | same names |

Struct fields have exactly the same names as in C; see `src/pngtypes.rs`.
Field access is `(*png_ptr).width`, `(*info_ptr).valid` etc.

## Pointers, memory and strings

* `NULL` → `core::ptr::null_mut()` / `core::ptr::null()`; test with
  `p.is_null()`.
* Pointer arithmetic: `p.offset(i as isize)`, `p.add(n)`, `p.sub(n)`,
  `*p.add(i)` for `p[i]`.  Pointer difference: `(a as isize) - (b as isize)`
  when the C code does `a - b` on `png_bytep` (bytes), else divide by the
  element size.
* `memcpy`, `memset`, `memcmp`, `strlen`, `malloc`, `free`, `abort`, `pow`,
  `floor`, `fabs`, `modf`, `frexp`, `strtod`, `gmtime`, `fopen`, `fread`,
  `fwrite`, `fflush`, `fclose`, `ferror`, `fprintf`, `fputc`, `remove`,
  `strerror`, `stderr` are declared in `ctypes.rs`: call them exactly like C
  does (`memcpy(dst as *mut c_void, src as *const c_void, n)`).  Prefer them
  over Rust slice code so behaviour matches byte for byte.
* `memset(x, 0, sizeof *x)` on a struct pointer →
  `core::ptr::write_bytes(x as *mut u8, 0, core::mem::size_of::<T>())`.
* `sizeof(T)` → `core::mem::size_of::<T>()`; `offsetof(T,f)` →
  `core::mem::offset_of!(T, f)`.
* C string literals: use the `cstr!` macro: `png_error(png_ptr, cstr!("bad IHDR"))`
  expands to a NUL terminated `*const c_char`.  Keep the message text
  **exactly** as in C (it is part of the observable output).
* A local C array `png_byte buf[5];` → `let mut buf: [png_byte; 5] = [0; 5];`
  and pass `buf.as_mut_ptr()`.  A local struct that C memsets to 0 →
  `let mut s: T = core::mem::zeroed();`
* `png_voidcast(type, value)`, `png_constcast(type, value)`,
  `png_aligncast*(...)` are just casts: `value as *mut Foo` etc.

## Arithmetic

* Mirror the C integer promotions with explicit `as` casts.  When C computes on
  `int` (e.g. `(int)a - (int)b`), cast to `c_int` first.
* Unsigned wrap-around is intentional in places: use `wrapping_add`,
  `wrapping_sub`, `wrapping_mul` when a C unsigned expression can overflow
  (e.g. `(a - b) & 0xffff`, CRC arithmetic, `0U - 1U`).  The release profile has
  overflow checks off, but be explicit where wrapping is the point.
* Shifts: `x << n` where `x` is unsigned in C stays unsigned in Rust.
  Beware `1 << 31` on `c_int`: use `1u32 << 31` if C used unsigned.
* Floating point: use the libm functions from `ctypes` (`pow(a,b)`,
  `floor(x)`, `modf`, `frexp`, `fabs`) — do **not** use `f64::powf` etc., so the
  results are bit-identical with the C build.
* Integer/float conversions: `x as c_int`, `y as f64`; C `(int)double`
  truncates toward zero, and so does Rust `as`.

## Function pointers

All C function pointer types are `Option<unsafe extern "C" fn(...)>`:

```rust
if (*png_ptr).error_fn.is_some() {
    ((*png_ptr).error_fn.unwrap())(png_ptr as png_structp, message);
}
```

`NULL` is `None`.  Assign a Rust function with `Some(png_default_read_data as unsafe extern "C" fn(png_structp, png_bytep, usize))`
when inference needs help, otherwise just `Some(png_default_read_data)`.

## zlib

`ctypes.rs` binds the real zlib: `deflateInit2(strm, level, method, bits, mem, strategy)`,
`deflate`, `deflateEnd`, `deflateReset`, `deflateBound`, `inflateInit2(strm, bits)`,
`inflate`, `inflateEnd`, `inflateReset`, `inflateReset2`, `crc32`, plus all the
`Z_*` constants and `ZLIB_VERNUM` (= 0x12b0, i.e. treat the build as zlib 1.2.11:
`#if ZLIB_VERNUM >= 0x1240` branches are ON, `< 0x1260` branches are OFF).
`z_stream` fields are named as in C (`next_in`, `avail_in`, `next_out`,
`avail_out`, `msg`, `state`, `zalloc`, `zfree`, `opaque`, `total_in`,
`total_out`, `adler`, `data_type`, `reserved`).
`PNGZ_MSG_CAST(s)`/`PNGZ_INPUT_CAST(b)` are no-ops (just casts).

## setjmp / longjmp

* `png_longjmp()` calls `(*png_ptr).longjmp_fn` (a caller supplied C `longjmp`)
  with `(*(*png_ptr).jmp_buf_ptr).as_mut_ptr()`.
* libpng's *internal* jmp_buf (`png_control::error_buf`, used by
  `png_safe_execute`/`png_safe_error`/`png_safe_warning`) uses the private pair
  `png_private_setjmp(buf: *mut __jmp_buf_tag) -> c_int` and
  `png_private_longjmp(buf: *mut __jmp_buf_tag, val: c_int) -> !` from
  `ctypes.rs` instead of the C library ones.  A `jmp_buf` value is
  `[__jmp_buf_tag; 1]`; `let mut b: jmp_buf = core::mem::zeroed();` and pass
  `b.as_mut_ptr()`.

## Macros already provided (do not re-implement)

From `pngconsts.rs`, as `#[inline] fn`s with the same names:
`PNG_ROWBYTES`, `PNG_TRAILBITS`, `PNG_PADBITS`, `PNG_DIV65535`, `PNG_DIV257`,
`PNG_U32`, `PNG_32b`, `PNG_32to8`, `PNG_CHUNK_NAME_VALID`,
`PNG_CHUNK_FROM_STRING`, `PNG_STRING_FROM_CHUNK`, `PNG_CSTRING_FROM_CHUNK`,
`PNG_CHUNK_ANCILLARY`, `PNG_CHUNK_CRITICAL`, `PNG_CHUNK_PRIVATE`,
`PNG_CHUNK_RESERVED`, `PNG_CHUNK_SAFE_TO_COPY`, `png_chunk_flag_from_index`,
`png_file_has_chunk(png_ptr, PNG_INDEX_xxxx)`, `png_file_add_chunk`,
`png_chunk_max`, `PNG_OUT_OF_RANGE`, `PNG_COLOR_DIST`, `png_isaligned(p, align)`,
`PNG_sRGB_FROM_LINEAR`, `PNG_PASS_*`, `PNG_ROW_FROM_PASS_ROW`,
`PNG_COL_FROM_PASS_COL`, `PNG_PASS_MASK`, `PNG_ROW_IN_INTERLACE_PASS`,
`PNG_COL_IN_INTERLACE_PASS`, `PNG_IMAGE_*` helpers, `PNG_ZLIB_MAX_SIZE`,
`PNG_FP_IS_ZERO/POSITIVE/NEGATIVE`, `PNG_COMPRESSION_BUFFER_SIZE(png_ptr)`,
plus every `PNG_*`/`png_*` constant (`png_IHDR`, `PNG_INDEX_IHDR`,
`PNG_FLAG_*`, `PNG_HAVE_*`, `PNG_INFO_*`, `PNG_ZBUF_SIZE`, ...).
`png_has_chunk(png_ptr, cHNK)` → `png_file_has_chunk(png_ptr, PNG_INDEX_cHNK)`.
`png_float(png_ptr, fixed, s)` → `png_float_of(fixed)`.
`PNG_INFLATE(pp, flush)` → `png_zlib_inflate(pp, flush)`.
`png_get_uint_32(buf)`/`png_get_uint_16(buf)`/`png_get_int_32(buf)` are the
exported functions of the same name — call them.

`PNG_WARNING_PARAMETERS(p)` declares `char p[8][32]`; in Rust write
`let mut p: [[c_char; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT] = [[0; 32]; 8];`
and pass `p.as_mut_ptr()` (the `png_warning_parameters` type is
`*mut [c_char; 32]`).

## Style rules that keep the crate compiling

* Never add `mod` declarations, `#[macro_export]`, or `extern crate`.
* Because chunk files are `include!`d, they must **not** start with an inner doc
  comment (`//!`); use plain `//` comments.
* Only `use crate::*;` at the top (nothing else is needed; `core::` paths can be
  written inline).
* Do not define a function that another module already defines: every symbol in
  `SIGNATURES.md` has exactly one home, which is the C file it is defined in.
* Do not use Rust features that need the standard library beyond `core` (no
  `Vec`, `String`, `println!`, `format!`).
* Loops: translate `for (i = 0; i < n; i++)` to `let mut i = 0; while i < n { ...; i += 1; }`
  when the body mutates `i` or uses `continue` in a way `for` cannot express;
  otherwise `for i in 0..n` is fine.  `do { } while (x)` → `loop { ...; if !x { break; } }`.
* `goto` in C: restructure with a labelled block/loop
  (`'label: loop { ... break 'label; }`) keeping the exact same behaviour.
