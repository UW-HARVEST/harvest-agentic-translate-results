//! Phase B — differential tests for the LOWEST-LEVEL exported entry points
//! (`_pcre2_*_8`), driven through the `.so` exports of both implementations.
mod common;

use common::*;
use std::ffi::c_void;

const SEED: u64 = 0xC0FFEE_1234_5678;

// ============================================================ string_utils
#[test]
fn strlen_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED);
    for _ in 0..2000 {
        let n = g.below(64) as usize;
        // non-zero bytes then NUL
        let mut v: Vec<u8> = (0..n).map(|_| g.range(1, 256) as u8).collect();
        v.push(0);
        unsafe {
            assert_eq!(
                (c.strlen)(v.as_ptr()),
                (r.strlen)(v.as_ptr()),
                "strlen len={}",
                n
            );
            assert_eq!((c.strlen)(v.as_ptr()), n, "strlen value");
        }
    }
}

#[test]
fn strlen_empty() {
    let (c, r) = both();
    let v = [0u8];
    unsafe {
        assert_eq!((c.strlen)(v.as_ptr()), (r.strlen)(v.as_ptr()));
        assert_eq!((c.strlen)(v.as_ptr()), 0);
    }
}

/// Build a random NUL-terminated string (bytes 1..=255).
fn rand_cstr(g: &mut Rng, maxlen: u32) -> Vec<u8> {
    let n = g.below(maxlen) as usize;
    let mut v: Vec<u8> = (0..n).map(|_| g.range(1, 256) as u8).collect();
    v.push(0);
    v
}

#[test]
fn strcmp_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 1);
    for _ in 0..4000 {
        let a = rand_cstr(&mut g, 12);
        // half the time make b a mutation of a, to hit the equal/near-equal paths
        let b = if g.bool() {
            let mut b = a.clone();
            if b.len() > 1 && g.bool() {
                let i = g.below(b.len() as u32 - 1) as usize;
                b[i] = g.range(1, 256) as u8;
            }
            b
        } else {
            rand_cstr(&mut g, 12)
        };
        unsafe {
            assert_eq!(
                (c.strcmp)(a.as_ptr(), b.as_ptr()),
                (r.strcmp)(a.as_ptr(), b.as_ptr()),
                "strcmp {:?} {:?}",
                a,
                b
            );
        }
    }
}

#[test]
fn strcmp_c8_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 2);
    for _ in 0..4000 {
        let a = rand_cstr(&mut g, 12);
        let b = if g.bool() { a.clone() } else { rand_cstr(&mut g, 12) };
        unsafe {
            assert_eq!(
                (c.strcmp_c8)(a.as_ptr(), b.as_ptr() as *const _),
                (r.strcmp_c8)(a.as_ptr(), b.as_ptr() as *const _),
                "strcmp_c8 {:?} {:?}",
                a,
                b
            );
        }
    }
}

#[test]
fn strncmp_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 3);
    for _ in 0..4000 {
        // fixed-size buffers so any len up to 16 is in bounds (incl. embedded NULs)
        let a: Vec<u8> = (0..16).map(|_| g.next_u32() as u8).collect();
        let mut b: Vec<u8> = (0..16).map(|_| g.next_u32() as u8).collect();
        if g.bool() {
            b = a.clone();
        }
        let len = g.below(17) as usize; // includes 0
        unsafe {
            assert_eq!(
                (c.strncmp)(a.as_ptr(), b.as_ptr(), len),
                (r.strncmp)(a.as_ptr(), b.as_ptr(), len),
                "strncmp len={} {:?} {:?}",
                len,
                a,
                b
            );
        }
    }
}

#[test]
fn strncmp_c8_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 4);
    for _ in 0..4000 {
        let a: Vec<u8> = (0..16).map(|_| g.next_u32() as u8).collect();
        let mut b: Vec<u8> = (0..16).map(|_| g.next_u32() as u8).collect();
        if g.bool() {
            b = a.clone();
        }
        let len = g.below(17) as usize;
        unsafe {
            assert_eq!(
                (c.strncmp_c8)(a.as_ptr(), b.as_ptr() as *const _, len),
                (r.strncmp_c8)(a.as_ptr(), b.as_ptr() as *const _, len),
                "strncmp_c8 len={}",
                len
            );
        }
    }
}

#[test]
fn strcpy_c8_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 5);
    for _ in 0..2000 {
        let src = rand_cstr(&mut g, 40);
        let mut cb = [0xAAu8; 64];
        let mut rb = [0xAAu8; 64];
        unsafe {
            let cn = (c.strcpy_c8)(cb.as_mut_ptr(), src.as_ptr() as *const _);
            let rn = (r.strcpy_c8)(rb.as_mut_ptr(), src.as_ptr() as *const _);
            assert_eq!(cn, rn, "strcpy_c8 return");
            assert_eq!(cb, rb, "strcpy_c8 buffer");
        }
    }
}

// ================================================================= ord2utf
#[test]
fn ord2utf_all_codepoints() {
    let (c, r) = both();
    // exhaustive over the whole Unicode range plus a bit past it
    for cp in 0u32..=0x11_0000 {
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            let cn = (c.ord2utf)(cp, cb.as_mut_ptr());
            let rn = (r.ord2utf)(cp, rb.as_mut_ptr());
            assert_eq!(cn, rn, "ord2utf({:#x}) length", cp);
            assert_eq!(cb, rb, "ord2utf({:#x}) bytes", cp);
        }
    }
}

#[test]
fn ord2utf_beyond_unicode() {
    // C accepts any uint32_t; check the >0x10FFFF encodings agree too.
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 6);
    for _ in 0..20000 {
        let cp = g.next_u32();
        let mut cb = [0xAAu8; 8];
        let mut rb = [0xAAu8; 8];
        unsafe {
            let cn = (c.ord2utf)(cp, cb.as_mut_ptr());
            let rn = (r.ord2utf)(cp, rb.as_mut_ptr());
            assert_eq!(cn, rn, "ord2utf({:#x}) length", cp);
            assert_eq!(cb, rb, "ord2utf({:#x}) bytes", cp);
        }
    }
}

// =============================================================== valid_utf
fn check_valid_utf(label: &str, bytes: &[u8]) {
    let (c, r) = both();
    unsafe {
        let mut co = usize::MAX;
        let mut ro = usize::MAX;
        let cr = (c.valid_utf)(bytes.as_ptr(), bytes.len(), &mut co);
        let rr = (r.valid_utf)(bytes.as_ptr(), bytes.len(), &mut ro);
        assert_eq!(cr, rr, "{}: valid_utf rc for {:02x?}", label, bytes);
        assert_eq!(co, ro, "{}: valid_utf offset for {:02x?}", label, bytes);
    }
}

#[test]
fn valid_utf_all_single_bytes() {
    for b in 0u32..256 {
        check_valid_utf("single", &[b as u8]);
    }
}

#[test]
fn valid_utf_all_two_byte_pairs() {
    // exhaustive 2-byte space: 65536 cases, hits every ERR path for lead bytes
    for a in 0u32..256 {
        for b in 0u32..256 {
            check_valid_utf("pair", &[a as u8, b as u8]);
        }
    }
}

#[test]
fn valid_utf_random_sequences() {
    let mut g = Rng::new(SEED ^ 7);
    for _ in 0..30000 {
        let n = g.below(12) as usize;
        let v: Vec<u8> = (0..n).map(|_| g.next_u32() as u8).collect();
        check_valid_utf("rand", &v);
    }
}

#[test]
fn valid_utf_biased_lead_bytes() {
    // Bias towards multi-byte lead bytes so 3- and 4-byte paths (and the
    // surrogate / overlong / too-big checks) are reached often.
    let leads: [u8; 12] = [
        0xC0, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xED, 0xEF, 0xF0, 0xF4, 0xF5, 0xFF,
    ];
    let conts: [u8; 6] = [0x80, 0x8F, 0xA0, 0xBF, 0x7F, 0xC0];
    let mut g = Rng::new(SEED ^ 8);
    for _ in 0..40000 {
        let mut v = Vec::new();
        let groups = g.range(1, 4);
        for _ in 0..groups {
            v.push(*g.pick(&leads));
            let ncont = g.below(4);
            for _ in 0..ncont {
                v.push(*g.pick(&conts));
            }
        }
        check_valid_utf("biased", &v);
    }
}

#[test]
fn valid_utf_valid_strings() {
    // Well-formed UTF-8 built from real codepoints must be accepted by both.
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 9);
    for _ in 0..5000 {
        let mut v = Vec::new();
        for _ in 0..g.below(10) {
            let cp = match g.below(4) {
                0 => g.below(0x80),
                1 => g.range(0x80, 0x800),
                2 => {
                    let x = g.range(0x800, 0x10000);
                    if (0xD800..0xE000).contains(&x) {
                        0x800
                    } else {
                        x
                    }
                }
                _ => g.range(0x10000, 0x110000),
            };
            let ch = char::from_u32(cp).unwrap_or('a');
            let mut buf = [0u8; 4];
            v.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
        }
        unsafe {
            let mut co = usize::MAX;
            let mut ro = usize::MAX;
            let cr = (c.valid_utf)(v.as_ptr(), v.len(), &mut co);
            let rr = (r.valid_utf)(v.as_ptr(), v.len(), &mut ro);
            assert_eq!(cr, 0, "valid string rejected by C: {:02x?}", v);
            assert_eq!(cr, rr);
            assert_eq!(co, ro);
        }
    }
}

#[test]
fn valid_utf_zero_length() {
    check_valid_utf("zero", &[]);
}

// ================================================================= newline
// PCRE2_NEWLINE_* : CR=1 LF=2 CRLF=3 ANY=4 ANYCRLF=5 NUL=6 (plus invalid 0, 7)
const NLTYPES: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

#[test]
fn is_newline_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 10);
    let alpha: [u8; 10] = [b'a', b'\r', b'\n', 0, 0x0B, 0x0C, 0x85, 0xE2, 0x80, 0xA8];
    for _ in 0..20000 {
        let n = g.range(1, 10) as usize;
        let v = g.bytes_from(n, &alpha);
        let ptr_at = g.below(n as u32) as usize;
        let nltype = *g.pick(&NLTYPES);
        let utf = g.bool() as i32;
        unsafe {
            let p = v.as_ptr().add(ptr_at);
            let end = v.as_ptr().add(n);
            let mut cl = 0u32;
            let mut rl = 0u32;
            let cr = (c.is_newline)(p, nltype, end, &mut cl, utf);
            let rr = (r.is_newline)(p, nltype, end, &mut rl, utf);
            assert_eq!(
                cr, rr,
                "is_newline rc nltype={} utf={} at={} {:02x?}",
                nltype, utf, ptr_at, v
            );
            // nllen is only meaningful when a newline was found
            if cr != 0 {
                assert_eq!(cl, rl, "is_newline nllen nltype={}", nltype);
            }
        }
    }
}

#[test]
fn was_newline_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 11);
    let alpha: [u8; 10] = [b'a', b'\r', b'\n', 0, 0x0B, 0x0C, 0x85, 0xE2, 0x80, 0xA8];
    for _ in 0..20000 {
        let n = g.range(1, 10) as usize;
        let v = g.bytes_from(n, &alpha);
        let ptr_at = g.range(1, n as u32 + 1) as usize; // 1..=n
        let nltype = *g.pick(&NLTYPES);
        let utf = g.bool() as i32;
        unsafe {
            let p = v.as_ptr().add(ptr_at);
            let start = v.as_ptr();
            let mut cl = 0u32;
            let mut rl = 0u32;
            let cr = (c.was_newline)(p, nltype, start, &mut cl, utf);
            let rr = (r.was_newline)(p, nltype, start, &mut rl, utf);
            assert_eq!(
                cr, rr,
                "was_newline rc nltype={} utf={} at={} {:02x?}",
                nltype, utf, ptr_at, v
            );
            if cr != 0 {
                assert_eq!(cl, rl, "was_newline nllen nltype={}", nltype);
            }
        }
    }
}

#[test]
fn newline_utf_multibyte() {
    // NEL (U+0085 = C2 85) and LS (U+2028 = E2 80 A8) are newlines under ANY+UTF
    let (c, r) = both();
    let cases: [&[u8]; 6] = [
        b"\xc2\x85",
        b"\xe2\x80\xa8",
        b"\xe2\x80\xa9",
        b"a\xc2\x85b",
        b"\r\n",
        b"\x0b",
    ];
    for v in cases {
        for nltype in NLTYPES {
            for utf in [0, 1] {
                for at in 0..v.len() {
                    unsafe {
                        let p = v.as_ptr().add(at);
                        let end = v.as_ptr().add(v.len());
                        let mut cl = 0u32;
                        let mut rl = 0u32;
                        let cr = (c.is_newline)(p, nltype, end, &mut cl, utf);
                        let rr = (r.is_newline)(p, nltype, end, &mut rl, utf);
                        assert_eq!(
                            cr, rr,
                            "is_newline {:02x?} at={} nltype={} utf={}",
                            v, at, nltype, utf
                        );
                        if cr != 0 {
                            assert_eq!(cl, rl);
                        }
                    }
                }
            }
        }
    }
}

// ================================================================ ckd_smul
#[test]
fn ckd_smul_random_and_boundaries() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 12);
    let mut cases: Vec<(i32, i32)> = vec![
        (0, 0),
        (1, 1),
        (-1, -1),
        (i32::MAX, 2),
        (2, i32::MAX),
        (i32::MIN, -1),
        (-1, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        (65536, 65536),
        (65535, 65537),
        (-65536, 65536),
        (i32::MAX, 1),
        (1, i32::MIN),
    ];
    for _ in 0..20000 {
        cases.push((g.next_u32() as i32, g.next_u32() as i32));
    }
    for _ in 0..20000 {
        // small values: the non-overflowing path
        cases.push((g.range(0, 1000) as i32, g.range(0, 1000) as i32));
    }
    for (a, b) in cases {
        unsafe {
            let mut cv = 0xDEAD_BEEFusize;
            let mut rv = 0xDEAD_BEEFusize;
            let cr = (c.ckd_smul)(&mut cv, a, b);
            let rr = (r.ckd_smul)(&mut rv, a, b);
            assert_eq!(cr, rr, "ckd_smul({}, {}) rc", a, b);
            assert_eq!(cv, rv, "ckd_smul({}, {}) out", a, b);
        }
    }
}

// ============================================================ memctl_malloc
#[repr(C)]
struct Memctl {
    malloc: Option<unsafe extern "C" fn(SIZE, *mut c_void) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    memory_data: *mut c_void,
}

static ALLOC_CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

unsafe extern "C" fn tracking_malloc(n: SIZE, _d: *mut c_void) -> *mut c_void {
    ALLOC_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    libc_malloc(n)
}
unsafe extern "C" fn tracking_free(p: *mut c_void, _d: *mut c_void) {
    libc_free(p)
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn memctl_malloc_copies_control_block() {
    let (c, r) = both();
    for size in [1usize, 8, 64, 1000] {
        unsafe {
            for api in [c, r] {
                let mut mc = Memctl {
                    malloc: Some(tracking_malloc),
                    free: Some(tracking_free),
                    memory_data: 0x1234 as *mut c_void,
                };
                let before = ALLOC_CALLS.load(std::sync::atomic::Ordering::Relaxed);
                let p = (api.memctl_malloc)(
                    size + std::mem::size_of::<Memctl>(),
                    &mut mc as *mut _ as *mut c_void,
                );
                assert!(!p.is_null(), "{}: memctl_malloc null", api.name);
                assert_eq!(
                    ALLOC_CALLS.load(std::sync::atomic::Ordering::Relaxed),
                    before + 1,
                    "{}: malloc call count", api.name
                );
                // the memctl block must have been copied to the head of the block
                let head = &*(p as *const Memctl);
                assert_eq!(
                    head.memory_data, mc.memory_data,
                    "{}: memory_data copied",
                    api.name
                );
                assert!(head.malloc.is_some() && head.free.is_some());
                tracking_free(p, std::ptr::null_mut());
            }
        }
    }
}

// ============================================================= script_run
#[test]
fn script_run_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 13);
    // mix of Latin, Greek, Cyrillic, Han, digits, common punctuation
    let cps: [u32; 20] = [
        0x41, 0x61, 0x7A, 0x30, 0x39, 0x2E, 0x5F, 0x391, 0x3B1, 0x410, 0x430,
        0x4E00, 0x6F22, 0x5B57, 0x660, 0x6F0, 0x3099, 0x30A2, 0x1E00, 0x2018,
    ];
    for _ in 0..8000 {
        let n = g.below(8) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            let cp = *g.pick(&cps);
            let ch = char::from_u32(cp).unwrap();
            let mut b = [0u8; 4];
            v.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
        }
        let utf = g.bool() as i32;
        unsafe {
            let p = v.as_ptr();
            let end = v.as_ptr().add(v.len());
            let cr = (c.script_run)(p, end, utf);
            let rr = (r.script_run)(p, end, utf);
            assert_eq!(cr, rr, "script_run utf={} {:02x?}", utf, v);
        }
    }
}

#[test]
fn script_run_latin1_bytes() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 14);
    for _ in 0..8000 {
        let n = g.below(8) as usize;
        let v: Vec<u8> = (0..n).map(|_| g.next_u32() as u8).collect();
        unsafe {
            let p = v.as_ptr();
            let end = v.as_ptr().add(v.len());
            // non-UTF mode: raw bytes are code points
            let cr = (c.script_run)(p, end, 0);
            let rr = (r.script_run)(p, end, 0);
            assert_eq!(cr, rr, "script_run non-utf {:02x?}", v);
        }
    }
}

// ================================================================== extuni
#[test]
fn extuni_random() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 15);
    // codepoints spanning many grapheme-break classes
    let cps: [u32; 24] = [
        0x41, 0x61, 0x30, 0x20, 0x0A, 0x0D, 0x300, 0x1100, 0x1160, 0x11A8,
        0xAC00, 0xAE4C, 0x200D, 0x261D, 0x1F1E6, 0x1F1E7, 0x1F466, 0x1F3FB,
        0x600, 0x903, 0x94D, 0x1F600, 0xFE0F, 0x20E3,
    ];
    for _ in 0..8000 {
        let n = g.range(1, 8) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            let ch = char::from_u32(*g.pick(&cps)).unwrap();
            let mut b = [0u8; 4];
            v.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
        }
        // first char
        let first = {
            let s = std::str::from_utf8(&v).unwrap();
            s.chars().next().unwrap() as u32
        };
        let firstlen = char::from_u32(first).unwrap().len_utf8();
        let utf = 1i32;
        unsafe {
            let start = v.as_ptr();
            let eptr = start.add(firstlen);
            let end = start.add(v.len());
            let mut cx = -1i32;
            let mut rx = -1i32;
            let cp = (c.extuni)(first, eptr, start, end, utf, &mut cx);
            let rp = (r.extuni)(first, eptr, start, end, utf, &mut rx);
            let coff = cp as usize - start as usize;
            let roff = rp as usize - start as usize;
            assert_eq!(coff, roff, "extuni offset for {:02x?}", v);
            assert_eq!(cx, rx, "extuni xcount for {:02x?}", v);
        }
    }
}

#[test]
fn extuni_non_utf() {
    let (c, r) = both();
    let mut g = Rng::new(SEED ^ 16);
    for _ in 0..8000 {
        let n = g.range(1, 8) as usize;
        let v: Vec<u8> = (0..n).map(|_| g.next_u32() as u8).collect();
        unsafe {
            let start = v.as_ptr();
            let eptr = start.add(1);
            let end = start.add(v.len());
            let mut cx = -1i32;
            let mut rx = -1i32;
            let cp = (c.extuni)(v[0] as u32, eptr, start, end, 0, &mut cx);
            let rp = (r.extuni)(v[0] as u32, eptr, start, end, 0, &mut rx);
            assert_eq!(
                cp as usize - start as usize,
                rp as usize - start as usize,
                "extuni non-utf offset {:02x?}",
                v
            );
            assert_eq!(cx, rx, "extuni non-utf xcount {:02x?}", v);
        }
    }
}

// =========================================================== find_bracket
#[test]
fn find_bracket_on_real_compiled_patterns() {
    // Drive _pcre2_find_bracket over the compiled bytecode of real patterns.
    let (c, r) = both();
    let pats: [&[u8]; 10] = [
        b"(a)(b)(c)",
        b"(?<x>a)(?<y>b)",
        b"(a(b(c)))",
        b"a|(b)|(c)",
        b"(?:x)(y)",
        b"(a)+(b)*",
        b"((((a))))",
        b"(?<n1>a)(?<n2>(?<n3>b))",
        b"(a)(?i)(b)",
        b"x(?=(a))(b)",
    ];
    unsafe {
        for pat in pats {
            for number in -1i32..=8 {
                for capture in [0i32, 1] {
                    let mut offs = [0usize; 2];
                    let mut codes = [std::ptr::null_mut(); 2];
                    for (i, api) in [c, r].iter().enumerate() {
                        let mut ec = 0;
                        let mut eo = 0usize;
                        let code = (api.compile)(
                            pat.as_ptr(),
                            pat.len(),
                            0,
                            &mut ec,
                            &mut eo,
                            std::ptr::null_mut(),
                        );
                        assert!(!code.is_null(), "compile {:?}", pat);
                        codes[i] = code;
                        // bytecode starts at offsetof(pcre2_real_code, ...) —
                        // use pattern_info(PCRE2_INFO_SIZE) region: instead we
                        // locate the code start via the documented layout by
                        // asking for the first bracket from the code pointer.
                        let start = code_start(api, code);
                        let p = (api.find_bracket)(start, capture, number);
                        offs[i] = if p.is_null() {
                            usize::MAX
                        } else {
                            p as usize - start as usize
                        };
                    }
                    assert_eq!(
                        offs[0], offs[1],
                        "find_bracket {:?} number={} capture={}",
                        pat, number, capture
                    );
                    (c.code_free)(codes[0]);
                    (r.code_free)(codes[1]);
                }
            }
        }
    }
}

/// Byte offset of the compiled bytecode inside a `pcre2_real_code`.
/// Obtained from PCRE2_INFO_SIZE minus the code length is not available, so we
/// use the fact that both implementations use the identical `#[repr(C)]` layout
/// and read the documented `code_start` field (PCRE2_INFO_NAMECOUNT etc. are
/// after it). The bytecode begins right after the name table.
unsafe fn code_start(api: &Api, code: *mut c_void) -> SPTR {
    // PCRE2_INFO_NAMECOUNT = 17, PCRE2_INFO_NAMEENTRYSIZE = 18,
    // PCRE2_INFO_NAMETABLE = 19
    let mut nametable: SPTR = std::ptr::null();
    let mut namecount: u32 = 0;
    let mut entrysize: u32 = 0;
    assert_eq!(
        (api.pattern_info)(code, 19, &mut nametable as *mut _ as *mut c_void),
        0
    );
    assert_eq!(
        (api.pattern_info)(code, 17, &mut namecount as *mut _ as *mut c_void),
        0
    );
    assert_eq!(
        (api.pattern_info)(code, 18, &mut entrysize as *mut _ as *mut c_void),
        0
    );
    nametable.add((namecount * entrysize) as usize)
}

// ============================================================= get_error_message
#[test]
fn get_error_message_all_codes() {
    let (c, r) = both();
    unsafe {
        // every plausible error number, plus far out-of-range values
        let mut codes: Vec<i32> = (-200..=200).collect();
        codes.extend_from_slice(&[i32::MIN, i32::MIN + 1, i32::MAX, -1000, 1000]);
        for code in codes {
            for bufsize in [0usize, 1, 2, 8, 64, 256] {
                let mut cb = vec![0xAAu8; bufsize.max(1) + 8];
                let mut rb = vec![0xAAu8; bufsize.max(1) + 8];
                let cr = (c.get_error_message)(code, cb.as_mut_ptr(), bufsize);
                let rr = (r.get_error_message)(code, rb.as_mut_ptr(), bufsize);
                assert_eq!(cr, rr, "get_error_message({}, buf={}) rc", code, bufsize);
                assert_eq!(
                    cb, rb,
                    "get_error_message({}, buf={}) buffer",
                    code, bufsize
                );
            }
        }
    }
}

// ================================================================ maketables
#[test]
fn maketables_matches_and_equals_default_tables() {
    let (c, r) = both();
    unsafe {
        let ct = (c.maketables)(std::ptr::null_mut());
        let rt = (r.maketables)(std::ptr::null_mut());
        assert!(!ct.is_null() && !rt.is_null());
        // tables_length for 8-bit: 3 * 256 (lcc/fcc/cbits offsets) — the real
        // size is cbits_offset + cbit_length + 256 = 1088 in PCRE2 10.x.
        let n = 1088usize;
        let cs = std::slice::from_raw_parts(ct, n);
        let rs = std::slice::from_raw_parts(rt, n);
        assert_eq!(cs, rs, "maketables bytes differ");

        // must also equal the exported default tables of the same library
        let cd = c.data_ptr("_pcre2_default_tables_8");
        let rd = r.data_ptr("_pcre2_default_tables_8");
        let cds = std::slice::from_raw_parts(cd, n);
        let rds = std::slice::from_raw_parts(rd, n);
        assert_eq!(cds, rds, "default_tables differ between C and Rust");
        assert_eq!(cs, cds, "C maketables != C default_tables");
        assert_eq!(rs, rds, "Rust maketables != Rust default_tables");

        (c.maketables_free)(std::ptr::null_mut(), ct);
        (r.maketables_free)(std::ptr::null_mut(), rt);
    }
}

// ============================================================== data tables
/// Every exported pointer-free data symbol, with its size as reported by
/// `nm -SD` on the C `.so` (identical in the Rust `.so`).
pub const DATA_TABLES: [(&str, usize); 27] = [
    ("_pcre2_OP_lengths_8", 173),
    ("_pcre2_callout_end_delims_8", 36),
    ("_pcre2_callout_start_delims_8", 36),
    ("_pcre2_default_tables_8", 1088),
    ("_pcre2_hspace_list_8", 80),
    ("_pcre2_posix_class_maps8", 168),
    ("_pcre2_ucd_boolprop_sets_8", 1528),
    ("_pcre2_ucd_caseless_sets_8", 472),
    ("_pcre2_ucd_digit_sets_8", 312),
    ("_pcre2_ucd_nocase_ranges_8", 336),
    ("_pcre2_ucd_nocase_ranges_size_8", 4),
    ("_pcre2_ucd_records_8", 18756),
    ("_pcre2_ucd_script_sets_8", 1904),
    ("_pcre2_ucd_stage1_8", 17408),
    ("_pcre2_ucd_stage2_8", 80384),
    ("_pcre2_ucd_turkish_dotted_i_caseset_8", 4),
    ("_pcre2_ucp_gbtable_8", 60),
    ("_pcre2_ucp_gentype_8", 120),
    ("_pcre2_utf8_table1", 24),
    ("_pcre2_utf8_table1_size", 4),
    ("_pcre2_utf8_table2", 24),
    ("_pcre2_utf8_table3", 24),
    ("_pcre2_utf8_table4", 64),
    ("_pcre2_utt_8", 3108),
    ("_pcre2_utt_names_8", 3834),
    ("_pcre2_utt_size_8", 8),
    ("_pcre2_vspace_list_8", 32),
];

#[test]
fn exported_data_tables_are_byte_identical() {
    let (c, r) = both();
    unsafe {
        for (name, len) in DATA_TABLES {
            let cp = c.data_ptr(name);
            let rp = r.data_ptr(name);
            let cs = std::slice::from_raw_parts(cp, len);
            let rs = std::slice::from_raw_parts(rp, len);
            assert_eq!(
                cs, rs,
                "data table `{}` ({} bytes) differs\n C={:02x?}\n R={:02x?}",
                name, len, cs, rs
            );
        }
    }
}

/// `_pcre2_unicode_version_8` is a `const char *`; compare the string it
/// points at, not the (necessarily different) pointer value.
#[test]
fn exported_unicode_version_string_matches() {
    let (c, r) = both();
    unsafe {
        let cp = *(c.data_ptr("_pcre2_unicode_version_8") as *const *const u8);
        let rp = *(r.data_ptr("_pcre2_unicode_version_8") as *const *const u8);
        assert!(!cp.is_null() && !rp.is_null());
        let cs = std::ffi::CStr::from_ptr(cp as *const _);
        let rs = std::ffi::CStr::from_ptr(rp as *const _);
        assert_eq!(cs, rs, "unicode_version string differs");
    }
}

/// The exported default contexts start with a `pcre2_memctl` (two function
/// pointers + a data pointer) and the compile/match contexts also hold callback
/// and `tables` pointers. Those pointer VALUES must differ between the two
/// libraries; compare the scalar fields, and check the pointers agree
/// structurally (all NULL, or `tables` == that library's own default tables).
#[test]
fn exported_default_contexts_match() {
    let (c, r) = both();
    const PTR: usize = std::mem::size_of::<usize>();
    const MEMCTL: usize = 3 * PTR; // malloc, free, memory_data

    unsafe {
        // ---- compile context: memctl | stack_guard | stack_guard_data | tables | scalars
        for api in [c, r] {
            let p = api.data_ptr("_pcre2_default_compile_context_8");
            let words = std::slice::from_raw_parts(p as *const usize, 88 / PTR);
            assert_eq!(words[3], 0, "{}: stack_guard must be NULL", api.name);
            assert_eq!(words[4], 0, "{}: stack_guard_data must be NULL", api.name);
            assert_eq!(
                words[5],
                api.data_ptr("_pcre2_default_tables_8") as usize,
                "{}: tables must point at its own default_tables",
                api.name
            );
        }
        let scalars = MEMCTL + 3 * PTR; // 48
        let cs = std::slice::from_raw_parts(
            c.data_ptr("_pcre2_default_compile_context_8").add(scalars),
            88 - scalars,
        );
        let rs = std::slice::from_raw_parts(
            r.data_ptr("_pcre2_default_compile_context_8").add(scalars),
            88 - scalars,
        );
        assert_eq!(cs, rs, "default_compile_context scalar fields differ");

        // ---- match context: memctl | 6 callback/data pointers | scalars
        for api in [c, r] {
            let p = api.data_ptr("_pcre2_default_match_context_8");
            let words = std::slice::from_raw_parts(p as *const usize, 96 / PTR);
            for (i, w) in words[3..9].iter().enumerate() {
                assert_eq!(*w, 0, "{}: match ctx callback ptr {} must be NULL", api.name, i);
            }
        }
        let scalars = MEMCTL + 6 * PTR; // 72
        let cs = std::slice::from_raw_parts(
            c.data_ptr("_pcre2_default_match_context_8").add(scalars),
            96 - scalars,
        );
        let rs = std::slice::from_raw_parts(
            r.data_ptr("_pcre2_default_match_context_8").add(scalars),
            96 - scalars,
        );
        assert_eq!(cs, rs, "default_match_context scalar fields differ");

        // ---- convert context: memctl | glob_separator | glob_escape
        let cs = std::slice::from_raw_parts(
            c.data_ptr("_pcre2_default_convert_context_8").add(MEMCTL),
            32 - MEMCTL,
        );
        let rs = std::slice::from_raw_parts(
            r.data_ptr("_pcre2_default_convert_context_8").add(MEMCTL),
            32 - MEMCTL,
        );
        assert_eq!(cs, rs, "default_convert_context scalar fields differ");
    }
}

// ================================================= remaining exported internals
// These four are directly callable from outside and self-contained, so they get
// head-on differential tests rather than only transitive coverage.

/// `_pcre2_compile_get_hash_from_name8(name, length) -> uint16_t`.
#[test]
fn get_hash_from_name_random_and_exhaustive_short() {
    let (c, r) = both();
    unsafe {
        // exhaustive over all 1- and 2-byte names
        for a in 0u32..256 {
            let v = [a as u8];
            assert_eq!(
                (c.get_hash_from_name)(v.as_ptr(), 1),
                (r.get_hash_from_name)(v.as_ptr(), 1),
                "hash of 1-byte name {:#02x}",
                a
            );
            for b in 0u32..256 {
                let v = [a as u8, b as u8];
                assert_eq!(
                    (c.get_hash_from_name)(v.as_ptr(), 2),
                    (r.get_hash_from_name)(v.as_ptr(), 2),
                    "hash of 2-byte name {:02x?}",
                    v
                );
            }
        }
        // NOTE: `length == 0` is a PRECONDITION VIOLATION, not an input:
        // pcre2_compile_cgroup.c:63 asserts `length > 0` and line 65 reads
        // `name[length - 1]`, i.e. `name[0xFFFFFFFF]` when length is 0. Verified:
        // the C library segfaults, so it is excluded.
        let mut g = Rng::new(SEED ^ 0x1111);
        for _ in 0..40000 {
            let n = g.range(1, 140) as usize; // 1..=139, spans MAX_NAME_SIZE (128)
            let v: Vec<u8> = (0..n).map(|_| g.next_u32() as u8).collect();
            let len = n as u32;
            assert_eq!(
                (c.get_hash_from_name)(v.as_ptr(), len),
                (r.get_hash_from_name)(v.as_ptr(), len),
                "hash len={} {:02x?}",
                len,
                v
            );
        }
    }
}

/// `_pcre2_update_classbits_8(ptype, pdata, negated, classbits)` — writes into a
/// 32-byte class bitmap.
///
/// `pdata` is a property VALUE whose meaning depends on `ptype`, and the C code
/// trusts it: `PT_SCX` indexes a bitmap with `MAPBIT(... , pdata)` and `PT_BOOL`
/// likewise, so an out-of-range `pdata` reads out of bounds (verified: the C
/// library segfaults). Each type is therefore swept over its REAL valid range,
/// taken from `c_src/src/pcre2_ucp.h`:
///   ucp_Script_Count = 175, ucp_Bprop_Count = 57,
///   bidi classes = 23, general categories (chartype) = 30, gentypes = 7.
#[test]
fn update_classbits_all_property_types() {
    let (c, r) = both();
    // PT_* constants from pcre2_internal.h:1445-1471
    const PT_LAMP: u32 = 0;
    const PT_GC: u32 = 1;
    const PT_PC: u32 = 2;
    const PT_SC: u32 = 3;
    const PT_SCX: u32 = 4;
    const PT_ALNUM: u32 = 5;
    const PT_SPACE: u32 = 6;
    const PT_PXSPACE: u32 = 7;
    const PT_WORD: u32 = 8;
    const PT_CLIST: u32 = 9;
    const PT_UCNC: u32 = 10;
    const PT_BIDICL: u32 = 11;
    const PT_BOOL: u32 = 12;
    const PT_ANY: u32 = 13;
    const PT_PXGRAPH: u32 = 14;
    const PT_PXPRINT: u32 = 15;
    const PT_PXPUNCT: u32 = 16;
    const PT_PXXDIGIT: u32 = 17;

    // (ptype, exclusive upper bound for pdata)
    let cases: [(u32, u32); 18] = [
        (PT_LAMP, 1),       // pdata unused
        (PT_GC, 7),         // ucp_C..ucp_Z
        (PT_PC, 30),        // general categories
        (PT_SC, 175),       // ucp_Script_Count
        (PT_SCX, 175),      // ucp_Script_Count (bitmap-indexed)
        (PT_ALNUM, 1),
        (PT_SPACE, 1),
        (PT_PXSPACE, 1),
        (PT_WORD, 1),
        (PT_CLIST, 1),      // pdata is a list offset; not exercised here
        (PT_UCNC, 1),
        (PT_BIDICL, 23),    // bidi classes
        (PT_BOOL, 57),      // ucp_Bprop_Count (bitmap-indexed)
        (PT_ANY, 1),
        (PT_PXGRAPH, 1),
        (PT_PXPRINT, 1),
        (PT_PXPUNCT, 1),
        (PT_PXXDIGIT, 1),
    ];
    unsafe {
        for (ptype, pmax) in cases {
            for pdata in 0..pmax {
                for negated in [0i32, 1] {
                    // several starting bitmap states, so the
                    // OR-into-existing-bits behaviour is compared too
                    for init in [0x00u8, 0xFF, 0xAA, 0x55, 0x0F] {
                        let mut cb = [init; 32];
                        let mut rb = [init; 32];
                        (c.update_classbits)(ptype, pdata, negated, cb.as_mut_ptr());
                        (r.update_classbits)(ptype, pdata, negated, rb.as_mut_ptr());
                        assert_eq!(
                            cb, rb,
                            "update_classbits(ptype={}, pdata={}, negated={}, init={:#02x})",
                            ptype, pdata, negated, init
                        );
                    }
                }
            }
        }
        // Unknown ptype values reach the `switch` default, which is well-defined
        // (it simply sets no bit) and does not touch pdata.
        for ptype in [18u32, 19, 20, 30, 100, 254] {
            for negated in [0i32, 1] {
                let mut cb = [0x00u8; 32];
                let mut rb = [0x00u8; 32];
                (c.update_classbits)(ptype, 0, negated, cb.as_mut_ptr());
                (r.update_classbits)(ptype, 0, negated, rb.as_mut_ptr());
                assert_eq!(
                    cb, rb,
                    "update_classbits(unknown ptype={}, negated={})",
                    ptype, negated
                );
            }
        }
    }
}

/// `_pcre2_study_8(code)` — run it directly on an already-compiled pattern and
/// compare both the return code and the resulting code block byte-for-byte.
///
/// `pcre2_compile` already calls `study` internally, so a second call must be
/// idempotent in the same way in both libraries.
#[test]
fn study_directly_on_compiled_codes() {
    use common::diff::*;
    let (c, r) = both();
    let pats: [&[u8]; 24] = [
        b"a", b"abc", b"a*b", b"(a|b)c", b"^abc", b"abc$", b".*x", b"[a-z]+9",
        b"(?:ab)+", b"a{2,5}b", b"(a)(b)(c)", b"\\d+\\w*", b"(?i)ABC",
        b"(?<n>a)+", b"a(?=b)", b"(?<=a)b", b"\\p{L}+", b"\\R\\X",
        b"(a+)+b", b"(?>a*)b", b"a|bb|ccc", b"(*MARK:m)a", b"[^\\n]*",
        b"(?(1)a|b)(x)",
    ];
    unsafe {
        for pat in pats {
            for opts in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_NO_START_OPTIMIZE,
                         PCRE2_ANCHORED, PCRE2_MULTILINE] {
                let cfg = CompileCfg::new(opts);
                let cc = compile_in(c, pat, pat.len(), &cfg);
                let rr = compile_in(r, pat, pat.len(), &cfg);
                if cc.code.is_null() {
                    assert!(rr.code.is_null());
                    continue;
                }
                // snapshot size, then call study again in both libraries
                let mut csize = 0usize;
                let mut rsize = 0usize;
                (c.pattern_info)(cc.code, 22, &mut csize as *mut _ as *mut _);
                (r.pattern_info)(rr.code, 22, &mut rsize as *mut _ as *mut _);
                assert_eq!(csize, rsize, "size before study for {:?}", pat);

                let crc = (c.study)(cc.code);
                let rrc = (r.study)(rr.code);
                assert_eq!(
                    crc, rrc,
                    "_pcre2_study_8 rc for {:?} opts={:#x}",
                    String::from_utf8_lossy(pat), opts
                );
                // the code blocks must still be byte-identical afterwards
                let cbytes = std::slice::from_raw_parts(cc.code as *const u8, csize);
                let rbytes = std::slice::from_raw_parts(rr.code as *const u8, rsize);
                // skip the leading pcre2_memctl (2 fn ptrs + data ptr) and the
                // `tables` / `executable_jit` pointers, which are per-library
                const SKIP: usize = 3 * std::mem::size_of::<usize>()
                    + 2 * std::mem::size_of::<usize>();
                assert_eq!(
                    &cbytes[SKIP..], &rbytes[SKIP..],
                    "code block differs after _pcre2_study_8 for {:?} opts={:#x}",
                    String::from_utf8_lossy(pat), opts
                );
                // and pattern_info must still agree
                assert_pattern_info_eq(
                    cc.code, rr.code,
                    &format!("after study {:?}", String::from_utf8_lossy(pat)),
                );
            }
        }
    }
}

/// The four exported JIT helpers. With `SUPPORT_JIT` undefined these are
/// no-op/constant stubs (`pcre2_jit_misc_inc.h:78,207,222` and
/// `pcre2_jit_compile.c`), so both libraries must agree exactly.
#[test]
fn exported_jit_helpers_are_identical_stubs() {
    let (c, r) = both();
    unsafe {
        // _pcre2_jit_get_target_8() -> const char *  ("JIT is not supported")
        let cp = (c.jit_get_target)();
        let rp = (r.jit_get_target)();
        assert!(!cp.is_null() && !rp.is_null());
        let cs = std::ffi::CStr::from_ptr(cp);
        let rs = std::ffi::CStr::from_ptr(rp);
        assert_eq!(cs, rs, "_pcre2_jit_get_target_8 string differs");

        // _pcre2_jit_get_size_8(x) -> 0 for any argument
        for arg in [std::ptr::null_mut(), 1usize as *mut c_void, usize::MAX as *mut c_void] {
            assert_eq!(
                (c.jit_get_size)(arg),
                (r.jit_get_size)(arg),
                "_pcre2_jit_get_size_8({:?})",
                arg
            );
            assert_eq!((c.jit_get_size)(arg), 0, "must be 0 in a non-JIT build");
        }

        // _pcre2_jit_free_8 / _pcre2_jit_free_rodata_8 are no-ops
        for api in [c, r] {
            (api.jit_free)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.jit_free_rodata)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.jit_free)(1usize as *mut c_void, std::ptr::null_mut());
            (api.jit_free_rodata)(1usize as *mut c_void, std::ptr::null_mut());
        }
    }
}
