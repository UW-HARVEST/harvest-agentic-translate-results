//! Randomised differential fuzzing with a fixed seed.
//!
//! Random byte strings are fed to the parsers, random JSON documents are round
//! tripped through parse/print/minify/duplicate/compare, and random raw strings
//! are escaped by the printers.  Every observable result is compared.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
    fn byte(&mut self) -> u8 {
        (self.next() >> 33) as u8
    }
}

/// A pool of bytes biased towards JSON syntax so that random strings sometimes
/// parse successfully.
const ALPHABET: &[u8] =
    b"{}[]\",:0123456789.eE+-truefalsnl \t\r\n\\/ux\x00\x01\x7f\xc3\xa9\xed\xa0\x80";

fn random_bytes(rng: &mut Rng, max_len: usize) -> Vec<u8> {
    let len = rng.below(max_len + 1);
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        if rng.below(4) == 0 {
            v.push(rng.byte());
        } else {
            let b = ALPHABET[rng.below(ALPHABET.len())];
            v.push(b);
        }
    }
    // drop interior NULs so that the NUL terminated and length based APIs see
    // the same input
    v.retain(|&b| b != 0);
    v.push(0);
    v
}

fn random_json(rng: &mut Rng, depth: usize, out: &mut String) {
    let choice = if depth >= 4 { rng.below(6) } else { rng.below(8) };
    match choice {
        0 => out.push_str("null"),
        1 => out.push_str(if rng.below(2) == 0 { "true" } else { "false" }),
        2 => {
            let kinds = [
                "0", "-0", "1", "-1", "3.5", "-2.25e3", "1e-7", "1e300", "2147483648",
                "-2147483649", "0.1", "123456789", "1e17", "9007199254740993",
            ];
            out.push_str(kinds[rng.below(kinds.len())]);
        }
        3 | 4 => {
            out.push('"');
            let n = rng.below(8);
            for _ in 0..n {
                let pieces = [
                    "a", "Z", "0", " ", "\\\"", "\\\\", "\\/", "\\b", "\\f", "\\n",
                    "\\r", "\\t", "\\u0041", "\\u00e9", "\\u20ac", "\\ud83d\\ude00",
                    "\u{00e9}", "\u{4f60}", "!", "~",
                ];
                out.push_str(pieces[rng.below(pieces.len())]);
            }
            out.push('"');
        }
        5 => {
            out.push('[');
            let n = rng.below(5);
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                random_json(rng, depth + 1, out);
            }
            out.push(']');
        }
        _ => {
            out.push('{');
            let n = rng.below(5);
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                let klen = rng.below(4) + 1;
                for _ in 0..klen {
                    out.push((b'a' + rng.byte() % 26) as char);
                }
                if rng.below(3) == 0 {
                    out.push('A');
                }
                out.push_str("\":");
                random_json(rng, depth + 1, out);
            }
            out.push('}');
        }
    }
}

#[test]
fn fuzz_random_bytes() {
    let _guard = serial();
    let a = apis();
    let mut rng = Rng(0x12345678_9ABCDEF0);
    unsafe {
        for iter in 0..20000 {
            let buf = random_bytes(&mut rng, 48);
            let p = buf.as_ptr() as *const c_char;
            let strlen = buf.len() - 1;

            let mut ce: *const c_char = std::ptr::null();
            let mut re: *const c_char = std::ptr::null();
            let rn = (iter % 2) as c_int;
            let ct = a.c.cJSON_ParseWithOpts(p, &mut ce, rn);
            let cerr = a.c.cJSON_GetErrorPtr();
            let rt = a.rust.cJSON_ParseWithOpts(p, &mut re, rn);
            let rerr = a.rust.cJSON_GetErrorPtr();
            let show = String::from_utf8_lossy(&buf[..strlen]).to_string();
            assert_eq!(ct.is_null(), rt.is_null(), "iter {iter}: Parse({show:?})");
            if !ct.is_null() {
                assert_tree_eq(&format!("iter {iter}: Parse({show:?})"), ct, rt);
            }
            assert_eq!(
                ce as isize - p as isize,
                re as isize - p as isize,
                "iter {iter}: parse_end for {show:?}"
            );
            assert_eq!(
                cerr.is_null(),
                rerr.is_null(),
                "iter {iter}: errptr nullness for {show:?}"
            );
            if !cerr.is_null() {
                assert_eq!(
                    cerr as isize - p as isize,
                    rerr as isize - p as isize,
                    "iter {iter}: errptr for {show:?}"
                );
            }
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);

            // length based parse over a random prefix
            let len = rng.below(strlen + 2);
            let ct = a.c.cJSON_ParseWithLength(p, len);
            let rt = a.rust.cJSON_ParseWithLength(p, len);
            assert_eq!(
                ct.is_null(),
                rt.is_null(),
                "iter {iter}: ParseWithLength({show:?},{len})"
            );
            if !ct.is_null() {
                assert_tree_eq(
                    &format!("iter {iter}: ParseWithLength({show:?},{len})"),
                    ct,
                    rt,
                );
            }
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);

            // minify the same buffer in both libraries
            let mut cbuf = buf.clone();
            let mut rbuf = buf.clone();
            a.c.cJSON_Minify(cbuf.as_mut_ptr() as *mut c_char);
            a.rust.cJSON_Minify(rbuf.as_mut_ptr() as *mut c_char);
            assert_eq!(cbuf, rbuf, "iter {iter}: Minify({show:?})");
        }
    }
}

#[test]
fn fuzz_random_documents() {
    let _guard = serial();
    let a = apis();
    let mut rng = Rng(0xDEADBEEF_CAFEBABE);
    unsafe {
        let mut prev_c: *mut cJSON = std::ptr::null_mut();
        let mut prev_r: *mut cJSON = std::ptr::null_mut();
        for iter in 0..4000 {
            let mut doc = String::new();
            random_json(&mut rng, 0, &mut doc);
            let mut buf: Vec<u8> = doc.as_bytes().to_vec();
            buf.push(0);
            let p = buf.as_ptr() as *const c_char;

            let ct = a.c.cJSON_Parse(p);
            let rt = a.rust.cJSON_Parse(p);
            assert_eq!(ct.is_null(), rt.is_null(), "iter {iter}: parse {doc}");
            if ct.is_null() {
                continue;
            }
            assert_tree_eq(&format!("iter {iter}: {doc}"), ct, rt);

            // buffered printing with a random prebuffer
            let pre = rng.below(80) as c_int;
            let fmt = (rng.below(2)) as c_int;
            let cp = a.c.cJSON_PrintBuffered(ct, pre, fmt);
            let rp = a.rust.cJSON_PrintBuffered(rt, pre, fmt);
            assert_eq!(
                cstr_bytes(cp),
                cstr_bytes(rp),
                "iter {iter}: PrintBuffered(pre={pre},fmt={fmt}) {doc}"
            );
            a.c.cJSON_free(cp as *mut c_void);
            a.rust.cJSON_free(rp as *mut c_void);

            // preallocated printing with an arbitrary length
            let cu = cstr_bytes(a.c.cJSON_PrintUnformatted(ct)).unwrap();
            let needed = cu.len();
            let len = rng.below(needed + 4);
            let mut cbuf = vec![0x33u8; needed + 8];
            let mut rbuf = vec![0x33u8; needed + 8];
            let cr = a.c.cJSON_PrintPreallocated(
                ct,
                cbuf.as_mut_ptr() as *mut c_char,
                len as c_int,
                fmt,
            );
            let rr = a.rust.cJSON_PrintPreallocated(
                rt,
                rbuf.as_mut_ptr() as *mut c_char,
                len as c_int,
                fmt,
            );
            assert_eq!(cr, rr, "iter {iter}: PrintPreallocated len={len} {doc}");
            assert_eq!(cbuf, rbuf, "iter {iter}: PrintPreallocated buffer {doc}");

            // duplicate + compare against the original and the previous document
            let recurse = rng.below(2) as c_int;
            let cd = a.c.cJSON_Duplicate(ct, recurse);
            let rd = a.rust.cJSON_Duplicate(rt, recurse);
            assert_eq!(cd.is_null(), rd.is_null());
            if !cd.is_null() {
                assert_tree_eq(&format!("iter {iter}: duplicate {doc}"), cd, rd);
                for cs in [0, 1] {
                    assert_eq!(
                        a.c.cJSON_Compare(ct, cd, cs),
                        a.rust.cJSON_Compare(rt, rd, cs),
                        "iter {iter}: Compare(orig,dup,{cs}) {doc}"
                    );
                }
            }
            a.c.cJSON_Delete(cd);
            a.rust.cJSON_Delete(rd);

            if !prev_c.is_null() {
                for cs in [0, 1] {
                    assert_eq!(
                        a.c.cJSON_Compare(prev_c, ct, cs),
                        a.rust.cJSON_Compare(prev_r, rt, cs),
                        "iter {iter}: Compare(prev,cur,{cs}) {doc}"
                    );
                }
                a.c.cJSON_Delete(prev_c);
                a.rust.cJSON_Delete(prev_r);
            }
            prev_c = ct;
            prev_r = rt;
        }
        a.c.cJSON_Delete(prev_c);
        a.rust.cJSON_Delete(prev_r);
    }
}

#[test]
fn fuzz_random_strings_and_numbers() {
    let _guard = serial();
    let a = apis();
    let mut rng = Rng(0x0BADC0DE_F00DFACE);
    unsafe {
        for iter in 0..5000 {
            // random NUL free byte string -> CreateString -> print
            let len = rng.below(24);
            let mut s: Vec<u8> = Vec::with_capacity(len + 1);
            for _ in 0..len {
                let mut b = rng.byte();
                if b == 0 {
                    b = 1;
                }
                s.push(b);
            }
            s.push(0);
            let cp = a.c.cJSON_CreateString(s.as_ptr() as *const c_char);
            let rp = a.rust.cJSON_CreateString(s.as_ptr() as *const c_char);
            assert_tree_eq(&format!("iter {iter}: CreateString"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            // random doubles through CreateNumber / SetNumberHelper
            let bits = rng.next();
            let d = f64::from_bits(bits);
            let cp = a.c.cJSON_CreateNumber(d);
            let rp = a.rust.cJSON_CreateNumber(d);
            assert_tree_eq(&format!("iter {iter}: CreateNumber({d:?})"), cp, rp);
            let cv = a.c.cJSON_SetNumberHelper(cp, -d);
            let rv = a.rust.cJSON_SetNumberHelper(rp, -d);
            assert_eq!(cv.to_bits(), rv.to_bits(), "iter {iter}: SetNumberHelper");
            assert_tree_eq(&format!("iter {iter}: SetNumberHelper({d:?})"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            // small random float / int / double arrays
            let n = rng.below(6) as c_int;
            let ints: Vec<c_int> = (0..6).map(|_| rng.next() as c_int).collect();
            let cp = a.c.cJSON_CreateIntArray(ints.as_ptr(), n);
            let rp = a.rust.cJSON_CreateIntArray(ints.as_ptr(), n);
            assert_tree_eq(&format!("iter {iter}: CreateIntArray"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            let floats: Vec<f32> = (0..6).map(|_| f32::from_bits(rng.next() as u32)).collect();
            let cp = a.c.cJSON_CreateFloatArray(floats.as_ptr(), n);
            let rp = a.rust.cJSON_CreateFloatArray(floats.as_ptr(), n);
            assert_tree_eq(&format!("iter {iter}: CreateFloatArray"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);

            let doubles: Vec<f64> = (0..6).map(|_| f64::from_bits(rng.next())).collect();
            let cp = a.c.cJSON_CreateDoubleArray(doubles.as_ptr(), n);
            let rp = a.rust.cJSON_CreateDoubleArray(doubles.as_ptr(), n);
            assert_tree_eq(&format!("iter {iter}: CreateDoubleArray"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }
    }
}
