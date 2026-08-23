# MuJS C -> Rust translation guide (READ FULLY BEFORE WRITING CODE)

We are transliterating the C library in `c_src/` into a Rust `cdylib` that
exports the **identical** public ABI and produces **byte-identical** behaviour.
This is a *mechanical, line-by-line transliteration* using raw pointers. Do NOT
redesign, do NOT "improve", do NOT fix bugs, do NOT change the order of checks.

`c_src/` is READ-ONLY. Never modify it.

## Crate layout

```
Cargo.toml                 [lib] crate-type = ["cdylib"], name = "mujs"
src/lib.rs                 module list (already written)
src/cstd.rs                extern "C" declarations of libc (already written)
src/macros.rs              error throwing macros (already written)
src/jsi.rs                 all types + constants + try/throw infra (already written)
src/vararg.rs              asm trampolines for the variadic entry points (already written)
src/<file>.rs              one module per c_src/src/<file>.c
```

Read `src/jsi.rs`, `src/cstd.rs`, `src/macros.rs` and (for style reference)
`src/jsrun.rs` + `src/jserror.rs` before starting. They are the authority for
type names, constant names, and helper names.

## Module header boilerplate

```rust
//! Translation of `c_src/src/<file>.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsi::*;
use crate::jsrun::*;
use crate::jsvalue::*;
use crate::jsproperty::*;
use core::ptr::{null, null_mut};
```
Add further `use crate::<module>::*;` as needed. Unused imports are fine
(warnings are globally allowed in lib.rs).

## Naming and linkage rules

* A **non-static** C function (i.e. it appears in `nm` output of the C object
  file with `T`) becomes
  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C-unwind" fn <exact_linker_name>(...) -> ... { }
  ```
  **The Rust name must be the final linker symbol.** Watch out for the
  `#define` renames in `src/utf.h` (`chartorune` -> `jsU_chartorune`, ...)
  and `src/regexp.h` (`regcomp` -> `js_regcomp`, ...). When a header renames a
  function, name the Rust function after the *renamed* symbol and, if the file
  uses the short name internally a lot, add
  `use ... as chartorune;`-style aliases or thin `#[inline]` wrappers.
* A **static** C function becomes `unsafe fn name(...)` (plain Rust ABI, no
  `no_mangle`) — but if its address is ever taken (e.g. it is registered as a
  `js_CFunction`, a `qsort` comparator, or stored in a table) it MUST be
  `unsafe extern "C-unwind" fn` (still no `no_mangle`).
* `js_CFunction` implementations (`Ap_push`, `Sp_charAt`, ... anything passed to
  `js_newcfunction`/`jsB_propf`/`js_newcconstructor`) are
  `unsafe extern "C-unwind" fn(J: *mut js_State)`, referenced as `Some(name)`.
* Never use plain `extern "C"` for functions that can throw — always
  `extern "C-unwind"`, otherwise a JS exception aborts the process.
* If a symbol must be visible to another Rust module, make it `pub`.

## C types

| C | Rust |
|---|---|
| `js_State *` | `*mut js_State` |
| `int`, `unsigned int` | `c_int`, `c_uint` |
| `const char *` | `*const c_char` |
| `char *` | `*mut c_char` |
| `double` | `f64` |
| `void *` | `*mut c_void` |
| `short`/`unsigned short` | `i16` / `u16` (`c_ushort`) |
| `size_t` | `size_t` (= `usize`) |
| function pointer typedefs | `js_CFunction`, `js_Finalize`, ... (already `Option<fn>`) |

Struct fields keep their C names, except `type` -> `type_` (`js_Object.type_`,
`js_Ast.type_`, `js_JumpList.type_`) and `delete` -> `delete`
(`js_ObjUser.delete` is fine).

Unions: `js_Object.u` is a `#[repr(C)] union js_ObjectU`; write
`(*obj).u.a.length`, `(*obj).u.c.name`, `(*obj).u.r.flags`, ... exactly like C.

## js_Value

`js_Value` is a 16-byte struct with accessor methods instead of a union
(the type tag lives in byte 15 and doubles as the NUL terminator of short
strings, exactly like C):

| C | Rust |
|---|---|
| `v->t.type` | `(*v).ty()` (returns `u32`) |
| `v->t.type = T` | `(*v).set_ty(T)` |
| `v->u.number` | `(*v).num()` / `(*v).set_num(x)` |
| `v->u.boolean` | `(*v).boolean()` / `(*v).set_boolean(x)` |
| `v->u.litstr` | `(*v).litstr()` / `(*v).set_litstr(p)` |
| `v->u.memstr` | `(*v).memstr()` / `(*v).set_memstr(p)` |
| `v->u.object` | `(*v).object()` / `(*v).set_object(p)` |
| `v->u.shrstr` | `(*v).shrstr()` (const ptr) / `(*v).shrstr_mut()` |
| `JSV_ISSTRING(v)` | `JSV_ISSTRING(v)` (returns `bool`) |
| `JSV_TOSTRING(v)` | `JSV_TOSTRING(v)` |

Type tags: `JS_TSHRSTR JS_TUNDEFINED JS_TNULL JS_TBOOLEAN JS_TNUMBER
JS_TLITSTR JS_TMEMSTR JS_TOBJECT` (all `u32`).
Classes: `JS_COBJECT JS_CARRAY JS_CFUNCTION JS_CSCRIPT JS_CCFUNCTION JS_CERROR
JS_CBOOLEAN JS_CNUMBER JS_CSTRING JS_CREGEXP JS_CDATE JS_CMATH JS_CJSON
JS_CARGUMENTS JS_CITERATOR JS_CUSERDATA` (all `c_int`).

## Strings and printf

* C string literals become C literals: `c"hello".as_ptr()`.
* `!strcmp(a,b)` -> `streq(a, b)` (returns `bool`); `strcmp(a,b)` is available
  directly from `cstd`.
* Use the real libc `snprintf`/`sprintf`/`printf`/`vsnprintf`/`strtod` from
  `crate::cstd` for anything format related, so the output bytes are identical.
  Calling variadic C functions from Rust is allowed and stable.
  Example: `snprintf(buf.as_mut_ptr(), 32, c"%.*f".as_ptr(), width, x);`
* Fixed C buffers: `let mut buf = [0 as c_char; 32];` then `buf.as_mut_ptr()`.
  Do **not** use Rust `String`/`format!` for anything observable.

## Integer / float conversion — IMPORTANT

Rust's `as` casts from float to integer *saturate*; C's truncate/are UB.
Use the helpers from `jsi.rs` for every C cast of a `double` to an integer
type where the value might be out of range:

* `(int)d` -> `cvt_i32(d)`
* `(long)d` / `(int64_t)d` -> `cvt_i64(d)`
* `(unsigned int)d` -> `cvt_u32(d)`
* `(unsigned short)d` -> `cvt_u16(d)`

Integer→integer casts and integer→float casts are plain `as`.
Signed shifts that can overflow: mimic C/gcc, e.g. `((a as u32) << n) as i32`.
Integer arithmetic must not panic: the release profile disables overflow checks,
but prefer `wrapping_*` where the C code deliberately overflows.

## Throwing errors

The 7 variadic error functions are exported from `src/vararg.rs` +
`src/jserror.rs`; **inside Rust code use the macros** (they format with libc
`snprintf` into a 256 byte buffer exactly like the C code):

```rust
js_typeerror!(J, c"not a %s".as_ptr(), tag);
js_rangeerror!(J, c"invalid array length".as_ptr());
js_error!(J, c"stack underflow!".as_ptr());
js_syntaxerror!(J, c"unexpected token: %s".as_ptr(), s);
js_referenceerror!(J, ...);  js_evalerror!(J, ...);  js_urierror!(J, ...);
```
They diverge (`-> !`), so they can be used where C relies on `JS_NORETURN`.
`js_throw(J)` also returns `!`.

## Exception handling (`js_try` / `js_endtry`)

There is no `setjmp` in Rust. Internal try frames are implemented with
`catch_unwind`. Translate

```c
if (js_try(J)) {
    /* HANDLER */
}
/* BODY */
js_endtry(J);
/* TAIL */
```

as

```rust
if js_do_try(J, || {
    /* BODY */
    js_endtry(J);
})
.is_none()
{
    /* HANDLER */
}
/* TAIL */
```

Notes:
* `js_do_try` returns `None` exactly when the C code would have taken the
  `if (js_try(J))` branch.
* The closure must call `js_endtry(J)` itself, like the C body does.
* If the body needs to return a value to the enclosing function, let the
  closure produce it: `match js_do_try(J, || { ...; js_endtry(J); v }) { None => ..., Some(v) => ... }`.
* If the C handler does `return x;`, put that `return x;` in the `is_none()`
  block. If it does `js_throw(J)`, put `js_throw(J)` there.
* Locals that the C code declares `volatile` because they are read in the
  handler must be shared with the closure through a raw pointer:
  ```rust
  let mut sab: *mut c_char = null_mut();
  let sabp = &mut sab as *mut *mut c_char;
  if js_do_try(J, || { *sabp = ...; js_endtry(J); }).is_none() { ... }
  ```

## Control flow translation patterns

* `goto label;` at the end of a function (`readonly:`, `dontconf:`, ...) ->
  labelled block:
  ```rust
  'readonly: {
      ...
      break 'readonly;   // was: goto readonly
      ...
      return;            // normal exit before the label
  }
  /* readonly: */
  ...
  ```
* `for (i = 0; i < n; ++i)` -> `let mut i = 0; while i < n { ...; i += 1; }`
  (careful with `continue` — the increment must still happen).
* `do { } while (cond);` -> `loop { ...; if !(cond) { break; } }`
* `switch` -> `match` on the value with `_ => {}` for `default`. C fallthrough
  must be written out explicitly. Remember `break` inside a C `switch` is not a
  loop break.
* C's `,` operator / assignment-as-expression must be unfolded into statements.
* `p[i]` on a raw pointer -> `*p.offset(i as isize)`.
* `*p++` -> `let c = *p; p = p.offset(1);`
* Recursive static helpers stay recursive.

## Memory

`js_malloc(J, n)`, `js_realloc(J, p, n)`, `js_free(J, p)`, `js_strdup(J, s)`
all live in `crate::jsrun` and take/return `*mut c_void`. Cast with `as *mut T`
/ `as *mut c_void`. `sizeof(T)` -> `core::mem::size_of::<T>() as c_int`.
`offsetof(S, f)` -> the pre-computed constants in `jsi.rs`
(`JS_PROPERTY_NAME_OFFSET`, `JS_STRING_P_OFFSET`, `JS_ITERATOR_NAME_OFFSET`,
`JS_STRINGNODE_STRING_OFFSET`, `JS_BUFFER_S_OFFSET`) or
`core::mem::offset_of!(S, f)`.

Structs with a C flexible array member (`char name[1]`) are allocated with
`offsetof + n` bytes; write their fields **individually**, never assign a whole
struct value (that would write past the allocation).

`assert(...)` in the C code: drop it (or use `debug_assert!`), it is compiled
out in the release C build we must match.

## Where things live (symbol -> module map)

* `jsi.rs` – types, constants, `js_do_try`, `js_pushtry_internal`, `cvt_*`,
  `streq`, `prop_sentinel`
* `jsrun.rs` – `js_malloc js_realloc js_free js_strdup jsV_newmemstring`,
  all `js_push*`, `js_is*`, `js_to*` (stack), `js_pop js_copy js_dup js_rot*`,
  `js_get/set/def/del property|index|global|registry`, `js_call js_construct
  js_eval js_pcall js_pconstruct js_throw js_endtry js_savetry js_savetrypc
  jsR_newenvironment jsR_unflattenarray js_trap js_ref js_unref js_gettop
  js_isarrayindex js_typeof js_type js_setlimit js_toregexp js_touserdata
  js_pushiterator js_nextiterator js_currentfunction js_currentfunctiondata`
  and `pub(crate)` internals `stackidx jsR_hasproperty jsR_getproperty
  jsR_setproperty jsR_defproperty jsR_delproperty`
* `jsvalue.rs` – `js_strtol jsV_numberto* jsV_toprimitive jsV_toboolean
  js_itoa js_stringtofloat jsV_stringtonumber jsV_tonumber jsV_tointeger
  jsV_numbertostring jsV_tostring jsV_toobject js_newobjectx js_newobject
  js_newarguments js_newarray js_newboolean js_newnumber js_newstring
  js_newfunction js_newscript js_newcfunctionx js_newcfunction
  js_newcconstructor js_newuserdatax js_newuserdata js_instanceof js_concat
  js_compare js_equal js_strictequal`
* `jsproperty.rs` – `jsV_newobject jsV_getownproperty jsV_getpropertyx
  jsV_getproperty jsV_setproperty jsV_delproperty jsV_newiterator
  jsV_nextiterator jsV_resizearray`
* `jsgc.rs` – `js_gc js_freestate`
* `jsintern.rs` – `js_putc js_puts js_putm js_intern jsS_dumpstrings
  jsS_freestrings`
* `jsdtoa.rs` – `js_fmtexp js_grisu2 js_strtod`
* `utf.rs` – `jsU_chartorune jsU_runetochar jsU_runelen jsU_isalpharune
  jsU_islowerrune jsU_isupperrune jsU_tolowerrune jsU_toupperrune
  jsU_tolowerrune_full jsU_toupperrune_full` (+ `pub use` aliases with the
  short names for internal callers)
* `utfdata.rs` – the Unicode tables from `src/utfdata.h`
* `regexp.rs` – `js_regcomp js_regcompx js_regexec js_regfree js_regfreex`,
  plus `pub struct Reprog`, `pub struct Resub`, `REG_ICASE REG_NEWLINE
  REG_NOTBOL REG_MAXSUB`
* `jslex.rs` – `jsY_*`
* `jsparse.rs` – `jsP_parse jsP_parsefunction jsP_freeparse`
* `jscompile.rs` – `jsC_compilescript jsC_compilefunction jsC_error_va`
* `jsstate.rs` – `js_newstate js_ploadstring js_dostring js_loadstring
  js_loadeval js_atpanic js_report js_setreport js_setcontext js_getcontext
  js_trystring js_trynumber js_tryinteger js_tryboolean`
* `jserror.rs` – `js_newerror... jsB_initerror`, `js_newerrorx`, `*_va`
* `jsbuiltin.rs` – `jsB_init jsB_propf jsB_propn jsB_props`
* `jsarray.rs` – `js_getlength js_setlength jsB_initarray`
* `jsstring.rs` – `js_runeat js_utflen js_utfptrtoidx jsB_initstring`
* `jsregexp.rs` – `js_newregexp js_RegExp_prototype_exec jsB_initregexp`
* `json.rs` – `jsB_initjson js_isnumberobject js_isstringobject
  js_isbooleanobject js_isdateobject`
* others: `jsB_initobject jsB_initfunction jsB_initboolean jsB_initnumber
  jsB_initmath jsB_initdate`, `jsrepr.rs` – `js_repr js_torepr js_tryrepr`

## Definition of done for your file

1. Every symbol that `nm --defined-only` reports as `T` (global text) for the
   corresponding C object file exists in your Rust module with
   `#[unsafe(no_mangle)] pub unsafe extern "C-unwind"` and the identical name —
   **except** functions that are `static` in C (those appear as `t`).
2. Every `static` C function in the file is translated too (as a private Rust
   `unsafe fn` / `unsafe extern "C-unwind" fn`).
3. `cargo build --release` may still fail because of *other* modules; that is
   expected. Make sure **your** file has no errors of its own that you can fix
   locally (run `cargo build --release 2>&1 | grep '<yourfile>.rs'`).
4. Do not create or edit any file other than the one assigned to you.
