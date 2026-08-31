//! Level 1: utf.c, memory.c, strbuffer.c, version.c, error.c
//!
//! Everything is called through the exported symbols of both shared objects.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void};

#[test]
fn utf8_encode_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnUtf8Encode> = c.sym("utf8_encode");
    let fr: Symbol<FnUtf8Encode> = r.sym("utf8_encode");

    let mut cps: Vec<i32> = vec![
        i32::MIN,
        -1000000,
        -2,
        -1,
        0,
        1,
        0x7e,
        0x7f,
        0x80,
        0x81,
        0x7ff,
        0x800,
        0x801,
        0xd7ff,
        0xd800,
        0xdfff,
        0xe000,
        0xfffd,
        0xffff,
        0x10000,
        0x10001,
        0x10ffff,
        0x110000,
        0x1fffff,
        0x7fffffff,
        i32::MAX,
    ];
    for i in 0..2000 {
        cps.push(i * 613 % 0x120000);
    }

    for cp in cps {
        let mut bc = [0u8; 8];
        let mut br = [0u8; 8];
        let mut sc: usize = 0xdead_beef;
        let mut sr: usize = 0xdead_beef;
        let rc = unsafe { fc(cp, bc.as_mut_ptr() as *mut c_char, &mut sc) };
        let rr = unsafe { fr(cp, br.as_mut_ptr() as *mut c_char, &mut sr) };
        assert_eq!(rc, rr, "utf8_encode({cp:#x}) return");
        assert_eq!(sc, sr, "utf8_encode({cp:#x}) size");
        assert_eq!(bc, br, "utf8_encode({cp:#x}) buffer");
    }
}

#[test]
fn utf8_check_first_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnUtf8CheckFirst> = c.sym("utf8_check_first");
    let fr: Symbol<FnUtf8CheckFirst> = r.sym("utf8_check_first");
    for b in 0u16..256 {
        let ch = b as u8 as c_char;
        assert_eq!(
            unsafe { fc(ch) },
            unsafe { fr(ch) },
            "utf8_check_first({b:#x})"
        );
    }
}

fn utf8_probe_buffers() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"ab".to_vec(),
        vec![0x00],
        vec![0x7f],
        vec![0x80],
        vec![0xbf],
        vec![0xc0, 0x80],
        vec![0xc1, 0xbf],
        vec![0xc2, 0x80],
        vec![0xc2, 0x7f],
        vec![0xc2, 0xc0],
        vec![0xdf, 0xbf],
        vec![0xe0, 0x80, 0x80],
        vec![0xe0, 0xa0, 0x80],
        vec![0xed, 0x9f, 0xbf],
        vec![0xed, 0xa0, 0x80], // surrogate D800
        vec![0xed, 0xbf, 0xbf], // surrogate DFFF
        vec![0xee, 0x80, 0x80],
        vec![0xef, 0xbf, 0xbd],
        vec![0xef, 0xbf, 0xbf],
        vec![0xf0, 0x80, 0x80, 0x80],
        vec![0xf0, 0x90, 0x80, 0x80],
        vec![0xf4, 0x8f, 0xbf, 0xbf],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xf5, 0x80, 0x80, 0x80],
        vec![0xff],
        vec![0xfe, 0xff],
        "héllo wörld".as_bytes().to_vec(),
        "日本語テキスト".as_bytes().to_vec(),
        "𝄞𝅘𝅥𝅮".as_bytes().to_vec(),
        vec![0xe2, 0x82], // truncated 3-byte
        vec![0xf0, 0x9f], // truncated 4-byte
        vec![0xf0, 0x9f, 0x98],
    ];
    // pseudo-random byte soup
    let mut s: u64 = 0x1234_5678_9abc_def0;
    for len in 0..6usize {
        for _ in 0..300 {
            let mut b = Vec::with_capacity(len);
            for _ in 0..len {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                b.push((s >> 33) as u8);
            }
            v.push(b);
        }
    }
    v
}

#[test]
fn utf8_check_full_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnUtf8CheckFull> = c.sym("utf8_check_full");
    let fr: Symbol<FnUtf8CheckFull> = r.sym("utf8_check_full");

    for buf in utf8_probe_buffers() {
        if buf.is_empty() {
            continue; // reads buffer[0] unconditionally
        }
        // Only sizes that are actually within the buffer, plus 0/1/5 which
        // return early in C.
        for size in [0usize, 1, 2, 3, 4, 5] {
            if size > buf.len() && (2..=4).contains(&size) {
                continue;
            }
            let mut cpc: i32 = -12345;
            let mut cpr: i32 = -12345;
            let rc = unsafe { fc(buf.as_ptr() as *const c_char, size, &mut cpc) };
            let rr = unsafe { fr(buf.as_ptr() as *const c_char, size, &mut cpr) };
            assert_eq!(rc, rr, "utf8_check_full({buf:02x?}, {size}) return");
            assert_eq!(cpc, cpr, "utf8_check_full({buf:02x?}, {size}) codepoint");
        }
        // NULL codepoint pointer
        for size in [2usize, 3, 4] {
            if size > buf.len() {
                continue;
            }
            let rc = unsafe { fc(buf.as_ptr() as *const c_char, size, std::ptr::null_mut()) };
            let rr = unsafe { fr(buf.as_ptr() as *const c_char, size, std::ptr::null_mut()) };
            assert_eq!(rc, rr, "utf8_check_full({buf:02x?}, {size}, NULL)");
        }
    }
}

#[test]
fn utf8_iterate_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnUtf8Iterate> = c.sym("utf8_iterate");
    let fr: Symbol<FnUtf8Iterate> = r.sym("utf8_iterate");

    for buf in utf8_probe_buffers() {
        for size in 0..=buf.len() {
            let p = buf.as_ptr() as *const c_char;
            let mut cpc: i32 = -12345;
            let mut cpr: i32 = -12345;
            let rc = unsafe { fc(p, size, &mut cpc) };
            let rr = unsafe { fr(p, size, &mut cpr) };
            let offc = if rc.is_null() {
                -1i64
            } else {
                unsafe { rc.offset_from(p) as i64 }
            };
            let offr = if rr.is_null() {
                -1i64
            } else {
                unsafe { rr.offset_from(p) as i64 }
            };
            assert_eq!(offc, offr, "utf8_iterate({buf:02x?}, {size}) ptr");
            assert_eq!(cpc, cpr, "utf8_iterate({buf:02x?}, {size}) codepoint");

            let rc2 = unsafe { fc(p, size, std::ptr::null_mut()) };
            let rr2 = unsafe { fr(p, size, std::ptr::null_mut()) };
            assert_eq!(
                rc2.is_null(),
                rr2.is_null(),
                "utf8_iterate({buf:02x?}, {size}, NULL)"
            );
        }
    }
}

#[test]
fn utf8_check_string_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnUtf8CheckString> = c.sym("utf8_check_string");
    let fr: Symbol<FnUtf8CheckString> = r.sym("utf8_check_string");

    for buf in utf8_probe_buffers() {
        for len in 0..=buf.len() {
            let p = buf.as_ptr() as *const c_char;
            assert_eq!(
                unsafe { fc(p, len) },
                unsafe { fr(p, len) },
                "utf8_check_string({buf:02x?}, {len})"
            );
        }
    }
}

// ------------------------------------------------------------------- memory

#[test]
fn jsonp_malloc_free_realloc_work() {
    let (c, r) = libs();
    for l in [c, r] {
        let m: Symbol<FnMalloc> = l.sym("jsonp_malloc");
        let re: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void> =
            l.sym("jsonp_realloc");
        let fr: Symbol<FnFree> = l.sym("jsonp_free");

        // jsonp_malloc(0) must return NULL in both
        assert!(unsafe { m(0) }.is_null(), "{}: jsonp_malloc(0)", l.name);

        unsafe {
            let p = m(32) as *mut u8;
            assert!(!p.is_null());
            for i in 0..32 {
                *p.add(i) = i as u8;
            }
            let p2 = re(p as *mut c_void, 32, 64) as *mut u8;
            assert!(!p2.is_null());
            for i in 0..32 {
                assert_eq!(*p2.add(i), i as u8, "{}: realloc kept data", l.name);
            }
            fr(p2 as *mut c_void);
            fr(std::ptr::null_mut());
        }
    }
}

#[test]
fn jsonp_strndup_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnStrndup> = c.sym("jsonp_strndup");
    let fr: Symbol<FnStrndup> = r.sym("jsonp_strndup");
    let freec: Symbol<FnFree> = c.sym("jsonp_free");
    let freer: Symbol<FnFree> = r.sym("jsonp_free");

    for s in [
        &b""[..],
        b"a",
        b"hello",
        b"hello\0world",
        &[0xffu8, 0x00, 0x41][..],
    ] {
        for len in 0..=s.len() {
            unsafe {
                let pc = fc(s.as_ptr() as *const c_char, len);
                let pr = fr(s.as_ptr() as *const c_char, len);
                assert!(!pc.is_null() && !pr.is_null());
                let bc = std::slice::from_raw_parts(pc as *const u8, len + 1);
                let br = std::slice::from_raw_parts(pr as *const u8, len + 1);
                assert_eq!(bc, br, "jsonp_strndup({s:02x?}, {len})");
                assert_eq!(bc[len], 0);
                freec(pc as *mut c_void);
                freer(pr as *mut c_void);
            }
        }
    }
}

// ---------------------------------------------------------------- strbuffer

struct Sb<'a> {
    l: &'a Lib,
    b: StrbufferT,
}

impl<'a> Sb<'a> {
    fn new(l: &'a Lib) -> Self {
        let mut b = StrbufferT::default();
        let f: Symbol<FnStrbufferInit> = l.sym("strbuffer_init");
        let rc = unsafe { f(&mut b) };
        assert_eq!(rc, 0);
        Sb { l, b }
    }
    fn state(&self) -> (usize, usize, Vec<u8>) {
        let v = if self.b.value.is_null() {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(self.b.value as *const u8, self.b.length + 1) }
                .to_vec()
        };
        (self.b.length, self.b.size, v)
    }
}

impl Drop for Sb<'_> {
    fn drop(&mut self) {
        let f: Symbol<FnStrbufferClose> = self.l.sym("strbuffer_close");
        unsafe { f(&mut self.b) };
    }
}

#[test]
fn strbuffer_lifecycle_matches() {
    let (c, r) = libs();
    let mut sc = Sb::new(c);
    let mut sr = Sb::new(r);
    assert_eq!(sc.state(), sr.state(), "after init");

    let apc: Symbol<FnStrbufferAppendBytes> = c.sym("strbuffer_append_bytes");
    let apr: Symbol<FnStrbufferAppendBytes> = r.sym("strbuffer_append_bytes");
    let abc: Symbol<FnStrbufferAppendByte> = c.sym("strbuffer_append_byte");
    let abr: Symbol<FnStrbufferAppendByte> = r.sym("strbuffer_append_byte");
    let popc: Symbol<FnStrbufferPop> = c.sym("strbuffer_pop");
    let popr: Symbol<FnStrbufferPop> = r.sym("strbuffer_pop");
    let clc: Symbol<FnStrbufferClear> = c.sym("strbuffer_clear");
    let clr: Symbol<FnStrbufferClear> = r.sym("strbuffer_clear");
    let vc: Symbol<FnStrbufferValue> = c.sym("strbuffer_value");
    let vr: Symbol<FnStrbufferValue> = r.sym("strbuffer_value");

    // grow across the 16-byte initial size several times
    let mut s: u64 = 99;
    for round in 0..300 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = ((s >> 33) % 40) as usize;
        let data: Vec<u8> = (0..n).map(|i| (i as u8).wrapping_add(round as u8) | 1).collect();
        let a = unsafe { apc(&mut sc.b, data.as_ptr() as *const c_char, n) };
        let b = unsafe { apr(&mut sr.b, data.as_ptr() as *const c_char, n) };
        assert_eq!(a, b, "append_bytes rc round {round}");
        assert_eq!(sc.state(), sr.state(), "append_bytes round {round}");

        let a = unsafe { abc(&mut sc.b, b'x' as c_char) };
        let b = unsafe { abr(&mut sr.b, b'x' as c_char) };
        assert_eq!(a, b, "append_byte rc round {round}");
        assert_eq!(sc.state(), sr.state(), "append_byte round {round}");

        if round % 7 == 0 {
            let a = unsafe { popc(&mut sc.b) };
            let b = unsafe { popr(&mut sr.b) };
            assert_eq!(a, b, "pop round {round}");
            assert_eq!(sc.state(), sr.state(), "pop round {round}");
        }
        if round % 53 == 0 {
            unsafe { clc(&mut sc.b) };
            unsafe { clr(&mut sr.b) };
            assert_eq!(sc.state(), sr.state(), "clear round {round}");
        }
        unsafe {
            let pc = vc(&sc.b);
            let pr = vr(&sr.b);
            assert_eq!(cbytes(pc), cbytes(pr), "value round {round}");
        }
    }

    // pop until empty, then past empty
    loop {
        let a = unsafe { popc(&mut sc.b) };
        let b = unsafe { popr(&mut sr.b) };
        assert_eq!(a, b, "drain pop");
        assert_eq!(sc.state(), sr.state(), "drain pop state");
        if a == 0 {
            break;
        }
    }
    for _ in 0..3 {
        assert_eq!(unsafe { popc(&mut sc.b) }, unsafe { popr(&mut sr.b) });
    }
}

#[test]
fn strbuffer_append_zero_bytes_matches() {
    let (c, r) = libs();
    let mut sc = Sb::new(c);
    let mut sr = Sb::new(r);
    let apc: Symbol<FnStrbufferAppendBytes> = c.sym("strbuffer_append_bytes");
    let apr: Symbol<FnStrbufferAppendBytes> = r.sym("strbuffer_append_bytes");
    unsafe {
        // size == 0 with a NULL data pointer: memcpy(dst, NULL, 0)
        assert_eq!(
            apc(&mut sc.b, std::ptr::null(), 0),
            apr(&mut sr.b, std::ptr::null(), 0)
        );
        assert_eq!(sc.state(), sr.state(), "append 0 bytes");
        // exactly filling to size-1 then one more, to hit the grow boundary
        let data = [b'z' as c_char; 15];
        assert_eq!(apc(&mut sc.b, data.as_ptr(), 15), apr(&mut sr.b, data.as_ptr(), 15));
        assert_eq!(sc.state(), sr.state(), "append 15 into size-16 buffer");
        assert_eq!(apc(&mut sc.b, data.as_ptr(), 1), apr(&mut sr.b, data.as_ptr(), 1));
        assert_eq!(sc.state(), sr.state(), "append past boundary");
        // huge size must fail the overflow guard identically
        assert_eq!(
            apc(&mut sc.b, data.as_ptr(), usize::MAX),
            apr(&mut sr.b, data.as_ptr(), usize::MAX),
            "append SIZE_MAX"
        );
        assert_eq!(sc.state(), sr.state(), "append SIZE_MAX state");
        assert_eq!(
            apc(&mut sc.b, data.as_ptr(), usize::MAX - 1),
            apr(&mut sr.b, data.as_ptr(), usize::MAX - 1),
            "append SIZE_MAX-1"
        );
        assert_eq!(sc.state(), sr.state(), "append SIZE_MAX-1 state");
    }
}

#[test]
fn strbuffer_steal_value_matches() {
    let (c, r) = libs();
    for l in [c, r] {
        let mut b = StrbufferT::default();
        let init: Symbol<FnStrbufferInit> = l.sym("strbuffer_init");
        let app: Symbol<FnStrbufferAppendBytes> = l.sym("strbuffer_append_bytes");
        let steal: Symbol<FnStrbufferStealValue> = l.sym("strbuffer_steal_value");
        let close: Symbol<FnStrbufferClose> = l.sym("strbuffer_close");
        let free: Symbol<FnFree> = l.sym("jsonp_free");
        unsafe {
            assert_eq!(init(&mut b), 0);
            assert_eq!(app(&mut b, b"abc".as_ptr() as *const c_char, 3), 0);
            let p = steal(&mut b);
            assert!(b.value.is_null(), "{}: steal nulls value", l.name);
            assert_eq!(cbytes(p).unwrap(), b"abc", "{}: stolen contents", l.name);
            free(p as *mut c_void);
            close(&mut b); // must tolerate NULL value
            assert_eq!(b.size, 0);
            assert_eq!(b.length, 0);
        }
    }
}

// ------------------------------------------------------------------ version

#[test]
fn version_matches() {
    let (c, r) = libs();
    let sc: Symbol<FnVersionStr> = c.sym("jansson_version_str");
    let sr: Symbol<FnVersionStr> = r.sym("jansson_version_str");
    unsafe {
        assert_eq!(cbytes(sc()), cbytes(sr()), "jansson_version_str");
    }
    let cc: Symbol<FnVersionCmp> = c.sym("jansson_version_cmp");
    let cr: Symbol<FnVersionCmp> = r.sym("jansson_version_cmp");
    for major in -1..4 {
        for minor in -1..20 {
            for micro in -1..4 {
                assert_eq!(
                    unsafe { cc(major, minor, micro) },
                    unsafe { cr(major, minor, micro) },
                    "jansson_version_cmp({major},{minor},{micro})"
                );
            }
        }
    }
}

// -------------------------------------------------------------------- error

type FnErrorSet =
    unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char, ...);
type FnErrorSetSource = unsafe extern "C" fn(*mut JsonError, *const c_char);

#[test]
fn jsonp_error_init_matches() {
    let (c, r) = libs();
    let fc: Symbol<unsafe extern "C" fn(*mut JsonError, *const c_char)> = c.sym("jsonp_error_init");
    let fr: Symbol<unsafe extern "C" fn(*mut JsonError, *const c_char)> = r.sym("jsonp_error_init");

    let sources: Vec<Option<Vec<u8>>> = vec![
        None,
        Some(b"".to_vec()),
        Some(b"x".to_vec()),
        Some(b"<string>".to_vec()),
        Some(b"/some/long/path/that/is/definitely/longer/than/eighty/characters/for/truncation/testing/purposes.json".to_vec()),
        Some(vec![b'a'; 79]),
        Some(vec![b'a'; 80]),
        Some(vec![b'a'; 81]),
        Some(vec![b'a'; 200]),
    ];

    for src in &sources {
        let mut ec = JsonError {
            line: 5,
            column: 6,
            position: 7,
            source: [0x41; JSON_ERROR_SOURCE_LENGTH],
            text: [0x42; JSON_ERROR_TEXT_LENGTH],
        };
        let mut er = ec;
        let (pc, _keep) = match src {
            None => (std::ptr::null::<c_char>(), None),
            Some(v) => {
                let cstr = std::ffi::CString::new(v.clone()).unwrap();
                let p = cstr.as_ptr();
                (p, Some(cstr))
            }
        };
        unsafe {
            fc(&mut ec, pc);
            fr(&mut er, pc);
        }
        assert_eq!(ec.raw(), er.raw(), "jsonp_error_init({src:?})");
        // NULL error pointer must be tolerated
        unsafe {
            fc(std::ptr::null_mut(), pc);
            fr(std::ptr::null_mut(), pc);
        }
    }
}

#[test]
fn jsonp_error_set_source_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnErrorSetSource> = c.sym("jsonp_error_set_source");
    let fr: Symbol<FnErrorSetSource> = r.sym("jsonp_error_set_source");

    for src in [
        "",
        "x",
        "abc",
        "a.b",
        "/path/to/file.json",
        "trailing.",
        ".leading",
        "no_dot_here",
        "many.dots.in.here.json",
    ] {
        let s = cs(src);
        let mut ec = JsonError {
            line: 1,
            column: 2,
            position: 3,
            source: [0x21; JSON_ERROR_SOURCE_LENGTH],
            text: [0x22; JSON_ERROR_TEXT_LENGTH],
        };
        let mut er = ec;
        unsafe {
            fc(&mut ec, s.as_ptr());
            fr(&mut er, s.as_ptr());
        }
        assert_eq!(ec.raw(), er.raw(), "jsonp_error_set_source({src:?})");
        unsafe {
            fc(std::ptr::null_mut(), s.as_ptr());
            fr(std::ptr::null_mut(), s.as_ptr());
            fc(&mut ec, std::ptr::null());
            fr(&mut er, std::ptr::null());
        }
        assert_eq!(ec.raw(), er.raw(), "jsonp_error_set_source NULL source");
    }
}

#[test]
fn jsonp_error_set_matches() {
    let (c, r) = libs();
    let fc: Symbol<FnErrorSet> = c.sym("jsonp_error_set");
    let fr: Symbol<FnErrorSet> = r.sym("jsonp_error_set");

    unsafe {
        for code in [0 as c_int, 1, 7, 12, 127] {
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let fmt = cs("plain message");
            fc(&mut ec, 1, 2, 3, code, fmt.as_ptr());
            fr(&mut er, 1, 2, 3, code, fmt.as_ptr());
            assert_eq!(ec.raw(), er.raw(), "error_set plain code={code}");

            // second call must be ignored ("error already set")
            let fmt2 = cs("second message");
            fc(&mut ec, 99, 98, 97, 3, fmt2.as_ptr());
            fr(&mut er, 99, 98, 97, 3, fmt2.as_ptr());
            assert_eq!(ec.raw(), er.raw(), "error_set already-set code={code}");
        }

        let mut ec = JsonError::default();
        let mut er = JsonError::default();
        let fmt = cs("%s / %d / %.*s / %c / %%");
        let arg = cs("hello");
        let part = cs("abcdef");
        fc(
            &mut ec,
            9,
            8,
            7,
            5,
            fmt.as_ptr(),
            arg.as_ptr(),
            42 as c_int,
            3 as c_int,
            part.as_ptr(),
            b'Q' as c_int,
        );
        fr(
            &mut er,
            9,
            8,
            7,
            5,
            fmt.as_ptr(),
            arg.as_ptr(),
            42 as c_int,
            3 as c_int,
            part.as_ptr(),
            b'Q' as c_int,
        );
        assert_eq!(ec.raw(), er.raw(), "error_set format specifiers");

        // overlong message: must be truncated identically
        for n in [150usize, 156, 157, 158, 159, 160, 161, 200, 400] {
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let long = cs(&"a".repeat(n));
            let fmt = cs("%s");
            fc(&mut ec, 3, 4, 5, 2, fmt.as_ptr(), long.as_ptr());
            fr(&mut er, 3, 4, 5, 2, fmt.as_ptr(), long.as_ptr());
            assert_eq!(ec.raw(), er.raw(), "error_set truncation n={n}");
        }

        // NULL error
        let fmt = cs("x");
        fc(std::ptr::null_mut(), 1, 1, 1, 1, fmt.as_ptr());
        fr(std::ptr::null_mut(), 1, 1, 1, 1, fmt.as_ptr());
    }
}
