//! Phase B — CONFIGS.md section A: the lowest-level exported entry points.
//! utf8_*, strbuffer_*, hashtable_*, jsonp_* memory/error/strconv, dtoa family,
//! version.  Every case is driven through both `.so`s and compared.
mod common;

use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

/* ================= A1 utf8_check_first — all 256 bytes ================= */

#[test]
fn a1_utf8_check_first_all_bytes() {
    let _g = lock();
    let p = pair();
    for b in 0u16..256 {
        let bb = b as u8 as c_char;
        unsafe {
            assert_eq!(
                (p.c.utf8_check_first)(bb),
                (p.r.utf8_check_first)(bb),
                "utf8_check_first(0x{b:02x})"
            );
        }
    }
}

/* ================= A2 utf8_check_full ================= */

#[test]
fn a2_utf8_check_full_sizes_and_random() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xA2);
    // exhaustive over the interesting size values, randomized buffers
    for _ in 0..20000 {
        let buf = rng.bytes(6);
        for size in 0..6usize {
            let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
            unsafe {
                let mut cc: i32 = -12345;
                let mut rc: i32 = -12345;
                let a = (p.c.utf8_check_full)(cb.as_ptr(), size, &mut cc);
                let b = (p.r.utf8_check_full)(cb.as_ptr(), size, &mut rc);
                assert_eq!((a, cc), (b, rc), "utf8_check_full({buf:?}, {size})");
                // also with a NULL codepoint out-param
                assert_eq!(
                    (p.c.utf8_check_full)(cb.as_ptr(), size, std::ptr::null_mut()),
                    (p.r.utf8_check_full)(cb.as_ptr(), size, std::ptr::null_mut())
                );
            }
        }
    }
    // hand-built valid sequences of each length + overlong + surrogate + >10FFFF
    let cases: &[&[u8]] = &[
        b"\xc2\x80",
        b"\xdf\xbf",
        b"\xe0\xa0\x80",
        b"\xef\xbf\xbf",
        b"\xf0\x90\x80\x80",
        b"\xf4\x8f\xbf\xbf",
        b"\xc0\x80",             // overlong
        b"\xe0\x80\x80",         // overlong
        b"\xf0\x80\x80\x80",     // overlong
        b"\xed\xa0\x80",         // surrogate D800
        b"\xed\xbf\xbf",         // surrogate DFFF
        b"\xf4\x90\x80\x80",     // > 0x10FFFF
        b"\xc2\x00",             // bad continuation
        b"\xe2\x82\x00",
    ];
    for c in cases {
        let cb: Vec<c_char> = c.iter().map(|&x| x as c_char).collect();
        unsafe {
            let mut cc: i32 = -1;
            let mut rc: i32 = -1;
            assert_eq!(
                ((p.c.utf8_check_full)(cb.as_ptr(), c.len(), &mut cc), cc),
                ((p.r.utf8_check_full)(cb.as_ptr(), c.len(), &mut rc), rc),
                "utf8_check_full {c:?}"
            );
        }
    }
}

/* ================= A3 utf8_encode ================= */

#[test]
fn a3_utf8_encode_all_classes() {
    let _g = lock();
    let p = pair();
    let mut cps: Vec<i32> = vec![
        i32::MIN,
        -1,
        0,
        1,
        0x7f,
        0x80,
        0x7ff,
        0x800,
        0xd7ff,
        0xd800,
        0xdfff,
        0xffff,
        0x10000,
        0x10fffe,
        0x10ffff,
        0x110000,
        0x7fffffff,
    ];
    let mut rng = Rng::new(0xA3);
    for _ in 0..5000 {
        cps.push(rng.range(-100, 0x120000) as i32);
    }
    for cp in cps {
        unsafe {
            let mut cb = [0i8; 8];
            let mut rb = [0i8; 8];
            let mut cs: usize = 0xdead;
            let mut rs: usize = 0xdead;
            let a = (p.c.utf8_encode)(cp, cb.as_mut_ptr(), &mut cs);
            let b = (p.r.utf8_encode)(cp, rb.as_mut_ptr(), &mut rs);
            assert_eq!((a, cs), (b, rs), "utf8_encode({cp:#x}) ret/size");
            if a == 0 {
                assert_eq!(&cb[..cs], &rb[..cs], "utf8_encode({cp:#x}) bytes");
            }
        }
    }
}

/* ================= A4 utf8_iterate ================= */

#[test]
fn a4_utf8_iterate() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xA4);
    for _ in 0..20000 {
        let buf = rng.bytes(5);
        for size in 0..5usize {
            let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
            unsafe {
                let mut cc: i32 = -1;
                let mut rc: i32 = -1;
                let a = (p.c.utf8_iterate)(cb.as_ptr(), size, &mut cc);
                let b = (p.r.utf8_iterate)(cb.as_ptr(), size, &mut rc);
                // compare as offsets from the base (pointers are identical buffers)
                let ao = if a.is_null() {
                    None
                } else {
                    Some(a as usize - cb.as_ptr() as usize)
                };
                let bo = if b.is_null() {
                    None
                } else {
                    Some(b as usize - cb.as_ptr() as usize)
                };
                assert_eq!((ao, cc), (bo, rc), "utf8_iterate({buf:?},{size})");
                assert_eq!(
                    (p.c.utf8_iterate)(cb.as_ptr(), size, std::ptr::null_mut()).is_null(),
                    (p.r.utf8_iterate)(cb.as_ptr(), size, std::ptr::null_mut()).is_null()
                );
            }
        }
    }
}

/* ================= A5 utf8_check_string ================= */

#[test]
fn a5_utf8_check_string() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xA5);
    for _ in 0..20000 {
        let n = rng.below(33);
        let buf = rng.bytes(n);
        let cb: Vec<c_char> = buf.iter().map(|&x| x as c_char).collect();
        unsafe {
            for len in [0usize, n / 2, n] {
                assert_eq!(
                    (p.c.utf8_check_string)(cb.as_ptr(), len),
                    (p.r.utf8_check_string)(cb.as_ptr(), len),
                    "utf8_check_string({buf:?},{len})"
                );
            }
        }
    }
    // valid UTF-8 strings (all lengths), plus truncated versions
    for _ in 0..2000 {
        let s = rng.spicy_string(10);
        let b = s.as_bytes();
        let cb: Vec<c_char> = b.iter().map(|&x| x as c_char).collect();
        unsafe {
            for len in 0..=b.len() {
                assert_eq!(
                    (p.c.utf8_check_string)(cb.as_ptr(), len),
                    (p.r.utf8_check_string)(cb.as_ptr(), len),
                    "utf8_check_string({s:?},{len})"
                );
            }
        }
    }
}

/* ================= A6..A11 strbuffer ================= */

unsafe fn sb_snapshot(api: &Api, sb: &StrbufferT) -> (usize, usize, Vec<u8>) {
    let v = if sb.value.is_null() {
        Vec::new()
    } else {
        unsafe { CStr::from_ptr((api.strbuffer_value)(sb)) }
            .to_bytes()
            .to_vec()
    };
    (sb.length, sb.size, v)
}

#[test]
fn a6_a7_a9_a10_strbuffer_basic() {
    let _g = lock();
    let p = pair();
    for api in [p.c, p.r] {
        let _ = api;
    }
    // run the same script on both and compare the snapshot sequence
    let script = |api: &'static Api| -> Vec<(usize, usize, Vec<u8>)> {
        let mut out = Vec::new();
        unsafe {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            out.push(sb_snapshot(api, &sb)); // A6 fresh
            for i in 0..40u8 {
                assert_eq!((api.strbuffer_append_byte)(&mut sb, (b'a' + i % 26) as c_char), 0);
                out.push(sb_snapshot(api, &sb)); // A7 growth boundaries 15/16/17
            }
            // A9 pop everything and one more
            for _ in 0..41 {
                let c = (api.strbuffer_pop)(&mut sb);
                out.push((sb.length, sb.size, vec![c as u8]));
            }
            // A10 clear then append
            assert_eq!((api.strbuffer_append_bytes)(&mut sb, b"hello\0".as_ptr() as *const c_char, 5), 0);
            out.push(sb_snapshot(api, &sb));
            (api.strbuffer_clear)(&mut sb);
            out.push(sb_snapshot(api, &sb));
            assert_eq!((api.strbuffer_append_bytes)(&mut sb, b"xy\0".as_ptr() as *const c_char, 2), 0);
            out.push(sb_snapshot(api, &sb));
            (api.strbuffer_close)(&mut sb);
            out.push((sb.length, sb.size, vec![sb.value.is_null() as u8]));
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b, "strbuffer basic script");
}

#[test]
fn a8_strbuffer_append_bytes_random() {
    let _g = lock();
    let p = pair();
    let _ = p;
    let script = |api: &'static Api| -> Vec<(usize, usize, u64)> {
        let mut rng = Rng::new(0xA8);
        let mut out = Vec::new();
        unsafe {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            for _ in 0..400 {
                let n = rng.below(300);
                let data = rng.bytes(n);
                let cb: Vec<c_char> = data.iter().map(|&x| x as c_char).collect();
                let r = (api.strbuffer_append_bytes)(&mut sb, cb.as_ptr(), n);
                assert_eq!(r, 0);
                // cheap content digest so the vector stays small
                let mut h: u64 = 1469598103934665603;
                let bytes = std::slice::from_raw_parts(sb.value as *const u8, sb.length);
                for &x in bytes {
                    h ^= x as u64;
                    h = h.wrapping_mul(1099511628211);
                }
                out.push((sb.length, sb.size, h));
            }
            (api.strbuffer_close)(&mut sb);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b, "strbuffer_append_bytes randomized");
}

#[test]
fn a11_strbuffer_steal_value() {
    let _g = lock();
    let script = |api: &'static Api| -> (Vec<u8>, usize, usize, bool) {
        unsafe {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            (api.strbuffer_append_bytes)(&mut sb, b"abcdef\0".as_ptr() as *const c_char, 6);
            let stolen = (api.strbuffer_steal_value)(&mut sb);
            let bytes = CStr::from_ptr(stolen).to_bytes().to_vec();
            let r = (bytes, sb.length, sb.size, sb.value.is_null());
            (api.jsonp_free)(stolen as *mut c_void);
            (api.strbuffer_close)(&mut sb); // value == NULL path
            r
        }
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ================= A12..A15 memory ================= */

#[test]
fn a12_a13_a15_memory() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<u64> {
        let mut out = Vec::new();
        unsafe {
            // A12 jsonp_malloc(0) must be NULL, others non-NULL
            out.push((api.jsonp_malloc)(0).is_null() as u64);
            let p1 = (api.jsonp_malloc)(1);
            out.push(p1.is_null() as u64);
            (api.jsonp_free)(p1);
            let p2 = (api.jsonp_malloc)(4096);
            out.push(p2.is_null() as u64);
            (api.jsonp_free)(p2);
            (api.jsonp_free)(std::ptr::null_mut()); // no-op

            // A13 jsonp_realloc grow / shrink / to-zero (default allocator)
            let mut q = (api.jsonp_malloc)(16);
            std::ptr::write_bytes(q as *mut u8, 0x5a, 16);
            q = (api.jsonp_realloc)(q, 16, 64);
            out.push(q.is_null() as u64);
            out.push(*(q as *const u8) as u64);
            q = (api.jsonp_realloc)(q, 64, 8);
            out.push(q.is_null() as u64);
            let z = (api.jsonp_realloc)(q, 8, 0);
            // glibc realloc(p,0) frees and returns NULL
            out.push(z.is_null() as u64);

            // A15 jsonp_strndup
            for (src, len) in [
                (&b"\0"[..], 0usize),
                (&b"a\0"[..], 1),
                (&b"abcdefghij\0"[..], 10),
                (&b"ab\0cd\0"[..], 5),
            ] {
                let d = (api.jsonp_strndup)(src.as_ptr() as *const c_char, len);
                let bytes = std::slice::from_raw_parts(d as *const u8, len + 1);
                for &x in bytes {
                    out.push(x as u64);
                }
                (api.jsonp_free)(d as *mut c_void);
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ================= A16..A24 hashtable ================= */

unsafe fn ht_walk(api: &Api, ht: *mut HashtableT) -> Vec<(Vec<u8>, usize, i32)> {
    let mut out = Vec::new();
    unsafe {
        let mut it = (api.hashtable_iter)(ht);
        while !it.is_null() {
            let k = (api.hashtable_iter_key)(it);
            let kl = (api.hashtable_iter_key_len)(it);
            let key = std::slice::from_raw_parts(k as *const u8, kl).to_vec();
            let v = (api.hashtable_iter_value)(it) as Jt;
            let t = if v.is_null() { -1 } else { (*v).type_ };
            out.push((key, kl, t));
            it = (api.hashtable_iter_next)(ht, it);
        }
    }
    out
}

#[test]
fn a16_a24_hashtable_full_lifecycle() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<String> {
        let mut out = Vec::new();
        let mut rng = Rng::new(0xB7);
        unsafe {
            for n in [0usize, 1, 7, 8, 9, 64] {
                let mut ht = HashtableT::zeroed();
                assert_eq!((api.hashtable_init)(&mut ht), 0); // A16
                let mut keys: Vec<Vec<u8>> = Vec::new();
                for i in 0..n {
                    let k = format!("key{i:03}").into_bytes();
                    keys.push(k.clone());
                    let v = (api.json_integer)(i as i64);
                    let r = (api.hashtable_set)(&mut ht, k.as_ptr() as *const c_char, k.len(), v);
                    out.push(format!("set {i} -> {r} size={}", ht.size));
                }
                out.push(format!("order={} size={}", ht.order, ht.size)); // A17 rehash
                out.push(format!("{:?}", ht_walk(api, &mut ht))); // A22 insertion order
                // A17 get present + absent
                for k in &keys {
                    let g = (api.hashtable_get)(&mut ht, k.as_ptr() as *const c_char, k.len());
                    out.push(format!("get {:?} -> {}", k, !g.is_null()));
                }
                let absent = b"nope";
                out.push(format!(
                    "get absent -> {}",
                    (api.hashtable_get)(&mut ht, absent.as_ptr() as *const c_char, 4).is_null()
                ));
                // A18 overwrite
                if n > 0 {
                    let k = &keys[n / 2];
                    let v = (api.json_string)(b"replaced\0".as_ptr() as *const c_char);
                    out.push(format!(
                        "overwrite -> {} size={}",
                        (api.hashtable_set)(&mut ht, k.as_ptr() as *const c_char, k.len(), v),
                        ht.size
                    ));
                    out.push(format!("{:?}", ht_walk(api, &mut ht)));
                    // A23 iter_at + resume
                    let it = (api.hashtable_iter_at)(&mut ht, k.as_ptr() as *const c_char, k.len());
                    out.push(format!("iter_at null={}", it.is_null()));
                    let mut cur = it;
                    let mut rest = Vec::new();
                    while !cur.is_null() {
                        let kk = (api.hashtable_iter_key)(cur);
                        let kl = (api.hashtable_iter_key_len)(cur);
                        rest.push(std::slice::from_raw_parts(kk as *const u8, kl).to_vec());
                        cur = (api.hashtable_iter_next)(&mut ht, cur);
                    }
                    out.push(format!("resume {rest:?}"));
                    // A24 iter_set
                    if !it.is_null() {
                        (api.hashtable_iter_set)(it, (api.json_integer)(999));
                        out.push(format!("{:?}", ht_walk(api, &mut ht)));
                    }
                    // A20 del first / middle / last / absent
                    for idx in [0usize, n / 2, n - 1] {
                        let kk = &keys[idx];
                        out.push(format!(
                            "del {idx} -> {} size={}",
                            (api.hashtable_del)(&mut ht, kk.as_ptr() as *const c_char, kk.len()),
                            ht.size
                        ));
                    }
                    out.push(format!(
                        "del absent -> {}",
                        (api.hashtable_del)(&mut ht, absent.as_ptr() as *const c_char, 4)
                    ));
                    out.push(format!("{:?}", ht_walk(api, &mut ht)));
                }
                // A21 clear then reuse
                (api.hashtable_clear)(&mut ht);
                let cleared = ht_walk(api, &mut ht);
                out.push(format!("cleared size={} {:?}", ht.size, cleared));
                let v = (api.json_integer)(7);
                (api.hashtable_set)(&mut ht, b"after".as_ptr() as *const c_char, 5, v);
                out.push(format!("reused {:?}", ht_walk(api, &mut ht)));
                (api.hashtable_close)(&mut ht);
            }
            // A19 keys with embedded NULs / equal prefixes / different key_len
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            let weird: &[&[u8]] = &[
                b"", b"a", b"a\0", b"a\0b", b"ab", b"abc", b"\0", b"\0\0", b"aa",
            ];
            for (i, k) in weird.iter().enumerate() {
                let v = (api.json_integer)(i as i64);
                out.push(format!(
                    "weird set {i} -> {}",
                    (api.hashtable_set)(&mut ht, k.as_ptr() as *const c_char, k.len(), v)
                ));
            }
            out.push(format!("weird {:?}", ht_walk(api, &mut ht)));
            for k in weird {
                out.push(format!(
                    "weird get {:?} -> {}",
                    k,
                    (api.hashtable_get)(&mut ht, k.as_ptr() as *const c_char, k.len()).is_null()
                ));
            }
            (api.hashtable_close)(&mut ht);

            // A17 randomized keys / lengths
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            for _ in 0..500 {
                let n = rng.below(12);
                let k = rng.bytes(n);
                let v = (api.json_integer)(rng.i64());
                (api.hashtable_set)(&mut ht, k.as_ptr() as *const c_char, n, v);
            }
            out.push(format!("rand size={} order={}", ht.size, ht.order));
            out.push(format!("rand walk {:?}", ht_walk(api, &mut ht)));
            (api.hashtable_close)(&mut ht);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "hashtable step {i}");
    }
}

/* ================= A25 seed ================= */

#[test]
fn a25_seed_pinned_identically() {
    let _g = lock();
    let p = pair();
    unsafe {
        // `pair()` already called json_object_seed(TEST_SEED) on both.
        assert_eq!(*p.c.hashtable_seed, *p.r.hashtable_seed);
        assert_eq!(*p.c.hashtable_seed, TEST_SEED as u32);
        // A second call must be a no-op on both (seed already non-zero).
        (p.c.json_object_seed)(0x1111);
        (p.r.json_object_seed)(0x1111);
        assert_eq!(*p.c.hashtable_seed, *p.r.hashtable_seed);
        assert_eq!(*p.c.hashtable_seed, TEST_SEED as u32);
    }
}

/* ================= A26..A28 jsonp_dtostr ================= */

fn dtostr_case(api: &'static Api, v: f64, prec: c_int, size: usize) -> (c_int, Vec<u8>) {
    unsafe {
        let mut buf = vec![0i8; size.max(1) + 8];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (0x7f - (i as i32 & 0x1f)) as c_char; // poison
        }
        let r = (api.jsonp_dtostr)(buf.as_mut_ptr(), size, v, prec);
        let bytes: Vec<u8> = buf.iter().map(|&c| c as u8).collect();
        (r, bytes)
    }
}

#[test]
fn a26_a27_jsonp_dtostr_precision_sweep() {
    let _g = lock();
    let mut rng = Rng::new(0xD0);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1e-5,
        1e-4,
        9.999e-5,
        1e15,
        1e16,
        1e17,
        1e-300,
        1e300,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::MIN,
        3.141592653589793,
        2.718281828459045,
        1.0 / 3.0,
        123456789012345678.0,
        0.1,
        0.2,
        0.3,
    ];
    for _ in 0..400 {
        vals.push(rng.tame_f64());
    }
    for _ in 0..400 {
        vals.push(rng.finite_f64());
    }
    for v in vals {
        for prec in 0..32 {
            let (a, b) = both(|api| dtostr_case(api, v, prec, 25));
            assert_eq!(a, b, "jsonp_dtostr({v:?}, prec={prec}, size=25)");
        }
    }
}

#[test]
fn a28_jsonp_dtostr_buffer_size_threshold() {
    let _g = lock();
    for v in [
        0.0f64,
        -1.0,
        1e300,
        -1e-300,
        1.7976931348623157e308,
        3.141592653589793,
        -5e-324,
    ] {
        for prec in [0, 1, 17, 31] {
            for size in 0..45usize {
                let (a, b) = both(|api| dtostr_case(api, v, prec, size));
                assert_eq!(a, b, "jsonp_dtostr({v:?}, prec={prec}, size={size})");
            }
        }
    }
}

/* ================= A29 jsonp_strtod ================= */

#[test]
fn a29_jsonp_strtod() {
    let _g = lock();
    let mut rng = Rng::new(0xD1);
    let mut lits: Vec<String> = vec![
        "0".into(),
        "-0".into(),
        "1".into(),
        "1.0".into(),
        "-1.5".into(),
        "1e10".into(),
        "1E+10".into(),
        "1e-10".into(),
        "-1.5e-3".into(),
        "0.1".into(),
        "3.141592653589793".into(),
        "2.2250738585072014e-308".into(),
        "1.7976931348623157e308".into(),
        "5e-324".into(),
        "1e308".into(),
        "1e309".into(),
        "-1e309".into(),
        "1e-400".into(),
        "9007199254740993".into(),
        "123456789012345678901234567890".into(),
    ];
    for _ in 0..2000 {
        let m = rng.range(-1_000_000_000, 1_000_000_000);
        let f = rng.below(1_000_000_000);
        let e = rng.range(-330, 330);
        lits.push(match rng.below(4) {
            0 => format!("{m}"),
            1 => format!("{m}.{f}"),
            2 => format!("{m}e{e}"),
            _ => format!("{m}.{f}e{e}"),
        });
    }
    for lit in lits {
        let (a, b) = both(|api| unsafe {
            let mut sb = StrbufferT::zeroed();
            assert_eq!((api.strbuffer_init)(&mut sb), 0);
            (api.strbuffer_append_bytes)(&mut sb, lit.as_ptr() as *const c_char, lit.len());
            let mut out: f64 = -12345.0;
            let r = (api.jsonp_strtod)(&mut sb, &mut out);
            (api.strbuffer_close)(&mut sb);
            (r, out.to_bits())
        });
        assert_eq!(a, b, "jsonp_strtod({lit})");
    }
}

/* ================= A30..A34 dtoa family ================= */

#[test]
fn a30_dtoa_r() {
    let _g = lock();
    let mut rng = Rng::new(0xD2);
    let mut vals: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1e-5,
        1e16,
        1e17,
        f64::MIN_POSITIVE,
        5e-324,
        f64::MAX,
        f64::MIN,
        3.141592653589793,
        1.0 / 3.0,
        1e300,
        1e-300,
    ];
    for _ in 0..300 {
        vals.push(rng.tame_f64());
    }
    for _ in 0..300 {
        vals.push(rng.finite_f64());
    }
    for v in vals {
        for mode in 0..6 {
            for nd in 0..=25 {
                let (a, b) = both(|api| unsafe {
                    let mut buf = [0i8; 64];
                    let mut decpt: c_int = -999;
                    let mut sign: c_int = -999;
                    let mut rve: *mut c_char = std::ptr::null_mut();
                    let r = (api.dtoa_r)(
                        v, mode, nd, &mut decpt, &mut sign, &mut rve, buf.as_mut_ptr(), 25,
                    );
                    let digits = if r.is_null() {
                        None
                    } else {
                        Some(CStr::from_ptr(r).to_bytes().to_vec())
                    };
                    let rvlen = if r.is_null() || rve.is_null() {
                        -1i64
                    } else {
                        (rve as usize - r as usize) as i64
                    };
                    (digits, decpt, sign, rvlen)
                });
                assert_eq!(a, b, "dtoa_r({v:?}, mode={mode}, nd={nd})");
            }
        }
    }
}

#[test]
fn a31_a32_dtoa_and_divmax() {
    let _g = lock();
    let p = pair();
    unsafe {
        assert_eq!(*p.c.dtoa_divmax, *p.r.dtoa_divmax, "dtoa_divmax");
    }
    let mut rng = Rng::new(0xD3);
    let mut vals: Vec<f64> = vec![0.0, -0.0, 1.0, -1.0, 1e300, 5e-324, f64::MAX, 0.1];
    for _ in 0..400 {
        vals.push(rng.tame_f64());
    }
    for _ in 0..200 {
        vals.push(rng.finite_f64());
    }
    for v in vals {
        for mode in 0..4 {
            for nd in [0, 1, 6, 17] {
                let (a, b) = both(|api| unsafe {
                    let mut decpt: c_int = -999;
                    let mut sign: c_int = -999;
                    let mut rve: *mut c_char = std::ptr::null_mut();
                    let r = (api.dtoa)(v, mode, nd, &mut decpt, &mut sign, &mut rve);
                    let digits = if r.is_null() {
                        None
                    } else {
                        Some(CStr::from_ptr(r).to_bytes().to_vec())
                    };
                    if !r.is_null() {
                        (api.freedtoa)(r);
                    }
                    (digits, decpt, sign)
                });
                assert_eq!(a, b, "dtoa({v:?}, mode={mode}, nd={nd})");
            }
        }
    }
}

#[test]
fn a33_gethex() {
    let _g = lock();
    let mut rng = Rng::new(0xD4);
    let mut lits: Vec<String> = vec![
        "0x1p+0".into(),
        "0X1P+0".into(),
        "0x1.8p3".into(),
        "0x0p0".into(),
        "0x0".into(),
        "0x".into(),
        "0xg".into(),
        "0x1".into(),
        "0x1.".into(),
        "0x.8p1".into(),
        "0xfffffffffffffp-52".into(),
        "0x1p-1074".into(),
        "0x1p-1075".into(),
        "0x1p+1024".into(),
        "0x1.fffffffffffffp+1023".into(),
        "0x1p1000000".into(),
        "0x1p-1000000".into(),
    ];
    for _ in 0..2000 {
        let mant: String = (0..1 + rng.below(14))
            .map(|_| b"0123456789abcdefABCDEF"[rng.below(22)] as char)
            .collect();
        let frac: String = (0..rng.below(14))
            .map(|_| b"0123456789abcdef"[rng.below(16)] as char)
            .collect();
        let e = rng.range(-1200, 1200);
        lits.push(match rng.below(3) {
            0 => format!("0x{mant}p{e}"),
            1 => format!("0x{mant}.{frac}p{e}"),
            _ => format!("0x{mant}.{frac}"),
        });
    }
    for lit in lits {
        let z = cstr(&lit);
        for rounding in 0..4 {
            for sign in 0..2 {
                let (a, b) = both(|api| unsafe {
                    let mut sp: *const c_char = z.as_ptr();
                    let mut rv: f64 = -12345.0;
                    (api.gethex)(&mut sp, &mut rv, rounding, sign);
                    let consumed = sp as usize - z.as_ptr() as usize;
                    (rv.to_bits(), consumed)
                });
                assert_eq!(a, b, "gethex({lit}, rounding={rounding}, sign={sign})");
            }
        }
    }
}

#[test]
fn a34_strtod_unused() {
    let _g = lock();
    let mut rng = Rng::new(0xD5);
    let mut lits: Vec<String> = vec![
        "".into(),
        " ".into(),
        "+".into(),
        "-".into(),
        "0".into(),
        "-0".into(),
        "1".into(),
        "  \t 1.5e3xyz".into(),
        "1e".into(),
        "1e+".into(),
        ".5".into(),
        "0x10".into(),
        "0X1p4".into(),
        "inf".into(),
        "nan".into(),
        "1e999".into(),
        "-1e999".into(),
        "1e-999".into(),
        "9007199254740993".into(),
        "0.30000000000000004".into(),
        "1.7976931348623157e308".into(),
        "2.2250738585072014e-308".into(),
        "4.9406564584124654e-324".into(),
    ];
    for _ in 0..3000 {
        let m = rng.range(-1_000_000_000, 1_000_000_000);
        let f = rng.below(1_000_000_000_000);
        let e = rng.range(-340, 340);
        lits.push(match rng.below(5) {
            0 => format!("{m}"),
            1 => format!("{m}.{f}"),
            2 => format!("{m}e{e}"),
            3 => format!("{m}.{f}e{e}"),
            _ => format!("{m}.{f}E{:+}", e),
        });
    }
    for lit in lits {
        let z = cstr(&lit);
        let (a, b) = both(|api| unsafe {
            let mut end: *mut c_char = std::ptr::null_mut();
            let v = (api.strtod__unused)(z.as_ptr(), &mut end);
            let consumed = if end.is_null() {
                -1i64
            } else {
                (end as usize - z.as_ptr() as usize) as i64
            };
            (v.to_bits(), consumed)
        });
        assert_eq!(a, b, "strtod__unused({lit:?})");
    }
}

/* ================= A35, A36 version ================= */

#[test]
fn a35_a36_version() {
    let _g = lock();
    let p = pair();
    unsafe {
        assert_eq!(
            CStr::from_ptr((p.c.jansson_version_str)()),
            CStr::from_ptr((p.r.jansson_version_str)())
        );
    }
    let mut rng = Rng::new(0xD6);
    let mut cases: Vec<(c_int, c_int, c_int)> = vec![
        (2, 15, 0),
        (2, 15, 1),
        (2, 14, 0),
        (2, 16, 0),
        (1, 0, 0),
        (3, 0, 0),
        (0, 0, 0),
        (-1, -1, -1),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
    ];
    for _ in 0..500 {
        cases.push((
            rng.range(-100, 100) as c_int,
            rng.range(-100, 100) as c_int,
            rng.range(-100, 100) as c_int,
        ));
    }
    for (a, b, c) in cases {
        same(&format!("jansson_version_cmp({a},{b},{c})"), |api| unsafe {
            (api.jansson_version_cmp)(a, b, c)
        });
    }
}

/* ================= A37..A39 error struct ================= */

#[test]
fn a37_a38_a39_error_struct() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(c_int, c_int, c_int, Vec<u8>, Vec<u8>)> {
        let mut out = Vec::new();
        unsafe {
            for srclen in [0usize, 1, 5, 78, 79, 80, 81, 200] {
                let src = "S".repeat(srclen);
                let zs = cstr(&src);
                for msglen in [0usize, 1, 10, 157, 158, 159, 300] {
                    for code in [0i32, 1, 8, 17, 200, 255] {
                        let msg = "m".repeat(msglen);
                        let zm = cstr(&msg);
                        let mut e = JsonError::zeroed();
                        (api.jsonp_error_init)(&mut e, if srclen == 0 { std::ptr::null() } else { zs.as_ptr() });
                        out.push(e.snapshot());
                        (api.jsonp_error_set)(&mut e, 3, 4, 5usize, code, b"%s\0".as_ptr() as *const c_char, zm.as_ptr());
                        out.push(e.snapshot());
                        // A39: second set is ignored
                        (api.jsonp_error_set)(&mut e, 9, 9, 9usize, 7, b"other\0".as_ptr() as *const c_char);
                        out.push(e.snapshot());
                        // A38: overwrite the source afterwards
                        (api.jsonp_error_set_source)(&mut e, zs.as_ptr());
                        out.push(e.snapshot());
                        // NULL guards
                        (api.jsonp_error_set_source)(&mut e, std::ptr::null());
                        (api.jsonp_error_init)(std::ptr::null_mut(), zs.as_ptr());
                        (api.jsonp_error_set_source)(std::ptr::null_mut(), zs.as_ptr());
                        (api.jsonp_error_set)(std::ptr::null_mut(), 1, 1, 1usize, 1, b"x\0".as_ptr() as *const c_char);
                        out.push(e.snapshot());
                    }
                }
            }
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(x, y, "error struct step {i}");
    }
}

/* ================= A40 jsonp_loop_check ================= */

#[test]
fn a40_jsonp_loop_check() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(c_int, usize, usize)> {
        let mut out = Vec::new();
        unsafe {
            let mut ht = HashtableT::zeroed();
            assert_eq!((api.hashtable_init)(&mut ht), 0);
            let a = (api.json_object)();
            let b = (api.json_array)();
            let mut key = [0i8; 32];
            let mut klen: usize = 0;
            // first insert succeeds
            let r1 = (api.jsonp_loop_check)(&mut ht, a, key.as_mut_ptr(), 32, &mut klen);
            out.push((r1, klen, ht.size));
            // same pointer again -> -1
            let r2 = (api.jsonp_loop_check)(&mut ht, a, key.as_mut_ptr(), 32, &mut klen);
            out.push((r2, klen, ht.size));
            // different pointer -> 0
            let r3 = (api.jsonp_loop_check)(&mut ht, b, key.as_mut_ptr(), 32, &mut klen);
            out.push((r3, klen, ht.size));
            // NULL key_len_out is allowed
            let r4 = (api.jsonp_loop_check)(&mut ht, b, key.as_mut_ptr(), 32, std::ptr::null_mut());
            out.push((r4, 0, ht.size));
            (api.hashtable_close)(&mut ht);
            decref(api, a);
            decref(api, b);
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}

/* ================= A41 jsonp_stringn_nocheck_own ================= */

#[test]
fn a41_stringn_nocheck_own() {
    let _g = lock();
    let script = |api: &'static Api| -> Vec<(usize, Vec<u8>, Option<Vec<u8>>)> {
        let mut out = Vec::new();
        unsafe {
            for src in [&b""[..], &b"x"[..], &b"hello world"[..], &b"a\0b"[..]] {
                let len = src.len();
                let buf = (api.jsonp_malloc)(len + 1) as *mut u8;
                std::ptr::copy_nonoverlapping(src.as_ptr(), buf, len);
                *buf.add(len) = 0;
                let s = (api.jsonp_stringn_nocheck_own)(buf as *const c_char, len);
                assert!(!s.is_null());
                let vlen = (api.json_string_length)(s);
                let v = std::slice::from_raw_parts((api.json_string_value)(s) as *const u8, vlen).to_vec();
                let d = dumps(api, s, JSON_ENCODE_ANY);
                out.push((vlen, v, d));
                decref(api, s);
            }
            // NULL -> NULL
            out.push((
                0,
                vec![(api.jsonp_stringn_nocheck_own)(std::ptr::null(), 0).is_null() as u8],
                None,
            ));
        }
        out
    };
    let (a, b) = both(script);
    assert_eq!(a, b);
}
