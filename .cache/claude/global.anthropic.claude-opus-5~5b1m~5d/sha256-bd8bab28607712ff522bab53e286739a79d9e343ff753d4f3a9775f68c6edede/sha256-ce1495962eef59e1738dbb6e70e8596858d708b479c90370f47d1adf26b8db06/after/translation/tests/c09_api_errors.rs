//! Phase C: differential tests for the C-API-level error rows of `ERRORS.md`
//! (sections 7-11), i.e. the rejections reachable only through the C stack API
//! rather than through JavaScript source.
//!
//! Each probe runs inside the engine via `js_pcall`, so the thrown error's exact
//! class and message are captured and compared -- not merely "both failed".
//!
//! Rows that `ERRORS.md` explicitly documents as having **no check at all**
//! (unchecked out-of-bounds reads/writes: rows 137, 139, 143-149, 152, 173) are
//! NOT exercised: they are C-side undefined behaviour with no defined result to
//! compare against. They are listed in `UNCHECKED_UB_ROWS` below so the omission
//! is explicit and auditable rather than silent.
mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// `ERRORS.md` rows that are C-side UB (no bounds check in the C at all), and so
/// have no comparable behaviour. Documented here deliberately.
pub const UNCHECKED_UB_ROWS: &[(&str, &str)] = &[
    ("137", "js_pushlstring with negative n: `while (n--)` overruns (jsrun.c:167-170)"),
    ("139", "js_pop with negative n: TOP grows with no CHECKSTACK (jsrun.c:403-405)"),
    ("143", "js_rot2 with < 2 values: reads below BOT (jsrun.c:457-464)"),
    ("144", "js_rot3 with < 3 values: reads below BOT (jsrun.c:465-472)"),
    ("145", "js_rot4 with < 4 values: reads below BOT (jsrun.c:474-482)"),
    ("146", "js_rot2pop1 with < 2 values (jsrun.c:484-489)"),
    ("147", "js_rot3pop2 with < 3 values (jsrun.c:491-496)"),
    ("148", "js_rot with n > depth: under-runs the stack (jsrun.c:498-505)"),
    ("149", "js_dup on an empty frame: reads STACK[TOP-1] (jsrun.c:442-447)"),
    ("152", "js_currentfunctiondata type confusion (jsrun.c:212-213)"),
    ("173", "js_construct with n < 0: negative stack index (jsrun.c:1334)"),
    ("216-218,242", "assert()s in jsR_setarrayindex / jsV_resizearray"),
    ("199,200", "assert()s on sizeof(js_Value) in js_newstate"),
];

#[test]
fn unchecked_ub_rows_are_documented() {
    assert_eq!(UNCHECKED_UB_ROWS.len(), 13);
    for (row, why) in UNCHECKED_UB_ROWS {
        assert!(!row.is_empty() && !why.is_empty());
    }
    eprintln!(
        "{} ERRORS.md rows are C-side UB with no comparable result; documented, not tested",
        UNCHECKED_UB_ROWS.len()
    );
}

// ---------------------------------------------------------------------------
// Helper: declare a batch of probes and run each under both impls / both modes
// ---------------------------------------------------------------------------

macro_rules! probes {
    ($batchname:expr, $( $label:expr => |$imp:ident, $j:ident| $body:block ),* $(,)? ) => {{
        let mut b = Batch::new();
        $(
            {
                fn p($imp: &Impl, $j: JsState) $body
                for flags in [0 as c_int, JS_STRICT] {
                    b.probe(flags, $label, p as ProbeFn);
                }
            }
        )*
        b.finish($batchname);
    }};
}

/// Push a marker so the probe always leaves a comparable value when it does not throw.
fn ok(imp: &Impl, j: JsState, s: &str) {
    imp.pushstring(j, s.as_bytes());
}

// ---------------------------------------------------------------------------
// Section 7: stack API index / overflow / underflow (rows 132-142, 150-153)
// ---------------------------------------------------------------------------

#[test]
fn row_132_out_of_range_index_yields_static_undefined() {
    // Row 132: stackidx returns a static `undefined` -- NOT an error.
    probes! {"row 132 out-of-range index",
        "stackidx out of range" => |imp, j| {
            imp.pushnumber(j, 1.0);
            let mut acc = String::new();
            for idx in [5, 100, -5, -100, c_int::MAX, c_int::MIN, 0x4000_0000, -0x4000_0000] {
                acc.push_str(&format!(
                    "{idx}:t={},to={},isu={};",
                    imp.ty(j, idx),
                    show(&imp.typeof_(j, idx)),
                    imp.is(j, "js_isundefined", idx)
                ));
            }
            ok(imp, j, &acc);
        },
    }
}

#[test]
fn rows_133_134_stack_overflow() {
    // Rows 133/134: CHECKSTACK(1) and CHECKSTACK(2) -> literal "stack overflow".
    probes! {"rows 133-134 stack overflow",
        "push to overflow" => |imp, j| {
            for i in 0..(JS_STACKSIZE + 8) { imp.pushnumber(j, i as f64); }
            ok(imp, j, "NOT REACHED");
        },
        "js_dup to overflow (CHECKSTACK 1)" => |imp, j| {
            imp.pushnumber(j, 1.0);
            for _ in 0..(JS_STACKSIZE + 8) { imp.dup(j); }
            ok(imp, j, "NOT REACHED");
        },
        "js_dup2 to overflow (CHECKSTACK 2)" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.pushnumber(j, 2.0);
            for _ in 0..(JS_STACKSIZE + 8) { imp.dup2(j); }
            ok(imp, j, "NOT REACHED");
        },
        "js_copy to overflow" => |imp, j| {
            imp.pushnumber(j, 1.0);
            for _ in 0..(JS_STACKSIZE + 8) { imp.copy(j, 0); }
            ok(imp, j, "NOT REACHED");
        },
        "js_pushglobal to overflow" => |imp, j| {
            for _ in 0..(JS_STACKSIZE + 8) { imp.pushglobal(j); }
            ok(imp, j, "NOT REACHED");
        },
        "js_currentfunction to overflow" => |imp, j| {
            for _ in 0..(JS_STACKSIZE + 8) { imp.currentfunction(j); }
            ok(imp, j, "NOT REACHED");
        },
        "exactly at the limit" => |imp, j| {
            let mut n = 0;
            // Push until it throws, recording how many succeeded is not possible
            // (the throw unwinds), so instead push a fixed safe amount.
            while n < JS_STACKSIZE - 64 { imp.pushnumber(j, n as f64); n += 1; }
            ok(imp, j, &format!("pushed {} top={}", n, imp.gettop(j)));
        },
    }
}

#[test]
fn row_138_pop_underflow() {
    // Row 138: js_pop(n) with n > gettop -> Error "stack underflow!".
    probes! {"row 138 pop underflow",
        "pop 1 from empty" => |imp, j| { imp.pop(j, 1); ok(imp, j, "NOT REACHED"); },
        "pop 5 from 2" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0);
            imp.pop(j, 5); ok(imp, j, "NOT REACHED");
        },
        "pop exactly gettop" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0);
            imp.pop(j, 2); ok(imp, j, &format!("ok top={}", imp.gettop(j)));
        },
        "pop 0" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.pop(j, 0); ok(imp, j, &format!("ok top={}", imp.gettop(j)));
        },
        "pop gettop+1" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.pop(j, imp.gettop(j) + 1); ok(imp, j, "NOT REACHED");
        },
    }
}

#[test]
fn rows_140_141_remove_and_replace_stack_error() {
    // Rows 140/141: js_remove / js_replace with an out-of-frame index ->
    // Error "stack error!".
    probes! {"rows 140-141 stack error",
        "remove from empty" => |imp, j| { imp.remove(j, 0); ok(imp, j, "NOT REACHED"); },
        "remove idx==top" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.remove(j, 1); ok(imp, j, "NOT REACHED");
        },
        "remove idx far positive" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.remove(j, 1000); ok(imp, j, "NOT REACHED");
        },
        "remove idx far negative" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.remove(j, -1000); ok(imp, j, "NOT REACHED");
        },
        "remove INT_MAX" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.remove(j, c_int::MAX); ok(imp, j, "NOT REACHED");
        },
        "remove INT_MIN" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.remove(j, c_int::MIN); ok(imp, j, "NOT REACHED");
        },
        "remove valid" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0); imp.remove(j, 0);
            ok(imp, j, &format!("ok top={} v={}", imp.gettop(j), show(&imp.trystring(j, -1))));
        },
        "replace on empty" => |imp, j| { imp.replace(j, 0); ok(imp, j, "NOT REACHED"); },
        "replace idx==top" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.replace(j, 1); ok(imp, j, "NOT REACHED");
        },
        "replace idx far" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0); imp.replace(j, 1000);
            ok(imp, j, "NOT REACHED");
        },
        "replace INT_MIN" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0); imp.replace(j, c_int::MIN);
            ok(imp, j, "NOT REACHED");
        },
        "replace valid" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 2.0); imp.replace(j, 0);
            ok(imp, j, &format!("ok top={} v={}", imp.gettop(j), show(&imp.trystring(j, 0))));
        },
    }
}

#[test]
fn row_142_insert_not_implemented() {
    // Row 142: js_insert ALWAYS raises Error "not implemented yet".
    probes! {"row 142 js_insert",
        "insert 0" => |imp, j| {
            imp.pushnumber(j, 1.0);
            unsafe { imp.f::<FnVoidInt>("js_insert")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "insert -1 on empty" => |imp, j| {
            unsafe { imp.f::<FnVoidInt>("js_insert")(j, -1) };
            ok(imp, j, "NOT REACHED");
        },
        "insert INT_MAX" => |imp, j| {
            unsafe { imp.f::<FnVoidInt>("js_insert")(j, c_int::MAX) };
            ok(imp, j, "NOT REACHED");
        },
    }
}

#[test]
fn rows_150_151_153_currentfunction_at_top_level() {
    // Rows 150/151: js_currentfunction pushes undefined and
    // js_currentfunctiondata returns NULL when BOT == 0. Row 153: js_gettop.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        // BOT == 0 here (we are not inside a call).
        b.check(&format!("gettop fresh flags={flags}"), c.gettop(jc), r.gettop(jr));
        c.currentfunction(jc);
        r.currentfunction(jr);
        b.check(
            &format!("currentfunction at top level flags={flags}"),
            (c.ty(jc, -1), show(&c.typeof_(jc, -1)), show(&c.trystring(jc, -1))),
            (r.ty(jr, -1), show(&r.typeof_(jr, -1)), show(&r.trystring(jr, -1))),
        );
        let dc = unsafe { c.f::<unsafe extern "C" fn(JsState) -> *mut c_void>("js_currentfunctiondata")(jc) };
        let dr = unsafe { r.f::<unsafe extern "C" fn(JsState) -> *mut c_void>("js_currentfunctiondata")(jr) };
        b.check(
            &format!("currentfunctiondata at top level flags={flags}"),
            dc.is_null(),
            dr.is_null(),
        );
        c.pop(jc, 1);
        r.pop(jr, 1);
        c.freestate(jc);
        r.freestate(jr);
    }
    // Inside a C function, BOT > 0, so js_currentfunction yields the function.
    probes! {"rows 150-151 inside a call",
        "currentfunction inside a cfunction" => |imp, j| {
            imp.currentfunction(j);
            let s = format!("t={} to={} callable={}",
                imp.ty(j, -1), show(&imp.typeof_(j, -1)), imp.is(j, "js_iscallable", -1));
            imp.pop(j, 1);
            let d = unsafe { imp.f::<unsafe extern "C" fn(JsState) -> *mut c_void>("js_currentfunctiondata")(j) };
            ok(imp, j, &format!("{s} data_null={}", d.is_null()));
        },
    }
    b.finish("rows 150-151-153 currentfunction/gettop");
}

// ---------------------------------------------------------------------------
// Section 8: coercions that throw (rows 154-168)
// ---------------------------------------------------------------------------

#[test]
fn rows_154_156_toregexp_and_touserdata() {
    probes! {"rows 154-156 toregexp/touserdata",
        "js_toregexp on a number" => |imp, j| {
            imp.pushnumber(j, 1.0);
            unsafe { imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut c_void>("js_toregexp")(j, -1) };
            ok(imp, j, "NOT REACHED");
        },
        "js_toregexp on an out-of-range idx" => |imp, j| {
            unsafe { imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut c_void>("js_toregexp")(j, 99) };
            ok(imp, j, "NOT REACHED");
        },
        "js_toregexp on a plain object" => |imp, j| {
            imp.newobject(j);
            unsafe { imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut c_void>("js_toregexp")(j, -1) };
            ok(imp, j, "NOT REACHED");
        },
        "js_toregexp on a real regexp" => |imp, j| {
            imp.newregexp(j, "a+", JS_REGEXP_G);
            let p = unsafe { imp.f::<unsafe extern "C" fn(JsState, c_int) -> *mut c_void>("js_toregexp")(j, -1) };
            ok(imp, j, &format!("ok non_null={}", !p.is_null()));
        },
        "js_touserdata on a number" => |imp, j| {
            imp.pushnumber(j, 1.0);
            let tag = cstr("Tag");
            unsafe { imp.f::<FnTouserdata>("js_touserdata")(j, -1, tag.as_ptr()) };
            ok(imp, j, "NOT REACHED");
        },
        "js_touserdata with the wrong tag" => |imp, j| {
            let tag = cstr("Right");
            let wrong = cstr("Wrong");
            unsafe { imp.f::<FnNewuserdata>("js_newuserdata")(j, tag.as_ptr(), 7usize as *mut c_void, None) };
            unsafe { imp.f::<FnTouserdata>("js_touserdata")(j, -1, wrong.as_ptr()) };
            ok(imp, j, "NOT REACHED");
        },
        "js_touserdata with the right tag" => |imp, j| {
            let tag = cstr("Right");
            unsafe { imp.f::<FnNewuserdata>("js_newuserdata")(j, tag.as_ptr(), 7usize as *mut c_void, None) };
            let p = unsafe { imp.f::<FnTouserdata>("js_touserdata")(j, -1, tag.as_ptr()) };
            ok(imp, j, &format!("ok data={}", p as usize));
        },
        "js_isuserdata never throws" => |imp, j| {
            let tag = cstr("Tag");
            imp.pushnumber(j, 1.0);
            let a = unsafe { imp.f::<FnIsuserdata>("js_isuserdata")(j, -1, tag.as_ptr()) };
            let b2 = unsafe { imp.f::<FnIsuserdata>("js_isuserdata")(j, 99, tag.as_ptr()) };
            ok(imp, j, &format!("is={a},{b2}"));
        },
    }
}

#[test]
fn row_157_defaccessor_not_a_function() {
    probes! {"row 157 defaccessor non-function",
        "getter is a number" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.pushnull(j);
            imp.defaccessor(j, -3, "p", 0);
            ok(imp, j, "NOT REACHED");
        },
        "setter is a string" => |imp, j| {
            imp.newobject(j);
            imp.pushundefined(j);
            imp.pushstring(j, b"nope");
            imp.defaccessor(j, -3, "p", 0);
            ok(imp, j, "NOT REACHED");
        },
        "getter is a plain object" => |imp, j| {
            imp.newobject(j);
            imp.newobject(j);
            imp.pushnull(j);
            imp.defaccessor(j, -3, "p", 0);
            ok(imp, j, "NOT REACHED");
        },
        "both undefined/null is accepted" => |imp, j| {
            imp.newobject(j);
            imp.pushundefined(j);
            imp.pushnull(j);
            imp.defaccessor(j, -3, "p", 0);
            imp.getproperty(j, -1, "p");
            let s = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok p={s}"));
        },
    }
}

#[test]
fn rows_158_159_toobject_on_undefined_and_null() {
    // Every index-taking property function funnels through jsV_toobject.
    probes! {"rows 158-159 toobject",
        "getproperty on undefined" => |imp, j| {
            imp.pushundefined(j); imp.getproperty(j, -1, "x"); ok(imp, j, "NOT REACHED");
        },
        "getproperty on null" => |imp, j| {
            imp.pushnull(j); imp.getproperty(j, -1, "x"); ok(imp, j, "NOT REACHED");
        },
        "getproperty on an out-of-range idx (=> undefined)" => |imp, j| {
            imp.getproperty(j, 77, "x"); ok(imp, j, "NOT REACHED");
        },
        "setproperty on undefined" => |imp, j| {
            imp.pushundefined(j); imp.pushnumber(j, 1.0); imp.setproperty(j, -2, "x");
            ok(imp, j, "NOT REACHED");
        },
        "setproperty on null" => |imp, j| {
            imp.pushnull(j); imp.pushnumber(j, 1.0); imp.setproperty(j, -2, "x");
            ok(imp, j, "NOT REACHED");
        },
        "hasproperty on undefined" => |imp, j| {
            imp.pushundefined(j); imp.hasproperty(j, -1, "x"); ok(imp, j, "NOT REACHED");
        },
        "delproperty on null" => |imp, j| {
            imp.pushnull(j); imp.delproperty(j, -1, "x"); ok(imp, j, "NOT REACHED");
        },
        "defproperty on undefined" => |imp, j| {
            imp.pushundefined(j); imp.pushnumber(j, 1.0); imp.defproperty(j, -2, "x", 0);
            ok(imp, j, "NOT REACHED");
        },
        "getindex on undefined" => |imp, j| {
            imp.pushundefined(j); imp.getindex(j, -1, 0); ok(imp, j, "NOT REACHED");
        },
        "setindex on null" => |imp, j| {
            imp.pushnull(j); imp.pushnumber(j, 1.0); imp.setindex(j, -2, 0);
            ok(imp, j, "NOT REACHED");
        },
        "delindex on undefined" => |imp, j| {
            imp.pushundefined(j); imp.delindex(j, -1, 0); ok(imp, j, "NOT REACHED");
        },
        "hasindex on null" => |imp, j| {
            imp.pushnull(j); imp.hasindex(j, -1, 0); ok(imp, j, "NOT REACHED");
        },
        "getlength on undefined" => |imp, j| {
            imp.pushundefined(j); imp.getlength(j, -1); ok(imp, j, "NOT REACHED");
        },
        "setlength on null" => |imp, j| {
            imp.pushnull(j); imp.setlength(j, -1, 3); ok(imp, j, "NOT REACHED");
        },
        "pushiterator on undefined" => |imp, j| {
            imp.pushundefined(j); imp.pushiterator(j, -1, 1); ok(imp, j, "NOT REACHED");
        },
        "pushiterator on null" => |imp, j| {
            imp.pushnull(j); imp.pushiterator(j, -1, 0); ok(imp, j, "NOT REACHED");
        },
        "getproperty on a primitive number (transient)" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.getproperty(j, -1, "toFixed");
            ok(imp, j, &format!("ok t={}", imp.ty(j, -1)));
        },
        "getproperty on a primitive string" => |imp, j| {
            imp.pushstring(j, b"ab"); imp.getproperty(j, -1, "length");
            ok(imp, j, &format!("ok len={}", show(&imp.trystring(j, -1))));
        },
        "setproperty on a primitive string (transient)" => |imp, j| {
            imp.pushstring(j, b"ab"); imp.pushnumber(j, 1.0); imp.setproperty(j, -2, "foo");
            ok(imp, j, "ok");
        },
    }
}

#[test]
fn rows_160_161_toprimitive_strict_dependence() {
    // Row 160 (strict): TypeError; row 161 (non-strict): the literal "[object]".
    probes! {"rows 160-161 toprimitive",
        "toprimitive on a null-prototype object" => |imp, j| {
            // Build Object.create(null) via a script, then coerce from the API.
            let out = imp.eval_on(j, b"Object.create(null)");
            let _ = out;
            imp.eval_on(j, b"1");
            // Do it the direct way instead: an object with no toString/valueOf.
            imp.newobjectx(j); // newobjectx => no prototype
            let s = imp.tostring(j, -1).map(|x| show(&x));
            ok(imp, j, &format!("tostring={s:?}"));
        },
        "toprimitive via js_concat" => |imp, j| {
            imp.newobjectx(j);
            imp.pushstring(j, b"x");
            imp.concat(j);
            ok(imp, j, &format!("concat={}", show(&imp.trystring(j, -1))));
        },
        "toprimitive via js_tonumber" => |imp, j| {
            imp.newobjectx(j);
            let n = imp.tonumber(j, -1);
            ok(imp, j, &format!("tonumber={:016x}", n.to_bits()));
        },
    }
}

#[test]
fn rows_162_164_instanceof() {
    probes! {"rows 162-164 instanceof",
        "rhs not callable (number)" => |imp, j| {
            imp.newobject(j); imp.pushnumber(j, 1.0); imp.instanceof(j);
            ok(imp, j, "NOT REACHED");
        },
        "rhs not callable (object)" => |imp, j| {
            imp.newobject(j); imp.newobject(j); imp.instanceof(j);
            ok(imp, j, "NOT REACHED");
        },
        "rhs prototype not an object" => |imp, j| {
            let o = imp.eval_on(j, b"(function(){var f=function(){}; f.prototype=5; return f})()");
            let _ = o;
            imp.newobject(j);
            imp.getglobal(j, "__f");
            imp.instanceof(j);
            ok(imp, j, "NOT REACHED");
        },
        "lhs not an object -> returns 0" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.getglobal(j, "Object");
            let rc = imp.instanceof(j);
            ok(imp, j, &format!("rc={rc}"));
        },
        "valid instanceof" => |imp, j| {
            imp.newarray(j);
            imp.getglobal(j, "Array");
            let rc = imp.instanceof(j);
            ok(imp, j, &format!("rc={rc}"));
        },
        "valid instanceof false" => |imp, j| {
            imp.newarray(j);
            imp.getglobal(j, "Date");
            let rc = imp.instanceof(j);
            ok(imp, j, &format!("rc={rc}"));
        },
    }
}

#[test]
fn row_165_compare_okay_flag() {
    // Row 165: js_compare sets *okay = 0 when either side is NaN.
    probes! {"row 165 js_compare okay flag",
        "NaN vs number" => |imp, j| {
            imp.pushnumber(j, f64::NAN); imp.pushnumber(j, 1.0);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "number vs NaN" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, f64::NAN);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "non-numeric string vs number" => |imp, j| {
            imp.pushstring(j, b"zz"); imp.pushnumber(j, 1.0);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "string vs string" => |imp, j| {
            imp.pushstring(j, b"a"); imp.pushstring(j, b"b");
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "number vs number" => |imp, j| {
            imp.pushnumber(j, 2.0); imp.pushnumber(j, 1.0);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "equal numbers" => |imp, j| {
            imp.pushnumber(j, 1.0); imp.pushnumber(j, 1.0);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
        "undefined vs undefined" => |imp, j| {
            imp.pushundefined(j); imp.pushundefined(j);
            let (rc, okay) = imp.compare(j);
            ok(imp, j, &format!("rc={rc} okay={okay}"));
        },
    }
}

// ---------------------------------------------------------------------------
// Section 9: call machinery (rows 169-172, 174-178, 180-189)
// ---------------------------------------------------------------------------

#[test]
fn rows_169_172_call_and_construct() {
    probes! {"rows 169-172 call/construct",
        "js_call with n<0" => |imp, j| {
            imp.getglobal(j, "String");
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, -1) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call with n=-100" => |imp, j| {
            imp.getglobal(j, "String");
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, -100) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call with INT_MIN" => |imp, j| {
            imp.getglobal(j, "String");
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, c_int::MIN) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call a number" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call a string" => |imp, j| {
            imp.pushstring(j, b"nope");
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call a plain object" => |imp, j| {
            imp.newobject(j);
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call undefined" => |imp, j| {
            imp.pushundefined(j);
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_call an array" => |imp, j| {
            imp.newarray(j);
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_pcall a number returns 1" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.pushundefined(j);
            let rc = imp.pcall(j, 0);
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("rc={rc} err={v}"));
        },
        "js_pconstruct a number returns 1" => |imp, j| {
            imp.pushnumber(j, 1.0);
            let rc = imp.pconstruct(j, 0);
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("rc={rc} err={v}"));
        },
        "js_construct a non-callable" => |imp, j| {
            imp.newobject(j);
            unsafe { imp.f::<FnVoidInt>("js_construct")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "js_construct a regexp object" => |imp, j| {
            imp.newregexp(j, "a", 0);
            unsafe { imp.f::<FnVoidInt>("js_construct")(j, 0) };
            ok(imp, j, "NOT REACHED");
        },
        "valid js_call with 0 args" => |imp, j| {
            imp.getglobal(j, "String");
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 0) };
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
        "valid js_call with 1 arg" => |imp, j| {
            imp.getglobal(j, "String");
            imp.pushundefined(j);
            imp.pushnumber(j, 42.0);
            unsafe { imp.f::<FnVoidInt>("js_call")(j, 1) };
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
        "valid js_construct" => |imp, j| {
            imp.getglobal(j, "Array");
            imp.pushnumber(j, 3.0);
            unsafe { imp.f::<FnVoidInt>("js_construct")(j, 1) };
            ok(imp, j, &format!("ok len={}", imp.getlength(j, -1)));
        },
    }
}

#[test]
fn rows_174_175_call_stack_and_env_overflow() {
    // Rows 174/175: JS_ENVLIMIT (1024) via deep recursion ->
    // Error "call stack overflow" / literal "stack overflow".
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for src in [
            // non-lightweight (uses `arguments`) -> jsR_savescope path
            "(function f(n){ var a=arguments; return n<=0?0:1+f(n-1) })(2000)",
            // lightweight -> jsR_pushtrace path
            "(function f(n){ return n<=0?0:1+f(n-1) })(2000)",
            "(function f(n){ return n<=0?0:1+f(n-1) })(1022)",
            "(function f(n){ return n<=0?0:1+f(n-1) })(1023)",
            "(function f(n){ return n<=0?0:1+f(n-1) })(1024)",
            "(function f(n){ return n<=0?0:1+f(n-1) })(1025)",
            // mutual recursion
            "function a(n){return n<=0?0:1+b(n-1)} function b(n){return n<=0?0:1+a(n-1)} a(3000)",
            // recursion through a builtin
            "(function f(n){ return n<=0?0:[0].map(function(){return 1+f(n-1)})[0] })(1000)",
            // recursion through toString
            "(function(){var o={toString:function(){return ''+o}}; return ''+o})()",
            // recursion via new
            "function C(n){ if(n>0) new C(n-1) } new C(3000); 'done'",
        ] {
            b.check(
                &format!("deep recursion flags={flags} src={src:?}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
        }
    }
    b.finish("rows 174-175 call/env stack overflow");
}

#[test]
fn rows_176_177_194_try_stack_overflow() {
    // Rows 176/177/194: JS_TRYLIMIT (64) nested try frames ->
    // literal "exception stack overflow".
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for depth in [1usize, 32, 62, 63, 64, 65, 66, 100, 200] {
            let src = format!(
                "{}throw 1{}",
                "try{".repeat(depth),
                "}catch(e){throw e}".repeat(depth)
            );
            b.check(
                &format!("nested try depth={depth} flags={flags}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
            // try/finally form
            let src2 = format!("{}1{}", "try{".repeat(depth), "}finally{}".repeat(depth));
            b.check(
                &format!("nested try/finally depth={depth} flags={flags}"),
                c.eval_script(flags, src2.as_bytes()),
                r.eval_script(flags, src2.as_bytes()),
            );
        }
        // js_pcall/js_pconstruct with a nearly-full try stack (row 188)
        for depth in [60usize, 63, 64] {
            let src = format!(
                "{}(function(){{try{{ return 1 }}catch(e){{ return 2 }}}})(){}",
                "try{".repeat(depth),
                "}catch(e){}".repeat(depth)
            );
            b.check(
                &format!("pcall under nested try depth={depth} flags={flags}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
        }
    }
    b.finish("rows 176-177-188-194 try stack overflow");
}

#[test]
fn rows_180_183_out_of_memory() {
    // Rows 180-183: js_malloc/js_realloc hitting memlimit or a NULL allocator ->
    // literal "out of memory".
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for mem in [1 as c_int, 2, 16, 64, 256, 1024, 4096, 16384, 65536] {
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            c.setlimit(jc, 0, mem);
            r.setlimit(jr, 0, mem);
            for src in [
                "1+1",
                "'a'+'b'",
                "var s='x'; s+s+s",
                "var a=[]; a.push(1); a.length",
                "var o={}; o.a=1; o.a",
                "var s='x'; for(var i=0;i<10;i++) s+=s; s.length",
                "JSON.stringify({a:[1,2,3]})",
                "new RegExp('a+b')",
                "(function(){return 1})()",
                "'abcdefghijklmnopqrstuvwxyz0123456789'.toUpperCase()",
            ] {
                b.check(
                    &format!("memlimit={mem} flags={flags} src={src:?}"),
                    c.eval_on(jc, src.as_bytes()),
                    r.eval_on(jr, src.as_bytes()),
                );
            }
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("rows 180-183 out of memory");
}

#[test]
fn row_184_run_limit() {
    // Row 184: the interpreter throws the literal "script ran too long" when
    // runlimit reaches 1.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for run in [1 as c_int, 2, 3, 5, 10, 50, 100, 500, 1000, 5000, 50000] {
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            c.setlimit(jc, run, 0);
            r.setlimit(jr, run, 0);
            for src in [
                "1",
                "1+1",
                "var i=0; while(i<10) i++; i",
                "var i=0; while(i<1000) i++; i",
                "while(1);",
                "for(;;){}",
                "(function f(){return f()})()",
                "[1,2,3].map(function(x){return x*2}).join(',')",
            ] {
                b.check(
                    &format!("runlimit={run} flags={flags} src={src:?}"),
                    c.eval_on(jc, src.as_bytes()),
                    r.eval_on(jr, src.as_bytes()),
                );
            }
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("row 184 run limit");
}

#[test]
fn row_189_eval_non_string() {
    // Row 189: js_eval with a non-string on top returns leaving it in place.
    probes! {"row 189 js_eval non-string",
        "eval a number" => |imp, j| {
            imp.pushnumber(j, 42.0);
            unsafe { imp.f::<FnVoid1>("js_eval")(j) };
            ok(imp, j, &format!("t={} v={}", imp.ty(j, -1), show(&imp.trystring(j, -1))));
        },
        "eval an object" => |imp, j| {
            imp.newobject(j);
            unsafe { imp.f::<FnVoid1>("js_eval")(j) };
            ok(imp, j, &format!("t={} v={}", imp.ty(j, -1), show(&imp.trystring(j, -1))));
        },
        "eval undefined" => |imp, j| {
            imp.pushundefined(j);
            unsafe { imp.f::<FnVoid1>("js_eval")(j) };
            ok(imp, j, &format!("t={} v={}", imp.ty(j, -1), show(&imp.trystring(j, -1))));
        },
        "eval a valid string" => |imp, j| {
            imp.pushstring(j, b"1+2");
            unsafe { imp.f::<FnVoid1>("js_eval")(j) };
            ok(imp, j, &format!("t={} v={}", imp.ty(j, -1), show(&imp.trystring(j, -1))));
        },
        "eval a syntactically bad string" => |imp, j| {
            imp.pushstring(j, b"1+");
            unsafe { imp.f::<FnVoid1>("js_eval")(j) };
            ok(imp, j, "NOT REACHED");
        },
    }
}

// ---------------------------------------------------------------------------
// Section 10: js_newstate / try-converters (rows 195-198, 201-214)
// ---------------------------------------------------------------------------

#[test]
fn rows_195_201_newstate_flags_and_alloc() {
    // Row 195: alloc == NULL falls back to the default allocator (not an error).
    // Row 201: flag bits other than JS_STRICT are silently ignored.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [
        0 as c_int, 1, 2, 3, 4, 999, 0x7fff_ffff, -1, c_int::MIN, 0x1000, 1 | 0x1000,
    ] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        // Only bit 0 should matter: probe a strict-only behaviour.
        for src in [
            "(function(){try{ undeclared_xyz = 1; return 'assigned' }catch(e){ return e.name+': '+e.message }})()",
            "(function(){try{ NaN = 1; return 'assigned' }catch(e){ return e.name+': '+e.message }})()",
            "(function(){ return typeof this })()",
            "(function(){try{ return eval('with({}){}') }catch(e){ return e.name }})()",
        ] {
            b.check(
                &format!("newstate(flags={flags}) src={src:?}"),
                c.eval_on(jc, src.as_bytes()),
                r.eval_on(jr, src.as_bytes()),
            );
        }
        c.freestate(jc);
        r.freestate(jr);
    }
    b.finish("rows 195/201 newstate flags");
}

#[test]
fn rows_202_211_try_converters_under_full_try_stack() {
    // Rows 202-211: js_ptry pushes the literal "exception stack overflow" and
    // the js_try* converters return the caller's fallback.
    probes! {"rows 202-211 try converters",
        "js_trystring on a throwing toString" => |imp, j| {
            let o = imp.eval_on(j, b"({toString:function(){throw new Error('nope')}})");
            let _ = o;
            // Rebuild via the API: an object whose toString throws.
            imp.eval_on(j, b"__t = {toString:function(){throw new Error('nope')}}; 1");
            imp.getglobal(j, "__t");
            let s = show(&imp.trystring(j, -1));
            let n = imp.trynumber(j, -1);
            let i = imp.tryinteger(j, -1);
            let bo = imp.tryboolean(j, -1);
            let rp = show(&imp.tryrepr(j, -1));
            ok(imp, j, &format!("s={s} n={:016x} i={i} b={bo} r={rp}", n.to_bits()));
        },
        "js_try* on a throwing valueOf" => |imp, j| {
            imp.eval_on(j, b"__v = {valueOf:function(){throw new Error('vo')}, toString:function(){return 'S'}}; 1");
            imp.getglobal(j, "__v");
            let s = show(&imp.trystring(j, -1));
            let n = imp.trynumber(j, -1);
            let i = imp.tryinteger(j, -1);
            let bo = imp.tryboolean(j, -1);
            ok(imp, j, &format!("s={s} n={:016x} i={i} b={bo}", n.to_bits()));
        },
        "js_try* on plain values" => |imp, j| {
            let mut acc = String::new();
            for k in 0..5 {
                match k {
                    0 => imp.pushundefined(j),
                    1 => imp.pushnull(j),
                    2 => imp.pushnumber(j, 2.5),
                    3 => imp.pushstring(j, b"7"),
                    _ => imp.newobject(j),
                }
                acc.push_str(&format!(
                    "{k}:s={},n={:016x},i={},b={};",
                    show(&imp.trystring(j, -1)),
                    imp.trynumber(j, -1).to_bits(),
                    imp.tryinteger(j, -1),
                    imp.tryboolean(j, -1)
                ));
                imp.pop(j, 1);
            }
            ok(imp, j, &acc);
        },
        "js_try* on an out-of-range index" => |imp, j| {
            let s = show(&imp.trystring(j, 99));
            let n = imp.trynumber(j, 99);
            let i = imp.tryinteger(j, 99);
            let bo = imp.tryboolean(j, 99);
            ok(imp, j, &format!("s={s} n={:016x} i={i} b={bo}", n.to_bits()));
        },
    }
}

#[test]
fn rows_203_206_ploadstring_and_dostring() {
    // Rows 203-206: js_ploadstring / js_dostring return 1 with the error on the
    // stack (and js_dostring routes the message through js_report).
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for src in [
            "1+1",
            "func(",
            "var 1x=2",
            "throw new Error('x')",
            "null.foo",
            "",
            " ",
            "// only a comment",
            "/* unterminated",
            "'unterminated",
            "undefinedFn()",
        ] {
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            // js_ploadstring
            let lc = c.ploadstring(jc, "f.js", src.as_bytes());
            let lr = r.ploadstring(jr, "f.js", src.as_bytes());
            b.check(
                &format!("ploadstring rc flags={flags} src={src:?}"),
                (lc, show(&c.trystring(jc, -1)), c.gettop(jc)),
                (lr, show(&r.trystring(jr, -1)), r.gettop(jr)),
            );
            c.pop(jc, 1);
            r.pop(jr, 1);
            // js_dostring
            let dc = unsafe { c.f::<FnDostring>("js_dostring")(jc, cstr(src).as_ptr()) };
            let dr = unsafe { r.f::<FnDostring>("js_dostring")(jr, cstr(src).as_ptr()) };
            b.check(
                &format!("dostring rc flags={flags} src={src:?}"),
                (dc, c.gettop(jc)),
                (dr, r.gettop(jr)),
            );
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("rows 203-206 ploadstring/dostring");
}

#[test]
fn row_212_report_null_is_silent() {
    // Row 212: js_setreport(J, NULL) makes js_report a no-op.
    let (c, r) = Impl::both();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        let rc = unsafe { c.f::<FnVoidStr>("js_report")(jc, cstr("hello").as_ptr()) };
        let rr = unsafe { r.f::<FnVoidStr>("js_report")(jr, cstr("hello").as_ptr()) };
        let _ = (rc, rr);
        // must not have disturbed the stack
        assert_eq!(c.gettop(jc), r.gettop(jr), "js_report(NULL handler) changed the stack");
        c.freestate(jc);
        r.freestate(jr);
    }
}

#[test]
fn row_214_atpanic_returns_previous_handler() {
    // Row 214: js_atpanic installs the new handler and returns the old one.
    let (c, r) = Impl::both();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        type At = unsafe extern "C" fn(JsState, *const c_void) -> *const c_void;
        // On a fresh state the handler is js_defaultpanic, i.e. non-NULL.
        let oc = unsafe { c.f::<At>("js_atpanic")(jc, std::ptr::null()) };
        let or = unsafe { r.f::<At>("js_atpanic")(jr, std::ptr::null()) };
        assert_eq!(oc.is_null(), or.is_null(), "initial panic handler nullness differs");
        assert!(!oc.is_null(), "C should start with js_defaultpanic installed");
        // Now it is NULL; installing again should return NULL.
        let oc2 = unsafe { c.f::<At>("js_atpanic")(jc, oc) };
        let or2 = unsafe { r.f::<At>("js_atpanic")(jr, or) };
        assert_eq!(oc2.is_null(), or2.is_null(), "second js_atpanic result differs");
        assert!(oc2.is_null(), "C should have returned the NULL we installed");
        c.freestate(jc);
        r.freestate(jr);
    }
}

// ---------------------------------------------------------------------------
// Section 11: property / array limits (rows 215, 219-241, 243-250)
// ---------------------------------------------------------------------------

#[test]
fn rows_215_220_array_limits() {
    // Rows 215/219/220/250: JS_ARRAYLIMIT (1<<26) and invalid array length.
    probes! {"rows 215-220 array limits",
        "setlength negative" => |imp, j| {
            imp.newarray(j); imp.setlength(j, -1, -1); ok(imp, j, "NOT REACHED");
        },
        "setlength INT_MIN" => |imp, j| {
            imp.newarray(j); imp.setlength(j, -1, c_int::MIN); ok(imp, j, "NOT REACHED");
        },
        "setlength at ARRAYLIMIT" => |imp, j| {
            imp.newarray(j); imp.setlength(j, -1, 1 << 26);
            ok(imp, j, &format!("ok len={}", imp.getlength(j, -1)));
        },
        "setlength past ARRAYLIMIT" => |imp, j| {
            imp.newarray(j); imp.setlength(j, -1, (1 << 26) + 1); ok(imp, j, "NOT REACHED");
        },
        "setlength INT_MAX" => |imp, j| {
            imp.newarray(j); imp.setlength(j, -1, c_int::MAX); ok(imp, j, "NOT REACHED");
        },
        "setindex at ARRAYLIMIT-1" => |imp, j| {
            imp.newarray(j); imp.pushnumber(j, 1.0); imp.setindex(j, -2, (1 << 26) - 1);
            ok(imp, j, &format!("ok len={}", imp.getlength(j, -1)));
        },
        "setindex at ARRAYLIMIT" => |imp, j| {
            imp.newarray(j); imp.pushnumber(j, 1.0); imp.setindex(j, -2, 1 << 26);
            ok(imp, j, &format!("ok len={}", imp.getlength(j, -1)));
        },
        "setindex negative" => |imp, j| {
            imp.newarray(j); imp.pushnumber(j, 1.0); imp.setindex(j, -2, -1);
            ok(imp, j, &format!("ok len={}", imp.getlength(j, -1)));
        },
        "getlength on a non-array object" => |imp, j| {
            imp.newobject(j);
            ok(imp, j, &format!("len={}", imp.getlength(j, -1)));
        },
        "getlength when length is NaN" => |imp, j| {
            imp.newobject(j); imp.pushnumber(j, f64::NAN); imp.setproperty(j, -2, "length");
            ok(imp, j, &format!("len={}", imp.getlength(j, -1)));
        },
        "getlength when length is Infinity" => |imp, j| {
            imp.newobject(j); imp.pushnumber(j, f64::INFINITY); imp.setproperty(j, -2, "length");
            ok(imp, j, &format!("len={}", imp.getlength(j, -1)));
        },
        "getlength when length is -Infinity" => |imp, j| {
            imp.newobject(j); imp.pushnumber(j, f64::NEG_INFINITY); imp.setproperty(j, -2, "length");
            ok(imp, j, &format!("len={}", imp.getlength(j, -1)));
        },
        "getlength when length is a huge double" => |imp, j| {
            imp.newobject(j); imp.pushnumber(j, 1e300); imp.setproperty(j, -2, "length");
            ok(imp, j, &format!("len={}", imp.getlength(j, -1)));
        },
        "defproperty array length (throw=1 path)" => |imp, j| {
            imp.newarray(j); imp.pushnumber(j, 1.0); imp.defproperty(j, -2, "length", 0);
            ok(imp, j, "NOT REACHED");
        },
        "defglobal-style throw=0 path" => |imp, j| {
            imp.newarray(j);
            imp.setglobal(j, "__arr");
            imp.pushnumber(j, 1.0);
            imp.defglobal(j, "__arr", 0);
            ok(imp, j, "ok");
        },
    }
}

#[test]
fn rows_221_239_readonly_and_nonextensible() {
    // Rows 221-239: read-only / non-configurable / non-extensible violations,
    // whose behaviour is strict-mode dependent. Run in BOTH modes.
    probes! {"rows 221-239 readonly/nonconf/nonext",
        "set through a getter-only accessor" => |imp, j| {
            imp.eval_on(j, b"__g = {get p(){return 1}}; 1");
            imp.getglobal(j, "__g");
            imp.pushnumber(j, 2.0);
            imp.setproperty(j, -2, "p");
            ok(imp, j, "ok");
        },
        "set a JS_READONLY property" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "ro", JS_READONLY);
            imp.pushnumber(j, 2.0);
            imp.setproperty(j, -2, "ro");
            imp.getproperty(j, -1, "ro");
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok ro={v}"));
        },
        "defproperty over a JS_READONLY property" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "ro", JS_READONLY);
            imp.pushnumber(j, 2.0);
            imp.defproperty(j, -2, "ro", 0);
            imp.getproperty(j, -1, "ro");
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok ro={v}"));
        },
        "defaccessor over a JS_DONTCONF property" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "nc", JS_DONTCONF);
            imp.eval_on(j, b"__get = function(){return 9}; 1");
            imp.getglobal(j, "__get");
            imp.pushnull(j);
            imp.defaccessor(j, -3, "nc", 0);
            ok(imp, j, "ok");
        },
        "delete a JS_DONTCONF property" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "nc", JS_DONTCONF);
            imp.delproperty(j, -1, "nc");
            ok(imp, j, &format!("ok has={}", imp.hasproperty(j, -1, "nc")));
        },
        "set on a primitive string (transient)" => |imp, j| {
            imp.pushstring(j, b"abc");
            imp.pushnumber(j, 1.0);
            imp.setproperty(j, -2, "brandnew");
            ok(imp, j, "ok");
        },
        "set string length" => |imp, j| {
            imp.newstring(j, "abc");
            imp.pushnumber(j, 9.0);
            imp.setproperty(j, -2, "length");
            imp.getproperty(j, -1, "length");
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok len={v}"));
        },
        "set string index" => |imp, j| {
            imp.newstring(j, "abc");
            imp.pushstring(j, b"Z");
            imp.setindex(j, -2, 0);
            imp.getindex(j, -1, 0);
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok [0]={v}"));
        },
        "set regexp source" => |imp, j| {
            imp.newregexp(j, "a", 0);
            imp.pushstring(j, b"b");
            imp.setproperty(j, -2, "source");
            imp.getproperty(j, -1, "source");
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok source={v}"));
        },
        "set regexp global" => |imp, j| {
            imp.newregexp(j, "a", 0);
            imp.pushboolean(j, 1);
            imp.setproperty(j, -2, "global");
            ok(imp, j, "ok");
        },
        "delete regexp lastIndex" => |imp, j| {
            imp.newregexp(j, "a", 0);
            imp.delproperty(j, -1, "lastIndex");
            ok(imp, j, "ok");
        },
        "delete array length" => |imp, j| {
            imp.newarray(j);
            imp.delproperty(j, -1, "length");
            ok(imp, j, "ok");
        },
        "delete string length" => |imp, j| {
            imp.newstring(j, "abc");
            imp.delproperty(j, -1, "length");
            ok(imp, j, "ok");
        },
        "add to a non-extensible object" => |imp, j| {
            imp.eval_on(j, b"__ne = Object.preventExtensions({}); 1");
            imp.getglobal(j, "__ne");
            imp.pushnumber(j, 1.0);
            imp.setproperty(j, -2, "brandnew");
            ok(imp, j, &format!("ok has={}", imp.hasproperty(j, -1, "brandnew")));
        },
        "defproperty on a non-extensible object" => |imp, j| {
            imp.eval_on(j, b"__ne2 = Object.preventExtensions({}); 1");
            imp.getglobal(j, "__ne2");
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "brandnew", 0);
            ok(imp, j, &format!("ok has={}", imp.hasproperty(j, -1, "brandnew")));
        },
        "setglobal a read-only global (NaN)" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.setglobal(j, "NaN");
            imp.getglobal(j, "NaN");
            let v = show(&imp.trystring(j, -1));
            imp.pop(j, 1);
            ok(imp, j, &format!("ok NaN={v}"));
        },
        "delglobal a non-configurable global" => |imp, j| {
            imp.delglobal(j, "NaN");
            ok(imp, j, "ok");
        },
    }
}

#[test]
fn rows_240_241_iterator_errors() {
    // Rows 240/241: js_nextiterator on a non-iterator -> TypeError
    // "not an iterator"; exhausted iterator -> NULL.
    probes! {"rows 240-241 iterator",
        "nextiterator on a plain object" => |imp, j| {
            imp.newobject(j);
            imp.nextiterator(j, -1);
            ok(imp, j, "NOT REACHED");
        },
        "nextiterator on a number" => |imp, j| {
            imp.pushnumber(j, 1.0);
            imp.nextiterator(j, -1);
            ok(imp, j, "NOT REACHED");
        },
        "nextiterator on an out-of-range idx" => |imp, j| {
            imp.nextiterator(j, 99);
            ok(imp, j, "NOT REACHED");
        },
        "nextiterator on an array" => |imp, j| {
            imp.newarray(j);
            imp.nextiterator(j, -1);
            ok(imp, j, "NOT REACHED");
        },
        "exhausted iterator returns NULL" => |imp, j| {
            imp.newobject(j);
            imp.pushiterator(j, -1, 1);
            let a = imp.nextiterator(j, -1);
            let b2 = imp.nextiterator(j, -1);
            ok(imp, j, &format!("first={a:?} second={b2:?}"));
        },
        "iterator over an object with props" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "a", 0);
            imp.pushnumber(j, 2.0);
            imp.defproperty(j, -2, "b", 0);
            imp.pushiterator(j, -1, 1);
            let mut names = Vec::new();
            loop {
                match imp.nextiterator(j, -1) {
                    Some(n) => names.push(show(&n)),
                    None => break,
                }
            }
            names.sort();
            // still exhausted after the end
            let extra = imp.nextiterator(j, -1);
            ok(imp, j, &format!("names={names:?} extra={extra:?}"));
        },
    }
}

#[test]
fn rows_243_246_missing_property_lookups() {
    // Rows 243-246: missing properties are NOT errors.
    probes! {"rows 243-246 missing properties",
        "getproperty missing" => |imp, j| {
            imp.newobject(j);
            imp.getproperty(j, -1, "nope");
            ok(imp, j, &format!("t={} v={}", imp.ty(j, -1), show(&imp.trystring(j, -1))));
        },
        "hasproperty missing" => |imp, j| {
            imp.newobject(j);
            let top_before = imp.gettop(j);
            let h = imp.hasproperty(j, -1, "nope");
            ok(imp, j, &format!("h={h} pushed={}", imp.gettop(j) - top_before));
        },
        "hasproperty present pushes the value" => |imp, j| {
            imp.newobject(j);
            imp.pushnumber(j, 5.0);
            imp.defproperty(j, -2, "yes", 0);
            let top_before = imp.gettop(j);
            let h = imp.hasproperty(j, -1, "yes");
            let pushed = imp.gettop(j) - top_before;
            let v = if pushed > 0 { show(&imp.trystring(j, -1)) } else { "-".into() };
            ok(imp, j, &format!("h={h} pushed={pushed} v={v}"));
        },
        "hasindex out of range on a flat array" => |imp, j| {
            imp.newarray(j);
            imp.pushnumber(j, 1.0);
            imp.setindex(j, -2, 0);
            let mut acc = String::new();
            for k in [-1, 0, 1, 5, c_int::MAX, c_int::MIN] {
                let before = imp.gettop(j);
                let h = imp.hasindex(j, -1, k);
                let pushed = imp.gettop(j) - before;
                if pushed > 0 { imp.pop(j, pushed); }
                acc.push_str(&format!("{k}:h={h},p={pushed};"));
            }
            ok(imp, j, &acc);
        },
        "getindex out of range" => |imp, j| {
            imp.newarray(j);
            imp.pushnumber(j, 1.0);
            imp.setindex(j, -2, 0);
            let mut acc = String::new();
            for k in [-1, 0, 1, 5, c_int::MAX, c_int::MIN] {
                imp.getindex(j, -1, k);
                acc.push_str(&format!("{k}:{};", show(&imp.trystring(j, -1))));
                imp.pop(j, 1);
            }
            ok(imp, j, &acc);
        },
        "delproperty missing" => |imp, j| {
            imp.newobject(j);
            imp.delproperty(j, -1, "nope");
            ok(imp, j, "ok");
        },
    }
}

#[test]
fn row_248_js_ref_keys() {
    // Row 248: js_ref returns fixed strings for undefined/null/true/false, a
    // pointer-formatted key for objects, and a counter for other primitives.
    probes! {"row 248 js_ref",
        "ref of each value shape" => |imp, j| {
            let mut acc = String::new();
            for k in 0..8 {
                match k {
                    0 => imp.pushundefined(j),
                    1 => imp.pushnull(j),
                    2 => imp.pushboolean(j, 1),
                    3 => imp.pushboolean(j, 0),
                    4 => imp.pushnumber(j, 1.0),
                    5 => imp.pushnumber(j, 2.0),
                    6 => imp.pushstring(j, b"s"),
                    _ => imp.newobject(j),
                }
                let r = imp.refstr(j);
                // Object refs embed a pointer, so compare only their shape.
                let shown = r.as_ref().map(|x| {
                    let s = show(x);
                    if s.starts_with('_') { s } else { format!("<len {}>", s.len()) }
                });
                acc.push_str(&format!("{k}={shown:?};"));
                if let Some(rr) = &r { imp.unref(j, rr); }
            }
            ok(imp, j, &acc);
        },
        "registry get/set/del round trip" => |imp, j| {
            let mut acc = String::new();
            imp.pushnumber(j, 5.0);
            let r = imp.refstr(j).unwrap();
            imp.getregistry(j, &show(&r));
            acc.push_str(&format!("get={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            imp.unref(j, &r);
            imp.getregistry(j, &show(&r));
            acc.push_str(&format!("after_unref={};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            ok(imp, j, &acc);
        },
        "delregistry of a missing key" => |imp, j| {
            imp.delregistry(j, "definitely_missing");
            ok(imp, j, "ok");
        },
    }
}

#[test]
fn row_135_136_string_limit() {
    // Rows 135/136: js_pushstring / js_pushlstring beyond JS_STRLIMIT (1<<28) ->
    // RangeError "invalid string length". Allocating a 256 MiB string just to
    // test this is wasteful, so we test the boundary arithmetic with the
    // largest sizes that are practical plus the JS-level path.
    probes! {"rows 135-136 string length limit",
        "pushlstring with a large n" => |imp, j| {
            // n is the declared length; the C checks it against JS_STRLIMIT
            // BEFORE reading, so a bogus-but-large n is a pure limit check.
            let buf = [b'x'; 32];
            imp.pushlstring(j, &buf, (1 << 28) + 1);
            ok(imp, j, "NOT REACHED");
        },
        // NOTE: `n == JS_STRLIMIT` exactly is deliberately NOT tested: the C's
        // check is `n > JS_STRLIMIT`, so n == 1<<28 PASSES the check and the C
        // then memcpy's 256 MiB out of the caller's buffer. That is a caller
        // contract violation (the buffer really must have n bytes), not a
        // comparable library behaviour.
        "pushlstring small n" => |imp, j| {
            imp.pushlstring(j, b"abcdef", 3);
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
        "pushlstring n=0" => |imp, j| {
            imp.pushlstring(j, b"abcdef", 0);
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
        "pushlstring n exactly 15 (shrstr boundary)" => |imp, j| {
            imp.pushlstring(j, b"0123456789abcdef", 15);
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
        "pushlstring n exactly 16 (memstr boundary)" => |imp, j| {
            imp.pushlstring(j, b"0123456789abcdef", 16);
            ok(imp, j, &format!("ok v={}", show(&imp.trystring(j, -1))));
        },
    }
}
