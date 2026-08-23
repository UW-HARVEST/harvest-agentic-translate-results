# mujs C -> Rust transliteration guide (READ FULLY BEFORE WRITING CODE)

We are translating the C library in `c_src/` into a Rust cdylib in `src/`.
One Rust module per C file: `c_src/src/jsfoo.c` -> `src/jsfoo.rs`.

The goal is a **mechanical, line-by-line transliteration** that produces
**byte-identical behaviour**. Do NOT redesign, do NOT "improve", do NOT fix
bugs, do NOT reorder checks, do NOT change messages or formats. Keep the same
function order and the same comments where useful.

`c_src/` is READ-ONLY. Never modify it.

## 1. File preamble

Every module starts with exactly:

```rust
//! Translated from c_src/src/<file>.c
use crate::jsi::*;
use crate::prelude::*;
```

`crate::jsi` has all types/constants/libc bindings. `crate::prelude` re-exports
every non-static function of every other module, so you can call cross-file
functions by their plain C name (`js_pushnumber(J, x)`).

## 2. Functions

* A **non-static** C function `void js_foo(js_State *J, int n)` becomes:

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_foo(J: *mut js_State, n: c_int) {
```

  The `#[unsafe(no_mangle)]` spelling with parentheses is required.
  IMPORTANT: every non-static C function must be exported this way; the linker
  symbol name must match the C name exactly (watch for `#define` renames in
  `utf.h` / `regexp.h`: e.g. `chartorune` is really `jsU_chartorune`, `regcomp`
  is really `js_regcomp` -- use the FINAL name as the Rust function name).

* A **static** C function becomes a plain module-private function, keeping the
  same name: `unsafe fn jsB_helper(J: *mut js_State) { ... }`
  (no `#[no_mangle]`, no `pub`, no `extern "C"`) -- UNLESS it is used as a
  function pointer (see §7), in which case it must be
  `unsafe extern "C" fn` (still not pub, still no no_mangle).

* Bodies of `unsafe fn` are implicitly unsafe blocks (edition 2021), so raw
  pointer dereferences need no extra `unsafe { }`.

* C functions marked `JS_NORETURN` (and any function that unconditionally
  throws) return `!` in Rust: `-> !`. `js_throw`, `js_error_str`, ... are `!`.

## 3. Types

| C | Rust |
|---|---|
| `int` | `c_int` |
| `unsigned int` | `c_uint` |
| `char` | `c_char` |
| `unsigned char` | `c_uchar` |
| `short` / `unsigned short` | `c_short` / `c_ushort` |
| `double` | `f64` |
| `const char *` | `*const c_char` |
| `char *` | `*mut c_char` |
| `void *` | `*mut c_void` |
| `size_t` | `usize` |
| `js_State *` | `*mut js_State` |
| `enum js_Class` / `enum js_AstType` / `enum js_OpCode` | `c_int` |
| `js_Value.t.type` | `c_char` |
| `Rune` | `Rune` (= `c_int`) |

All struct/union/enum definitions already exist in `crate::jsi` with the same
names and the same field names, EXCEPT the field `type` which is spelled
`r#type` in Rust (`(*obj).r#type`, `(*v).t.r#type`, `(*node).r#type`).

Enum constants (`JS_TNUMBER`, `JS_CARRAY`, `OP_ADD`, `TK_IF`, `EXP_ADD`,
`AST_LIST`, `JS_HNONE`, `JS_READONLY`, ...) are all in `crate::jsi`.
`JS_T*` constants are `c_char`; all others are `c_int`.

## 4. Statements and expressions

* `a->b` -> `(*a).b`; `a->b->c` -> `(*(*a).b).c`; `*p` -> `*p`; `&x` -> `&mut x`
  as a raw pointer: prefer `std::ptr::addr_of_mut!(x)` or `&mut x as *mut _`.
* Pointer arithmetic: `p + 1` -> `p.add(1)` / `p.offset(1)`, `p - q` ->
  `p.offset_from(q)`, `p[i]` -> `*p.add(i as usize)` (use `.offset(i as isize)`
  if `i` may be negative). Never use Rust slice indexing on a raw pointer.
* Fixed C arrays (`char buf[32]`) -> `let mut buf: [c_char; 32] = [0; 32];`
  and pass `buf.as_mut_ptr()`. Indexing a fixed array with a Rust `usize` index
  is fine (`buf[i as usize]`), but if the C code can write past the declared
  length (flexible array members) you MUST use pointer arithmetic.
* Flexible array members have helpers in `jsi`: `js_String_p(s)`,
  `js_Property_name(p)`, `js_Iterator_name(p)`, `js_StringNode_string(p)`,
  `js_Buffer_s(b)`, `js_Value_shrstr(v)`, `js_Object_shrstr(o)`.
  `soffsetof(js_Buffer, s)` etc. are the `SOFFSETOF_*` constants in `jsi`.
* `while (*s)` -> `while *s != 0`; C truthiness must become explicit
  comparisons (`if p.is_null()`, `if x != 0`, `if !b`).
* `switch` -> `match` on the value with `_ =>` for `default`. Beware C
  fallthrough: replicate it explicitly. `match` arms in Rust need `c_int`
  patterns, so match on the integer directly:
  `match tok { TK_IF => {...}, _ => {...} }` works because the constants are
  `const` items - use them as patterns directly (they are allowed as patterns).
* `goto` must be restructured with labelled loops/blocks:
  `'label: loop { ... break 'label; }` for forward jumps, or duplicate a small
  block. A common mujs pattern is `goto readonly;` at the end of a function --
  translate as an inner labelled block:
  ```rust
  'readonly: {
      ... if cond { break 'readonly; } ...
      return;
  }
  /* readonly: */
  if (*J).strict != 0 { js_typeerror!(...); }
  ```
  Make sure the control flow is *exactly* equivalent.
* Integer overflow: the crate is built with `overflow-checks = false`, so `+`,
  `-`, `*` wrap like gcc. Keep plain operators. For shifts by >= bit width or
  by a variable that could be >= 32 use `wrapping_shl/wrapping_shr` only if the
  C code relies on it (mujs masks with `& 0x1F` already).
* Division/modulo of integers by zero would panic in Rust; mujs never does it
  on the integer path (JS uses doubles). Keep plain operators.
* Casts use `as`. Note `double -> int` in Rust saturates instead of being UB;
  that is the desired behaviour here.
* Comma expressions `return *idx = n, 1;` -> `{ *idx = n; return 1; }`.
* `a ? b : c` -> `if a { b } else { c }`.
* String literals: use C string literals, e.g. `c"length".as_ptr()`
  (type `*const c_char`). For a `char` literal use `b'x' as c_char` or
  `'x' as c_int` depending on context.
* `static` local variables inside a function -> module-level
  `static mut NAME: T = ...;` accessed directly (we are in unsafe code). Keep
  the same initial value. If it is a read-only table prefer `static NAME: [T; N]`.
* `sizeof(T)` -> `std::mem::size_of::<T>()` (usually `as c_int`).
* `memcpy`/`memset`/`strcmp`/`strlen`/... are declared in `jsi` -- call them
  directly, with `as *mut c_void` / `as *const c_void` casts as needed.
* `printf`, `snprintf`, `sprintf`, `fprintf`, `puts`, `putchar`, `fputs`,
  `fputc` are declared in `jsi` as C variadics; call them with
  `c"fmt".as_ptr()` and the same arguments (cast ints to `c_int`, keep `f64`).
  Never replace them with Rust formatting -- output must be byte-identical.
* Math: `floor`, `ceil`, `fabs`, `fmod`, `pow`, `sqrt`, `exp`, `log`, `sin`,
  `cos`, `tan`, `asin`, `acos`, `atan`, `atan2` are extern "C" from libm.
  `isnan(x)`, `isinf(x)`, `isfinite(x)`, `signbit(x)` are Rust fns returning
  `bool`. `INFINITY`, `NAN`, `INT_MIN`, `INT_MAX`, `UINT_MAX`, `DBL_MAX`,
  `DBL_MIN`, `DBL_EPSILON` are constants in `jsi`.
* `isdigit`, `isalpha`, `isupper`, `islower`, `isspace`, `toupper`, `tolower`
  are in `jsi` (C-locale versions, taking/returning `c_int`, the `is*` ones
  return `bool`).
* `assert(x)` -> just drop it (the C release build defines NDEBUG). Do not
  add panics.
* `nelem(a)` -> the array length as `c_int`.

## 5. Error / exception handling

* `js_throw(J)` is `-> !`.
* The variadic error functions are macros in Rust (defined in `lib.rs`):

```c
js_typeerror(J, "'%s' is read-only", name);
js_error(J, "stack overflow");
js_rangeerror(J, "invalid array length");
```
becomes
```rust
js_typeerror!(J, c"'%s' is read-only".as_ptr(), name);
js_error!(J, c"stack overflow".as_ptr());
js_rangeerror!(J, c"invalid array length".as_ptr());
```
  Available macros: `js_error!`, `js_evalerror!`, `js_rangeerror!`,
  `js_referenceerror!`, `js_syntaxerror!`, `js_typeerror!`, `js_urierror!`,
  `jsC_error!(J, node, fmt, ...)`.
  They all diverge (`-> !`), so `js_typeerror!(J, ...);` as a statement is
  fine, and `return js_typeerror!(...)` also works.
  NOTE: pass integer args as `c_int` and pointers as pointers, exactly like C.
  A `%s` argument must be `*const c_char`; a `%d` argument must be `c_int`;
  `%g`/`%f` must be `f64`.

* If the C file defines its own static variadic error helper (e.g.
  `jsY_error`, `jsP_error`, `jsP_warning` in jslex.c/jsparse.c), translate it
  into a non-variadic function taking the already formatted message plus a
  `macro_rules!` wrapper with the same name that formats into the same size
  buffer with `snprintf` and calls it. Example:

```rust
unsafe fn jsY_error_str(J: *mut js_State, msgbuf: *const c_char) -> ! {
    let mut buf: [c_char; 512] = [0; 512];
    snprintf(buf.as_mut_ptr(), 512, c"%s:%d: %s".as_ptr(), (*J).filename, (*J).line, msgbuf);
    js_newsyntaxerror(J, buf.as_ptr());
    js_throw(J)
}
macro_rules! jsY_error {
    ($J:expr, $($a:expr),*) => {{
        let mut msgbuf__: [c_char; 256] = [0; 256];
        snprintf(msgbuf__.as_mut_ptr(), 256, $($a),*);
        jsY_error_str($J, msgbuf__.as_ptr())
    }};
}
```
  (Declare such a macro *before* its first use in the file.)

* `if (js_try(J)) { ...handler... }` becomes `if js_try!(J) { ...handler... }`.
  `js_trypc(J, pc)` becomes `js_trypc!(J, pc)`.
  These use real `setjmp`/`longjmp`.
* **Any C local declared `volatile`** (because of setjmp clobbering) must be
  accessed with the volatile helpers so the value survives the longjmp:
  ```rust
  let mut out: *mut c_char = null_mut();          // char * volatile out = NULL;
  vwrite(&mut out, js_malloc(J, n) as *mut c_char); // out = js_malloc(...)
  let p = vread(&out);                              // reading out
  ```
  Do this for every read and write of such a variable.

## 6. Values and the stack

`js_Value` is a `Copy` union: `(*v).t.r#type`, `(*v).u.number`,
`(*v).u.boolean`, `(*v).u.litstr`, `(*v).u.memstr`, `(*v).u.object`,
and `js_Value_shrstr(v)` for `v->u.shrstr`.
`js_Value` literal for `{ { {0}, JS_TUNDEFINED } }` -> `js_Value::undef()`.
Copying: `let v: js_Value = *js_tovalue(J, -1);`.
The `JSV_ISSTRING(v)` / `JSV_TOSTRING(v)` macros are functions in `jsi`.

## 7. Function pointers

`js_CFunction`, `js_Finalize`, `js_HasProperty`, `js_Put`, `js_Delete`,
`js_Panic`, `js_Alloc`, `js_Report` are `Option<unsafe extern "C" fn ...>`.
So `js_newcfunction(J, Ap_join, "Array.prototype.join", 1)` becomes
`js_newcfunction(J, Some(Ap_join), c"Array.prototype.join".as_ptr(), 1)`,
and a C `NULL` function pointer is `None`.
Calling through one: `if let Some(f) = (*obj).u.c.function { f(J) }` or
`((*obj).u.c.function.unwrap())(J)` when C would call unconditionally.
Testing: `if (*obj).u.user.has.is_some()`.
Any static C function used as such a callback must be declared
`unsafe extern "C" fn`.

## 8. What NOT to do

* Do not add `mod` declarations, do not touch `src/lib.rs`, `src/jsi.rs`,
  `Cargo.toml` or any other module's file.
* Do not define a function that belongs to a different C file. Other modules
  may still be empty stubs while you work; if `cargo build` says
  "cannot find function `js_xyz`" and `js_xyz` is a non-static function of
  another `c_src/src/*.c` file, that error is expected -- ignore it.
* Do not use `std::collections`, `String`, `Vec`, `format!`, iterators over
  raw pointers, or any allocation other than the C ones (`js_malloc`,
  `js_realloc`, `js_free`, `malloc`, `realloc`, `free`).
* Do not leave `todo!()`, `unimplemented!()`, stubs or omitted functions.
  **Every** function in the C file must be translated completely.
* Do not use `panic!`/`unwrap()` on data-dependent paths.

## 9. Verifying your work

Run `cargo build --release --message-format=short 2>&1 | grep '^src/<yourfile>.rs'`
and fix every error/warning that belongs to *your* file (ignore errors about
missing functions from other C files, and ignore warnings from other files).
`cargo build` takes a lock, so if another agent is building, just retry.
