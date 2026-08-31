// Level 4: js_State lifecycle, value stack, primitive conversions.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn both_sessions(flags: c_int) -> (Session, Session) {
    (Session::new(Side::C, flags), Session::new(Side::Rust, flags))
}

#[test]
fn newstate_and_freestate() {
    for flags in [0, 1] {
        let (c, r) = both_sessions(flags);
        assert_eq!(c.top(), r.top(), "initial top (flags={})", flags);
    }
}

#[test]
fn push_and_type_predicates() {
    let (cs, rs) = both_sessions(0);
    let strings = [
        "", "a", "hello", "0", "1", "NaN", "true", "false", "null", "undefined",
        "\u{00e9}\u{1F600}", "  12  ", "1e3",
    ];
    let numbers = [
        0.0f64, -0.0, 1.0, -1.0, 0.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        1e21, 1e-7, 123456789.0, -2147483648.0, 4294967296.0, 0.1,
    ];

    // Build a mixed stack of the same values on both sides.
    let mut kinds: Vec<Box<dyn Fn(&Session)>> = Vec::new();
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushundefined)(s.j) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushnull)(s.j) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushboolean)(s.j, 0) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushboolean)(s.j, 1) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushboolean)(s.j, 42) }));
    for &n in numbers.iter() {
        kinds.push(Box::new(move |s: &Session| unsafe {
            (s.vm.pushnumber)(s.j, n)
        }));
    }
    for st in strings.iter() {
        let owned = CString::new(*st).unwrap();
        kinds.push(Box::new(move |s: &Session| unsafe {
            (s.vm.pushstring)(s.j, owned.as_ptr())
        }));
    }
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.newobject)(s.j) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.newarray)(s.j) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.newnumber)(s.j, 7.5) }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.newboolean)(s.j, 1) }));
    kinds.push(Box::new(|s: &Session| {
        let v = CString::new("boxed").unwrap();
        unsafe { (s.vm.newstring)(s.j, v.as_ptr()) }
    }));
    kinds.push(Box::new(|s: &Session| {
        let p = CString::new("a(b)c*").unwrap();
        unsafe { (s.vm.newregexp)(s.j, p.as_ptr(), 0) }
    }));
    kinds.push(Box::new(|s: &Session| unsafe { (s.vm.pushglobal)(s.j) }));

    for k in kinds.iter() {
        k(&cs);
        k(&rs);
        let idx = cs.top() - 1;
        assert_eq!(cs.top(), rs.top(), "top after push");
        compare_slot(&cs, &rs, idx);
        compare_slot(&cs, &rs, -1);
    }

    // Absolute and negative indices over the whole stack.
    let n = cs.top();
    for i in 0..n {
        compare_slot(&cs, &rs, i);
        compare_slot(&cs, &rs, i - n);
    }
}

fn compare_slot(cs: &Session, rs: &Session, idx: c_int) {
    macro_rules! cmp_i {
        ($f:ident) => {{
            let a = unsafe { (cs.vm.$f)(cs.j, idx) };
            let b = unsafe { (rs.vm.$f)(rs.j, idx) };
            assert_eq!(a, b, concat!(stringify!($f), " at idx {}"), idx);
        }};
    }
    cmp_i!(isdefined);
    cmp_i!(isundefined);
    cmp_i!(isnull);
    cmp_i!(isboolean);
    cmp_i!(isnumber);
    cmp_i!(isstring);
    cmp_i!(isprimitive);
    cmp_i!(isobject);
    cmp_i!(isarray);
    cmp_i!(isregexp);
    cmp_i!(iscoercible);
    cmp_i!(iscallable);
    cmp_i!(iserror);
    cmp_i!(isnumberobject);
    cmp_i!(isstringobject);
    cmp_i!(isbooleanobject);
    cmp_i!(isdateobject);
    cmp_i!(type_);
    cmp_i!(toboolean);
    cmp_i!(tointeger);
    cmp_i!(toint32);
    cmp_i!(touint32);
    cmp_i!(toint16);
    cmp_i!(touint16);

    let a = unsafe { (cs.vm.tonumber)(cs.j, idx) };
    let b = unsafe { (rs.vm.tonumber)(rs.j, idx) };
    assert_eq!(a.to_bits(), b.to_bits(), "tonumber at idx {}", idx);

    let a = unsafe { common::cstr_to_bytes((cs.vm.tostring)(cs.j, idx)) };
    let b = unsafe { common::cstr_to_bytes((rs.vm.tostring)(rs.j, idx)) };
    assert_eq!(a, b, "tostring at idx {}", idx);

    let a = unsafe { common::cstr_to_bytes((cs.vm.typeof_)(cs.j, idx)) };
    let b = unsafe { common::cstr_to_bytes((rs.vm.typeof_)(rs.j, idx)) };
    assert_eq!(a, b, "typeof at idx {}", idx);

    let e = CString::new("<err>").unwrap();
    let a = unsafe { common::cstr_to_bytes((cs.vm.tryrepr)(cs.j, idx, e.as_ptr())) };
    let b = unsafe { common::cstr_to_bytes((rs.vm.tryrepr)(rs.j, idx, e.as_ptr())) };
    assert_eq!(a, b, "tryrepr at idx {}", idx);
}

#[test]
fn pushlstring_and_literal() {
    let (cs, rs) = both_sessions(0);
    let cases: Vec<(&[u8], c_int)> = vec![
        (b"hello", 5),
        (b"hello", 3),
        (b"hello", 0),
        (b"hello", 1),
        (b"\xc3\xa9abc", 5),
        (b"\xc3\xa9abc", 1), // truncated in the middle of a UTF-8 sequence
        (b"\xff\xfe", 2),
        (b"a\0b", 3),
        (b"\xe4\xb8\xad\xe6\x96\x87", 6),
        (b"\xe4\xb8\xad\xe6\x96\x87", 4),
    ];
    for (bytes, n) in cases {
        let mut v = bytes.to_vec();
        v.push(0);
        unsafe { (cs.vm.pushlstring)(cs.j, v.as_ptr() as *const c_char, n) };
        unsafe { (rs.vm.pushlstring)(rs.j, v.as_ptr() as *const c_char, n) };
        let a = unsafe { common::cstr_to_bytes((cs.vm.tostring)(cs.j, -1)) };
        let b = unsafe { common::cstr_to_bytes((rs.vm.tostring)(rs.j, -1)) };
        assert_eq!(a, b, "pushlstring({:?},{})", bytes, n);
        compare_slot(&cs, &rs, -1);
        unsafe { (cs.vm.pop)(cs.j, 1) };
        unsafe { (rs.vm.pop)(rs.j, 1) };
    }

    for s in ["", "x", "literal string", "\u{00e9}"] {
        let c = CString::new(s).unwrap();
        unsafe { (cs.vm.pushliteral)(cs.j, c.as_ptr()) };
        unsafe { (rs.vm.pushliteral)(rs.j, c.as_ptr()) };
        compare_slot(&cs, &rs, -1);
        unsafe { (cs.vm.pop)(cs.j, 1) };
        unsafe { (rs.vm.pop)(rs.j, 1) };
    }
}

#[test]
fn stack_manipulation_matches() {
    let (cs, rs) = both_sessions(0);

    // Every op runs inside a protected frame so that ops which raise JS errors
    // (js_insert, out-of-range js_remove/js_replace, js_pop underflow) can be
    // compared rather than aborting the process.
    macro_rules! same {
        ($label:expr, $body:expr) => {{
            let f = |vm: &Vm, j: JsPtr| {
                for i in 0..8 {
                    unsafe { (vm.pushnumber)(j, i as f64) };
                }
                let op: fn(&Vm, JsPtr) = $body;
                op(vm, j);
                for l in stack_snapshot(vm, j) {
                    logln(l);
                }
            };
            assert_same_protected(&cs, &rs, $label, f);
        }};
    }

    same!("dup", |vm, j| unsafe { (vm.dup)(j) });
    same!("dup2", |vm, j| unsafe { (vm.dup2)(j) });
    same!("rot2", |vm, j| unsafe { (vm.rot2)(j) });
    same!("rot3", |vm, j| unsafe { (vm.rot3)(j) });
    same!("rot4", |vm, j| unsafe { (vm.rot4)(j) });
    same!("rot2pop1", |vm, j| unsafe { (vm.rot2pop1)(j) });
    same!("rot3pop2", |vm, j| unsafe { (vm.rot3pop2)(j) });

    macro_rules! same_idx {
        ($label:expr, $field:ident, $range:expr) => {{
            for k in $range {
                let f = move |vm: &Vm, j: JsPtr| {
                    for i in 0..8 {
                        unsafe { (vm.pushnumber)(j, i as f64) };
                    }
                    unsafe { (vm.$field)(j, k) };
                    for l in stack_snapshot(vm, j) {
                        logln(l);
                    }
                };
                assert_same_protected(&cs, &rs, &format!("{}({})", $label, k), f);
            }
        }};
    }
    same_idx!("rot", rot, 1..7i32);
    same_idx!("copy", copy, -12..12i32);
    same_idx!("pushvalue", pushvalue, -12..12i32);
    same_idx!("remove", remove, -12..12i32);
    same_idx!("insert", insert, -12..12i32);
    same_idx!("replace", replace, -12..12i32);
    same_idx!("pop", pop, -2..12i32);
}

#[test]
fn concat_compare_equal_matches() {
    let (cs, rs) = both_sessions(0);
    let vals: Vec<Box<dyn Fn(&Session)>> = vec![
        Box::new(|s: &Session| unsafe { (s.vm.pushnumber)(s.j, 1.0) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushnumber)(s.j, 2.0) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushnumber)(s.j, f64::NAN) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushnumber)(s.j, -0.0) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushboolean)(s.j, 1) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushboolean)(s.j, 0) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushnull)(s.j) }),
        Box::new(|s: &Session| unsafe { (s.vm.pushundefined)(s.j) }),
        Box::new(|s: &Session| {
            let c = CString::new("1").unwrap();
            unsafe { (s.vm.pushstring)(s.j, c.as_ptr()) }
        }),
        Box::new(|s: &Session| {
            let c = CString::new("abc").unwrap();
            unsafe { (s.vm.pushstring)(s.j, c.as_ptr()) }
        }),
        Box::new(|s: &Session| {
            let c = CString::new("").unwrap();
            unsafe { (s.vm.pushstring)(s.j, c.as_ptr()) }
        }),
        Box::new(|s: &Session| unsafe { (s.vm.newobject)(s.j) }),
        Box::new(|s: &Session| unsafe { (s.vm.newarray)(s.j) }),
        Box::new(|s: &Session| unsafe { (s.vm.newnumber)(s.j, 1.0) }),
    ];

    for a in vals.iter() {
        for b in vals.iter() {
            // concat
            a(&cs);
            b(&cs);
            a(&rs);
            b(&rs);
            unsafe { (cs.vm.concat)(cs.j) };
            unsafe { (rs.vm.concat)(rs.j) };
            let x = unsafe { common::cstr_to_bytes((cs.vm.tostring)(cs.j, -1)) };
            let y = unsafe { common::cstr_to_bytes((rs.vm.tostring)(rs.j, -1)) };
            assert_eq!(x, y, "concat");
            unsafe { (cs.vm.pop)(cs.j, 1) };
            unsafe { (rs.vm.pop)(rs.j, 1) };

            // compare
            a(&cs);
            b(&cs);
            a(&rs);
            b(&rs);
            let mut oc: c_int = -99;
            let mut or: c_int = -99;
            let x = unsafe { (cs.vm.compare)(cs.j, &mut oc) };
            let y = unsafe { (rs.vm.compare)(rs.j, &mut or) };
            assert_eq!((x, oc), (y, or), "compare");
            unsafe { (cs.vm.pop)(cs.j, 2) };
            unsafe { (rs.vm.pop)(rs.j, 2) };

            // equal / strictequal
            a(&cs);
            b(&cs);
            a(&rs);
            b(&rs);
            let x = unsafe { (cs.vm.equal)(cs.j) };
            let y = unsafe { (rs.vm.equal)(rs.j) };
            assert_eq!(x, y, "equal");
            unsafe { (cs.vm.pop)(cs.j, 2) };
            unsafe { (rs.vm.pop)(rs.j, 2) };

            a(&cs);
            b(&cs);
            a(&rs);
            b(&rs);
            let x = unsafe { (cs.vm.strictequal)(cs.j) };
            let y = unsafe { (rs.vm.strictequal)(rs.j) };
            assert_eq!(x, y, "strictequal");
            unsafe { (cs.vm.pop)(cs.j, 2) };
            unsafe { (rs.vm.pop)(rs.j, 2) };
        }
    }
    assert_eq!(cs.top(), rs.top());
}

#[test]
fn intern_and_arrayindex_match() {
    let (cs, rs) = both_sessions(0);
    for s in [
        "", "0", "1", "9", "10", "007", "4294967294", "4294967295", "4294967296",
        "-1", "1.5", "abc", " 1", "1 ", "+1", "01", "2147483647", "99999999999999999999",
        "0x10", "1e3",
    ] {
        let c = CString::new(s).unwrap();
        let mut ic: c_int = -7;
        let mut ir: c_int = -7;
        let a = unsafe { (cs.vm.isarrayindex)(cs.j, c.as_ptr(), &mut ic) };
        let b = unsafe { (rs.vm.isarrayindex)(rs.j, c.as_ptr(), &mut ir) };
        assert_eq!((a, ic), (b, ir), "js_isarrayindex({:?})", s);

        let pa = unsafe { (cs.vm.intern)(cs.j, c.as_ptr()) };
        let pb = unsafe { (rs.vm.intern)(rs.j, c.as_ptr()) };
        assert_eq!(
            unsafe { common::cstr_to_bytes(pa) },
            unsafe { common::cstr_to_bytes(pb) },
            "js_intern({:?})",
            s
        );
        // interning twice must return the identical pointer
        let pa2 = unsafe { (cs.vm.intern)(cs.j, c.as_ptr()) };
        let pb2 = unsafe { (rs.vm.intern)(rs.j, c.as_ptr()) };
        assert_eq!(pa == pa2, pb == pb2, "js_intern stability for {:?}", s);
    }
}

#[test]
fn utflen_matches() {
    let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(*const c_char) -> c_int>("js_utflen") };
    for s in [
        "", "a", "abc", "\u{00e9}", "\u{00e9}abc", "\u{4e2d}\u{6587}", "\u{1F600}",
        "mixed \u{00e9} \u{4e2d} \u{1F600} end",
    ] {
        let cs = CString::new(s).unwrap();
        assert_eq!(
            unsafe { c(cs.as_ptr()) },
            unsafe { r(cs.as_ptr()) },
            "js_utflen({:?})",
            s
        );
    }
    // invalid UTF-8 sequences
    for bytes in [
        vec![0xffu8, 0],
        vec![0xc3, 0],
        vec![0xe4, 0xb8, 0],
        vec![0x80, 0x80, 0],
        vec![0xf0, 0x9f, 0x98, 0],
    ] {
        assert_eq!(
            unsafe { c(bytes.as_ptr() as *const c_char) },
            unsafe { r(bytes.as_ptr() as *const c_char) },
            "js_utflen({:02X?})",
            bytes
        );
    }
}
