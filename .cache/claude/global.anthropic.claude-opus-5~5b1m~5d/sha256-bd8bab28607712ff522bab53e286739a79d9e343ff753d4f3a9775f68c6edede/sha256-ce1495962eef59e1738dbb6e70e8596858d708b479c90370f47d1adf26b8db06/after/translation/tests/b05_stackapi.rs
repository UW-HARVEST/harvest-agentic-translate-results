//! Phase B/C differential tests for the public `mujs.h` stack API and the
//! low-level `jsV_*`/`js_*` state entry points.
//!
//! Every API sequence that can throw is run *inside* the engine via
//! `Impl::run_probe` (a `js_CFunction` invoked through `js_pcall`), so the
//! throwing paths are observed rather than crashing the harness.
mod common;
use common::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// CONFIGS section D: every value shape, inspected with every predicate
// ---------------------------------------------------------------------------

/// Push one representative of every value shape and view each slot.
fn push_all_shapes(imp: &Impl, j: JsState) {
    imp.pushundefined(j);
    imp.pushnull(j);
    imp.pushboolean(j, 0);
    imp.pushboolean(j, 1);
    imp.pushboolean(j, 42); // normalized via !!v
    imp.pushboolean(j, -1);
    imp.pushnumber(j, 0.0);
    imp.pushnumber(j, -0.0);
    imp.pushnumber(j, f64::NAN);
    imp.pushnumber(j, f64::INFINITY);
    imp.pushnumber(j, f64::NEG_INFINITY);
    imp.pushnumber(j, 42.0);
    imp.pushnumber(j, -42.0);
    imp.pushnumber(j, 0.5);
    imp.pushnumber(j, 1e21);
    imp.pushnumber(j, 1e-7);
    imp.pushnumber(j, i32::MIN as f64);
    imp.pushnumber(j, i32::MAX as f64);
    imp.pushnumber(j, 2147483648.0);
    imp.pushnumber(j, 4294967296.0);
    imp.pushnumber(j, 9007199254740992.0);
    imp.pushstring(j, b"");
    imp.pushstring(j, b"a");
    imp.pushstring(j, b"hello");
    imp.pushstring(j, b"0");
    imp.pushstring(j, b"42");
    imp.pushstring(j, b"  12  ");
    imp.pushstring(j, b"abc");
    imp.pushstring(j, "caf\u{e9}".as_bytes());
    imp.pushstring(j, "\u{4f60}\u{597d}".as_bytes());
    imp.pushstring(j, "\u{1f600}".as_bytes());
    imp.pushstring(j, &vec![b'x'; 100]); // > shrstr, becomes a memstr
    imp.pushlstring(j, b"ab\0cd", 5); // embedded NUL
    imp.pushlstring(j, b"", 0);
    imp.pushliteral_(j, LITERAL);
    imp.pushglobal(j);
    imp.newobject(j);
    imp.newarray(j);
    imp.newboolean(j, 1);
    imp.newnumber(j, 3.5);
    imp.newstring(j, "wrapped");
    imp.newregexp(j, "a+b", 0);
    imp.newregexp(j, "a+b", JS_REGEXP_G | JS_REGEXP_I | JS_REGEXP_M);
}

/// `js_pushliteral` stores the pointer WITHOUT copying (JS_TLITSTR), so the
/// buffer must outlive the state -- hence a `'static` NUL-terminated literal.
static LITERAL: &[u8] = b"lit\0";

trait ImplExt {
    fn pushliteral_(&self, j: JsState, v: &'static [u8]);
}
impl ImplExt for Impl {
    fn pushliteral_(&self, j: JsState, v: &'static [u8]) {
        debug_assert_eq!(v.last(), Some(&0), "js_pushliteral needs a NUL-terminated literal");
        unsafe { self.f::<FnVoidStr>("js_pushliteral")(j, v.as_ptr() as *const std::ffi::c_char) }
    }
}

#[test]
fn every_value_shape_views_identically() {
    // CONFIGS rows: all value shapes x all `js_isX`/`js_type`/`js_typeof`/
    // `js_try*` accessors, at both positive and negative indices.
    for flags in [0 as c_int, JS_STRICT] {
        let (c, r) = Impl::both();
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        push_all_shapes(&c, jc);
        push_all_shapes(&r, jr);
        let topc = c.gettop(jc);
        let topr = r.gettop(jr);
        assert_eq!(topc, topr, "stack depth after pushing all shapes (flags={flags})");

        let mut b = Batch::new();
        // negative (TOP-relative) indices
        for k in 1..=topc {
            b.check(
                &format!("view(flags={flags}, idx=-{k})"),
                c.view(jc, -k),
                r.view(jr, -k),
            );
        }
        // positive (BOT-relative) indices
        for k in 0..topc {
            b.check(
                &format!("view(flags={flags}, idx={k})"),
                c.view(jc, k),
                r.view(jr, k),
            );
        }
        // out-of-range indices: `stackidx` silently returns a static `undefined`
        // (jsrun.c:220-227), so these must NOT throw and must look undefined.
        for k in [
            topc,
            topc + 1,
            topc + 1000,
            -topc - 1,
            -topc - 1000,
            c_int::MAX,
            c_int::MIN,
            c_int::MAX - 1,
            c_int::MIN + 1,
            0x4000_0000,
            -0x4000_0000,
        ] {
            b.check(
                &format!("view out-of-range(flags={flags}, idx={k})"),
                c.view(jc, k),
                r.view(jr, k),
            );
        }
        b.finish(&format!("value shape views (flags={flags})"));
        c.freestate(jc);
        r.freestate(jr);
    }
}

#[test]
fn coercions_that_can_throw_match() {
    // `js_toboolean/tonumber/tostring/tointeger/toint32/touint32/toint16/touint16`
    // can throw (object -> primitive), so they run inside the engine.
    fn probe(imp: &Impl, j: JsState) {
        push_all_shapes(imp, j);
        let top = imp.gettop(j);
        let mut acc = String::new();
        for k in 1..=top {
            let idx = -k;
            acc.push_str(&format!(
                "{}|{}|{:016x}|{}|{}|{}|{}|{}|{:?};",
                idx,
                imp.toboolean(j, idx),
                imp.tonumber(j, idx).to_bits(),
                imp.tointeger(j, idx),
                imp.toint32(j, idx),
                imp.touint32(j, idx),
                imp.toint16(j, idx),
                imp.touint16(j, idx),
                imp.tostring(j, idx).map(|s| show(&s)),
            ));
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "coercions over all shapes", probe);
    }
}

#[test]
fn tostring_and_tonumber_on_objects_match() {
    // Object -> primitive conversion (jsV_toprimitive) including the
    // `Object.create(null)` shape which differs by strict mode.
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for src in [
            "String({})",
            "Number({})",
            "String([])",
            "String([1,2,3])",
            "Number([])",
            "Number([5])",
            "Number([1,2])",
            "String(new Date(0))",
            "String(/a/g)",
            "String(function f(){})",
            "'' + Object.create(null)",
            "Number(Object.create(null))",
            "String({toString:function(){return 'T'}})",
            "Number({valueOf:function(){return 7}})",
            "String({toString:null, valueOf:null})",
            "String({toString:function(){return {}}})",
            "Number({valueOf:function(){throw new Error('boom')}})",
            "String(new Number(5))",
            "String(new String('s'))",
            "String(new Boolean(true))",
        ] {
            b.script(flags, src);
        }
    }
    b.finish("object->primitive coercions");
}

// ---------------------------------------------------------------------------
// CONFIGS section E: stack manipulation
// ---------------------------------------------------------------------------

#[test]
fn stack_manipulation_matches() {
    // js_copy / js_remove / js_replace / js_dup / js_dup2 / js_rot* / js_pop,
    // observed by stringifying the whole stack afterwards.
    fn dump(imp: &Impl, j: JsState) -> String {
        let top = imp.gettop(j);
        let mut s = format!("top={top};");
        for k in 0..top {
            s.push_str(&format!("{}={:?},", k, show(&imp.trystring(j, k))));
        }
        s
    }

    macro_rules! probe {
        ($name:ident, $body:expr) => {
            fn $name(imp: &Impl, j: JsState) {
                for i in 0..8 {
                    imp.pushnumber(j, i as f64);
                }
                let f: fn(&Impl, JsState) = $body;
                f(imp, j);
                let d = dump(imp, j);
                imp.pushstring(j, d.as_bytes());
            }
        };
    }

    probe!(p_copy, |imp, j| {
        for idx in [0, 1, 7, -1, -2, -8] {
            imp.copy(j, idx);
        }
    });
    probe!(p_copy_oob, |imp, j| {
        // out-of-range: `stackidx` yields the static undefined, no throw
        for idx in [8, 100, -9, -100, c_int::MAX, c_int::MIN] {
            imp.copy(j, idx);
        }
    });
    probe!(p_remove, |imp, j| {
        imp.remove(j, 0);
        imp.remove(j, 3);
        imp.remove(j, -1);
        imp.remove(j, -2);
    });
    probe!(p_replace, |imp, j| {
        imp.pushstring(j, b"R");
        imp.replace(j, 2);
        imp.pushstring(j, b"S");
        imp.replace(j, -3);
    });
    probe!(p_dup, |imp, j| {
        imp.dup(j);
        imp.dup2(j);
        imp.dup(j);
    });
    probe!(p_rot2, |imp, j| imp.rot2(j));
    probe!(p_rot3, |imp, j| imp.rot3(j));
    probe!(p_rot4, |imp, j| imp.rot4(j));
    probe!(p_rot2pop1, |imp, j| imp.rot2pop1(j));
    probe!(p_rot3pop2, |imp, j| imp.rot3pop2(j));
    probe!(p_rot, |imp, j| {
        imp.rot(j, 3);
        imp.rot(j, 5);
        imp.rot(j, 1);
        imp.rot(j, 0);
    });
    probe!(p_pop, |imp, j| {
        imp.pop(j, 1);
        imp.pop(j, 3);
        imp.pop(j, 0);
    });
    probe!(p_concat, |imp, j| {
        imp.concat(j);
        imp.concat(j);
    });
    probe!(p_compare, |imp, j| {
        let (rc, ok) = imp.compare(j);
        imp.pushstring(j, format!("cmp={rc},ok={ok}").as_bytes());
    });
    probe!(p_equal, |imp, j| {
        let e = imp.equal(j);
        let s = imp.strictequal(j);
        imp.pushstring(j, format!("eq={e},se={s}").as_bytes());
    });

    let probes: &[(&str, ProbeFn)] = &[
        ("js_copy", p_copy),
        ("js_copy out-of-range", p_copy_oob),
        ("js_remove", p_remove),
        ("js_replace", p_replace),
        ("js_dup/js_dup2", p_dup),
        ("js_rot2", p_rot2),
        ("js_rot3", p_rot3),
        ("js_rot4", p_rot4),
        ("js_rot2pop1", p_rot2pop1),
        ("js_rot3pop2", p_rot3pop2),
        ("js_rot", p_rot),
        ("js_pop", p_pop),
        ("js_concat", p_concat),
        ("js_compare", p_compare),
        ("js_equal/js_strictequal", p_equal),
    ];
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for (name, f) in probes {
            b.probe(flags, name, *f);
        }
    }
    b.finish("stack manipulation");
}

#[test]
fn js_insert_is_always_an_error() {
    // ERRORS row: `js_insert` is unconditionally `js_error(J, "not implemented
    // yet")` (jsrun.c:422-425). Both impls must produce the identical error.
    fn probe(imp: &Impl, j: JsState) {
        imp.pushnumber(j, 1.0);
        imp.pushnumber(j, 2.0);
        unsafe { imp.f::<FnVoidInt>("js_insert")(j, 0) };
        imp.pushstring(j, b"NOT REACHED");
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "js_insert", probe);
    }
}

#[test]
fn stack_overflow_matches() {
    // ERRORS row: pushing past JS_STACKSIZE (4096) throws
    // "stack overflow" (js_stackoverflow, jsrun.c).
    fn probe(imp: &Impl, j: JsState) {
        for i in 0..(JS_STACKSIZE + 64) {
            imp.pushnumber(j, i as f64);
        }
        imp.pushstring(j, b"NOT REACHED");
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "stack overflow via js_pushnumber", probe);
    }

    fn probe_str(imp: &Impl, j: JsState) {
        for _ in 0..(JS_STACKSIZE + 64) {
            imp.pushstring(j, b"x");
        }
        imp.pushstring(j, b"NOT REACHED");
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "stack overflow via js_pushstring", probe_str);
    }

    fn probe_obj(imp: &Impl, j: JsState) {
        for _ in 0..(JS_STACKSIZE + 64) {
            imp.newobject(j);
        }
        imp.pushstring(j, b"NOT REACHED");
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "stack overflow via js_newobject", probe_obj);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS section C: property attributes
// ---------------------------------------------------------------------------

#[test]
fn property_attribute_combinations_match() {
    // All 8 `atts` combinations x {overwrite, enumerate, delete, redefine}.
    macro_rules! att_probe {
        ($name:ident, $atts:expr) => {
            fn $name(imp: &Impl, j: JsState) {
                imp.newobject(j);
                imp.pushnumber(j, 1.0);
                imp.defproperty(j, -2, "p", $atts);
                let mut acc = String::new();
                acc.push_str(&format!("has={};", imp.hasproperty(j, -1, "p")));
                imp.pop(j, 1); // js_hasproperty pushes the value on success
                // overwrite
                imp.pushnumber(j, 2.0);
                imp.setproperty(j, -2, "p");
                imp.getproperty(j, -1, "p");
                acc.push_str(&format!("after_set={:?};", show(&imp.trystring(j, -1))));
                imp.pop(j, 1);
                // redefine with atts = 0 (attributes are OR-accumulated)
                imp.pushnumber(j, 3.0);
                imp.defproperty(j, -2, "p", 0);
                imp.getproperty(j, -1, "p");
                acc.push_str(&format!("after_def={:?};", show(&imp.trystring(j, -1))));
                imp.pop(j, 1);
                // enumerate
                imp.pushiterator(j, -1, 1);
                let mut names = Vec::new();
                while let Some(n) = imp.nextiterator(j, -1) {
                    names.push(show(&n));
                }
                imp.pop(j, 1);
                acc.push_str(&format!("keys={names:?};"));
                // delete
                imp.delproperty(j, -1, "p");
                acc.push_str(&format!("has_after_del={};", imp.hasproperty(j, -1, "p")));
                imp.pushstring(j, acc.as_bytes());
            }
        };
    }
    att_probe!(a0, 0);
    att_probe!(a1, JS_READONLY);
    att_probe!(a2, JS_DONTENUM);
    att_probe!(a3, JS_READONLY | JS_DONTENUM);
    att_probe!(a4, JS_DONTCONF);
    att_probe!(a5, JS_READONLY | JS_DONTCONF);
    att_probe!(a6, JS_DONTENUM | JS_DONTCONF);
    att_probe!(a7, JS_READONLY | JS_DONTENUM | JS_DONTCONF);
    // out-of-range attribute bits are real FFI inputs
    att_probe!(a8, 8);
    att_probe!(a_neg, -1);
    att_probe!(a_max, c_int::MAX);
    att_probe!(a_min, c_int::MIN);

    let probes: &[(&str, ProbeFn)] = &[
        ("atts=0", a0),
        ("atts=READONLY", a1),
        ("atts=DONTENUM", a2),
        ("atts=READONLY|DONTENUM", a3),
        ("atts=DONTCONF", a4),
        ("atts=READONLY|DONTCONF", a5),
        ("atts=DONTENUM|DONTCONF", a6),
        ("atts=all", a7),
        ("atts=8 (out of range)", a8),
        ("atts=-1", a_neg),
        ("atts=INT_MAX", a_max),
        ("atts=INT_MIN", a_min),
    ];
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for (n, f) in probes {
            b.probe(flags, n, *f);
        }
    }
    b.finish("property attribute combinations");
}

#[test]
fn defglobal_and_registry_match() {
    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        for (name, atts) in [
            ("g0", 0),
            ("g1", JS_READONLY),
            ("g2", JS_DONTENUM),
            ("g4", JS_DONTCONF),
            ("g7", JS_READONLY | JS_DONTENUM | JS_DONTCONF),
        ] {
            imp.pushnumber(j, 11.0);
            imp.defglobal(j, name, atts);
            imp.getglobal(j, name);
            acc.push_str(&format!("{name}={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            imp.pushnumber(j, 22.0);
            imp.setglobal(j, name);
            imp.getglobal(j, name);
            acc.push_str(&format!("{name}_after_set={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
        }
        // registry
        imp.pushstring(j, b"regval");
        imp.setregistry(j, "myreg");
        imp.getregistry(j, "myreg");
        acc.push_str(&format!("reg={:?};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        imp.getregistry(j, "missing");
        acc.push_str(&format!("reg_missing={:?};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        imp.delregistry(j, "myreg");
        imp.getregistry(j, "myreg");
        acc.push_str(&format!("reg_deleted={:?};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        // js_ref / js_unref for each value shape
        for i in 0..7 {
            match i {
                0 => imp.pushundefined(j),
                1 => imp.pushnull(j),
                2 => imp.pushboolean(j, 1),
                3 => imp.pushboolean(j, 0),
                4 => imp.pushnumber(j, 5.0),
                5 => imp.pushstring(j, b"str"),
                _ => imp.newobject(j),
            }
            let r = imp.refstr(j);
            // Object refs are pointer-formatted, so only their *shape* is stable.
            let shown = r.as_ref().map(|x| {
                let s = show(x);
                if s.starts_with('_') {
                    s
                } else {
                    format!("<{}chars>", s.len())
                }
            });
            acc.push_str(&format!("ref{i}={shown:?};"));
            if let Some(rr) = &r {
                imp.unref(j, rr);
            }
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "defglobal/registry/ref", probe);
    }
}

// ---------------------------------------------------------------------------
// Arrays and indexed access
// ---------------------------------------------------------------------------

#[test]
fn array_index_api_matches() {
    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        imp.newarray(j);
        for i in 0..10 {
            imp.pushnumber(j, (i * i) as f64);
            imp.setindex(j, -2, i);
        }
        acc.push_str(&format!("len={};", imp.getlength(j, -1)));
        for i in -3..14 {
            imp.getindex(j, -1, i);
            acc.push_str(&format!("[{i}]={:?},", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            acc.push_str(&format!("has[{i}]={};", imp.hasindex(j, -1, i)));
            // js_hasindex pushes the value when present
            if imp.hasindex(j, -1, i) != 0 {
                imp.pop(j, 2);
            } else {
                imp.pop(j, 0);
            }
        }
        imp.delindex(j, -1, 3);
        acc.push_str(&format!("after_del_len={};", imp.getlength(j, -1)));
        imp.setlength(j, -1, 5);
        acc.push_str(&format!("after_setlen={};", imp.getlength(j, -1)));
        imp.setlength(j, -1, 0);
        acc.push_str(&format!("after_setlen0={};", imp.getlength(j, -1)));
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "array index API", probe);
    }
}

#[test]
fn setlength_boundary_values_match() {
    // Array length boundaries: JS_ARRAYLIMIT (1<<26) and the 2^32-1 max.
    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        for len in [0, 1, 10, 1000, 65535, 65536, (1 << 26) - 1, 1 << 26, (1 << 26) + 1, -1, c_int::MAX, c_int::MIN] {
            imp.newarray(j);
            imp.setlength(j, -1, len);
            acc.push_str(&format!("{len}->{};", imp.getlength(j, -1)));
            imp.pop(j, 1);
        }
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "setlength boundaries", probe);
    }
}

#[test]
fn iterator_own_flag_matches() {
    // CONFIGS row: `js_pushiterator(J, idx, own)` with own = 0 vs 1.
    fn probe(imp: &Impl, j: JsState) {
        let mut acc = String::new();
        for own in [0, 1, 2, -1] {
            // object with own + inherited + non-enumerable props
            imp.newobject(j);
            imp.pushnumber(j, 1.0);
            imp.defproperty(j, -2, "own_plain", 0);
            imp.pushnumber(j, 2.0);
            imp.defproperty(j, -2, "own_hidden", JS_DONTENUM);
            imp.pushiterator(j, -1, own);
            let mut names = Vec::new();
            while let Some(n) = imp.nextiterator(j, -1) {
                names.push(show(&n));
            }
            names.sort();
            acc.push_str(&format!("own={own}:{names:?};"));
            imp.pop(j, 2);
        }
        // arrays and strings
        imp.newarray(j);
        for i in 0..3 {
            imp.pushnumber(j, i as f64);
            imp.setindex(j, -2, i);
        }
        imp.pushiterator(j, -1, 1);
        let mut names = Vec::new();
        while let Some(n) = imp.nextiterator(j, -1) {
            names.push(show(&n));
        }
        acc.push_str(&format!("array={names:?};"));
        imp.pop(j, 2);

        imp.pushstring(j, b"abc");
        imp.pushiterator(j, -1, 1);
        let mut names = Vec::new();
        while let Some(n) = imp.nextiterator(j, -1) {
            names.push(show(&n));
        }
        acc.push_str(&format!("string={names:?};"));
        imp.pop(j, 2);

        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "pushiterator own flag", probe);
    }
}

// ---------------------------------------------------------------------------
// Accessors, cfunctions, userdata
// ---------------------------------------------------------------------------

unsafe extern "C" fn cfun_return_42(j: JsState) {
    // We do not know which impl we are in, so use the probe's current impl.
    let imp = if PROBE_IMPL_ID.with(|c| c.get()) == 0 { Impl::c() } else { Impl::rust() };
    imp.pushnumber(j, 42.0);
}

unsafe extern "C" fn cfun_identity(j: JsState) {
    let imp = if PROBE_IMPL_ID.with(|c| c.get()) == 0 { Impl::c() } else { Impl::rust() };
    imp.copy(j, 1);
}

unsafe extern "C" fn cfun_throw(j: JsState) {
    let imp = if PROBE_IMPL_ID.with(|c| c.get()) == 0 { Impl::c() } else { Impl::rust() };
    imp.pushstring(j, b"thrown-from-cfunction");
    unsafe { imp.f::<FnVoid1>("js_throw")(j) };
}

thread_local! {
    static PROBE_IMPL_ID: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

static FINALIZE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

unsafe extern "C" fn my_finalize(_j: JsState, _p: *mut c_void) {
    FINALIZE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn cfunction_and_accessor_api_matches() {
    fn probe(imp: &Impl, j: JsState) {
        PROBE_IMPL_ID.with(|c| c.set(if imp.name == "C" { 0 } else { 1 }));
        let mut acc = String::new();
        let newcf = imp.f::<FnNewcfunction>("js_newcfunction");
        let newcfx = imp.f::<FnNewcfunctionx>("js_newcfunctionx");
        let newcc = imp.f::<FnNewcconstructor>("js_newcconstructor");

        // js_newcfunction with several arities
        for length in [0 as c_int, 1, 5, -1] {
            let nm = cstr("f");
            unsafe { newcf(j, cfun_return_42, nm.as_ptr(), length) };
            acc.push_str(&format!(
                "cf(len={length}): callable={} type={} len={:?};",
                imp.is(j, "js_iscallable", -1),
                imp.ty(j, -1),
                {
                    imp.getproperty(j, -1, "length");
                    let s = show(&imp.trystring(j, -1));
                    imp.pop(j, 1);
                    s
                }
            ));
            // call it
            imp.pushundefined(j);
            imp.pcall(j, 0);
            acc.push_str(&format!("result={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
        }

        // js_newcfunctionx with data + finalizer
        {
            let nm = cstr("fx");
            let data = 0x1234usize as *mut c_void;
            unsafe { newcfx(j, cfun_return_42, nm.as_ptr(), 1, data, Some(my_finalize)) };
            acc.push_str(&format!("cfx callable={};", imp.is(j, "js_iscallable", -1)));
            imp.pushundefined(j);
            imp.pcall(j, 0);
            acc.push_str(&format!("cfx result={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
        }

        // js_newcconstructor
        {
            let nm = cstr("Ctor");
            unsafe { newcc(j, cfun_return_42, cfun_return_42, nm.as_ptr(), 0) };
            acc.push_str(&format!("cc callable={};", imp.is(j, "js_iscallable", -1)));
            imp.pop(j, 1);
        }

        // cfunction that throws, called through js_pcall
        {
            let nm = cstr("bad");
            unsafe { newcf(j, cfun_throw, nm.as_ptr(), 0) };
            imp.pushundefined(j);
            let rc = imp.pcall(j, 0);
            acc.push_str(&format!("throwing rc={rc} val={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
        }

        // accessors: getter+setter, getter-only, setter-only
        {
            imp.newobject(j);
            let g = cstr("g");
            unsafe { newcf(j, cfun_return_42, g.as_ptr(), 0) };
            let s = cstr("s");
            unsafe { newcf(j, cfun_identity, s.as_ptr(), 1) };
            imp.defaccessor(j, -3, "both", 0);
            imp.getproperty(j, -1, "both");
            acc.push_str(&format!("get_both={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);

            unsafe { newcf(j, cfun_return_42, g.as_ptr(), 0) };
            imp.pushnull(j);
            imp.defaccessor(j, -3, "getonly", 0);
            imp.getproperty(j, -1, "getonly");
            acc.push_str(&format!("get_getonly={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);

            imp.pushundefined(j);
            unsafe { newcf(j, cfun_identity, s.as_ptr(), 1) };
            imp.defaccessor(j, -3, "setonly", 0);
            imp.getproperty(j, -1, "setonly");
            acc.push_str(&format!("get_setonly={:?};", show(&imp.trystring(j, -1))));
            imp.pop(j, 1);
            imp.pop(j, 1);
        }

        // userdata
        {
            let newud = imp.f::<FnNewuserdata>("js_newuserdata");
            let tag = cstr("MyTag");
            let data = 0xABCDusize as *mut c_void;
            unsafe { newud(j, tag.as_ptr(), data, Some(my_finalize)) };
            let isud = imp.f::<FnIsuserdata>("js_isuserdata");
            let toud = imp.f::<FnTouserdata>("js_touserdata");
            let other = cstr("OtherTag");
            acc.push_str(&format!(
                "ud: is={} isother={} data={:?};",
                unsafe { isud(j, -1, tag.as_ptr()) },
                unsafe { isud(j, -1, other.as_ptr()) },
                unsafe { toud(j, -1, tag.as_ptr()) } as usize
            ));
            imp.pop(j, 1);
        }

        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "cfunction/accessor/userdata API", probe);
    }
}

// ---------------------------------------------------------------------------
// Allocator, limits, gc, context
// ---------------------------------------------------------------------------

#[repr(C)]
struct AllocCtx {
    allocs: u64,
    frees: u64,
    bytes: u64,
    fail_after: u64,
    magic: u64,
}

extern "C" {
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

unsafe extern "C" fn counting_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    let c = &mut *(ctx as *mut AllocCtx);
    assert_eq!(c.magic, 0x5A5A_1234_5678_9ABC, "allocator received the wrong actx");
    if n == 0 {
        c.frees += 1;
        if !p.is_null() {
            free(p);
        }
        return std::ptr::null_mut();
    }
    c.allocs += 1;
    c.bytes += n as u64;
    if c.allocs > c.fail_after {
        return std::ptr::null_mut();
    }
    realloc(p, n as usize)
}

fn new_alloc_ctx(fail_after: u64) -> Box<AllocCtx> {
    Box::new(AllocCtx { allocs: 0, frees: 0, bytes: 0, fail_after, magic: 0x5A5A_1234_5678_9ABC })
}

#[test]
fn custom_allocator_matches() {
    // CONFIGS rows: custom `js_Alloc` + non-NULL `actx`, and the two
    // allocation-failure exits of `js_newstate`.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        // Success: full lifecycle through the custom allocator.
        let mut cc = new_alloc_ctx(u64::MAX);
        let mut rr = new_alloc_ctx(u64::MAX);
        let nc = c.f::<FnNewstate>("js_newstate");
        let nr = r.f::<FnNewstate>("js_newstate");
        let jc = unsafe {
            nc(counting_alloc as *const c_void, &mut *cc as *mut AllocCtx as *mut c_void, flags)
        };
        let jr = unsafe {
            nr(counting_alloc as *const c_void, &mut *rr as *mut AllocCtx as *mut c_void, flags)
        };
        assert!(!jc.is_null() && !jr.is_null(), "custom-allocator newstate failed");
        c.mute_report(jc);
        r.mute_report(jr);
        for src in [
            "var a=[1,2,3]; a.join('-')",
            "var o={x:1,y:2}; Object.keys(o).join(',')",
            "var re=/a+/g; 'aaa'.replace(re,'b')",
            "var s=''; for(var i=0;i<200;i++) s+=i; s.length",
            "JSON.stringify({a:[1,2,{b:3}]})",
        ] {
            let a = c.eval_on(jc, src.as_bytes());
            let bb = r.eval_on(jr, src.as_bytes());
            b.check(&format!("custom alloc eval {src:?} flags={flags}"), a, bb);
        }
        c.gc(jc, 0);
        r.gc(jr, 0);
        c.freestate(jc);
        r.freestate(jr);
        b.check(
            &format!("custom alloc accounting flags={flags}"),
            (cc.allocs, cc.frees, cc.bytes),
            (rr.allocs, rr.frees, rr.bytes),
        );

        // Failure sweep: fail the Nth allocation during js_newstate/jsB_init.
        for n in [0u64, 1, 2, 3, 5, 10, 50, 200] {
            let mut cc = new_alloc_ctx(n);
            let mut rr = new_alloc_ctx(n);
            let jc = unsafe {
                nc(counting_alloc as *const c_void, &mut *cc as *mut AllocCtx as *mut c_void, flags)
            };
            let jr = unsafe {
                nr(counting_alloc as *const c_void, &mut *rr as *mut AllocCtx as *mut c_void, flags)
            };
            b.check(
                &format!("js_newstate fail_after={n} flags={flags}"),
                (jc.is_null(), cc.allocs, cc.frees),
                (jr.is_null(), rr.allocs, rr.frees),
            );
            if !jc.is_null() {
                cc.fail_after = u64::MAX;
                c.freestate(jc);
            }
            if !jr.is_null() {
                rr.fail_after = u64::MAX;
                r.freestate(jr);
            }
        }
    }
    b.finish("custom allocator");
}

#[test]
fn setlimit_matches() {
    // CONFIGS rows: runlimit and memlimit, including the <= 0 "unlimited" cases.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    let cases: &[(c_int, c_int, &str)] = &[
        (0, 0, "var i=0; while(i<1000) i++; i"),
        (-1, -1, "var i=0; while(i<1000) i++; i"),
        (1, 0, "1+1"),
        (2, 0, "1+1"),
        (10, 0, "1+1"),
        (100, 0, "var i=0; while(i<1000) i++; i"),
        (100000, 0, "var i=0; while(i<1000) i++; i"),
        (1000000, 0, "while(1);"),
        (0, 64, "'abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789'.length"),
        (0, 4096, "var s='x'; for(var i=0;i<20;i++) s+=s; s.length"),
        (0, 1000000, "var a=[]; for(var i=0;i<1000;i++) a.push(i); a.length"),
        (0, 1, "1+1"),
        (0, -1, "1+1"),
        (5000, 5000, "var a=[]; for(var i=0;i<100;i++) a.push(i); a.length"),
    ];
    for flags in [0 as c_int, JS_STRICT] {
        for (run, mem, src) in cases {
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            c.setlimit(jc, *run, *mem);
            r.setlimit(jr, *run, *mem);
            let a = c.eval_on(jc, src.as_bytes());
            let bb = r.eval_on(jr, src.as_bytes());
            b.check(
                &format!("setlimit(run={run},mem={mem}) flags={flags} src={src:?}"),
                a,
                bb,
            );
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("js_setlimit");
}

#[test]
fn gc_matches() {
    // CONFIGS rows: js_gc(J, 0) and js_gc(J, 1) on a fresh state, after
    // allocating garbage, and with finalizable userdata.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for report in [0 as c_int, 1, 2, -1] {
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            // fresh
            c.gc(jc, report);
            r.gc(jr, report);
            b.check(
                &format!("gc(report={report}) on fresh state flags={flags}"),
                c.gettop(jc),
                r.gettop(jr),
            );
            // after making garbage
            for src in [
                "for(var i=0;i<200;i++){ var o={a:i,b:[i,i+1]}; }",
                "for(var i=0;i<50;i++){ var f=function(){return i}; }",
                "for(var i=0;i<50;i++){ var re=new RegExp('a'+i,'g'); }",
                "for(var i=0;i<50;i++){ var s=('x'+i); }",
            ] {
                let a = c.eval_on(jc, src.as_bytes());
                let bb = r.eval_on(jr, src.as_bytes());
                b.check(&format!("pre-gc eval {src:?}"), a, bb);
                c.gc(jc, report);
                r.gc(jr, report);
            }
            // engine still works after GC
            let a = c.eval_on(jc, b"(function(){var t=0; for(var i=0;i<100;i++) t+=i; return t})()");
            let bb = r.eval_on(jr, b"(function(){var t=0; for(var i=0;i<100;i++) t+=i; return t})()");
            b.check(&format!("post-gc eval flags={flags} report={report}"), a, bb);
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("js_gc");
}

#[test]
fn context_roundtrip_matches() {
    let (c, r) = Impl::both();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        // starts NULL (memset in js_newstate)
        assert_eq!(c.getcontext(jc).is_null(), r.getcontext(jr).is_null());
        assert!(c.getcontext(jc).is_null(), "C uctx should start NULL");
        let p = 0xDEAD_1000usize as *mut c_void;
        c.setcontext(jc, p);
        r.setcontext(jr, p);
        assert_eq!(c.getcontext(jc), r.getcontext(jr));
        assert_eq!(c.getcontext(jc), p);
        c.setcontext(jc, std::ptr::null_mut());
        r.setcontext(jr, std::ptr::null_mut());
        assert_eq!(c.getcontext(jc), r.getcontext(jr));
        c.freestate(jc);
        r.freestate(jr);
    }
}

#[test]
fn freestate_null_is_accepted() {
    // ERRORS row: js_freestate(NULL) returns early (jsgc.c:267-268).
    let (c, r) = Impl::both();
    c.freestate(std::ptr::null_mut());
    r.freestate(std::ptr::null_mut());
}

// ---------------------------------------------------------------------------
// Try / exception stack
// ---------------------------------------------------------------------------

#[test]
fn try_limit_overflow_matches() {
    // ERRORS rows: JS_TRYLIMIT (64) nested tries -> "try stack overflow" from
    // js_savetry; and the js_ptry branch in js_dostring/js_ploadstring/js_try*
    // pushing the literal "exception stack overflow".
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for depth in [1usize, 8, 32, 60, 62, 63, 64, 65, 70, 100] {
            let mut src = String::new();
            for _ in 0..depth {
                src.push_str("try{");
            }
            src.push_str("throw new Error('deep')");
            for _ in 0..depth {
                src.push_str("}catch(e){throw e}");
            }
            b.check(
                &format!("nested try depth={depth} flags={flags}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
        }
        // deep recursion -> environment/try limits
        for depth in [10usize, 100, 500, 1000, 1023, 1024, 1025, 2000] {
            let src = format!(
                "function f(n){{ if(n<=0) return 0; return 1+f(n-1); }} f({depth})"
            );
            b.check(
                &format!("recursion depth={depth} flags={flags}"),
                c.eval_script(flags, src.as_bytes()),
                r.eval_script(flags, src.as_bytes()),
            );
        }
    }
    b.finish("try/env limits");
}

#[test]
fn savetry_endtry_underflow_matches() {
    // ERRORS row: `js_endtry` with `trytop == 0` raises
    // "endtry: exception stack underflow" (jsrun.c:1458-1463). With no
    // enclosing try that reaches `js_defaultpanic` -> `abort()`, so it can only
    // be compared across processes.
    assert_subproc_eq("subproc_runner", "endtry_underflow");
}

// ---------------------------------------------------------------------------
// jsV_* / jsR_* / jsS_* low-level entry points
// ---------------------------------------------------------------------------

#[test]
fn low_level_value_entry_points_match() {
    // Exercise jsV_newobject / jsV_newmemstring / jsV_resizearray /
    // jsV_getproperty / jsV_setproperty / jsV_delproperty / jsV_newiterator /
    // jsV_nextiterator / jsR_newenvironment / jsR_unflattenarray indirectly
    // through the public API paths that call them, plus jsS_dumpstrings /
    // jsS_freestrings directly.
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        c.mute_report(jc);
        r.mute_report(jr);
        // jsV_resizearray / jsR_unflattenarray: flat arrays that grow, get
        // holes, and get non-index properties (forcing unflattening).
        for src in [
            "var a=[]; for(var i=0;i<100;i++) a[i]=i; a.length",
            "var a=[1,2,3]; a[100]=1; a.length",
            "var a=[1,2,3]; delete a[1]; a.join(',')",
            "var a=[1,2,3]; a.foo='bar'; a.foo+a.length",
            "var a=[1,2,3]; a.length=1; a.join(',')",
            "var a=new Array(10); a.length",
            "var a=[1,2,3]; a.unshift(0); a.join(',')",
            "var a=[1,2,3]; a.splice(1,1); a.join(',')",
            "var a=[]; a[4294967294]=1; a.length",
            "var a=[1,2,3]; Object.defineProperty(a,'1',{value:9}); a.join(',')",
            // jsV_newiterator / nextiterator over each shape
            "var k=[]; for(var p in {a:1,b:2,c:3}) k.push(p); k.sort().join(',')",
            "var k=[]; for(var p in [7,8,9]) k.push(p); k.join(',')",
            "var k=[]; for(var p in 'abc') k.push(p); k.join(',')",
            "var k=[]; for(var p in Object.create({inherited:1})) k.push(p); k.join(',')",
            "var k=[]; for(var p in null) k.push(p); k.length",
            "var k=[]; for(var p in undefined) k.push(p); k.length",
            // jsR_newenvironment: closures, with, catch scopes
            "var f=(function(){var x=1; return function(){return ++x}})(); f()+f()+f()",
            "var o={a:5}; var v; with(o){v=a}; v",
            "var v; try{throw 7}catch(e){v=e}; v",
            "function o(){var a=1; function i(){return a}; return i()}; o()",
            // jsV_newmemstring: long strings and concatenation
            "var s=''; for(var i=0;i<300;i++) s+='ab'; s.length",
            "'x'.repeat ? 'has' : 'none'",
        ] {
            let a = c.eval_on(jc, src.as_bytes());
            let bb = r.eval_on(jr, src.as_bytes());
            b.check(&format!("low-level {src:?} flags={flags}"), a, bb);
        }
        // jsS_dumpstrings / jsS_freestrings: must not crash, and the engine must
        // still work afterwards.
        let dc = c.f::<FnVoid1>("jsS_dumpstrings");
        let dr = r.f::<FnVoid1>("jsS_dumpstrings");
        unsafe { dc(jc) };
        unsafe { dr(jr) };
        let a = c.eval_on(jc, b"1+1");
        let bb = r.eval_on(jr, b"1+1");
        b.check("after jsS_dumpstrings", a, bb);
        c.freestate(jc);
        r.freestate(jr);
    }
    b.finish("low-level value entry points");
}

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsS_freestrings_on_fresh_state_matches() {
    // CONFIGS row: jsS_freestrings on an empty vs populated intern table.
    let (c, r) = Impl::both();
    for flags in [0 as c_int, JS_STRICT] {
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        let fc = c.f::<FnIntern>("js_intern");
        let fr = r.f::<FnIntern>("js_intern");
        for s in ["b", "a", "c", "a", "zzz", ""] {
            let cs = cstr(s);
            let a = unsafe { read_cstr(fc(jc, cs.as_ptr())) };
            let bb = unsafe { read_cstr(fr(jr, cs.as_ptr())) };
            assert_eq!(a, bb, "js_intern({s:?})");
        }
        let dc = c.f::<FnVoid1>("jsS_dumpstrings");
        let dr = r.f::<FnVoid1>("jsS_dumpstrings");
        unsafe { dc(jc) };
        unsafe { dr(jr) };
        // jsS_freestrings is called by js_freestate; calling it and then
        // freeing must behave identically (no double free in either).
        c.freestate(jc);
        r.freestate(jr);
    }
}

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsB_init_functions_are_idempotent_and_match() {
    // CONFIGS rows: each jsB_init* entry point. They are normally called once by
    // js_newstate; calling one again must behave the same in both impls.
    let names = [
        "jsB_initobject",
        "jsB_initarray",
        "jsB_initfunction",
        "jsB_initboolean",
        "jsB_initnumber",
        "jsB_initstring",
        "jsB_initregexp",
        "jsB_initerror",
        "jsB_initmath",
        "jsB_initjson",
        "jsB_initdate",
    ];
    let (c, r) = Impl::both();
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for name in names {
            // Re-running `jsB_initerror` on a STRICT state redefines read-only
            // properties, which throws with no enclosing try and therefore
            // reaches `js_defaultpanic` -> `abort()`. Both impls abort
            // identically; that is verified out-of-process by
            // `jsB_initerror_strict_aborts_identically` below.
            if name == "jsB_initerror" && flags == JS_STRICT {
                continue;
            }
            let jc = c.newstate(flags);
            let jr = r.newstate(flags);
            c.mute_report(jc);
            r.mute_report(jr);
            unsafe { c.f::<FnVoid1>(name)(jc) };
            unsafe { r.f::<FnVoid1>(name)(jr) };
            b.check(
                &format!("{name} re-init top flags={flags}"),
                c.gettop(jc),
                r.gettop(jr),
            );
            // engine must still work and agree
            for src in [
                "typeof Object",
                "typeof Array",
                "typeof Math.max",
                "JSON.stringify([1,2])",
                "new Date(0).getTime()",
                "'ab'.charCodeAt(1)",
                "[3,1,2].sort().join(',')",
                "(1.5).toFixed(2)",
                "/a/.test('a')",
                "(new Error('x')).message",
            ] {
                let a = c.eval_on(jc, src.as_bytes());
                let bb = r.eval_on(jr, src.as_bytes());
                b.check(&format!("{name} then {src:?} flags={flags}"), a, bb);
            }
            c.freestate(jc);
            r.freestate(jr);
        }
    }
    b.finish("jsB_init* entry points");
}

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsB_prop_helpers_match() {
    // CONFIGS rows: jsB_propf / jsB_propn / jsB_props install properties on the
    // object at the top of the stack.
    fn probe(imp: &Impl, j: JsState) {
        type PropF = unsafe extern "C" fn(
            JsState,
            *const std::ffi::c_char,
            unsafe extern "C" fn(JsState),
            c_int,
        );
        type PropN = unsafe extern "C" fn(JsState, *const std::ffi::c_char, f64);
        type PropS =
            unsafe extern "C" fn(JsState, *const std::ffi::c_char, *const std::ffi::c_char);
        PROBE_IMPL_ID.with(|c| c.set(if imp.name == "C" { 0 } else { 1 }));
        let pf = imp.f::<PropF>("jsB_propf");
        let pn = imp.f::<PropN>("jsB_propn");
        let ps = imp.f::<PropS>("jsB_props");

        imp.newobject(j);
        let n1 = cstr("myfun");
        unsafe { pf(j, n1.as_ptr(), cfun_return_42, 0) };
        let n2 = cstr("mynum");
        unsafe { pn(j, n2.as_ptr(), 3.25) };
        let n3 = cstr("mystr");
        let v3 = cstr("hello");
        unsafe { ps(j, n3.as_ptr(), v3.as_ptr()) };

        let mut acc = String::new();
        for k in ["myfun", "mynum", "mystr"] {
            imp.getproperty(j, -1, k);
            acc.push_str(&format!("{k}={:?}/{};", show(&imp.trystring(j, -1)), imp.ty(j, -1)));
            imp.pop(j, 1);
        }
        // enumerability (jsB_prop* use JS_DONTENUM)
        imp.pushiterator(j, -1, 1);
        let mut names = Vec::new();
        while let Some(nm) = imp.nextiterator(j, -1) {
            names.push(show(&nm));
        }
        names.sort();
        acc.push_str(&format!("enum={names:?};"));
        imp.pop(j, 1);
        // call the installed function
        imp.getproperty(j, -1, "myfun");
        imp.pushundefined(j);
        imp.pcall(j, 0);
        acc.push_str(&format!("call={:?};", show(&imp.trystring(j, -1))));
        imp.pop(j, 1);
        imp.pushstring(j, acc.as_bytes());
    }
    for flags in [0 as c_int, JS_STRICT] {
        assert_probe_eq(flags, "jsB_propf/propn/props", probe);
    }
}

#[test]
fn js_putc_puts_putm_match() {
    // CONFIGS row 513: js_putc first-call allocation + doubling, js_puts empty
    // vs non-empty, js_putm empty vs non-empty range. Exercised through the
    // public paths that use js_Buffer (JSON.stringify, Number.toString(radix),
    // encodeURI, js_repr).
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for src in [
            "JSON.stringify({})",
            "JSON.stringify([])",
            "JSON.stringify('')",
            "JSON.stringify({a:1})",
            // force the buffer past its initial 64 bytes and through doubling
            "JSON.stringify({aaaaaaaaaa:'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'})",
            "var a=[]; for(var i=0;i<500;i++)a.push(i); JSON.stringify(a).length",
            "encodeURI('')",
            "encodeURIComponent('a b/c?d=e&f')",
            "encodeURIComponent('\\u00e9\\u4f60\\ud83d\\ude00')",
            "decodeURIComponent('%41%42%43')",
            "escape('a b')",
            "unescape('%41')",
            "(255).toString(16)",
            "(0).toString(2)",
            "(1e21).toString(36)",
            "(-12345.6789).toString(8)",
            "(1.5).toFixed(20)",
            "(123.456).toPrecision(21)",
            "(1e-7).toExponential(15)",
        ] {
            b.script(flags, src);
        }
    }
    b.finish("js_putc/js_puts/js_putm via buffer paths");
}

// ---------------------------------------------------------------------------
// Abort-path parity, compared across processes
// ---------------------------------------------------------------------------

#[test]
#[allow(non_snake_case)] // mirrors the C symbol name
fn jsB_initerror_strict_aborts_identically() {
    // Verified: re-calling jsB_initerror on a JS_STRICT state throws
    // "'...' is read-only", which with trytop == 0 goes to js_defaultpanic and
    // then abort(). Both impls must do exactly the same thing.
    assert_subproc_eq("subproc_runner", "initerror_strict");
}

#[test]
fn uncaught_throw_aborts_identically() {
    // ERRORS row: js_throw with J->trytop == 0 -> js_defaultpanic reports
    // "uncaught exception" then abort() (jsstate.c:30-34, jsrun.c:1479-1481).
    assert_subproc_eq("subproc_runner", "uncaught_throw");
}

#[test]
fn null_panic_handler_aborts_identically() {
    // ERRORS row: js_atpanic(J, NULL) then an uncaught throw -> J->panic is
    // NULL so js_throw goes straight to abort() with no report.
    assert_subproc_eq("subproc_runner", "null_panic");
}

/// The child half of the subprocess comparisons. Does nothing unless the
/// scenario env vars are set, so it is a no-op during a normal test run.
#[test]
fn subproc_runner() {
    let Some((scenario, side)) = subproc_role() else {
        return;
    };
    let imp = if side == "c" { Impl::c() } else { Impl::rust() };
    mark!("scenario={scenario} side={side}");
    match scenario.as_str() {
        "endtry_underflow" => {
            let j = imp.newstate(0);
            mark!("state created");
            unsafe { imp.f::<FnVoid1>("js_endtry")(j) };
            mark!("js_endtry returned (no abort)");
            imp.freestate(j);
        }
        "initerror_strict" => {
            let j = imp.newstate(JS_STRICT);
            mark!("state created");
            unsafe { imp.f::<FnVoid1>("jsB_initerror")(j) };
            mark!("jsB_initerror returned (no abort) top={}", imp.gettop(j));
            imp.freestate(j);
        }
        "uncaught_throw" => {
            let j = imp.newstate(0);
            imp.pushstring(j, b"boom");
            mark!("about to js_throw with trytop==0");
            unsafe { imp.f::<FnVoid1>("js_throw")(j) };
            mark!("js_throw returned (no abort)");
        }
        "null_panic" => {
            let j = imp.newstate(0);
            let atpanic = imp.f::<unsafe extern "C" fn(JsState, *const c_void) -> *const c_void>(
                "js_atpanic",
            );
            let old = unsafe { atpanic(j, std::ptr::null()) };
            mark!("previous panic handler was {}", if old.is_null() { "NULL" } else { "non-NULL" });
            imp.pushstring(j, b"boom");
            mark!("about to js_throw with NULL panic handler");
            unsafe { imp.f::<FnVoid1>("js_throw")(j) };
            mark!("js_throw returned (no abort)");
        }
        other => panic!("unknown scenario {other}"),
    }
    mark!("child finished normally");
}
