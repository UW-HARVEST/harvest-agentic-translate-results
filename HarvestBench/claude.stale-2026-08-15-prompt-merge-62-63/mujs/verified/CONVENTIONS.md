# Translation conventions (MuJS C -> Rust cdylib)

Goal: byte-identical behavior with the C library. Faithful transliteration, not
a rewrite. Do NOT fix bugs. Preserve order of operations, error checks, and all
observable output exactly.

## Shared types
All shared types/consts live in `crate::types` (see src/types.rs). Import with
`use crate::types::*;`. Key points:
- `js_State`, `js_Object`, `js_Value` (a `#[repr(C)]` union), `js_Property`,
  `js_Function`, `js_Ast`, `js_Environment`, `js_Regexp`, `js_Buffer`, etc.
- `js_Value` union access: `v.t.type_` is the type tag (a `c_char`). Value
  fields: `v.u.boolean`, `v.u.number`, `v.u.litstr`, `v.u.memstr`, `v.u.object`,
  `v.u.shrstr`. Use `js_Value::zeroed()` to make a zeroed value.
- `js_Object.type_` is the class (JS_COBJECT..). Object union: `(*o).u.number`,
  `(*o).u.boolean`, `(*o).u.s` (ObjS), `(*o).u.a` (ObjA), `(*o).u.f` (ObjF),
  `(*o).u.c` (ObjC), `(*o).u.r` (js_Regexp), `(*o).u.iter` (ObjIter),
  `(*o).u.user` (ObjUser). Note C field `delete` -> Rust field `delete` (ok).
- Enum tags are plain `pub const ... : c_int` (or `c_char` for value types).
- `#define foo NAMESPACE(foo)` etc: the LINKER symbol names are defined in
  regexp.h/utf.h. Exported symbol must match the final name.

## Function signatures & exports
- Every function that is a public/linker symbol (see nm list) MUST be
  `#[no_mangle] pub unsafe extern "C-unwind" fn NAME(...) -> ...`.
  Use `#[no_mangle]` (crate uses edition 2021; write `#[no_mangle]`).
- Internal/static C functions become `pub(crate) unsafe fn` (NOT no_mangle) so
  other modules can call them. Keep the same name (prefix module if collision).
- Use C types from `std::os::raw`: `c_char, c_int, c_uint, c_ushort, c_double`(=f64),
  `c_void`. Pointers: `*const c_char`, `*mut js_State`, etc.
- All functions are `unsafe` and operate on raw pointers, exactly like C.

## Calling other modules
Cross-module functions are referenced as `crate::MODULE::func(...)`. A registry
of who-defines-what is in the C headers (jsi.h/mujs.h/regexp.h/utf.h). Common:
- memory/stack/run: `crate::jsrun::*` (js_malloc, js_realloc, js_free, js_pushX,
  js_pop, stackidx, js_call, js_throw, ...)
- values: `crate::jsvalue::*` (jsV_tostring, jsV_tonumber, js_itoa, ...)
- properties: `crate::jsproperty::*` (jsV_newobject, jsV_getproperty, ...)
- intern/buffer: `crate::jsintern::*` (js_intern, js_putc, js_puts, js_putm,
  js_strdup, jsS_freestrings)
- lexer: `crate::jslex::*`; parser: `crate::jsparse::*`; compiler:`crate::jscompile::*`
- utf: `crate::utf::*` (jsU_chartorune etc; but call via the short names too)
- dtoa: `crate::jsdtoa::*` (js_strtod, js_grisu2, js_fmtexp)
- regexp: `crate::regexp::*` (js_regcomp, js_regexec, js_regfree, js_regcompx, js_regfreex)
- errors: `crate::jserror::*` (js_typeerror, js_rangeerror, js_throw is in jsrun)
Exact ownership is given per-task. If unsure where a symbol lives, grep the C
headers. The public API (js_* from mujs.h) lives across jsrun/jsvalue/jsstate/etc
as in the C sources.

## Exceptions (setjmp/longjmp)
C uses `if (js_try(J)) { handler; } body; js_endtry(J);`. Translate this pattern
using `crate::except::protect`:

```
if crate::except::protect(J, || {
    // body up to js_endtry (do NOT call js_endtry inside; protect handles the
    // frame accounting via savetry, and body must NOT call js_endtry — instead
    // the normal js_endtry that appears AFTER the C body is emitted by having
    // body end right before it, then we call js_endtry after protect returns false)
}) {
    // C handler block (state already restored, exception value on stack)
    // e.g. js_free(J, sb); js_throw(J);
} else {
    // frame completed normally: call js_endtry(J) here, then continue
}
```

IMPORTANT detail: In C, the sequence is:
```
if (js_try(J)) { HANDLER }   // setjmp
BODY
js_endtry(J);
REST
```
Translate as:
```
let caught = crate::except::protect(J, || { BODY });
if caught { HANDLER } else { crate::jsrun::js_endtry(J); }
REST
```
Note BODY must not early-return out of the closure in a way that skips endtry;
use the closure to run exactly the statements between setjmp and js_endtry.
For functions where the handler ends in `js_throw(J)` (re-throw) that's fine —
js_throw panics and unwinds further.

`js_throw` (in jsrun) pops trybuf, restores E/envtop/tracetop/top/bot/strict,
pushes the value, and `panic`s with `JsThrow{target}`. If trytop==0 it calls
panic handler and aborts.

The main interpreter loop `jsR_run` handles OP_TRY by using `protect` around the
remainder — but jsR_run is provided/owned by the jsrun task; other modules only
use `protect` for their local `js_try` sites.

## Numbers / formatting
- `NAN`, `INFINITY`: use `f64::NAN`, `f64::INFINITY`. `isnan`->`x.is_nan()`,
  `isinf`->`x.is_infinite()`, `isfinite`->`x.is_finite()`, `signbit`->`x.is_sign_negative()`,
  `floor/ceil/fabs/fmod/pow/...`-> f64 methods or libc.
- `sprintf`/`snprintf`/`vsnprintf` with format strings: use `libc::snprintf`
  via a fixed stack buffer to keep byte-identical printf semantics (%g, %e, %f,
  %d, %04d, %+d, %p, %02d:%02d etc). Build the format C-string as a byte literal
  `b"...\0"`. Do NOT reimplement float formatting in Rust — call libc.
- integer casts follow C truncation semantics: `x as c_int` on f64 in Rust is a
  saturating cast, NOT C's UB truncation. When C does `(int)double`, and the
  value is known in range it's fine; where C relies on wrap/truncation for
  in-range values it's equivalent. Match code paths exactly as written.

## String buffers & C strings
- C string literals: use `b"...\0".as_ptr() as *const c_char`. Define a helper
  `cstr!` pattern inline, or `b"foo\0".as_ptr() as *const c_char`.
- Character access `s[i]`: `*s.add(i)` (as c_char). Cast to unsigned via `as u8`.
- `*s++` idioms: track a pointer `let mut s = ...; let c = *s; s = s.add(1);`.

## Static/global C data
- `static const` tables -> `static` arrays or `const` arrays in Rust.
- Function-local `static` mutable (e.g. LocalTZA once/tza, stackidx undefined) ->
  use a `static mut` or a module-level cache; preserve exact behavior.
- The property AA-tree `sentinel` and intern `sentinel` are shared module
  statics — owned by jsproperty and jsintern respectively; expose via a getter.

## Don'ts
- Don't reorder checks. Don't add bounds checks C doesn't have. Don't "fix" UB.
- Don't use Rust std collections for engine data; use the raw C structures.
