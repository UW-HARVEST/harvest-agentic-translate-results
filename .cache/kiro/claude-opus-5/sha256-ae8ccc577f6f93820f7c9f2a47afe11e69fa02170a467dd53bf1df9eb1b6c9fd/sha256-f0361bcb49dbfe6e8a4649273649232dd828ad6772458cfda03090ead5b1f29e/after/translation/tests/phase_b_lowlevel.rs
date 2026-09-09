//! Phase B — low-level entry points (CONFIGS rows 1-14, 81-82).
//!
//! Every call goes through `dlsym` on the two shared libraries.
#![allow(unused_unsafe, dead_code, unsafe_op_in_unsafe_fn)]
mod common;
use common::*;
use std::os::raw::{c_char, c_double, c_int, c_void};

// ------------------------------------------------------------------- row 1

#[test]
fn row01_utf8_encode() {
    let (c, r) = both();
    let f_c = sym!(c, "utf8_encode", (i32, *mut c_char, *mut usize) -> c_int);
    let f_r = sym!(r, "utf8_encode", (i32, *mut c_char, *mut usize) -> c_int);

    let mut cases: Vec<i32> = vec![
        i32::MIN,
        -1000,
        -1,
        0,
        1,
        0x7E,
        0x7F,
        0x80,
        0x81,
        0x7FE,
        0x7FF,
        0x800,
        0x801,
        0xD7FF,
        0xD800,
        0xDBFF,
        0xDC00,
        0xDFFF,
        0xE000,
        0xFFFE,
        0xFFFF,
        0x10000,
        0x10001,
        0x10FFFE,
        0x10FFFF,
        0x110000,
        0x1FFFFF,
        i32::MAX,
    ];
    let mut rng = Rng::new(1);
    for _ in 0..4096 {
        cases.push(rng.next_u32() as i32);
        cases.push(rng.below(0x120000) as i32);
    }

    for cp in cases {
        let mut bc = [0u8; 8];
        let mut br = [0u8; 8];
        let mut sc: usize = 0xAAAA_AAAA;
        let mut sr: usize = 0xAAAA_AAAA;
        let rc = unsafe { f_c(cp, bc.as_mut_ptr() as *mut c_char, &mut sc) };
        let rr = unsafe { f_r(cp, br.as_mut_ptr() as *mut c_char, &mut sr) };
        assert_eq!(rc, rr, "utf8_encode({}) ret", cp);
        assert_eq!(sc, sr, "utf8_encode({}) size", cp);
        assert_eq!(bc, br, "utf8_encode({}) buffer", cp);
    }
}

// ------------------------------------------------------------------- row 2

#[test]
fn row02_utf8_check_first() {
    let (c, r) = both();
    let f_c = sym!(c, "utf8_check_first", (c_char) -> usize);
    let f_r = sym!(r, "utf8_check_first", (c_char) -> usize);
    for b in 0u16..256 {
        let byte = b as u8 as c_char;
        assert_eq!(
            unsafe { f_c(byte) },
            unsafe { f_r(byte) },
            "utf8_check_first(0x{:02x})",
            b
        );
    }
}

// ------------------------------------------------------------------- row 3

#[test]
fn row03_utf8_check_full() {
    let (c, r) = both();
    let f_c = sym!(c, "utf8_check_full", (*const c_char, usize, *mut i32) -> usize);
    let f_r = sym!(r, "utf8_check_full", (*const c_char, usize, *mut i32) -> usize);

    let mut cases: Vec<(Vec<u8>, usize)> = vec![
        (vec![0xC2, 0xA9], 2),
        (vec![0xC0, 0x80], 2), // overlong
        (vec![0xC1, 0xBF], 2), // overlong
        (vec![0xE0, 0x80, 0x80], 3),
        (vec![0xE0, 0xA0, 0x80], 3),
        (vec![0xED, 0xA0, 0x80], 3), // surrogate D800
        (vec![0xED, 0xBF, 0xBF], 3), // surrogate DFFF
        (vec![0xEE, 0x80, 0x80], 3),
        (vec![0xEF, 0xBF, 0xBF], 3),
        (vec![0xF0, 0x80, 0x80, 0x80], 4), // overlong
        (vec![0xF0, 0x90, 0x80, 0x80], 4),
        (vec![0xF4, 0x8F, 0xBF, 0xBF], 4), // 10FFFF
        (vec![0xF4, 0x90, 0x80, 0x80], 4), // 110000 out of range
        (vec![0xF7, 0xBF, 0xBF, 0xBF], 4),
        (vec![0xC2, 0x41], 2), // bad continuation
        (vec![0xE2, 0x82, 0x41], 3),
        (vec![0x41, 0x42, 0x43, 0x44], 0),
        (vec![0x41, 0x42, 0x43, 0x44], 1),
        (vec![0x41, 0x42, 0x43, 0x44], 5),
    ];
    let mut rng = Rng::new(3);
    for _ in 0..2048 {
        let size = rng.below(6);
        let buf: Vec<u8> = (0..6).map(|_| rng.next_u32() as u8).collect();
        cases.push((buf, size));
    }
    for _ in 0..2048 {
        // biased toward plausible sequences
        let size = 2 + rng.below(3);
        let lead = match size {
            2 => 0xC0u8 | (rng.below(0x20) as u8),
            3 => 0xE0u8 | (rng.below(0x10) as u8),
            _ => 0xF0u8 | (rng.below(0x08) as u8),
        };
        let mut buf = vec![lead];
        for _ in 1..size {
            buf.push(0x80u8 | (rng.below(0x40) as u8));
        }
        while buf.len() < 6 {
            buf.push(0);
        }
        cases.push((buf, size));
    }

    for (buf, size) in cases {
        let mut cpc: i32 = -12345;
        let mut cpr: i32 = -12345;
        let rc = unsafe { f_c(buf.as_ptr() as *const c_char, size, &mut cpc) };
        let rr = unsafe { f_r(buf.as_ptr() as *const c_char, size, &mut cpr) };
        assert_eq!(rc, rr, "utf8_check_full({:02x?},{}) ret", buf, size);
        assert_eq!(cpc, cpr, "utf8_check_full({:02x?},{}) cp", buf, size);
        // codepoint == NULL variant
        let rc2 = unsafe { f_c(buf.as_ptr() as *const c_char, size, std::ptr::null_mut()) };
        let rr2 = unsafe { f_r(buf.as_ptr() as *const c_char, size, std::ptr::null_mut()) };
        assert_eq!(rc2, rr2, "utf8_check_full null-cp({:02x?},{})", buf, size);
    }
}

// ------------------------------------------------------------------- row 4

#[test]
fn row04_utf8_iterate() {
    let (c, r) = both();
    let f_c = sym!(c, "utf8_iterate", (*const c_char, usize, *mut i32) -> *const c_char);
    let f_r = sym!(r, "utf8_iterate", (*const c_char, usize, *mut i32) -> *const c_char);

    let mut cases: Vec<(Vec<u8>, usize)> = Vec::new();
    for s in interesting_strings() {
        let b = s.as_bytes().to_vec();
        for n in 0..=b.len().min(6) {
            cases.push((b.clone(), n));
        }
    }
    let mut rng = Rng::new(4);
    for _ in 0..4096 {
        let buf: Vec<u8> = (0..8).map(|_| rng.next_u32() as u8).collect();
        cases.push((buf, rng.below(6)));
    }

    for (buf, size) in cases {
        let mut b = buf.clone();
        b.resize(buf.len().max(8), 0);
        let mut cpc: i32 = -12345;
        let mut cpr: i32 = -12345;
        let pc = unsafe { f_c(b.as_ptr() as *const c_char, size, &mut cpc) };
        let pr = unsafe { f_r(b.as_ptr() as *const c_char, size, &mut cpr) };
        let offc = if pc.is_null() {
            None
        } else {
            Some(unsafe { pc.offset_from(b.as_ptr() as *const c_char) })
        };
        let offr = if pr.is_null() {
            None
        } else {
            Some(unsafe { pr.offset_from(b.as_ptr() as *const c_char) })
        };
        assert_eq!(offc, offr, "utf8_iterate({:02x?},{}) ptr", b, size);
        assert_eq!(cpc, cpr, "utf8_iterate({:02x?},{}) cp", b, size);
    }
}

// ------------------------------------------------------------------- row 5

#[test]
fn row05_utf8_check_string() {
    let (c, r) = both();
    let f_c = sym!(c, "utf8_check_string", (*const c_char, usize) -> c_int);
    let f_r = sym!(r, "utf8_check_string", (*const c_char, usize) -> c_int);

    let mut cases: Vec<(Vec<u8>, usize)> = Vec::new();
    for s in interesting_strings() {
        let b = s.as_bytes().to_vec();
        for n in 0..=b.len() {
            cases.push((b.clone(), n));
        }
    }
    let mut rng = Rng::new(5);
    for _ in 0..4096 {
        let len = rng.below(17);
        let buf: Vec<u8> = (0..len.max(1)).map(|_| rng.next_u32() as u8).collect();
        cases.push((buf, len));
    }
    for _ in 0..2048 {
        // valid utf8 with random tail truncation
        let mut s = String::new();
        for _ in 0..rng.below(6) {
            s.push(char::from_u32(rng.below(0x11000) as u32).unwrap_or('a'));
        }
        let b = s.into_bytes();
        let n = if b.is_empty() { 0 } else { rng.below(b.len() + 1) };
        cases.push((b, n));
    }

    for (buf, len) in cases {
        let p = if buf.is_empty() {
            b"".as_ptr()
        } else {
            buf.as_ptr()
        };
        let rc = unsafe { f_c(p as *const c_char, len) };
        let rr = unsafe { f_r(p as *const c_char, len) };
        assert_eq!(rc, rr, "utf8_check_string({:02x?},{})", buf, len);
    }
}

// ------------------------------------------------------------------- row 6

#[derive(Clone, Debug)]
enum SbOp {
    AppendBytes(Vec<u8>),
    AppendByte(u8),
    Pop,
    Clear,
}

#[test]
fn row06_strbuffer() {
    let (c, r) = both();
    let mut rng = Rng::new(6);

    for trial in 0..400 {
        let n_ops = 1 + rng.below(40);
        let mut ops = Vec::new();
        for _ in 0..n_ops {
            match rng.below(10) {
                0..=5 => {
                    let n = rng.below(40);
                    ops.push(SbOp::AppendBytes(
                        (0..n).map(|_| 1 + (rng.next_u32() % 255) as u8).collect(),
                    ));
                }
                6 | 7 => ops.push(SbOp::AppendByte(1 + (rng.next_u32() % 255) as u8)),
                8 => ops.push(SbOp::Pop),
                _ => ops.push(SbOp::Clear),
            }
        }
        let a = run_strbuffer(c, &ops);
        let b = run_strbuffer(r, &ops);
        assert_eq!(a, b, "strbuffer trial {} ops {:?}", trial, ops);
    }
}

fn run_strbuffer(l: &Lib, ops: &[SbOp]) -> Vec<String> {
    let init = sym!(l, "strbuffer_init", (*mut StrBuffer) -> c_int);
    let close = sym!(l, "strbuffer_close", (*mut StrBuffer));
    let clear = sym!(l, "strbuffer_clear", (*mut StrBuffer));
    let value = sym!(l, "strbuffer_value", (*const StrBuffer) -> *const c_char);
    let ap_b = sym!(l, "strbuffer_append_byte", (*mut StrBuffer, c_char) -> c_int);
    let ap_bs = sym!(l, "strbuffer_append_bytes", (*mut StrBuffer, *const c_char, usize) -> c_int);
    let pop = sym!(l, "strbuffer_pop", (*mut StrBuffer) -> c_char);
    let steal = sym!(l, "strbuffer_steal_value", (*mut StrBuffer) -> *mut c_char);
    let free = sym!(l, "jsonp_free", (*mut c_void));

    let mut sb = StrBuffer::zeroed();
    let mut log = Vec::new();
    unsafe {
        log.push(format!("init={}", init(&mut sb)));
        log.push(format!("size={} len={}", sb.size, sb.length));
        for op in ops {
            match op {
                SbOp::AppendBytes(d) => {
                    let rc = ap_bs(&mut sb, d.as_ptr() as *const c_char, d.len());
                    log.push(format!("apbs({})={}", d.len(), rc));
                }
                SbOp::AppendByte(b) => {
                    let rc = ap_b(&mut sb, *b as c_char);
                    log.push(format!("apb={}", rc));
                }
                SbOp::Pop => {
                    let ch = pop(&mut sb);
                    log.push(format!("pop={}", ch as u8));
                }
                SbOp::Clear => {
                    clear(&mut sb);
                    log.push("clear".into());
                }
            }
            let v = value(&sb);
            let bytes = std::ffi::CStr::from_ptr(v).to_bytes().to_vec();
            log.push(format!(
                "len={} size={} val={:02x?}",
                sb.length, sb.size, bytes
            ));
        }
        // steal + close
        let p = steal(&mut sb);
        let stolen = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
        log.push(format!("stolen={:02x?}", stolen));
        free(p as *mut c_void);
        close(&mut sb);
        log.push(format!("closed size={} len={}", sb.size, sb.length));
    }
    log
}

// ------------------------------------------------------------------- row 7

#[derive(Clone, Debug)]
enum HtOp {
    Set(Vec<u8>, i64),
    Get(Vec<u8>),
    Del(Vec<u8>),
    Clear,
    IterWalk,
    IterAt(Vec<u8>),
}

#[test]
fn row07_hashtable() {
    let (c, r) = both();
    let mut rng = Rng::new(7);

    let keys: Vec<Vec<u8>> = {
        let mut v: Vec<Vec<u8>> = Vec::new();
        v.push(b"".to_vec());
        v.push(b"a".to_vec());
        v.push(b"b".to_vec());
        v.push(b"ab".to_vec());
        v.push(b"ba".to_vec());
        v.push(vec![b'x', 0, b'y']);
        v.push(vec![b'x', 0, b'z']);
        for i in 0..80 {
            v.push(format!("key{:03}", i).into_bytes());
        }
        v
    };

    for trial in 0..120 {
        let n_ops = 1 + rng.below(160);
        let mut ops = Vec::new();
        for _ in 0..n_ops {
            let k = keys[rng.below(keys.len())].clone();
            match rng.below(12) {
                0..=6 => ops.push(HtOp::Set(k, rng.i64())),
                7 | 8 => ops.push(HtOp::Get(k)),
                9 => ops.push(HtOp::Del(k)),
                10 => ops.push(HtOp::IterAt(k)),
                _ => ops.push(HtOp::IterWalk),
            }
        }
        ops.push(HtOp::IterWalk);
        if rng.below(4) == 0 {
            ops.push(HtOp::Clear);
            ops.push(HtOp::IterWalk);
        }
        let a = run_hashtable(c, &ops);
        let b = run_hashtable(r, &ops);
        assert_eq!(a, b, "hashtable trial {}", trial);
    }
}

fn run_hashtable(l: &Lib, ops: &[HtOp]) -> Vec<String> {
    let init = sym!(l, "hashtable_init", (*mut HashTable) -> c_int);
    let close = sym!(l, "hashtable_close", (*mut HashTable));
    let set = sym!(l, "hashtable_set", (*mut HashTable, *const c_char, usize, *mut JsonT) -> c_int);
    let get = sym!(l, "hashtable_get", (*mut HashTable, *const c_char, usize) -> *mut c_void);
    let del = sym!(l, "hashtable_del", (*mut HashTable, *const c_char, usize) -> c_int);
    let clear = sym!(l, "hashtable_clear", (*mut HashTable));
    let iter = sym!(l, "hashtable_iter", (*mut HashTable) -> *mut c_void);
    let iter_at = sym!(l, "hashtable_iter_at", (*mut HashTable, *const c_char, usize) -> *mut c_void);
    let iter_next = sym!(l, "hashtable_iter_next", (*mut HashTable, *mut c_void) -> *mut c_void);
    let iter_key = sym!(l, "hashtable_iter_key", (*mut c_void) -> *mut c_void);
    let iter_key_len = sym!(l, "hashtable_iter_key_len", (*mut c_void) -> usize);
    let iter_value = sym!(l, "hashtable_iter_value", (*mut c_void) -> *mut c_void);
    let iter_set = sym!(l, "hashtable_iter_set", (*mut c_void, *mut JsonT));
    let json_integer = sym!(l, "json_integer", (i64) -> *mut JsonT);
    let integer_value = sym!(l, "json_integer_value", (*const JsonT) -> i64);

    let mut ht = HashTable::zeroed();
    let mut log = Vec::new();
    unsafe {
        log.push(format!("init={}", init(&mut ht)));
        log.push(format!("size={} order={}", ht.words[0], ht.words[2]));
        for op in ops {
            match op {
                HtOp::Set(k, v) => {
                    let jv = json_integer(*v);
                    let rc = set(&mut ht, k.as_ptr() as *const c_char, k.len(), jv);
                    log.push(format!("set({:?})={} size={}", k, rc, ht.words[0]));
                }
                HtOp::Get(k) => {
                    let p = get(&mut ht, k.as_ptr() as *const c_char, k.len()) as *mut JsonT;
                    log.push(if p.is_null() {
                        format!("get({:?})=NULL", k)
                    } else {
                        format!("get({:?})={}", k, integer_value(p))
                    });
                }
                HtOp::Del(k) => {
                    let rc = del(&mut ht, k.as_ptr() as *const c_char, k.len());
                    log.push(format!("del({:?})={} size={}", k, rc, ht.words[0]));
                }
                HtOp::Clear => {
                    clear(&mut ht);
                    log.push(format!("clear size={}", ht.words[0]));
                }
                HtOp::IterAt(k) => {
                    let it = iter_at(&mut ht, k.as_ptr() as *const c_char, k.len());
                    if it.is_null() {
                        log.push(format!("iter_at({:?})=NULL", k));
                    } else {
                        let kp = iter_key(it) as *const u8;
                        let kl = iter_key_len(it);
                        let kb = std::slice::from_raw_parts(kp, kl).to_vec();
                        let vp = iter_value(it) as *mut JsonT;
                        log.push(format!(
                            "iter_at({:?})=({:?},{})",
                            k,
                            kb,
                            integer_value(vp)
                        ));
                        // exercise iter_set
                        let nv = json_integer(integer_value(vp).wrapping_add(1));
                        iter_set(it, nv);
                        log.push(format!("iter_set -> {}", integer_value(iter_value(it) as *mut JsonT)));
                    }
                }
                HtOp::IterWalk => {
                    let mut it = iter(&mut ht);
                    let mut n = 0;
                    while !it.is_null() {
                        let kp = iter_key(it) as *const u8;
                        let kl = iter_key_len(it);
                        let kb = std::slice::from_raw_parts(kp, kl).to_vec();
                        let vp = iter_value(it) as *mut JsonT;
                        log.push(format!("  walk[{}]=({:?},{})", n, kb, integer_value(vp)));
                        n += 1;
                        it = iter_next(&mut ht, it);
                    }
                    log.push(format!("walk n={}", n));
                }
            }
        }
        close(&mut ht);
    }
    log
}

// ------------------------------------------------------------------- row 8

#[test]
fn row08_memory_primitives() {
    let (c, r) = both();
    for l in [c, r] {
        let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
        let free = sym!(l, "jsonp_free", (*mut c_void));
        unsafe {
            assert!(malloc(0).is_null(), "{}: jsonp_malloc(0) must be NULL", l.tag);
            free(std::ptr::null_mut());
            let p = malloc(32);
            assert!(!p.is_null());
            free(p);
        }
    }
    // jsonp_realloc grow / shrink, and jsonp_strndup over embedded NUL
    let mut rng = Rng::new(8);
    for _ in 0..200 {
        let n = 1 + rng.below(64);
        let m = 1 + rng.below(64);
        let data: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        let mut out = Vec::new();
        for l in [c, r] {
            let malloc = sym!(l, "jsonp_malloc", (usize) -> *mut c_void);
            let realloc = sym!(l, "jsonp_realloc", (*mut c_void, usize, usize) -> *mut c_void);
            let free = sym!(l, "jsonp_free", (*mut c_void));
            let strndup = sym!(l, "jsonp_strndup", (*const c_char, usize) -> *mut c_char);
            unsafe {
                let p = malloc(n) as *mut u8;
                std::ptr::copy_nonoverlapping(data.as_ptr(), p, n);
                let q = realloc(p as *mut c_void, n, m) as *mut u8;
                assert!(!q.is_null());
                let keep = n.min(m);
                let got = std::slice::from_raw_parts(q, keep).to_vec();
                free(q as *mut c_void);

                let d = strndup(data.as_ptr() as *const c_char, n);
                let dup = std::slice::from_raw_parts(d as *const u8, n + 1).to_vec();
                free(d as *mut c_void);
                out.push((got, dup));
            }
        }
        assert_eq!(out[0], out[1], "jsonp_realloc/strndup mismatch");
    }
}

// ------------------------------------------------------------------- row 11

#[test]
fn row11_error_helpers() {
    let (c, r) = both();
    let sources = [
        "",
        "x",
        "short",
        &"a".repeat(78),
        &"a".repeat(79),
        &"b".repeat(80),
        &"c".repeat(81),
        &"d".repeat(200),
        "<string>",
    ];
    let msgs = [
        "hello",
        "",
        &"m".repeat(150),
        &"m".repeat(157),
        &"m".repeat(158),
        &"m".repeat(159),
        &"m".repeat(400),
    ];
    for src in sources {
        for msg in msgs {
            for code in [0u8, 1, 5, 8, 17, 200, 255] {
                let mut out = Vec::new();
                for l in [c, r] {
                    let init = sym!(l, "jsonp_error_init", (*mut JsonError, *const c_char));
                    let set_src = sym!(l, "jsonp_error_set_source", (*mut JsonError, *const c_char));
                    let set = sym!(l, "jsonp_error_set", (*mut JsonError, c_int, c_int, usize, c_int, *const c_char));
                    let mut e = JsonError::new();
                    // fill with a marker so untouched bytes are compared too
                    e.source = [0x7f; ERR_SRC_LEN];
                    e.text = [0x7f; ERR_TEXT_LEN];
                    let cs = cstr(src);
                    let cm = cstr(msg);
                    unsafe {
                        init(&mut e, cs.as_ptr());
                        set(&mut e, 3, 4, 5, code as c_int, cm.as_ptr());
                        // second set must be ignored
                        let cm2 = cstr("second message");
                        set(&mut e, 9, 9, 9, 1, cm2.as_ptr());
                        set_src(&mut e, cs.as_ptr());
                        // NULL tolerance
                        init(std::ptr::null_mut(), cs.as_ptr());
                        set_src(std::ptr::null_mut(), cs.as_ptr());
                        set_src(&mut e, std::ptr::null());
                        set(std::ptr::null_mut(), 0, 0, 0, 0, cm.as_ptr());
                    }
                    out.push(e.raw());
                }
                assert_eq!(
                    out[0], out[1],
                    "jsonp_error_* mismatch src.len={} msg.len={} code={}",
                    src.len(),
                    msg.len(),
                    code
                );
            }
        }
    }
    // fresh-init state must match too
    let mut out = Vec::new();
    for l in [c, r] {
        let init = sym!(l, "jsonp_error_init", (*mut JsonError, *const c_char));
        let mut e = JsonError::new();
        e.source = [0x7f; ERR_SRC_LEN];
        e.text = [0x7f; ERR_TEXT_LEN];
        unsafe { init(&mut e, std::ptr::null()) };
        out.push(e.raw());
    }
    assert_eq!(out[0], out[1], "jsonp_error_init(NULL source) mismatch");
}

// ------------------------------------------------------------------- row 12

#[test]
fn row12_jsonp_dtostr() {
    let (c, r) = both();
    let f_c = sym!(c, "jsonp_dtostr", (*mut c_char, usize, c_double, c_int) -> c_int);
    let f_r = sym!(r, "jsonp_dtostr", (*mut c_char, usize, c_double, c_int) -> c_int);

    let mut vals: Vec<f64> = interesting_reals().to_vec();
    vals.extend([
        1.0, 10.0, 100.0, 1e15, 1e16, 1e17, 1e-1, 1e-2, 1e-3, 1e-4, 1e-5, 1e-6, -1e16,
        123.456, 1.0e-323, 2.5, 0.125, 1.0 / 7.0, 1e-9, 9.999999999999999e15,
        1.7976931348623155e308,
    ]);
    let mut rng = Rng::new(12);
    for _ in 0..4096 {
        vals.push(rng.f64());
    }
    for _ in 0..2048 {
        // "nice" magnitudes
        let m = rng.below(40) as i32 - 20;
        let mant = (rng.below(1_000_000) as f64) / 1000.0;
        vals.push(mant * 10f64.powi(m));
    }

    for &v in &vals {
        for prec in 0..32i32 {
            for size in [40usize, 32, 25, 20, 10, 5, 3, 1] {
                let mut bc = vec![0x7fu8; 64];
                let mut br = vec![0x7fu8; 64];
                let rc = unsafe { f_c(bc.as_mut_ptr() as *mut c_char, size, v, prec) };
                let rr = unsafe { f_r(br.as_mut_ptr() as *mut c_char, size, v, prec) };
                assert_eq!(rc, rr, "jsonp_dtostr({:e},prec={},size={}) ret", v, prec, size);
                if rc >= 0 {
                    let n = rc as usize;
                    assert_eq!(
                        &bc[..=n],
                        &br[..=n],
                        "jsonp_dtostr({:e},prec={},size={}) text C={:?} RUST={:?}",
                        v,
                        prec,
                        size,
                        String::from_utf8_lossy(&bc[..n]),
                        String::from_utf8_lossy(&br[..n])
                    );
                }
            }
        }
    }
}

// ------------------------------------------------------------------- row 13

#[test]
fn row13_jsonp_strtod() {
    let (c, r) = both();
    let mut texts: Vec<String> = vec![
        "0".into(),
        "-0".into(),
        "0.0".into(),
        "1".into(),
        "-1".into(),
        "1.5".into(),
        "1e10".into(),
        "1E10".into(),
        "1e+10".into(),
        "1e-10".into(),
        "1.7976931348623157e308".into(),
        "1.8e308".into(),
        "-1.8e308".into(),
        "1e-400".into(),
        "5e-324".into(),
        "2.2250738585072014e-308".into(),
        "123456789012345678901234567890".into(),
        "0.1".into(),
        "0.3333333333333333".into(),
        "9007199254740993".into(),
        "1e309".into(),
        "-1e309".into(),
        "1e-323".into(),
    ];
    let mut rng = Rng::new(13);
    for _ in 0..2048 {
        let digits = 1 + rng.below(20);
        let mut s = String::new();
        if rng.bool() {
            s.push('-');
        }
        for _ in 0..digits {
            s.push(char::from(b'0' + rng.below(10) as u8));
        }
        if rng.bool() {
            s.push('.');
            for _ in 0..1 + rng.below(20) {
                s.push(char::from(b'0' + rng.below(10) as u8));
            }
        }
        if rng.bool() {
            s.push('e');
            if rng.bool() {
                s.push('-');
            }
            s.push_str(&rng.below(400).to_string());
        }
        texts.push(s);
    }

    for t in &texts {
        let mut out = Vec::new();
        for l in [c, r] {
            let init = sym!(l, "strbuffer_init", (*mut StrBuffer) -> c_int);
            let close = sym!(l, "strbuffer_close", (*mut StrBuffer));
            let ap = sym!(l, "strbuffer_append_bytes", (*mut StrBuffer, *const c_char, usize) -> c_int);
            let strtod = sym!(l, "jsonp_strtod", (*mut StrBuffer, *mut c_double) -> c_int);
            let mut sb = StrBuffer::zeroed();
            unsafe {
                assert_eq!(init(&mut sb), 0);
                assert_eq!(ap(&mut sb, t.as_ptr() as *const c_char, t.len()), 0);
                let mut d: f64 = -12345.0;
                let rc = strtod(&mut sb, &mut d);
                close(&mut sb);
                out.push((rc, d.to_bits()));
            }
        }
        assert_eq!(out[0], out[1], "jsonp_strtod({:?})", t);
    }
}

// ------------------------------------------------------------------- row 14

#[test]
fn row14_dtoa_family() {
    let (c, r) = both();
    let mut vals: Vec<f64> = interesting_reals().to_vec();
    vals.extend([
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1.0,
        -1.0,
        1e-320,
        1.5e-310,
    ]);
    let mut rng = Rng::new(14);
    for _ in 0..2048 {
        vals.push(rng.f64());
    }
    for _ in 0..1024 {
        vals.push(f64::from_bits(rng.next_u64()));
    }

    // dtoa_divmax global must have the same value
    for l in [c, r] {
        let p: libloading::Symbol<*mut c_int> =
            unsafe { l.lib.get(b"dtoa_divmax\0") }.unwrap();
        assert_eq!(unsafe { **p }, 2, "{}: dtoa_divmax", l.tag);
    }

    for &v in &vals {
        for mode in 0..6i32 {
            for nd in [0i32, 1, 2, 3, 5, 8, 15, 17, 18, 25, 30] {
                // dtoa_r with caller buffer
                let mut bufc = vec![0u8; 64];
                let mut bufr = vec![0u8; 64];
                let mut dc = 0i32;
                let mut dr = 0i32;
                let mut sc = 0i32;
                let mut sr = 0i32;
                let mut rvc: *mut c_char = std::ptr::null_mut();
                let mut rvr: *mut c_char = std::ptr::null_mut();
                let (oc, or) = unsafe {
                    let fc = sym!(c, "dtoa_r", (c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize) -> *mut c_char);
                    let fr = sym!(r, "dtoa_r", (c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char, *mut c_char, usize) -> *mut c_char);
                    let pc = fc(v, mode, nd, &mut dc, &mut sc, &mut rvc, bufc.as_mut_ptr() as *mut c_char, 64);
                    let pr = fr(v, mode, nd, &mut dr, &mut sr, &mut rvr, bufr.as_mut_ptr() as *mut c_char, 64);
                    let s = |p: *mut c_char, rv: *mut c_char| {
                        if p.is_null() {
                            None
                        } else {
                            Some((
                                std::ffi::CStr::from_ptr(p).to_bytes().to_vec(),
                                if rv.is_null() { -1 } else { rv.offset_from(p) },
                            ))
                        }
                    };
                    (s(pc, rvc), s(pr, rvr))
                };
                assert_eq!(
                    (oc, dc, sc),
                    (or, dr, sr),
                    "dtoa_r({:e},mode={},nd={})",
                    v,
                    mode,
                    nd
                );

                // dtoa (allocating) + freedtoa
                let mut dc2 = 0i32;
                let mut dr2 = 0i32;
                let mut sc2 = 0i32;
                let mut sr2 = 0i32;
                let mut rvc2: *mut c_char = std::ptr::null_mut();
                let mut rvr2: *mut c_char = std::ptr::null_mut();
                let (oc2, or2) = unsafe {
                    let fc = sym!(c, "dtoa", (c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char);
                    let fr = sym!(r, "dtoa", (c_double, c_int, c_int, *mut c_int, *mut c_int, *mut *mut c_char) -> *mut c_char);
                    let free_c = sym!(c, "freedtoa", (*mut c_char));
                    let free_r = sym!(r, "freedtoa", (*mut c_char));
                    let pc = fc(v, mode, nd, &mut dc2, &mut sc2, &mut rvc2);
                    let pr = fr(v, mode, nd, &mut dr2, &mut sr2, &mut rvr2);
                    let take = |p: *mut c_char, rv: *mut c_char| {
                        if p.is_null() {
                            None
                        } else {
                            Some((
                                std::ffi::CStr::from_ptr(p).to_bytes().to_vec(),
                                if rv.is_null() { -1 } else { rv.offset_from(p) },
                            ))
                        }
                    };
                    let a = take(pc, rvc2);
                    let b = take(pr, rvr2);
                    if !pc.is_null() {
                        free_c(pc);
                    }
                    if !pr.is_null() {
                        free_r(pr);
                    }
                    (a, b)
                };
                assert_eq!(
                    (oc2, dc2, sc2),
                    (or2, dr2, sr2),
                    "dtoa({:e},mode={},nd={})",
                    v,
                    mode,
                    nd
                );
            }
        }
    }
}

#[test]
fn row14b_strtod_unused_and_gethex() {
    let (c, r) = both();
    let mut texts: Vec<String> = vec![
        "0".into(),
        "1".into(),
        "-1".into(),
        "1.5".into(),
        "1e10".into(),
        "1e400".into(),
        "1e-400".into(),
        "  12.5xyz".into(),
        "0x1p3".into(),
        "0X1.8p1".into(),
        "0x".into(),
        "inf".into(),
        "nan".into(),
        "".into(),
        "abc".into(),
        "1.7976931348623157e308".into(),
        "9007199254740993".into(),
        "0.000000000000000000001".into(),
    ];
    let mut rng = Rng::new(140);
    for _ in 0..1024 {
        let mut s = String::new();
        for _ in 0..1 + rng.below(24) {
            s.push(
                b"0123456789.eE+-xXpPabcdefinfa "[rng.below(30)] as char,
            );
        }
        texts.push(s);
    }
    for t in &texts {
        let cs = cstr(&t.replace('\0', ""));
        let mut out = Vec::new();
        for l in [c, r] {
            let f = sym!(l, "strtod__unused", (*const c_char, *mut *mut c_char) -> c_double);
            unsafe {
                let mut end: *mut c_char = std::ptr::null_mut();
                let v = f(cs.as_ptr(), &mut end);
                let off = if end.is_null() {
                    -1
                } else {
                    end.offset_from(cs.as_ptr())
                };
                out.push((v.to_bits(), off));
            }
        }
        assert_eq!(out[0], out[1], "strtod__unused({:?})", t);
    }

    // gethex: parses a hex float body (after the "0x"), advancing *sp
    let hex_bodies = [
        "1p3", "1.8p1", "0p0", "ffp-4", "1", "1.", ".8p0", "1p+1000", "1p-1000",
        "abcdefp0", "1p", "zz", "", "1.0000000000000001p0", "8000000000000000p0",
    ];
    for body in hex_bodies {
        for rounding in 0..4i32 {
            for sign in [0i32, 1] {
                let cs = cstr(body);
                let mut out = Vec::new();
                for l in [c, r] {
                    // NOTE: C declares `gethex` as returning `void`.
                    let f = sym!(l, "gethex", (*mut *const c_char, *mut u64, c_int, c_int));
                    unsafe {
                        let mut sp: *const c_char = cs.as_ptr();
                        let mut u: u64 = 0;
                        f(&mut sp, &mut u, rounding, sign);
                        out.push((u, sp.offset_from(cs.as_ptr()) as u64));
                    }
                }
                assert_eq!(
                    out[0], out[1],
                    "gethex({:?},rounding={},sign={})",
                    body, rounding, sign
                );
            }
        }
    }
}

// ------------------------------------------------------------- rows 81 & 82

#[test]
fn row81_object_seed() {
    let (c, r) = both();
    // libs() already seeded both with FIXED_SEED; the global must agree and
    // further calls must be no-ops.
    for l in [c, r] {
        let seed = sym!(l, "json_object_seed", (usize));
        let g: libloading::Symbol<*mut u32> = unsafe { l.lib.get(b"hashtable_seed\0") }.unwrap();
        unsafe {
            assert_eq!(**g, FIXED_SEED as u32, "{}: hashtable_seed", l.tag);
            seed(0);
            seed(999);
            assert_eq!(**g, FIXED_SEED as u32, "{}: seed must be sticky", l.tag);
        }
    }
}

#[test]
fn row82_version() {
    let (c, r) = both();
    let mut out = Vec::new();
    for l in [c, r] {
        let vs = sym!(l, "jansson_version_str", () -> *const c_char);
        let cmp = sym!(l, "jansson_version_cmp", (c_int, c_int, c_int) -> c_int);
        unsafe {
            let s = std::ffi::CStr::from_ptr(vs()).to_bytes().to_vec();
            let mut cmps = Vec::new();
            for a in [-2i32, -1, 0, 1, 2, 3, 15, 100, i32::MIN, i32::MAX] {
                for b in [-1i32, 0, 14, 15, 16, 100, i32::MIN, i32::MAX] {
                    for m in [-1i32, 0, 1, 100, i32::MIN, i32::MAX] {
                        cmps.push(cmp(a, b, m));
                    }
                }
            }
            out.push((s, cmps));
        }
    }
    assert_eq!(out[0], out[1], "version surface mismatch");
    assert_eq!(&out[0].0[..], b"2.15.0");
}
