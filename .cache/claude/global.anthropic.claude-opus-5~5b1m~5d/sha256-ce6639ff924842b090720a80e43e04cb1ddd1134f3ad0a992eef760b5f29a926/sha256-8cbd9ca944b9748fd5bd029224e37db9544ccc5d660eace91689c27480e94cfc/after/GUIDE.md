# MuJS C -> Rust translation conventions (READ FULLY BEFORE WRITING CODE)

We are translating the MuJS C library in `c_src/` into the Rust cdylib in
`translation/`. The Rust library must be a *mechanical, behaviour identical*
translation: same algorithms, same order of checks, same output bytes, same
bugs. Do NOT improve, refactor, modernise or "fix" anything.

## Ground rules

* Work only on the file you were assigned. NEVER modify any other file in
  `translation/src/`, and NEVER modify anything in `c_src/`.
* The crate is one Rust crate; every C file becomes one Rust module.
* Everything is `unsafe` raw pointer code. Do not use `String`, `Vec`, `Box`,
  `format!`, `println!`, iterators, or any Rust std allocation. Use only the
  libc bindings and the helpers already declared in `src/jsi.rs`.
* Never print with Rust macros. Use the libc `printf`/`putchar`/`fputs`/
  `snprintf`/`sprintf`/`fprintf` bindings from `src/jsi.rs` so output bytes and
  buffering match the C library exactly.
* Integer arithmetic must wrap like C: the crate is built with
  `overflow-checks = false`, but where the C code clearly relies on wrapping or
  on shifts >= bit width, use `wrapping_*` methods.

## Module skeleton

```rust
//! Translation of <name>.c

use crate::*;            // types, constants, libc bindings, other modules

/* ... translated code ... */
```

`use crate::*;` gives you: everything from `src/jsi.rs` (all types, constants,
libc `extern "C"` bindings, helper functions) plus every public function of
every other module. Macros (`cs!`, `js_typeerror!`, `js_try!`, `vol!`, ...) are
available everywhere automatically (`#[macro_use] mod jsi`).

If you need a libc function that is not declared yet, add your own
`extern "C" { ... }` block at the top of your file (do not edit jsi.rs).

## Naming / linkage

* A C function that is NOT `static` becomes
  ```rust
  #[unsafe(no_mangle)]
  pub unsafe extern "C" fn name(args) -> ret { ... }
  ```
  Its exported symbol name must match the C linker name exactly.
* A C `static` function becomes a private `unsafe fn name(...)`. Do not put
  `#[no_mangle]` or `pub` on it (private avoids name clashes between modules).
* The stub file you are replacing already contains the exact signatures of the
  non-static functions of your C file. Keep those signatures byte for byte.
* Functions used as `js_CFunction` callbacks must be
  `unsafe extern "C" fn(J: *mut js_State)` and are passed as `Some(name)`.

## Type mapping

| C | Rust |
|---|---|
| `int`, `unsigned`, `short`, `char`, `double` | `c_int`, `c_uint`, `c_short`, `c_char`, `f64` |
| `char *` / `const char *` | `*mut c_char` / `*const c_char` |
| `void *` | `*mut c_void` |
| `NULL` | `null()` / `null_mut()` |
| `js_State *J` | `*mut js_State` |
| struct field named `type` | field named `type_` |
| `enum js_Class` / `enum js_Type` field | `c_int` with the `JS_C*` / `JS_T*` constants |

All structs are declared in `src/jsi.rs` with `#[repr(C)]` and identical
layout. Read that file before starting.

### js_Value

`js_Value` is a `#[repr(C)]` union. Use the helpers from jsi.rs:

```rust
vtype(v)                  /* v->t.type as c_int */
setvtype(v, JS_TNUMBER)   /* v->t.type = JS_TNUMBER */
(*v).u.number             /* v->u.number */
(*v).u.boolean, (*v).u.litstr, (*v).u.memstr, (*v).u.object
shrstrp(v)                /* char* to v->u.shrstr */
strp(memstr)              /* char* to js_String->p */
jsv_isstring(v)           /* JSV_ISSTRING(v) */
jsv_tostring_raw(v)       /* JSV_TOSTRING(v) */
OFF_VALUE_TYPE            /* soffsetof(js_Value, t.type) == 15 */
```

Flexible array members (`js_Property.name`, `js_Iterator.name`,
`js_String.p`, `js_StringNode.string`) are `[c_char; 1]` fields; get a pointer
with `propname(p)`, `itername(p)`, `strp(s)`, `nodestring(n)`, and allocate
with the `OFF_*` offset constants (e.g. `OFF_PROPERTY_NAME + n`).

## String literals

`cs!("hello")` produces a NUL terminated `*const c_char`. Use it for every C
string literal, including format strings.

## Errors and exceptions

The `js_*error` functions are variadic in C. Use these macros, which format
into a 256 byte buffer with libc `snprintf` exactly like the C originals and
then throw:

```rust
js_error!(J, "stack underflow!");
js_typeerror!(J, "'%s' is read-only", name);
js_rangeerror!(J, "invalid array length");
js_referenceerror!(J, "'%s' is not defined", str);
js_syntaxerror!(J, "...", ...);
js_evalerror!(J, "...");
js_urierror!(J, "...");
```

They diverge (`-> !`), so no `return` is needed after them. `js_throw(J)` also
diverges.

If your C file has its own `static` variadic error helper (for example
`jsY_error` in jslex.c), translate it as a non-variadic
`unsafe fn jsY_error_str(J: *mut js_State, msg: *const c_char) -> !` plus a
local macro that formats with `snprintf` into a buffer **of exactly the same
size as the C original** and calls it:

```rust
macro_rules! jsY_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut __b: [c_char; 512] = [0; 512];
        snprintf(__b.as_mut_ptr(), 512, cs!($fmt) $(, $a)*);
        jsY_error_str($J, __b.as_ptr())
    }};
}
```

`jsC_error(J, node, fmt, ...)` already exists in `src/varargs.rs`; from Rust
code call `crate::varargs::jsC_error_str(J, node, msg)` through a local macro
in the same style (buffer size 256, as in the C `msgbuf`).

## try/catch (setjmp/longjmp)

```rust
if js_try!(J) != 0 {
    /* exception path */
    js_throw(J);
}
/* body */
js_endtry(J);
```

`js_try!` MUST be written directly in the function whose frame the exception
returns to (never inside a helper function or closure).

**Important:** any local variable that is assigned inside the `try` body and
read in the exception path must be accessed with the volatile helpers, exactly
where the C source used the `volatile` qualifier:

```rust
let mut out: *mut c_char = null_mut();
if js_try!(J) != 0 {
    js_free(J, vol!(out) as *mut c_void);
    js_throw(J);
}
setvol!(out, js_malloc(J, n) as *mut c_char);
... use vol!(out) ...
js_endtry(J);
js_free(J, vol!(out) as *mut c_void);
```

## Control flow translation

* `goto label;` inside a function: use a labelled block
  ```rust
  'body: { ... break 'body; ... }   /* code after the block == label target */
  ```
  or a boolean flag plus restructuring. Keep the semantics identical.
* C `switch` with fallthrough: replicate the fallthrough explicitly.
* `for (a; b; c) { ... continue; }` -> `while` loop, but remember `continue`
  must still execute the increment `c`.
* `while (x--)`, `while (--x)` etc: translate the pre/post semantics carefully.
* Do not use `match` on constants unless every arm is a `pub const` (they are).

## Static (file scope) variables

```rust
static mut FOO: c_int = 0;              /* mutable C static */
static TABLE: [c_int; 3] = [1, 2, 3];   /* const C static */
```
For arrays of string pointers use `[&str; N]` with `"...\0"` entries and take
`.as_ptr() as *const c_char` (raw pointers are not `Sync` so they cannot be in
a plain `static`). Self-referential statics (like the AA-tree sentinels) are
initialised lazily; see `src/jsproperty.rs` for the pattern.

## Examples already finished (read them for style)

`src/jsvalue.rs`, `src/jsrun.rs`, `src/jsproperty.rs`, `src/jsgc.rs`,
`src/jsintern.rs`, `src/jsstate.rs`, `src/jserror.rs`, `src/utf.rs`.

## Verifying

From `translation/`:

```
cargo build --release 2>&1 | grep -A8 "src/<yourfile>.rs"
```

Other stub modules are still unimplemented, so ignore errors that are not in
your own file. Keep iterating until your file has no errors and no warnings
other than dead-code ones. Do not stop with a partially translated file: every
function of your C file must be present.
