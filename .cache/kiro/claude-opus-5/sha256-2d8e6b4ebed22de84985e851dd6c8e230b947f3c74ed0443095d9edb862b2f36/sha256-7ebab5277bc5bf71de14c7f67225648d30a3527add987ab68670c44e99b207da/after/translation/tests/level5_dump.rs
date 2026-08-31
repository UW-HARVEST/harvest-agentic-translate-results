//! Level 5: dump.c
//!
//! Every dump entry point is compared byte-for-byte over the whole flag space.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void};

const SEED: usize = 0x5eed_1234;

const JSON_COMPACT: usize = 0x20;
const JSON_ENSURE_ASCII: usize = 0x40;
const JSON_SORT_KEYS: usize = 0x80;
const JSON_PRESERVE_ORDER: usize = 0x100;
const JSON_ENCODE_ANY: usize = 0x200;
const JSON_ESCAPE_SLASH: usize = 0x400;
const JSON_EMBED: usize = 0x10000;
fn json_indent(n: usize) -> usize {
    n & 0x1f
}
fn json_real_precision(n: usize) -> usize {
    (n & 0x1f) << 11
}

fn seed_both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe { f(SEED) };
    }
    (c, r)
}

/// The set of dump flag combinations worth trying.
fn flag_matrix() -> Vec<usize> {
    let mut v = Vec::new();
    for indent in [0usize, 1, 2, 4, 8, 31] {
        for extra in [
            0usize,
            JSON_COMPACT,
            JSON_ENSURE_ASCII,
            JSON_SORT_KEYS,
            JSON_PRESERVE_ORDER,
            JSON_ESCAPE_SLASH,
            JSON_COMPACT | JSON_ENSURE_ASCII,
            JSON_SORT_KEYS | JSON_COMPACT,
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | JSON_SORT_KEYS,
            JSON_EMBED,
            JSON_EMBED | JSON_COMPACT,
        ] {
            v.push(json_indent(indent) | extra | JSON_ENCODE_ANY);
            v.push(json_indent(indent) | extra);
        }
    }
    for p in [0usize, 1, 2, 5, 10, 15, 16, 17, 31] {
        v.push(json_real_precision(p) | JSON_ENCODE_ANY);
        v.push(json_real_precision(p) | JSON_ENCODE_ANY | JSON_COMPACT | json_indent(2));
    }
    // a few raw bit patterns including unknown/high bits
    for raw in [
        0usize,
        0x1f,
        0xffff,
        0x1_ffff,
        usize::MAX,
        0x8000_0000_0000_0000,
    ] {
        v.push(raw);
    }
    v
}

/// Construct a corpus of values on `l`. The scripts are identical for both
/// libraries so the resulting trees are structurally identical.
unsafe fn corpus(l: &Lib) -> Vec<*mut JsonT> {
    let obj: Symbol<FnNew0> = l.sym("json_object");
    let arr: Symbol<FnNew0> = l.sym("json_array");
    let tru: Symbol<FnNew0> = l.sym("json_true");
    let fls: Symbol<FnNew0> = l.sym("json_false");
    let nul: Symbol<FnNew0> = l.sym("json_null");
    let int: Symbol<FnJsonInteger> = l.sym("json_integer");
    let real: Symbol<FnJsonReal> = l.sym("json_real");
    let strn: Symbol<FnJsonStringn> = l.sym("json_stringn_nocheck");
    let oset: Symbol<FnJsonObjectSetnNew> = l.sym("json_object_setn_new_nocheck");
    let aapp: Symbol<FnJsonArrayAppendNew> = l.sym("json_array_append_new");

    let mut out: Vec<*mut JsonT> = Vec::new();

    // scalars
    out.push(tru());
    out.push(fls());
    out.push(nul());
    for i in [
        0i64,
        1,
        -1,
        42,
        -42,
        i64::MAX,
        i64::MIN,
        9007199254740993,
        1000000000000000000,
    ] {
        out.push(int(i));
    }
    for f in [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.1,
        1.0 / 3.0,
        1e-5,
        1e-4,
        1e15,
        1e16,
        1e17,
        1e20,
        1e-300,
        1e300,
        f64::MAX,
        f64::MIN,
        5e-324,
        f64::EPSILON,
        3.141592653589793,
        123456.789,
        -98765.4321e-30,
    ] {
        let v = real(f);
        if !v.is_null() {
            out.push(v);
        }
    }
    for s in [
        &b""[..],
        b"a",
        b"hello",
        b"tab\there",
        b"nl\nhere",
        b"cr\rhere",
        b"quote\"here",
        b"back\\slash",
        b"slash/here",
        b"\x08\x0c",
        b"\x00\x01\x1f",
        "\u{7f}".as_bytes(),
        "\u{80}".as_bytes(),
        "\u{a0}".as_bytes(),
        "héllo".as_bytes(),
        "日本語".as_bytes(),
        "\u{10ffff}".as_bytes(),
        "𝄞".as_bytes(),
        "\u{2028}\u{2029}".as_bytes(),
        "\u{feff}".as_bytes(),
        &[0xffu8, 0xfe][..],       // invalid UTF-8
        &[0xc0u8, 0x80][..],       // overlong
        &[0xedu8, 0xa0, 0x80][..], // surrogate
        &[0xe2u8, 0x82][..],       // truncated
    ] {
        out.push(strn(s.as_ptr() as *const c_char, s.len()));
    }

    // empty containers
    out.push(obj());
    out.push(arr());

    // flat array
    let a = arr();
    for i in 0..5i64 {
        aapp(a, int(i));
    }
    out.push(a);

    // flat object with keys in non-sorted order
    let o = obj();
    for k in ["zeta", "alpha", "Mu", "beta", "0", "", "a\tb"] {
        oset(o, k.as_ptr() as *const c_char, k.len(), int(k.len() as i64));
    }
    out.push(o);

    // object with a key containing an embedded NUL and invalid UTF-8
    let o = obj();
    oset(o, b"nul\0key".as_ptr() as *const c_char, 7, int(1));
    oset(o, [0xffu8, 0x41].as_ptr() as *const c_char, 2, int(2));
    oset(o, "ünïcödé".as_ptr() as *const c_char, "ünïcödé".len(), int(3));
    out.push(o);

    // mixed nested structure
    let root = obj();
    let ia = arr();
    for i in 0..4i64 {
        aapp(ia, int(i));
    }
    oset(root, b"ints".as_ptr() as *const c_char, 4, ia);
    let ra = arr();
    for f in [0.5f64, 1e100, -1e-100] {
        aapp(ra, real(f));
    }
    oset(root, b"reals".as_ptr() as *const c_char, 5, ra);
    let sa = arr();
    for s in ["x", "ünï", "日本"] {
        aapp(sa, strn(s.as_ptr() as *const c_char, s.len()));
    }
    oset(root, b"strs".as_ptr() as *const c_char, 4, sa);
    let ba = arr();
    aapp(ba, tru());
    aapp(ba, fls());
    aapp(ba, nul());
    oset(root, b"bools".as_ptr() as *const c_char, 5, ba);
    oset(root, b"empty_obj".as_ptr() as *const c_char, 9, obj());
    oset(root, b"empty_arr".as_ptr() as *const c_char, 9, arr());
    out.push(root);

    // deeply nested arrays (well below JSON_PARSER_MAX_DEPTH)
    let mut cur = arr();
    let deep = cur;
    for _ in 0..64 {
        let inner = arr();
        aapp(cur, inner);
        cur = inner;
    }
    aapp(cur, int(1));
    out.push(deep);

    // array of many objects with many keys (exercises sorting)
    let big = arr();
    for i in 0..12i64 {
        let o = obj();
        for j in 0..12i64 {
            let k = format!("k{}", (j * 7 + i) % 12);
            oset(o, k.as_ptr() as *const c_char, k.len(), int(i * 100 + j));
        }
        aapp(big, o);
    }
    out.push(big);

    out
}

unsafe fn free_corpus(l: &Lib, v: &[*mut JsonT]) {
    let del: Symbol<FnJsonDelete> = l.sym("json_delete");
    for &p in v {
        del(p);
    }
}

#[test]
fn json_dumps_matches() {
    let (c, r) = seed_both();
    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        assert_eq!(cc.len(), rr.len());
        let freec: Symbol<FnFree> = c.sym("jsonp_free");
        let freer: Symbol<FnFree> = r.sym("jsonp_free");
        let fc: Symbol<FnJsonDumps> = c.sym("json_dumps");
        let fr: Symbol<FnJsonDumps> = r.sym("json_dumps");

        for flags in flag_matrix() {
            for i in 0..cc.len() {
                let a = fc(cc[i], flags);
                let b = fr(rr[i], flags);
                let ab = if a.is_null() {
                    None
                } else {
                    Some(std::ffi::CStr::from_ptr(a).to_bytes().to_vec())
                };
                let bb = if b.is_null() {
                    None
                } else {
                    Some(std::ffi::CStr::from_ptr(b).to_bytes().to_vec())
                };
                assert_eq!(
                    ab.as_ref().map(|x| String::from_utf8_lossy(x).into_owned()),
                    bb.as_ref().map(|x| String::from_utf8_lossy(x).into_owned()),
                    "json_dumps(corpus[{i}], flags {flags:#x})"
                );
                assert_eq!(ab, bb, "json_dumps(corpus[{i}], flags {flags:#x}) bytes");
                if !a.is_null() {
                    freec(a as *mut c_void);
                }
                if !b.is_null() {
                    freer(b as *mut c_void);
                }
            }
            // NULL value
            assert_eq!(
                fc(std::ptr::null(), flags).is_null(),
                fr(std::ptr::null(), flags).is_null(),
                "json_dumps(NULL, {flags:#x})"
            );
        }

        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

#[test]
fn json_dumpb_matches() {
    let (c, r) = seed_both();
    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        let fc: Symbol<FnJsonDumpb> = c.sym("json_dumpb");
        let fr: Symbol<FnJsonDumpb> = r.sym("json_dumpb");

        for flags in [
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | JSON_COMPACT,
            JSON_ENCODE_ANY | json_indent(2),
            JSON_ENCODE_ANY | JSON_SORT_KEYS | JSON_ENSURE_ASCII,
            0,
        ] {
            for i in 0..cc.len() {
                // size 0 with NULL buffer returns the required length
                let n1 = fc(cc[i], std::ptr::null_mut(), 0, flags);
                let n2 = fr(rr[i], std::ptr::null_mut(), 0, flags);
                assert_eq!(n1, n2, "json_dumpb(corpus[{i}], NULL, 0, {flags:#x})");

                // exact size, one byte short, one byte long
                for size in [n1, n1.saturating_sub(1), n1 + 1, 0, 1, 4] {
                    let mut ba = vec![0xa5u8; size + 8];
                    let mut bb = vec![0xa5u8; size + 8];
                    let x = fc(cc[i], ba.as_mut_ptr() as *mut c_char, size, flags);
                    let y = fr(rr[i], bb.as_mut_ptr() as *mut c_char, size, flags);
                    assert_eq!(
                        x, y,
                        "json_dumpb(corpus[{i}], size {size}, {flags:#x}) return"
                    );
                    // only the reported prefix is defined
                    let n = x.min(size);
                    assert_eq!(
                        &ba[..n],
                        &bb[..n],
                        "json_dumpb(corpus[{i}], size {size}, {flags:#x}) bytes"
                    );
                }
            }
            assert_eq!(
                fc(std::ptr::null(), std::ptr::null_mut(), 0, flags),
                fr(std::ptr::null(), std::ptr::null_mut(), 0, flags),
                "json_dumpb(NULL)"
            );
        }
        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

// dump callbacks: collect into a Vec, and a failing variant.

struct Sink {
    out: Vec<u8>,
    fail_after: usize,
    calls: usize,
}

unsafe extern "C" fn sink_cb(buf: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let s = &mut *(data as *mut Sink);
    s.calls += 1;
    if s.calls > s.fail_after {
        return -1;
    }
    if !buf.is_null() && size > 0 {
        s.out
            .extend_from_slice(std::slice::from_raw_parts(buf as *const u8, size));
    }
    0
}

#[test]
fn json_dump_callback_matches() {
    let (c, r) = seed_both();
    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        let fc: Symbol<FnJsonDumpCallback> = c.sym("json_dump_callback");
        let fr: Symbol<FnJsonDumpCallback> = r.sym("json_dump_callback");

        for flags in [
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | JSON_COMPACT | json_indent(3),
            JSON_ENCODE_ANY | JSON_SORT_KEYS,
            JSON_ENCODE_ANY | JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH,
            JSON_ENCODE_ANY | JSON_EMBED,
            0,
        ] {
            for i in 0..cc.len() {
                let mut sa = Sink {
                    out: Vec::new(),
                    fail_after: usize::MAX,
                    calls: 0,
                };
                let mut sb = Sink {
                    out: Vec::new(),
                    fail_after: usize::MAX,
                    calls: 0,
                };
                let x = fc(
                    cc[i],
                    sink_cb as *mut c_void,
                    &mut sa as *mut Sink as *mut c_void,
                    flags,
                );
                let y = fr(
                    rr[i],
                    sink_cb as *mut c_void,
                    &mut sb as *mut Sink as *mut c_void,
                    flags,
                );
                assert_eq!(x, y, "json_dump_callback(corpus[{i}], {flags:#x}) rc");
                assert_eq!(
                    String::from_utf8_lossy(&sa.out),
                    String::from_utf8_lossy(&sb.out),
                    "json_dump_callback(corpus[{i}], {flags:#x}) output"
                );
                assert_eq!(sa.out, sb.out, "... bytes");
                assert_eq!(
                    sa.calls, sb.calls,
                    "json_dump_callback(corpus[{i}], {flags:#x}) call count"
                );

                // failing callback: the error must propagate at the same point
                for fail_after in [0usize, 1, 2, 3, 5] {
                    let mut sa = Sink {
                        out: Vec::new(),
                        fail_after,
                        calls: 0,
                    };
                    let mut sb = Sink {
                        out: Vec::new(),
                        fail_after,
                        calls: 0,
                    };
                    let x = fc(
                        cc[i],
                        sink_cb as *mut c_void,
                        &mut sa as *mut Sink as *mut c_void,
                        flags,
                    );
                    let y = fr(
                        rr[i],
                        sink_cb as *mut c_void,
                        &mut sb as *mut Sink as *mut c_void,
                        flags,
                    );
                    assert_eq!(
                        (x, sa.calls, &sa.out),
                        (y, sb.calls, &sb.out),
                        "failing callback (after {fail_after}) corpus[{i}] {flags:#x}"
                    );
                }
            }
            // NULL value. (A NULL *callback* is not probed: dump.c calls it
            // unconditionally, so both libraries would fault identically.)
            let mut sa = Sink {
                out: Vec::new(),
                fail_after: usize::MAX,
                calls: 0,
            };
            let mut sb = Sink {
                out: Vec::new(),
                fail_after: usize::MAX,
                calls: 0,
            };
            assert_eq!(
                fc(
                    std::ptr::null(),
                    sink_cb as *mut c_void,
                    &mut sa as *mut Sink as *mut c_void,
                    flags
                ),
                fr(
                    std::ptr::null(),
                    sink_cb as *mut c_void,
                    &mut sb as *mut Sink as *mut c_void,
                    flags
                ),
                "json_dump_callback NULL value"
            );
        }
        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

#[test]
fn json_dump_file_matches() {
    let (c, r) = seed_both();
    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        let fc: Symbol<FnJsonDumpFile> = c.sym("json_dump_file");
        let fr: Symbol<FnJsonDumpFile> = r.sym("json_dump_file");

        let dir = std::env::temp_dir();
        let pa = dir.join(format!("jansson_c_{}.json", std::process::id()));
        let pb = dir.join(format!("jansson_r_{}.json", std::process::id()));
        let za = std::ffi::CString::new(pa.to_str().unwrap()).unwrap();
        let zb = std::ffi::CString::new(pb.to_str().unwrap()).unwrap();

        for flags in [
            JSON_ENCODE_ANY,
            JSON_ENCODE_ANY | json_indent(4) | JSON_SORT_KEYS,
            0,
        ] {
            for i in 0..cc.len() {
                let x = fc(cc[i], za.as_ptr(), flags);
                let y = fr(rr[i], zb.as_ptr(), flags);
                assert_eq!(x, y, "json_dump_file(corpus[{i}], {flags:#x}) rc");
                let a = std::fs::read(&pa).unwrap_or_default();
                let b = std::fs::read(&pb).unwrap_or_default();
                assert_eq!(
                    String::from_utf8_lossy(&a),
                    String::from_utf8_lossy(&b),
                    "json_dump_file(corpus[{i}], {flags:#x}) contents"
                );
                assert_eq!(a, b, "... bytes");
            }
            // unwritable path
            let bad = cs("/definitely/not/a/real/dir/x.json");
            assert_eq!(
                fc(cc[0], bad.as_ptr(), flags),
                fr(rr[0], bad.as_ptr(), flags),
                "json_dump_file bad path"
            );
        }
        let _ = std::fs::remove_file(&pa);
        let _ = std::fs::remove_file(&pb);
        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

#[test]
fn json_dumpfd_matches() {
    let (c, r) = seed_both();
    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        let fc: Symbol<FnJsonDumpfd> = c.sym("json_dumpfd");
        let fr: Symbol<FnJsonDumpfd> = r.sym("json_dumpfd");

        let dir = std::env::temp_dir();
        for flags in [JSON_ENCODE_ANY, JSON_ENCODE_ANY | json_indent(2), 0] {
            for i in 0..cc.len() {
                let pa = dir.join(format!("jansson_fd_c_{}.json", std::process::id()));
                let pb = dir.join(format!("jansson_fd_r_{}.json", std::process::id()));
                let (x, a) = {
                    let f = std::fs::File::create(&pa).unwrap();
                    let fd = std::os::fd::AsRawFd::as_raw_fd(&f);
                    let x = fc(cc[i], fd, flags);
                    drop(f);
                    (x, std::fs::read(&pa).unwrap())
                };
                let (y, b) = {
                    let f = std::fs::File::create(&pb).unwrap();
                    let fd = std::os::fd::AsRawFd::as_raw_fd(&f);
                    let y = fr(rr[i], fd, flags);
                    drop(f);
                    (y, std::fs::read(&pb).unwrap())
                };
                assert_eq!(x, y, "json_dumpfd(corpus[{i}], {flags:#x}) rc");
                assert_eq!(
                    String::from_utf8_lossy(&a),
                    String::from_utf8_lossy(&b),
                    "json_dumpfd(corpus[{i}], {flags:#x}) contents"
                );
                let _ = std::fs::remove_file(&pa);
                let _ = std::fs::remove_file(&pb);
            }
            // invalid fd
            assert_eq!(
                fc(cc[0], -1, flags),
                fr(rr[0], -1, flags),
                "json_dumpfd(-1)"
            );
        }
        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

#[test]
fn json_dumpf_matches() {
    let (c, r) = seed_both();
    type FnFopen = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
    type FnFclose = unsafe extern "C" fn(*mut c_void) -> c_int;
    type FnDumpf = unsafe extern "C" fn(*const JsonT, *mut c_void, usize) -> c_int;
    extern "C" {
        fn fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
        fn fclose(f: *mut c_void) -> c_int;
    }
    let _: (FnFopen, FnFclose) = (fopen, fclose);

    unsafe {
        let cc = corpus(c);
        let rr = corpus(r);
        let fc: Symbol<FnDumpf> = c.sym("json_dumpf");
        let fr: Symbol<FnDumpf> = r.sym("json_dumpf");

        let dir = std::env::temp_dir();
        let pa = dir.join(format!("jansson_f_c_{}.json", std::process::id()));
        let pb = dir.join(format!("jansson_f_r_{}.json", std::process::id()));
        let za = std::ffi::CString::new(pa.to_str().unwrap()).unwrap();
        let zb = std::ffi::CString::new(pb.to_str().unwrap()).unwrap();
        let mode = cs("wb");

        for flags in [JSON_ENCODE_ANY, JSON_ENCODE_ANY | json_indent(2) | JSON_SORT_KEYS, 0] {
            for i in 0..cc.len() {
                let f1 = fopen(za.as_ptr(), mode.as_ptr());
                let x = fc(cc[i], f1, flags);
                fclose(f1);
                let f2 = fopen(zb.as_ptr(), mode.as_ptr());
                let y = fr(rr[i], f2, flags);
                fclose(f2);
                assert_eq!(x, y, "json_dumpf(corpus[{i}], {flags:#x}) rc");
                let a = std::fs::read(&pa).unwrap();
                let b = std::fs::read(&pb).unwrap();
                assert_eq!(
                    String::from_utf8_lossy(&a),
                    String::from_utf8_lossy(&b),
                    "json_dumpf(corpus[{i}], {flags:#x}) contents"
                );
            }
        }
        let _ = std::fs::remove_file(&pa);
        let _ = std::fs::remove_file(&pb);
        free_corpus(c, &cc);
        free_corpus(r, &rr);
    }
}

#[test]
fn dump_circular_reference_matches() {
    // jansson detects cycles during dumping and fails; both must fail the same
    // way and leave the same partial output.
    let (c, r) = seed_both();
    unsafe {
        for flags in [JSON_ENCODE_ANY, JSON_ENCODE_ANY | json_indent(2)] {
            let mut outs = Vec::new();
            let mut rcs = Vec::new();
            for l in [c, r] {
                let arr: Symbol<FnNew0> = l.sym("json_array");
                let obj: Symbol<FnNew0> = l.sym("json_object");
                let app: Symbol<FnJsonArrayAppendNew> = l.sym("json_array_append_new");
                let oset: Symbol<FnJsonObjectSetNew> = l.sym("json_object_set_new");
                let dumpcb: Symbol<FnJsonDumpCallback> = l.sym("json_dump_callback");
                type FnIncref = unsafe extern "C" fn(*mut JsonT);
                let _ = std::mem::size_of::<FnIncref>();

                // a -> [a] via manual refcount bump so json_delete still works
                let a = arr();
                (*a).refcount += 1; // the self reference we are about to create
                let inner = arr();
                app(a, inner);
                // make `inner` contain `a` again
                app(inner, a);

                let mut s = Sink {
                    out: Vec::new(),
                    fail_after: usize::MAX,
                    calls: 0,
                };
                let rc = dumpcb(
                    a,
                    sink_cb as *mut c_void,
                    &mut s as *mut Sink as *mut c_void,
                    flags,
                );
                rcs.push(rc);
                outs.push(s.out);

                // break the cycle before dropping
                let clr: Symbol<FnJsonArrayClear> = l.sym("json_array_clear");
                clr(inner);
                (*a).refcount -= 1;
                let del: Symbol<FnJsonDelete> = l.sym("json_delete");
                del(a);
                let _ = obj;
                let _ = oset;
            }
            assert_eq!(rcs[0], rcs[1], "cycle dump rc, flags {flags:#x}");
            assert_eq!(
                String::from_utf8_lossy(&outs[0]),
                String::from_utf8_lossy(&outs[1]),
                "cycle dump output, flags {flags:#x}"
            );
        }
    }
}

#[test]
fn dump_real_precision_sweep_matches() {
    // Reals are the most delicate part of dumping; sweep every precision.
    let (c, r) = seed_both();
    unsafe {
        let rc_: Symbol<FnJsonReal> = c.sym("json_real");
        let rr_: Symbol<FnJsonReal> = r.sym("json_real");
        let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
        let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, 0.1, 1.0 / 3.0, 1e-5, 1e-4, 1e15, 1e16, 1e17, 1e21, 1e22,
            1e-300, 1e300, 5e-324, f64::MAX, f64::MIN, f64::EPSILON,
            3.141592653589793, 2.718281828459045, 1e-323,
            9007199254740992.0, 123456789012345678.0, 1.0000000000000002,
        ];
        let mut s: u64 = 0xc0ffee;
        for _ in 0..600 {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            let f = f64::from_bits(s);
            if f.is_finite() {
                vals.push(f);
            }
        }
        for v in vals {
            let a = rc_(v);
            let b = rr_(v);
            if a.is_null() {
                assert!(b.is_null());
                continue;
            }
            for p in 0usize..=31 {
                let flags = json_real_precision(p) | JSON_ENCODE_ANY;
                assert_eq!(
                    dump(c, a, flags),
                    dump(r, b, flags),
                    "real {v:e} [{:#018x}] precision {p}",
                    v.to_bits()
                );
            }
            dc(a);
            dr(b);
        }
    }
}
