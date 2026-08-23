//! Phase C — error/rejection paths reachable only through the **C embedding
//! API** (not from JavaScript), plus the generic FFI boundaries: NULL pointers,
//! zero/oversized lengths, one-past-range values and out-of-range enum values.
//!
//! Most of these error sites end in `js_throw()`, which `abort()`s when there is
//! no protected frame. Since a Rust `cdylib` cannot support the `js_try()`
//! `setjmp` macro from an external caller (the Rust side models `longjmp` with
//! `panic`), the abuses are performed from inside a **cfunction registered into
//! the interpreter** and invoked from a JS `try{}catch(e){}`. That gives both
//! libraries an identical, properly protected frame.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ------------------------------------------------------------------ */
/*  cfunction trampoline: run an arbitrary abuse inside the interpreter */
/* ------------------------------------------------------------------ */

/// Which abuse the `abuse` cfunction should perform. Set before running the
/// script; read by the trampoline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Abuse {
    PopUnderflow(c_int),
    RemoveBad(c_int),
    InsertAny(c_int),
    ReplaceBad(c_int),
    ToUserDataWrongTag,
    ToRegexpNotRegexp,
    CallNegative(c_int),
    ConstructNotCallable,
    PushLstringNegative(c_int),
    SetLength(c_int),
    GetLengthOnPrimitive,
    NewObjectClass(c_int),
    PushIterator(c_int),
    DefPropertyAtts(c_int),
    DefGlobalAtts(c_int),
    DefAccessorAtts(c_int),
    NewRegexpFlags(c_int),
    GcReport(c_int),
    NextIteratorNotIterator,
    JsvNextIteratorNotIterator,
    ResizeArray(c_int),
    UnflattenNonArray,
    Rot(c_int),
    Copy(c_int),
    Type(c_int),
    Typeof(c_int),
    Intern(&'static str),
    Trap(c_int),
    Repr(c_int),
    StackOverflow,
    TryLimitOverflow,
    Nop,
}

thread_local! {
    static ABUSE: std::cell::Cell<Abuse> = std::cell::Cell::new(Abuse::Nop);
    static CUR: std::cell::Cell<*const Api> = std::cell::Cell::new(std::ptr::null());
}

fn set_abuse(a: Abuse) {
    ABUSE.with(|c| c.set(a));
}

/// The cfunction. Runs the configured abuse against `J` using the *currently
/// bound* Api (so the calls go back into the same library that called us).
unsafe extern "C-unwind" fn abuse_cb(J: State) {
    let api: &Api = &*CUR.with(|c| c.get());
    let what = ABUSE.with(|c| c.get());
    match what {
        Abuse::PopUnderflow(n) => (api.js_pop)(J, n),
        Abuse::RemoveBad(i) => (api.js_remove)(J, i),
        Abuse::InsertAny(i) => (api.js_insert)(J, i),
        Abuse::ReplaceBad(i) => (api.js_replace)(J, i),
        Abuse::ToUserDataWrongTag => {
            (api.js_newuserdata)(J, cs("TagA").as_ptr(), 0x1234 as *mut c_void, None);
            let p = (api.js_touserdata)(J, -1, cs("TagB").as_ptr());
            (api.js_pushnumber)(J, p as usize as f64);
        }
        Abuse::ToRegexpNotRegexp => {
            (api.js_newobject)(J);
            let p = (api.js_toregexp)(J, -1);
            (api.js_pushnumber)(J, p as usize as f64);
        }
        Abuse::CallNegative(n) => {
            (api.js_getglobal)(J, cs("print").as_ptr());
            (api.js_pushundefined)(J);
            (api.js_call)(J, n);
        }
        Abuse::ConstructNotCallable => {
            (api.js_pushnumber)(J, 1.0);
            (api.js_construct)(J, 0);
        }
        Abuse::PushLstringNegative(n) => {
            // The only length the C VALIDATES is `n > JS_STRLIMIT` (checked
            // before any byte is read). A negative n makes `while (n--)` write
            // unboundedly, and 0 < n > strlen(v) reads past the buffer — both
            // are UB in the C, so the source buffer here is 64 bytes and only
            // n <= 64 or n > JS_STRLIMIT are exercised.
            let src = cs("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+/");
            (api.js_pushlstring)(J, src.as_ptr(), n);
            (api.js_pushnumber)(J, (api.js_type)(J, -1) as f64);
            (api.js_pushstring)(J, (api.js_tostring)(J, -2));
        }
        Abuse::SetLength(n) => {
            (api.js_newarray)(J);
            (api.js_setlength)(J, -1, n);
            (api.js_pushnumber)(J, (api.js_getlength)(J, -1) as f64);
        }
        Abuse::GetLengthOnPrimitive => {
            (api.js_pushnumber)(J, 42.0);
            let n = (api.js_getlength)(J, -1);
            (api.js_pushnumber)(J, n as f64);
        }
        Abuse::NewObjectClass(cl) => {
            let o = (api.jsV_newobject)(J, cl, std::ptr::null_mut());
            (api.js_pushobject)(J, o);
            (api.js_pushstring)(J, (api.js_typeof)(J, -1));
        }
        Abuse::PushIterator(own) => {
            (api.js_newobject)(J);
            (api.js_setproperty)(J, -1, cs("dummy").as_ptr()); // pops nothing useful
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 1.0);
            (api.js_setproperty)(J, -2, cs("a").as_ptr());
            (api.js_pushiterator)(J, -1, own);
            let mut names: Vec<String> = Vec::new();
            loop {
                let p = (api.js_nextiterator)(J, -1);
                if p.is_null() {
                    break;
                }
                names.push(cstr_string(p).unwrap());
                if names.len() > 200 {
                    break;
                }
            }
            (api.js_pushstring)(J, cs(&names.join(",")).as_ptr());
        }
        Abuse::DefPropertyAtts(atts) => {
            (api.js_newobject)(J);
            (api.js_pushnumber)(J, 7.0);
            (api.js_defproperty)(J, -2, cs("k").as_ptr(), atts);
            // observable: can we overwrite / enumerate / delete it?
            (api.js_pushnumber)(J, 9.0);
            let mut s = String::new();
            let J2 = J;
            (api.js_setproperty)(J2, -2, cs("k").as_ptr());
            (api.js_getproperty)(J2, -1, cs("k").as_ptr());
            s.push_str(&format!("set->{} ", (api.js_tonumber)(J2, -1)));
            (api.js_pop)(J2, 1);
            (api.js_delproperty)(J2, -1, cs("k").as_ptr());
            s.push_str(&format!("has_after_del={} ", (api.js_hasproperty)(J2, -1, cs("k").as_ptr())));
            (api.js_pop)(J2, 1);
            (api.js_pushstring)(J, cs(&s).as_ptr());
        }
        Abuse::DefGlobalAtts(atts) => {
            (api.js_pushnumber)(J, 5.0);
            (api.js_defglobal)(J, cs("gk").as_ptr(), atts);
            (api.js_getglobal)(J, cs("gk").as_ptr());
        }
        Abuse::DefAccessorAtts(atts) => {
            (api.js_newobject)(J);
            (api.js_getglobal)(J, cs("print").as_ptr());
            (api.js_pushnull)(J);
            (api.js_defaccessor)(J, -3, cs("acc").as_ptr(), atts);
            (api.js_pushnumber)(J, (api.js_hasproperty)(J, -1, cs("acc").as_ptr()) as f64);
        }
        Abuse::NewRegexpFlags(f) => {
            (api.js_newregexp)(J, cs("a(b)c").as_ptr(), f);
            (api.js_getproperty)(J, -1, cs("source").as_ptr());
            let src = cstr_string((api.js_tostring)(J, -1)).unwrap();
            (api.js_pop)(J, 1);
            (api.js_getproperty)(J, -1, cs("global").as_ptr());
            let g = (api.js_toboolean)(J, -1);
            (api.js_pop)(J, 1);
            (api.js_getproperty)(J, -1, cs("ignoreCase").as_ptr());
            let i = (api.js_toboolean)(J, -1);
            (api.js_pop)(J, 1);
            (api.js_getproperty)(J, -1, cs("multiline").as_ptr());
            let m = (api.js_toboolean)(J, -1);
            (api.js_pop)(J, 1);
            (api.js_pushstring)(J, cs(&format!("{} g={} i={} m={}", src, g, i, m)).as_ptr());
        }
        Abuse::GcReport(r) => {
            (api.js_gc)(J, r);
            (api.js_pushnumber)(J, 1.0);
        }
        Abuse::NextIteratorNotIterator => {
            (api.js_newobject)(J);
            let p = (api.js_nextiterator)(J, -1);
            (api.js_pushnumber)(J, p as usize as f64);
        }
        Abuse::JsvNextIteratorNotIterator => {
            (api.js_newobject)(J);
            let o = (api.js_toobject)(J, -1);
            let p = (api.jsV_nextiterator)(J, o);
            (api.js_pushnumber)(J, p as usize as f64);
        }
        Abuse::ResizeArray(n) => {
            (api.js_newarray)(J);
            let o = (api.js_toobject)(J, -1);
            // jsproperty.c:325 asserts !obj->u.a.simple, and the C .so is built
            // WITHOUT -DNDEBUG, so a fresh (flat) array must be unflattened
            // first — exactly what every in-tree caller does.
            (api.jsR_unflattenarray)(J, o);
            (api.jsV_resizearray)(J, o, n);
            (api.js_pushnumber)(J, (api.js_getlength)(J, -1) as f64);
        }
        Abuse::UnflattenNonArray => {
            (api.js_newobject)(J);
            let o = (api.js_toobject)(J, -1);
            (api.jsR_unflattenarray)(J, o);
            (api.js_pushnumber)(J, 1.0);
        }
        Abuse::Rot(n) => {
            (api.js_pushnumber)(J, 1.0);
            (api.js_pushnumber)(J, 2.0);
            (api.js_rot)(J, n);
            (api.js_pushnumber)(J, (api.js_gettop)(J) as f64);
        }
        Abuse::Copy(i) => {
            (api.js_pushnumber)(J, 1.0);
            (api.js_copy)(J, i);
        }
        Abuse::Type(i) => {
            (api.js_pushnumber)(J, (api.js_type)(J, i) as f64);
        }
        Abuse::Typeof(i) => {
            let p = (api.js_typeof)(J, i);
            (api.js_pushstring)(J, p);
        }
        Abuse::Intern(s) => {
            let p = (api.js_intern)(J, cs(s).as_ptr());
            (api.js_pushstring)(J, p);
        }
        Abuse::Trap(pc) => {
            (api.js_trap)(J, pc);
            (api.js_pushnumber)(J, 1.0);
        }
        Abuse::Repr(i) => {
            (api.js_pushnumber)(J, 1.5);
            let p = (api.js_torepr)(J, i);
            (api.js_pushstring)(J, p);
        }
        Abuse::StackOverflow => {
            // CHECKSTACK: push until TOP + n >= JS_STACKSIZE -> "stack overflow"
            for i in 0..(JS_STACKSIZE + 100) {
                (api.js_pushnumber)(J, i as f64);
            }
            (api.js_pushnumber)(J, 0.0);
        }
        Abuse::TryLimitOverflow => {
            // ERRORS.md L3..L8: every `js_ptry` guard. The caller wraps this in
            // N nested JS `try` blocks so that `J->trytop` is already at (or
            // just below) JS_TRYLIMIT when these protected helpers run; each
            // then takes its `trytop == JS_TRYLIMIT` early-out.
            //
            // (`js_savetry` itself cannot be driven from here: it hands back a
            // jmp_buf that the caller is required to `setjmp`, and the Rust
            // cdylib models the matching `longjmp` with a panic. Calling it
            // unpaired would longjmp into an uninitialised jmp_buf in the C.)
            (api.js_pushnumber)(J, 1.0);
            let s = cstr_string((api.js_trystring)(J, -1, cs("FB").as_ptr()));
            let n = (api.js_trynumber)(J, -1, -1.0);
            let i = (api.js_tryinteger)(J, -1, -1);
            let b = (api.js_tryboolean)(J, -1, 9);
            let rp = cstr_string((api.js_tryrepr)(J, -1, cs("RFB").as_ptr()));
            let pl = (api.js_ploadstring)(J, cs("x.js").as_ptr(), cs("1+1").as_ptr());
            let plbad = (api.js_ploadstring)(J, cs("x.js").as_ptr(), cs("(").as_ptr());
            let ds = (api.js_dostring)(J, cs("print('inner')").as_ptr());
            let dsbad = (api.js_dostring)(J, cs("null.x").as_ptr());
            (api.js_pushstring)(
                J,
                cs(&format!(
                    "trystring={:?} trynumber={} tryinteger={} tryboolean={} tryrepr={:?} pload={} ploadbad={} dostring={} dostringbad={}",
                    s, n, i, b, rp, pl, plbad, ds, dsbad
                ))
                .as_ptr(),
            );
        }
        Abuse::Nop => (api.js_pushundefined)(J),
    }
}

/// Run `script` with a global `abuse()` cfunction bound, performing `what`.
/// Returns (dostring rc, captured output).
fn run_abuse(api: &Api, what: Abuse, script: &str) -> (c_int, Vec<u8>) {
    unsafe {
        set_abuse(what);
        CUR.with(|c| c.set(api as *const Api));
        out_clear();
        let J = new_state(api, 0);
        (api.js_newcfunction)(J, Some(abuse_cb), cs("abuse").as_ptr(), 0);
        (api.js_setglobal)(J, cs("abuse").as_ptr());
        let rc = (api.js_dostring)(J, cs(script).as_ptr());
        (api.js_freestate)(J);
        CUR.with(|c| c.set(std::ptr::null()));
        (rc, out_take())
    }
}

const CATCH: &str =
    "try { print('ret', abuse()) } catch (e) { print('caught', e.name + ': ' + e.message) }";

#[track_caller]
fn diff_abuse(label: &str, what: Abuse) {
    let (c, r) = both(|api, _| run_abuse(api, what, CATCH));
    assert_eq!(
        c,
        r,
        "DIVERGENCE for {} ({:?}):\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
        label,
        what,
        c.0,
        String::from_utf8_lossy(&c.1),
        r.0,
        String::from_utf8_lossy(&r.1)
    );
}

/// Same, but unprotected at the JS level: the exception escapes to
/// `js_dostring`, so the *report* path is compared instead of `catch`.
#[track_caller]
fn diff_abuse_unprotected(label: &str, what: Abuse) {
    let (c, r) = both(|api, _| run_abuse(api, what, "print('ret', abuse())"));
    assert_eq!(
        c,
        r,
        "DIVERGENCE (unprotected) for {} ({:?}):\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
        label,
        what,
        c.0,
        String::from_utf8_lossy(&c.1),
        r.0,
        String::from_utf8_lossy(&r.1)
    );
}

/* ================================================================== */
/*  jsrun.c stack-abuse rejections                                     */
/* ================================================================== */

// NOTE on the scope of the next few tests.
//
// Some jsrun.c stack primitives DO validate their index/count and reject with a
// `js_error`; those are real rows of ERRORS.md and are covered exhaustively:
//   js_pop     -> "stack underflow!"   (checks TOP - n < BOT)
//   js_remove  -> "stack error!"       (checks idx < BOT || idx >= TOP)
//   js_replace -> "stack error!"       (same check)
//   js_insert  -> "not implemented yet" (unconditional)
//
// Others perform NO validation at all — `stackidx()` is
//     `idx < 0 ? &STACK[TOP+idx] : &STACK[BOT+idx]`
// with no bounds check, and `js_rot`/`js_copy` index straight off TOP. Passing
// an out-of-range index to those is undefined behaviour in the C itself: it
// reads or writes past the value stack. `js_pop` with a NEGATIVE n likewise
// *raises* TOP and then exposes an uninitialised stack slot (the stack is
// allocated with `alloc(...)`, never zeroed). No translation can reproduce
// "whatever heap garbage was there", so those inputs are deliberately excluded
// rather than silently ignored — the C has no rejection to match.
#[test]
fn e_js_pop_stack_underflow() {
    // jsrun.c:408  js_error(J, "stack underflow!")
    // n >= 0 only: see the note above.
    for n in [0, 1, 2, 5, 100, 4096, i32::MAX] {
        diff_abuse(&format!("js_pop({})", n), Abuse::PopUnderflow(n));
        diff_abuse_unprotected(&format!("js_pop({})", n), Abuse::PopUnderflow(n));
    }
}

#[test]
fn e_js_remove_stack_error() {
    // jsrun.c:416  js_error(J, "stack error!")
    for i in [0, 1, 2, 100, -1, -2, -100, i32::MIN, i32::MAX] {
        diff_abuse(&format!("js_remove({})", i), Abuse::RemoveBad(i));
    }
}

#[test]
fn e_js_insert_not_implemented() {
    // jsrun.c:424  js_error(J, "not implemented yet")
    for i in [0, 1, -1, -2, 100, i32::MIN, i32::MAX] {
        diff_abuse(&format!("js_insert({})", i), Abuse::InsertAny(i));
    }
}

#[test]
fn e_js_replace_stack_error() {
    // jsrun.c:431  js_error(J, "stack error!")
    for i in [0, 1, 2, 100, -1, -2, -100, i32::MIN, i32::MAX] {
        diff_abuse(&format!("js_replace({})", i), Abuse::ReplaceBad(i));
    }
}

#[test]
fn e_js_rot_and_copy_boundaries() {
    // Unchecked in the C (see the note above): only in-range values are
    // meaningful. The cfunction frame has `this` at 0 plus the two numbers the
    // abuse pushes, so gettop() == 3 and indices 0..2 / -1..-3 are valid.
    for n in [0, 1, 2, 3] {
        diff_abuse(&format!("js_rot({})", n), Abuse::Rot(n));
    }
    for i in [0, 1, -1, -2] {
        diff_abuse(&format!("js_copy({})", i), Abuse::Copy(i));
    }
}

#[test]
fn e_stack_overflow_checkstack() {
    // jsrun.c:106 CHECKSTACK -> js_stackoverflow -> "stack overflow"
    diff_abuse("CHECKSTACK overflow", Abuse::StackOverflow);
    diff_abuse_unprotected("CHECKSTACK overflow", Abuse::StackOverflow);
}

#[test]
fn e_trylimit_overflow_via_nested_js_try() {
    // ERRORS.md L2: jsrun.c:1433 js_savetrypc -> js_trystackoverflow ->
    // "exception stack overflow". Driven purely from JS: each entered `try`
    // block executes OP_TRY -> js_savetrypc, so JS_TRYLIMIT (64) nested live
    // try blocks exhaust J->trybuf.
    for depth in [1, 2, 10, 32, 60, 62, 63, 64, 65, 66, 70, 100, 200] {
        let src = format!(
            "{}print('deep {}'){}",
            "try{".repeat(depth),
            depth,
            "}catch(e){print('c',e.name||e,e.message||'')}".repeat(depth)
        );
        let (c, r) = both(|api, _| run_script(api, 0, &src));
        assert_eq!(
            c,
            r,
            "DIVERGENCE nested-try depth {}:\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            depth,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

#[test]
fn e_ptry_guards_at_trylimit() {
    // ERRORS.md L3..L8: js_ptry's `trytop == JS_TRYLIMIT` early-out in
    // js_dostring, js_ploadstring, js_trystring, js_trynumber, js_tryinteger
    // and js_tryboolean. `abuse()` calls all of them; the nesting depth here
    // puts J->trytop at / around JS_TRYLIMIT when it runs.
    for depth in [0, 1, 30, 58, 59, 60, 61, 62, 63, 64] {
        let script = format!(
            "{}print('ret', abuse()){}",
            "try{".repeat(depth),
            "}catch(e){print('c',e.name||e,e.message||'')}".repeat(depth)
        );
        let (c, r) = both(|api, _| run_abuse(api, Abuse::TryLimitOverflow, &script));
        assert_eq!(
            c,
            r,
            "DIVERGENCE js_ptry guards at try depth {}:\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            depth,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

/* ================================================================== */
/*  type-tag rejections in the embedding API                           */
/* ================================================================== */

#[test]
fn e_touserdata_wrong_tag() {
    // jsrun.c:382  js_typeerror(J, "not a %s", tag)
    diff_abuse("js_touserdata wrong tag", Abuse::ToUserDataWrongTag);
    diff_abuse_unprotected("js_touserdata wrong tag", Abuse::ToUserDataWrongTag);
}

#[test]
fn e_toregexp_not_a_regexp() {
    // jsrun.c:373  js_typeerror(J, "not a regexp")
    diff_abuse("js_toregexp non-regexp", Abuse::ToRegexpNotRegexp);
}

#[test]
fn e_call_negative_argument_count() {
    // jsrun.c:1304  js_rangeerror(J, "number of arguments cannot be negative")
    for n in [-1, -2, -1000, i32::MIN] {
        diff_abuse(&format!("js_call({})", n), Abuse::CallNegative(n));
    }
}

#[test]
fn e_construct_not_callable() {
    // jsrun.c:1341  js_typeerror(J, "%s is not callable", ...)
    diff_abuse("js_construct non-callable", Abuse::ConstructNotCallable);
}

#[test]
fn e_nextiterator_not_an_iterator() {
    // jsproperty.c:303  js_typeerror(J, "not an iterator")
    diff_abuse("js_nextiterator non-iterator", Abuse::NextIteratorNotIterator);
    diff_abuse("jsV_nextiterator non-iterator", Abuse::JsvNextIteratorNotIterator);
}

/* ================================================================== */
/*  out-of-range / boundary values for non-enum integer parameters      */
/* ================================================================== */

#[test]
fn e_pushlstring_negative_and_oversized_length() {
    for n in [
        0,
        1,
        6,
        7,
        14,
        15, // soffsetof(js_Value, t.type) -> last shrstr length
        16, // first memstr length
        17,
        63,
        64,
        JS_STRLIMIT + 1, // the only rejected length: RangeError
        JS_STRLIMIT + 2,
        i32::MAX,
    ] {
        diff_abuse(
            &format!("js_pushlstring(n={})", n),
            Abuse::PushLstringNegative(n),
        );
    }
}

#[test]
fn e_setlength_boundaries() {
    for n in [0, 1, 5, -1, -100, i32::MAX, JS_ARRAYLIMIT - 1, JS_ARRAYLIMIT, JS_ARRAYLIMIT + 1] {
        diff_abuse(&format!("js_setlength({})", n), Abuse::SetLength(n));
    }
}

#[test]
fn e_getlength_on_primitive() {
    diff_abuse("js_getlength(number)", Abuse::GetLengthOnPrimitive);
}

#[test]
fn e_resizearray_boundaries() {
    for n in [0, 1, 10, -1, -100, i32::MAX, JS_ARRAYLIMIT, JS_ARRAYLIMIT + 1] {
        diff_abuse(&format!("jsV_resizearray({})", n), Abuse::ResizeArray(n));
    }
}

#[test]
fn e_unflatten_non_array() {
    diff_abuse("jsR_unflattenarray(plain object)", Abuse::UnflattenNonArray);
}

#[test]
fn e_stack_index_boundaries() {
    // `stackidx()` performs NO bounds check, so only in-range indices are
    // defined behaviour in the C (see the note above). The abuse frame has
    // `this` at 0 plus whatever the variant pushes.
    for i in [0, 1, -1] {
        diff_abuse(&format!("js_type({})", i), Abuse::Type(i));
        diff_abuse(&format!("js_typeof({})", i), Abuse::Typeof(i));
    }
    for i in [0, 1, -1, -2] {
        diff_abuse(&format!("js_torepr({})", i), Abuse::Repr(i));
    }
}

/* ================================================================== */
/*  OUT-OF-RANGE ENUM VALUES ACROSS THE FFI BOUNDARY                   */
/*  (C enums accept any int; a value with no valid variant is a real    */
/*   input both libraries must handle identically)                     */
/* ================================================================== */

#[test]
fn enum_js_newstate_flags_out_of_range() {
    // mujs.h: enum { JS_STRICT = 1 }. Only bit 0 is read.
    for flags in [
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        0x10,
        0x7fffffff,
        -1,
        -2,
        i32::MIN,
        0x40000000,
    ] {
        let (c, r) = both(|api, _| {
            run_script(api, flags, "print(typeof this); x=1; print(x); print((function(){return this})() === undefined)")
        });
        assert_eq!(
            c,
            r,
            "DIVERGENCE js_newstate(flags={:#x}):\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            flags,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

#[test]
fn enum_jsV_newobject_class_out_of_range() {
    // enum js_Class has 16 variants (JS_COBJECT..JS_CUSERDATA).
    for cl in [
        JS_COBJECT,
        JS_CARRAY,
        JS_CFUNCTION,
        JS_CSCRIPT,
        JS_CCFUNCTION,
        JS_CERROR,
        JS_CBOOLEAN,
        JS_CNUMBER,
        JS_CSTRING,
        JS_CREGEXP,
        JS_CDATE,
        JS_CMATH,
        JS_CJSON,
        JS_CARGUMENTS,
        JS_CITERATOR,
        JS_CUSERDATA,
        16,
        17,
        100,
        -1,
        i32::MAX,
        i32::MIN,
    ] {
        diff_abuse(
            &format!("jsV_newobject(class={})", cl),
            Abuse::NewObjectClass(cl),
        );
    }
}

#[test]
fn enum_js_pushiterator_own_out_of_range() {
    for own in [0, 1, 2, 100, -1, i32::MAX, i32::MIN] {
        diff_abuse(
            &format!("js_pushiterator(own={})", own),
            Abuse::PushIterator(own),
        );
    }
}

#[test]
fn enum_property_attribute_flags_out_of_range() {
    // enum { JS_READONLY=1, JS_DONTENUM=2, JS_DONTCONF=4 }
    let all: Vec<c_int> = (0..8)
        .chain([8, 9, 15, 16, 100, -1, i32::MAX, i32::MIN].into_iter())
        .collect();
    for a in all {
        diff_abuse(&format!("js_defproperty(atts={})", a), Abuse::DefPropertyAtts(a));
        diff_abuse(&format!("js_defglobal(atts={})", a), Abuse::DefGlobalAtts(a));
        diff_abuse(
            &format!("js_defaccessor(atts={})", a),
            Abuse::DefAccessorAtts(a),
        );
    }
}

#[test]
fn enum_js_newregexp_flags_out_of_range() {
    // enum { JS_REGEXP_G=1, JS_REGEXP_I=2, JS_REGEXP_M=4 }
    let all: Vec<c_int> = (0..8)
        .chain([8, 9, 15, 16, 100, -1, i32::MAX, i32::MIN].into_iter())
        .collect();
    for f in all {
        diff_abuse(&format!("js_newregexp(flags={})", f), Abuse::NewRegexpFlags(f));
    }
}

#[test]
fn enum_js_gc_report_out_of_range() {
    // js_gc(J, report) prints statistics for any non-zero report value.
    // Its stdout is not captured here; we only assert both libraries agree on
    // *behaviour* (no crash, same subsequent state).
    for r in [0, 1, 2, -1, i32::MAX, i32::MIN] {
        diff_abuse(&format!("js_gc(report={})", r), Abuse::GcReport(r));
    }
}

#[test]
fn enum_js_regcomp_cflags_and_regexec_eflags_out_of_range() {
    let patterns = ["a", "a(b)c", "^a$", "[a-z]+", "a|b", "(?:x)*"];
    let subjects = ["", "a", "abc", "A", "AbC\nabc", "zzz"];
    let cflags = [
        0,
        REG_ICASE,
        REG_NEWLINE,
        REG_ICASE | REG_NEWLINE,
        4,
        8,
        16,
        0x7fffffff,
        -1,
        i32::MIN,
    ];
    let eflags = [0, REG_NOTBOL, 1, 2, 8, 16, 0x7fffffff, -1, i32::MIN];
    for p in patterns {
        for &cf in &cflags {
            for s in subjects {
                for &ef in &eflags {
                    let (c, r) = both(|api, _| unsafe {
                        let pat = cs(p);
                        let mut err: *const c_char = std::ptr::null();
                        let prog = (api.js_regcomp)(pat.as_ptr(), cf, &mut err);
                        if prog.is_null() {
                            return (true, cstr_string(err), -99, 0, Vec::new());
                        }
                        let subj = cs(s);
                        let base = subj.as_ptr() as usize;
                        let mut sub = Resub::default();
                        let rc = (api.js_regexec)(prog, subj.as_ptr(), &mut sub, ef);
                        let spans: Vec<Option<(isize, isize)>> = sub
                            .sub
                            .iter()
                            .map(|sp| {
                                if sp.sp.is_null() {
                                    None
                                } else {
                                    Some((
                                        sp.sp as usize as isize - base as isize,
                                        sp.ep as usize as isize - base as isize,
                                    ))
                                }
                            })
                            .collect();
                        (api.js_regfree)(prog);
                        (false, None, rc, sub.nsub, spans)
                    });
                    assert_eq!(
                        c, r,
                        "DIVERGENCE regcomp/regexec pat={:?} cflags={:#x} subj={:?} eflags={:#x}\n  C={:?}\n  R={:?}",
                        p, cf, s, ef, c, r
                    );
                }
            }
        }
    }
}

/* ================================================================== */
/*  NULL pointer handling on the documented-optional pointers          */
/* ================================================================== */

#[test]
fn null_regcomp_errorp() {
    // regexp.c guards both `*errorp` writes with `if (errorp)`.
    for p in ["a", "(", "[z-a]", "", "a{255}"] {
        let (c, r) = both(|api, _| unsafe {
            let pat = cs(p);
            let prog = (api.js_regcomp)(pat.as_ptr(), 0, std::ptr::null_mut());
            let isnull = prog.is_null();
            if !isnull {
                (api.js_regfree)(prog);
            }
            isnull
        });
        assert_eq!(c, r, "DIVERGENCE regcomp({:?}, NULL errorp)", p);
    }
}

#[test]
fn null_regexec_sub() {
    for p in ["a", "a(b)c", "^$", "(a)(b)(c)(d)"] {
        for s in ["", "a", "abc", "abcd"] {
            let (c, r) = both(|api, _| unsafe {
                let pat = cs(p);
                let mut err: *const c_char = std::ptr::null();
                let prog = (api.js_regcomp)(pat.as_ptr(), 0, &mut err);
                if prog.is_null() {
                    return -99;
                }
                let subj = cs(s);
                let rc = (api.js_regexec)(prog, subj.as_ptr(), std::ptr::null_mut(), 0);
                (api.js_regfree)(prog);
                rc
            });
            assert_eq!(c, r, "DIVERGENCE regexec({:?}, {:?}, NULL sub)", p, s);
        }
    }
}

#[test]
fn null_js_newstate_alloc_uses_default() {
    // js_newstate(NULL, ...) must install js_defaultalloc.
    let (c, r) = both(|api, _| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let ok = !J.is_null();
        if ok {
            (api.js_freestate)(J);
        }
        ok
    });
    assert_eq!(c, r, "DIVERGENCE js_newstate(NULL alloc)");
    assert!(c, "js_newstate(NULL alloc) must succeed");
}

#[test]
fn null_setreport_and_atpanic_reset_to_null() {
    // js_setreport(J, NULL) makes js_report a no-op; js_atpanic returns the old
    // handler. Both are observable.
    let (c, r) = both(|api, _| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        (api.js_setreport)(J, None);
        (api.js_report)(J, cs("this must be swallowed").as_ptr());
        let old = (api.js_atpanic)(J, None);
        let had_old = old.is_some();
        let old2 = (api.js_atpanic)(J, None);
        let had_old2 = old2.is_some();
        // js_dostring reports through the (now NULL) report hook.
        let rc = (api.js_dostring)(J, cs("null.x").as_ptr());
        (api.js_freestate)(J);
        (had_old, had_old2, rc)
    });
    assert_eq!(c, r, "DIVERGENCE js_setreport(NULL)/js_atpanic(NULL)");
}

#[test]
fn context_roundtrip_including_null() {
    let (c, r) = both(|api, _| unsafe {
        let J = (api.js_newstate)(None, std::ptr::null_mut(), 0);
        let a = (api.js_getcontext)(J);
        (api.js_setcontext)(J, 0xDEAD as *mut c_void);
        let b = (api.js_getcontext)(J);
        (api.js_setcontext)(J, std::ptr::null_mut());
        let c2 = (api.js_getcontext)(J);
        (api.js_freestate)(J);
        (a as usize, b as usize, c2 as usize)
    });
    assert_eq!(c, r, "DIVERGENCE js_setcontext/js_getcontext");
}

/* ================================================================== */
/*  return-code contracts of the protected entry points                */
/* ================================================================== */

#[test]
fn rc_js_dostring() {
    let cases = [
        ("1+1", 0),
        ("print(1)", 0),
        ("", 0),
        ("   ", 0),
        ("//comment", 0),
        ("(", 1),          // parse error
        ("null.x", 1),     // runtime error
        ("throw 1", 1),    // uncaught throw of a primitive
        ("throw null", 1),
        ("throw undefined", 1),
        ("throw {}", 1),
        ("throw new Error('x')", 1),
        ("(function f(){f()})()", 1), // call stack overflow
    ];
    for (src, want) in cases {
        let (c, r) = both(|api, _| run_script(api, 0, src));
        assert_eq!(
            c,
            r,
            "DIVERGENCE js_dostring({:?}):\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            src,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
        assert_eq!(c.0, want, "js_dostring({:?}) expected rc {}", src, want);
    }
}

#[test]
fn rc_js_ploadstring() {
    for (name, src) in [
        ("ok.js", "1+1"),
        ("ok.js", ""),
        ("bad.js", "("),
        ("bad.js", "var 1"),
        ("bad.js", "\"use strict\"; with({}){}"),
        ("weird\u{e9}.js", "1"),
        ("a-very-long-filename-that-exceeds-the-256-byte-snprintf-prefix-buffer-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.js", "("),
    ] {
        let (c, r) = both(|api, _| unsafe {
            out_clear();
            let J = new_state(api, 0);
            let rc = (api.js_ploadstring)(J, cs(name).as_ptr(), cs(src).as_ptr());
            let top = (api.js_gettop)(J);
            // if it succeeded, a function is on the stack: report its repr
            let extra = if rc == 0 && top > 0 {
                cstr_string((api.js_tryrepr)(J, -1, cs("?").as_ptr()))
            } else if top > 0 {
                cstr_string((api.js_trystring)(J, -1, cs("?").as_ptr()))
            } else {
                None
            };
            (api.js_freestate)(J);
            (rc, top, extra, out_take())
        });
        assert_eq!(
            c, r,
            "DIVERGENCE js_ploadstring({:?}, {:?}):\n  C={:?}\n  R={:?}",
            name, src, c, r
        );
    }
}

#[test]
fn rc_js_pcall_and_pconstruct() {
    // n = 0,1,many; callable and non-callable; a function that throws.
    let setups: &[(&str, c_int)] = &[
        ("(function(){return 1})", 0),
        ("(function(a){return a})", 1),
        ("(function(a,b,c){return a+b+c})", 3),
        ("(function(){throw new Error('boom')})", 0),
        ("(function(){null.x})", 0),
        ("42", 0),
        ("'str'", 0),
        ("null", 0),
        ("undefined", 0),
        ("({})", 0),
        ("[]", 0),
        ("Math.max", 3),
        ("Object", 1),
        ("Error", 1),
    ];
    for &(expr, n) in setups {
        for construct in [false, true] {
            let (c, r) = both(|api, _| unsafe {
                out_clear();
                let J = new_state(api, 0);
                // evaluate the callee expression via js_dostring into a global
                let prep = format!("var CALLEE = {};", expr);
                let prc = (api.js_dostring)(J, cs(&prep).as_ptr());
                // Both js_pcall and js_pconstruct compute
                //     savetop = TOP - n - 2
                // and, on the exception path, do `STACK[savetop] = STACK[TOP-1]`.
                // js_call really consumes callee + `this` + n args, so for
                // js_pcall savetop is exactly the callee slot. js_construct
                // however consumes only callee + n args, so js_pconstruct's
                // savetop is ONE SLOT BELOW the callee. A caller must therefore
                // keep a spare slot below the callee or js_pconstruct will write
                // outside its own region. Push a sentinel to make that safe and
                // to make the quirk observable in both libraries.
                (api.js_pushnumber)(J, 7777.0);
                (api.js_getglobal)(J, cs("CALLEE").as_ptr());
                if !construct {
                    (api.js_pushundefined)(J); // `this`
                }
                for i in 0..n {
                    (api.js_pushnumber)(J, (i + 1) as f64);
                }
                let rc = if construct {
                    (api.js_pconstruct)(J, n)
                } else {
                    (api.js_pcall)(J, n)
                };
                let top = (api.js_gettop)(J);
                let res = cstr_string((api.js_tryrepr)(J, -1, cs("?").as_ptr()));
                let slot0 = cstr_string((api.js_tryrepr)(J, 0, cs("?").as_ptr()));
                (api.js_freestate)(J);
                (prc, rc, top, res, slot0, out_take())
            });
            assert_eq!(
                c, r,
                "DIVERGENCE js_p{}({}) callee={:?}:\n  C={:?}\n  R={:?}",
                if construct { "construct" } else { "call" },
                n,
                expr,
                c,
                r
            );
        }
    }
}

#[test]
fn rc_js_try_helpers_on_throwing_conversions() {
    // js_trystring / js_trynumber / js_tryinteger / js_tryboolean must return the
    // caller's fallback when the conversion throws.
    let exprs = [
        "({toString:function(){throw new Error('no')}, valueOf:function(){throw 1}})",
        "({toString:function(){return 'ok'}})",
        "({valueOf:function(){return 7}})",
        "'plain'",
        "42",
        "null",
        "undefined",
        "true",
        "[1,2,3]",
        "({})",
        "(function(){})",
        "Object.create(null)",
    ];
    for e in exprs {
        let (c, r) = both(|api, _| unsafe {
            out_clear();
            let J = new_state(api, 0);
            let prep = format!("var V = {};", e);
            let prc = (api.js_dostring)(J, cs(&prep).as_ptr());
            (api.js_getglobal)(J, cs("V").as_ptr());
            let s = cstr_string((api.js_trystring)(J, -1, cs("FALLBACK").as_ptr()));
            let n = (api.js_trynumber)(J, -1, -12345.0);
            let i = (api.js_tryinteger)(J, -1, -999);
            let b = (api.js_tryboolean)(J, -1, 2);
            let rp = cstr_string((api.js_tryrepr)(J, -1, cs("RFALLBACK").as_ptr()));
            let top = (api.js_gettop)(J);
            (api.js_freestate)(J);
            (prc, s, n.to_bits(), i, b, rp, top, out_take())
        });
        assert_eq!(c, r, "DIVERGENCE js_try* for {:?}:\n  C={:?}\n  R={:?}", e, c, r);
    }
}

/* ================================================================== */
/*  resource limits                                                    */
/* ================================================================== */

#[test]
fn limit_runlimit_script_ran_too_long() {
    // jsrun.c:1602  runlimit == 1 -> js_runlimit -> "script ran too long"
    for rl in [0, 1, 2, 3, 10, 100, 1000, 100000, -1, -100] {
        let (c, r) = both(|api, _| unsafe {
            out_clear();
            let J = new_state(api, 0);
            (api.js_setlimit)(J, rl, 0);
            let rc = (api.js_dostring)(
                J,
                cs("var s=0; for (var i=0;i<100000;++i) s+=i; print(s)").as_ptr(),
            );
            (api.js_freestate)(J);
            (rc, out_take())
        });
        assert_eq!(
            c,
            r,
            "DIVERGENCE runlimit={}:\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            rl,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

#[test]
fn limit_memlimit_out_of_memory() {
    // jsrun.c:56/71  size >= memlimit -> js_outofmemory -> "out of memory"
    for ml in [0, 1, 2, 16, 64, 256, 1024, 4096, 65536, 1 << 20, -1, -1000] {
        let (c, r) = both(|api, _| unsafe {
            out_clear();
            let J = new_state(api, 0);
            (api.js_setlimit)(J, 0, ml);
            let rc = (api.js_dostring)(
                J,
                cs("var a=[]; for (var i=0;i<2000;++i) a.push('x'+i); print(a.length)").as_ptr(),
            );
            (api.js_freestate)(J);
            (rc, out_take())
        });
        assert_eq!(
            c,
            r,
            "DIVERGENCE memlimit={}:\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            ml,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}

#[test]
fn limit_call_stack_overflow() {
    // jsrun.c:1290  jsR_pushtrace -> js_error(J, "call stack overflow")
    // jsrun.c:1161  js_stackoverflow -> "stack overflow"
    let scripts = [
        "function f(){return f()} try { f() } catch(e) { print('caught', e.name, e.message) }",
        "function f(){return f()} f()",
        "function f(n){return n<=0?0:1+f(n-1)} print(f(100))",
        "function f(n){return n<=0?0:1+f(n-1)} try{print(f(100000))}catch(e){print('caught',e.message)}",
        "(function g(){ return g.apply(null, []) })()",
        "var f = function(){ return [].concat(f()) }; try{f()}catch(e){print('caught',e.message)}",
    ];
    diff_scripts(0, &scripts);
    diff_scripts(JS_STRICT, &scripts);
}

#[test]
fn limit_ast_recursion() {
    // jsparse.c:24 INCREC -> "too much recursion" past JS_ASTLIMIT (400)
    for depth in [1, 10, 100, 200, 396, 397, 398, 399, 400, 401, 402, 500, 1000] {
        let src = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        let (c, r) = both(|api, _| run_script(api, 0, &src));
        assert_eq!(
            c,
            r,
            "DIVERGENCE nested-paren depth {}:\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            depth,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
    // nested arrays / objects / unary operators recurse through different
    // INCREC sites
    for depth in [100, 399, 400, 401, 500] {
        for (open, close) in [("[", "]"), ("({a:", "})"), ("!", ""), ("-", ""), ("typeof ", "")] {
            let src = format!("print({}1{})", open.repeat(depth), close.repeat(depth));
            let (c, r) = both(|api, _| run_script(api, 0, &src));
            assert_eq!(
                c, r,
                "DIVERGENCE nested {:?} depth {}:\n  C={:?}\n  R={:?}",
                open, depth, c, r
            );
        }
    }
}

/* ================================================================== */
/*  jsY_tokenstring: out-of-range token ids                            */
/* ================================================================== */

#[test]
fn enum_tokenstring_out_of_range() {
    // jslex.c:66  guarded by `token >= 0 && token < nelem(tokenstring)`,
    // then a NULL entry also falls through to "<unknown>".
    let (c, r) = both(|api, _| {
        let mut v = Vec::new();
        for t in -1000..2000 {
            v.push(unsafe { cstr_string((api.jsY_tokenstring)(t)) });
        }
        for t in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
            v.push(unsafe { cstr_string((api.jsY_tokenstring)(t)) });
        }
        v
    });
    for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(
            a,
            b,
            "DIVERGENCE jsY_tokenstring({}): C={:?} Rust={:?}",
            i as i32 - 1000,
            a,
            b
        );
    }
}

#[test]
fn boundary_char_class_helpers_out_of_range() {
    let (c, r) = both(|api, _| {
        let mut v = Vec::new();
        for x in (-1000i64..0x11000).map(|x| x as c_int) {
            v.push((
                unsafe { (api.jsY_iswhite)(x) },
                unsafe { (api.jsY_isnewline)(x) },
                unsafe { (api.jsY_ishex)(x) },
                unsafe { (api.jsY_tohex)(x) },
            ));
        }
        v
    });
    for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(
            a,
            b,
            "DIVERGENCE lex char helpers at {}: C={:?} Rust={:?}",
            i as i64 - 1000,
            a,
            b
        );
    }
}
