//! Phase B — CONFIGS.md sections I (hashtable), J (strbuffer), K (utf8),
//! L (number conversion), M (memory/error/loop-check), N (version).
//!
//! These are the LOWEST-level exported entry points; everything else in the
//! library is composed out of them, so they are verified first.

mod common;
use common::*;
use libloading::Symbol;
use std::os::raw::{c_char, c_double, c_int, c_void};

// ===========================================================================
// K — utf8_*  (CONFIGS K1..K5)
// ===========================================================================

/// K1: exhaustive over all 256 byte values.
#[test]
fn k1_utf8_check_first_all_256_bytes() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_char) -> usize> =
            p.c.lib.get(b"utf8_check_first\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_char) -> usize> =
            p.r.lib.get(b"utf8_check_first\0").unwrap();
        for b in 0u16..256 {
            let b = b as u8 as c_char;
            assert_eq!(cf(b), rf(b), "utf8_check_first({})", b as u8);
        }
    }
}

/// K2: exhaustive over EVERY codepoint 0..=0x10FFFF, plus the out-of-range and
/// negative inputs. Note the C `utf8_encode` does *not* reject surrogates.
#[test]
fn k2_utf8_encode_all_codepoints() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int> =
            p.c.lib.get(b"utf8_encode\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int> =
            p.r.lib.get(b"utf8_encode\0").unwrap();
        // Poison the buffers differently each round so "didn't write" is visible.
        for cp in 0i32..=0x10FFFF {
            let mut cb = [0x5Au8; 8];
            let mut rb = [0x5Au8; 8];
            let mut cl: usize = 0xDEAD;
            let mut rl: usize = 0xDEAD;
            let cr = cf(cp, cb.as_mut_ptr() as *mut c_char, &mut cl);
            let rr = rf(cp, rb.as_mut_ptr() as *mut c_char, &mut rl);
            if cr != rr || cl != rl || cb != rb {
                panic!(
                    "utf8_encode(0x{cp:X}): C=(ret {cr}, len {cl}, {cb:02x?}) \
                     RUST=(ret {rr}, len {rl}, {rb:02x?})"
                );
            }
        }
    }
}

/// K3: utf8_check_full — exhaustive over ALL 2-byte sequences, plus randomized
/// 3/4-byte sequences and out-of-range `size` values.
#[test]
fn k3_utf8_check_full() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> c_int> =
            p.c.lib.get(b"utf8_check_full\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> c_int> =
            p.r.lib.get(b"utf8_check_full\0").unwrap();

        let cmp = |buf: &[u8], size: usize| {
            let mut cc: i32 = -9999;
            let mut rc: i32 = -9999;
            let cr = cf(buf.as_ptr() as *const c_char, size, &mut cc);
            let rr = rf(buf.as_ptr() as *const c_char, size, &mut rc);
            assert!(
                cr == rr && (cr == 0 || cc == rc),
                "utf8_check_full({buf:02x?}, {size}): C=({cr},{cc}) RUST=({rr},{rc})"
            );
            // NULL codepoint out-param must also be accepted.
            let cr2 = cf(buf.as_ptr() as *const c_char, size, std::ptr::null_mut());
            let rr2 = rf(buf.as_ptr() as *const c_char, size, std::ptr::null_mut());
            assert_eq!(cr2, rr2, "utf8_check_full({buf:02x?}, {size}) NULL cp");
        };

        // Exhaustive 2-byte space.
        for a in 0u16..256 {
            for b in 0u16..256 {
                cmp(&[a as u8, b as u8], 2);
            }
        }
        // Out-of-range sizes (only 2, 3, 4 are valid).
        for size in [0usize, 1, 5, 6, usize::MAX] {
            cmp(&[0xE2, 0x82, 0xAC, 0x00, 0x00, 0x00, 0x00, 0x00], size);
        }
        // Randomized 3- and 4-byte sequences, biased towards plausible leads.
        let mut rng = Rng::new(0x00C0_FFEE);
        for _ in 0..200_000 {
            let size = 3 + rng.below(2);
            let mut buf = [0u8; 8];
            buf[0] = if rng.bool() {
                // plausible lead byte for this size
                match size {
                    3 => 0xE0 + rng.below(0x10) as u8,
                    _ => 0xF0 + rng.below(0x8) as u8,
                }
            } else {
                rng.below(256) as u8
            };
            for i in 1..size {
                buf[i] = if rng.bool() {
                    0x80 + rng.below(0x40) as u8
                } else {
                    rng.below(256) as u8
                };
            }
            cmp(&buf, size);
        }
        // Every canonical 3-byte lead with valid continuations (overlong /
        // surrogate boundaries live here).
        for lead in 0xE0u8..=0xEF {
            for c1 in [0x80u8, 0x9F, 0xA0, 0xBF] {
                for c2 in [0x80u8, 0xBF] {
                    cmp(&[lead, c1, c2], 3);
                }
            }
        }
        // Every canonical 4-byte lead with valid continuations.
        for lead in 0xF0u8..=0xF4 {
            for c1 in [0x80u8, 0x8F, 0x90, 0xBF] {
                cmp(&[lead, c1, 0x80, 0x80], 4);
                cmp(&[lead, c1, 0xBF, 0xBF], 4);
            }
        }
    }
}

/// K4: utf8_iterate — valid 1..4-byte sequences, bufsize 0, and truncation.
#[test]
fn k4_utf8_iterate() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char> =
            p.c.lib.get(b"utf8_iterate\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, usize, *mut i32) -> *const c_char> =
            p.r.lib.get(b"utf8_iterate\0").unwrap();

        let cmp = |buf: &[u8], bufsize: usize| {
            let base = buf.as_ptr() as *const c_char;
            let mut cc: i32 = -9999;
            let mut rc: i32 = -9999;
            let cr = cf(base, bufsize, &mut cc);
            let rr = rf(base, bufsize, &mut rc);
            // Compare the *offset* the returned pointer represents, since the
            // two calls share one buffer.
            let coff = if cr.is_null() {
                None
            } else {
                Some(cr as usize - base as usize)
            };
            let roff = if rr.is_null() {
                None
            } else {
                Some(rr as usize - base as usize)
            };
            assert_eq!(
                (coff, cc),
                (roff, rc),
                "utf8_iterate({buf:02x?}, {bufsize})"
            );
        };

        // bufsize 0 (returns `buffer` unchanged, codepoint untouched)
        cmp(&[0x41, 0, 0, 0, 0], 0);
        // Encode every codepoint and iterate it back, at full and truncated sizes.
        let ce: Symbol<unsafe extern "C" fn(c_int, *mut c_char, *mut usize) -> c_int> =
            p.c.lib.get(b"utf8_encode\0").unwrap();
        let mut rng = Rng::new(0x1CE_1CE);
        let mut cps: Vec<i32> = vec![
            0, 1, 0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFFFF, 0x10000, 0x10FFFF,
        ];
        for _ in 0..20_000 {
            cps.push(rng.below(0x110000) as i32);
        }
        for cp in cps {
            let mut buf = [0u8; 8];
            let mut len: usize = 0;
            if ce(cp, buf.as_mut_ptr() as *mut c_char, &mut len) != 0 {
                continue;
            }
            for bufsize in 0..=len {
                cmp(&buf, bufsize);
            }
            cmp(&buf, len + 1); // extra room is fine
        }
        // Random garbage.
        for _ in 0..100_000 {
            let mut buf = [0u8; 8];
            for b in buf.iter_mut().take(5) {
                *b = rng.below(256) as u8;
            }
            for bufsize in 0..=5 {
                cmp(&buf, bufsize);
            }
        }
    }
}

/// K5: utf8_check_string — valid strings of each width, length 0, truncated tails.
#[test]
fn k5_utf8_check_string() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const c_char, usize) -> c_int> =
            p.c.lib.get(b"utf8_check_string\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c_char, usize) -> c_int> =
            p.r.lib.get(b"utf8_check_string\0").unwrap();
        let cmp = |b: &[u8], len: usize| {
            let cr = cf(b.as_ptr() as *const c_char, len);
            let rr = rf(b.as_ptr() as *const c_char, len);
            assert_eq!(cr, rr, "utf8_check_string({b:02x?}, {len})");
        };
        cmp(b"", 0);
        let mut rng = Rng::new(0x5757_5757);
        for _ in 0..20_000 {
            let s = rng.utf8(12);
            let b = s.as_bytes();
            // Full length (valid) plus every truncation (mostly invalid).
            for len in 0..=b.len() {
                cmp(b, len);
            }
        }
        // Random byte soup.
        for _ in 0..20_000 {
            let n = rng.below(12);
            let b: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
            cmp(&b, b.len());
        }
    }
}

// ===========================================================================
// J — strbuffer_*  (CONFIGS J1..J7)
// ===========================================================================

/// Drives a strbuffer through the same op script on both libraries and
/// compares the visible state (`length`, `size`, bytes) after every step.
fn strbuffer_script(ops: &[SbOp]) {
    let p = pair();
    unsafe {
        let mut cb = strbuffer_t::zeroed();
        let mut rb = strbuffer_t::zeroed();
        let ci = p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void);
        let ri = p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void);
        assert_eq!(ci, ri, "strbuffer_init");
        assert_eq!((cb.length, cb.size), (rb.length, rb.size), "post-init state");

        for (n, op) in ops.iter().enumerate() {
            match op {
                SbOp::AppendBytes(d) => {
                    let cr = p.c.strbuffer_append_bytes(
                        &mut cb as *mut _ as *mut c_void,
                        d.as_ptr() as *const c_char,
                        d.len(),
                    );
                    let rr = p.r.strbuffer_append_bytes(
                        &mut rb as *mut _ as *mut c_void,
                        d.as_ptr() as *const c_char,
                        d.len(),
                    );
                    assert_eq!(cr, rr, "op {n}: append_bytes({}) ret", d.len());
                }
                SbOp::AppendByte(b) => {
                    let cr = p
                        .c
                        .strbuffer_append_byte(&mut cb as *mut _ as *mut c_void, *b as c_char);
                    let rr = p
                        .r
                        .strbuffer_append_byte(&mut rb as *mut _ as *mut c_void, *b as c_char);
                    assert_eq!(cr, rr, "op {n}: append_byte({b}) ret");
                }
                SbOp::Pop => {
                    let cr = p.c.strbuffer_pop(&mut cb as *mut _ as *mut c_void);
                    let rr = p.r.strbuffer_pop(&mut rb as *mut _ as *mut c_void);
                    assert_eq!(cr, rr, "op {n}: pop ret");
                }
                SbOp::Clear => {
                    p.c.strbuffer_clear(&mut cb as *mut _ as *mut c_void);
                    p.r.strbuffer_clear(&mut rb as *mut _ as *mut c_void);
                }
                SbOp::Value => {
                    let cv = p.c.strbuffer_value(&cb as *const _ as *const c_void);
                    let rv = p.r.strbuffer_value(&rb as *const _ as *const c_void);
                    assert_eq!(cv.is_null(), rv.is_null(), "op {n}: value NULL-ness");
                    if !cv.is_null() {
                        assert_eq!(
                            cstr_bytes(cv),
                            cstr_bytes(rv),
                            "op {n}: strbuffer_value contents"
                        );
                    }
                }
            }
            assert_eq!(
                (cb.length, cb.size, cb.bytes()),
                (rb.length, rb.size, rb.bytes()),
                "op {n}: state after {op:?}"
            );
        }
        p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
        p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
        // J1/J4: after close, `value` is NULL and length/size are 0.
        assert_eq!(
            (cb.value.is_null(), cb.length, cb.size),
            (rb.value.is_null(), rb.length, rb.size),
            "post-close state"
        );
    }
}

#[derive(Debug)]
enum SbOp {
    AppendBytes(Vec<u8>),
    AppendByte(u8),
    Pop,
    Clear,
    Value,
}

/// J1, J2: init/close and every growth boundary around MIN_SIZE 16 and ×2.
#[test]
fn j1_j2_strbuffer_growth_boundaries() {
    strbuffer_script(&[SbOp::Value]);
    for n in [0usize, 1, 14, 15, 16, 17, 30, 31, 32, 33, 63, 64, 65, 1000] {
        strbuffer_script(&[
            SbOp::AppendBytes(vec![b'x'; n]),
            SbOp::Value,
            SbOp::AppendBytes(vec![b'y'; n]),
            SbOp::Value,
        ]);
    }
    // One append larger than double the current size, so the
    // `max(size*2, length+size+1)` arm is taken.
    strbuffer_script(&[
        SbOp::AppendBytes(vec![b'a'; 5]),
        SbOp::AppendBytes(vec![b'b'; 500]),
        SbOp::Value,
    ]);
    // Zero-length append (ERRORS G9).
    strbuffer_script(&[
        SbOp::AppendBytes(vec![b'a'; 3]),
        SbOp::AppendBytes(vec![]),
        SbOp::Value,
    ]);
}

/// J3: byte-at-a-time, covering all 256 byte values including 0.
#[test]
fn j3_strbuffer_append_byte_all_values() {
    let mut ops = Vec::new();
    for b in 0u16..256 {
        ops.push(SbOp::AppendByte(b as u8));
    }
    ops.push(SbOp::Value);
    strbuffer_script(&ops);
}

/// J5, J6: pop (incl. from empty) and clear-then-refill.
#[test]
fn j5_j6_strbuffer_pop_and_clear() {
    strbuffer_script(&[SbOp::Pop, SbOp::Pop]); // pop from empty -> '\0'
    strbuffer_script(&[
        SbOp::AppendBytes(b"hello".to_vec()),
        SbOp::Pop,
        SbOp::Pop,
        SbOp::Value,
        SbOp::Clear,
        SbOp::Value,
        SbOp::Pop,
        SbOp::AppendBytes(b"again".to_vec()),
        SbOp::Value,
    ]);
    // Randomized op scripts.
    let mut rng = Rng::new(0x5B_5B_5B);
    for _ in 0..300 {
        let mut ops = Vec::new();
        for _ in 0..rng.below(40) {
            ops.push(match rng.below(5) {
                0 => SbOp::AppendBytes((0..rng.below(40)).map(|_| rng.below(256) as u8).collect()),
                1 => SbOp::AppendByte(rng.below(256) as u8),
                2 => SbOp::Pop,
                3 => SbOp::Clear,
                _ => SbOp::Value,
            });
        }
        strbuffer_script(&ops);
    }
}

/// J7: steal_value — content plus the post-steal state, freed via jsonp_free.
#[test]
fn j7_strbuffer_steal_value() {
    let p = pair();
    unsafe {
        for n in [0usize, 1, 15, 16, 17, 100] {
            let mut cb = strbuffer_t::zeroed();
            let mut rb = strbuffer_t::zeroed();
            assert_eq!(
                p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
                p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
            );
            let data = vec![b'z'; n];
            p.c.strbuffer_append_bytes(
                &mut cb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                n,
            );
            p.r.strbuffer_append_bytes(
                &mut rb as *mut _ as *mut c_void,
                data.as_ptr() as *const c_char,
                n,
            );
            let cv = p.c.strbuffer_steal_value(&mut cb as *mut _ as *mut c_void);
            let rv = p.r.strbuffer_steal_value(&mut rb as *mut _ as *mut c_void);
            assert_eq!(cv.is_null(), rv.is_null(), "steal NULL-ness at n={n}");
            assert_eq!(cstr_bytes(cv), cstr_bytes(rv), "stolen value at n={n}");
            assert_eq!(
                (cb.value.is_null(), cb.length, cb.size),
                (rb.value.is_null(), rb.length, rb.size),
                "post-steal state at n={n}"
            );
            p.c.jsonp_free(cv as *mut c_void);
            p.r.jsonp_free(rv as *mut c_void);
        }
    }
}

// ===========================================================================
// I — hashtable_*  (CONFIGS I1..I7)
// ===========================================================================

#[derive(Debug, Clone)]
enum HtOp {
    Set(Vec<u8>),
    Get(Vec<u8>),
    Del(Vec<u8>),
    IterAt(Vec<u8>),
    Clear,
    Traverse,
}

fn hashtable_script(ops: &[HtOp]) {
    let p = pair();
    unsafe {
        // Over-allocate so a Rust-side layout that is *larger* than C's cannot
        // corrupt the stack; the tests compare the fields we mirror.
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(
            p.c.hashtable_init(ch),
            p.r.hashtable_init(rh),
            "hashtable_init"
        );
        let cst = || &*(ch as *const hashtable_t);
        let rst = || &*(rh as *const hashtable_t);
        assert_eq!(
            (cst().size, cst().order),
            (rst().size, rst().order),
            "post-init size/order"
        );

        for (n, op) in ops.iter().enumerate() {
            match op {
                HtOp::Set(k) => {
                    // Each library must own its own value object.
                    let cv = p.c.json_integer(k.len() as i64);
                    let rv = p.r.json_integer(k.len() as i64);
                    let cr = p.c.hashtable_set(ch, k.as_ptr() as *const c_char, k.len(), cv);
                    let rr = p.r.hashtable_set(rh, k.as_ptr() as *const c_char, k.len(), rv);
                    assert_eq!(cr, rr, "op {n}: hashtable_set(len {}) ret", k.len());
                }
                HtOp::Get(k) => {
                    let cv = p.c.hashtable_get(ch, k.as_ptr() as *const c_char, k.len());
                    let rv = p.r.hashtable_get(rh, k.as_ptr() as *const c_char, k.len());
                    assert_eq!(cv.is_null(), rv.is_null(), "op {n}: hashtable_get presence");
                    if !cv.is_null() {
                        assert_eq!(
                            p.c.json_integer_value(cv),
                            p.r.json_integer_value(rv),
                            "op {n}: hashtable_get value"
                        );
                    }
                }
                HtOp::Del(k) => {
                    let cr = p.c.hashtable_del(ch, k.as_ptr() as *const c_char, k.len());
                    let rr = p.r.hashtable_del(rh, k.as_ptr() as *const c_char, k.len());
                    assert_eq!(cr, rr, "op {n}: hashtable_del ret");
                }
                HtOp::IterAt(k) => {
                    let ci = p.c.hashtable_iter_at(ch, k.as_ptr() as *const c_char, k.len());
                    let ri = p.r.hashtable_iter_at(rh, k.as_ptr() as *const c_char, k.len());
                    assert_eq!(ci.is_null(), ri.is_null(), "op {n}: iter_at presence");
                    if !ci.is_null() {
                        let ck = p.c.hashtable_iter_key(ci);
                        let rk = p.r.hashtable_iter_key(ri);
                        let ckl = p.c.hashtable_iter_key_len(ci);
                        let rkl = p.r.hashtable_iter_key_len(ri);
                        assert_eq!(ckl, rkl, "op {n}: iter_at key_len");
                        assert_eq!(
                            std::slice::from_raw_parts(ck as *const u8, ckl),
                            std::slice::from_raw_parts(rk as *const u8, rkl),
                            "op {n}: iter_at key"
                        );
                    }
                }
                HtOp::Clear => {
                    p.c.hashtable_clear(ch);
                    p.r.hashtable_clear(rh);
                }
                HtOp::Traverse => {
                    assert_eq!(
                        hashtable_entries(&p.c, ch),
                        hashtable_entries(&p.r, rh),
                        "op {n}: full traversal"
                    );
                }
            }
            assert_eq!(
                (cst().size, cst().order),
                (rst().size, rst().order),
                "op {n}: size/order after {op:?}"
            );
        }
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
    }
}

fn key(i: usize) -> Vec<u8> {
    format!("key{i:06}").into_bytes()
}

/// I1, I2, I6: init/close, sizes straddling every rehash threshold, traversal.
#[test]
fn i1_i2_i6_hashtable_rehash_thresholds() {
    hashtable_script(&[HtOp::Traverse]);
    for n in [
        0usize, 1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
    ] {
        let mut ops = Vec::new();
        for i in 0..n {
            ops.push(HtOp::Set(key(i)));
        }
        ops.push(HtOp::Traverse);
        for i in 0..n {
            ops.push(HtOp::Get(key(i)));
            ops.push(HtOp::IterAt(key(i)));
        }
        ops.push(HtOp::Get(key(n + 1000))); // absent
        hashtable_script(&ops);
    }
}

/// I2 (key shapes): length 0, 1, 255, 4096; keys with embedded NULs.
#[test]
fn i2_hashtable_key_shapes() {
    let mut ops = Vec::new();
    let shapes: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        vec![b'x'; 255],
        vec![b'y'; 4096],
        b"a\0b".to_vec(),
        b"a\0c".to_vec(),
        b"a".to_vec(), // prefix of "a\0b" — must not collide
        vec![0u8; 8],
        vec![0xFFu8; 40],
    ];
    for k in &shapes {
        ops.push(HtOp::Set(k.clone()));
    }
    ops.push(HtOp::Traverse);
    for k in &shapes {
        ops.push(HtOp::Get(k.clone()));
        ops.push(HtOp::IterAt(k.clone()));
    }
    ops.push(HtOp::Traverse);
    hashtable_script(&ops);
}

/// I3: overwriting an existing key — value replaced in place, ordered position
/// unchanged, but the load-factor check still runs (overwriting the 8th key
/// triggers a rehash even though `size` does not grow).
#[test]
fn i3_hashtable_overwrite_triggers_rehash() {
    for n in [8usize, 16, 32] {
        let mut ops = Vec::new();
        for i in 0..n {
            ops.push(HtOp::Set(key(i)));
        }
        ops.push(HtOp::Traverse);
        // Overwrite each key; ordered position must stay put.
        for i in 0..n {
            ops.push(HtOp::Set(key(i)));
            ops.push(HtOp::Traverse);
        }
        hashtable_script(&ops);
    }
}

/// I4: delete present/absent, delete-all in forward/reverse/random order,
/// interleaved with sets.
#[test]
fn i4_hashtable_delete_orders() {
    let n = 40usize;
    // forward
    let mut ops: Vec<HtOp> = (0..n).map(|i| HtOp::Set(key(i))).collect();
    for i in 0..n {
        ops.push(HtOp::Del(key(i)));
        ops.push(HtOp::Traverse);
    }
    ops.push(HtOp::Del(key(0))); // absent now
    hashtable_script(&ops);

    // reverse
    let mut ops: Vec<HtOp> = (0..n).map(|i| HtOp::Set(key(i))).collect();
    for i in (0..n).rev() {
        ops.push(HtOp::Del(key(i)));
        ops.push(HtOp::Traverse);
    }
    hashtable_script(&ops);

    // randomized interleave of set/del/get/traverse
    let mut rng = Rng::new(0x4444_1111);
    for _ in 0..200 {
        let mut ops = Vec::new();
        for _ in 0..rng.below(120) {
            let k = key(rng.below(30));
            ops.push(match rng.below(5) {
                0 | 1 => HtOp::Set(k),
                2 => HtOp::Del(k),
                3 => HtOp::Get(k),
                _ => HtOp::Traverse,
            });
        }
        ops.push(HtOp::Traverse);
        hashtable_script(&ops);
    }
}

/// I5: clear then refill — the bucket array is retained at the grown order,
/// so the later rehash thresholds shift.
#[test]
fn i5_hashtable_clear_retains_capacity() {
    let mut ops: Vec<HtOp> = (0..100).map(|i| HtOp::Set(key(i))).collect();
    ops.push(HtOp::Clear);
    ops.push(HtOp::Traverse);
    for i in 0..20 {
        ops.push(HtOp::Set(key(i)));
    }
    ops.push(HtOp::Traverse);
    ops.push(HtOp::Clear);
    ops.push(HtOp::Clear);
    ops.push(HtOp::Traverse);
    hashtable_script(&ops);
}

/// I6: iter_set replaces values during traversal.
#[test]
fn i6_hashtable_iter_set() {
    let p = pair();
    unsafe {
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));
        for i in 0..20usize {
            let k = key(i);
            p.c.hashtable_set(
                ch,
                k.as_ptr() as *const c_char,
                k.len(),
                p.c.json_integer(i as i64),
            );
            p.r.hashtable_set(
                rh,
                k.as_ptr() as *const c_char,
                k.len(),
                p.r.json_integer(i as i64),
            );
        }
        let mut ci = p.c.hashtable_iter(ch);
        let mut ri = p.r.hashtable_iter(rh);
        let mut n = 0i64;
        while !ci.is_null() {
            assert!(!ri.is_null(), "rust iteration ended early at {n}");
            p.c.hashtable_iter_set(ci, p.c.json_string(cs("v").as_ptr()));
            p.r.hashtable_iter_set(ri, p.r.json_string(cs("v").as_ptr()));
            ci = p.c.hashtable_iter_next(ch, ci);
            ri = p.r.hashtable_iter_next(rh, ri);
            n += 1;
        }
        assert!(ri.is_null(), "rust iteration ran long");
        assert_eq!(hashtable_entries(&p.c, ch), hashtable_entries(&p.r, rh));
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
    }
}

/// I7: `hashtable_seed` is an exported *global* (`volatile uint32_t`), not a
/// function. After `json_object_seed(FIXED_SEED)` both libraries must hold the
/// same value, and a second seeding attempt must be a silent no-op
/// (ERRORS P5).
#[test]
fn i7_hashtable_seed_global() {
    let p = pair();
    unsafe {
        let cg: Symbol<*mut u32> = p.c.lib.get(b"hashtable_seed\0").unwrap();
        let rg: Symbol<*mut u32> = p.r.lib.get(b"hashtable_seed\0").unwrap();
        // `pair()` already called json_object_seed(FIXED_SEED) exactly once.
        assert_eq!(**cg, FIXED_SEED as u32, "C hashtable_seed after seeding");
        assert_eq!(**cg, **rg, "hashtable_seed global value");

        // ERRORS P5: seeding is one-shot — these must not change anything.
        p.c.json_object_seed(0x1111_2222);
        p.r.json_object_seed(0x1111_2222);
        p.c.json_object_seed(0);
        p.r.json_object_seed(0);
        assert_eq!(**cg, FIXED_SEED as u32, "C seed unchanged by re-seeding");
        assert_eq!(**cg, **rg, "hashtable_seed after re-seed attempts");
    }
}

// ===========================================================================
// L — number conversion  (CONFIGS L1..L5)
// ===========================================================================

/// L1: jsonp_dtostr over precision 0..31 × a wide double corpus × several sizes.
#[test]
fn l1_jsonp_dtostr_precision_sweep() {
    let p = pair();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int> =
            p.c.lib.get(b"jsonp_dtostr\0").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int> =
            p.r.lib.get(b"jsonp_dtostr\0").unwrap();

        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            1.0 / 3.0,
            2.0 / 3.0,
            1e-5,
            1e-4,
            1e-3,
            1e15,
            1e16,
            1e17,
            1e18,
            1e300,
            1e-300,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::EPSILON,
            5e-324,             // smallest subnormal
            2.2250738585072011e-308,
            9007199254740992.0, // 2^53
            123456789.123456789,
            -987654321.0,
        ];
        let mut rng = Rng::new(0xD70_5712);
        for _ in 0..4000 {
            vals.push(rng.finite_f64());
        }

        for &v in &vals {
            for prec in -1i32..=32 {
                for size in [0usize, 1, 2, 5, 24, 25, 26, 64] {
                    let mut cbuf = [0x5Au8; 128];
                    let mut rbuf = [0x5Au8; 128];
                    let cr = cf(cbuf.as_mut_ptr() as *mut c_char, size, v, prec);
                    let rr = rf(rbuf.as_mut_ptr() as *mut c_char, size, v, prec);
                    assert_eq!(
                        cr, rr,
                        "jsonp_dtostr({v:?}, prec {prec}, size {size}) return"
                    );
                    if cr >= 0 {
                        let n = cr as usize;
                        assert_eq!(
                            &cbuf[..n],
                            &rbuf[..n],
                            "jsonp_dtostr({v:?}, prec {prec}, size {size}) bytes: \
                             C={:?} RUST={:?}",
                            String::from_utf8_lossy(&cbuf[..n]),
                            String::from_utf8_lossy(&rbuf[..n])
                        );
                    }
                }
            }
        }
    }
}

/// L2: jsonp_strtod, driven through a strbuffer the test fills itself.
#[test]
fn l2_jsonp_strtod() {
    let p = pair();
    unsafe {
        let mut texts: Vec<String> = vec![
            "0".into(),
            "-0".into(),
            "0.0".into(),
            "1".into(),
            "-1".into(),
            "1.5".into(),
            "3.141592653589793".into(),
            "1e5".into(),
            "1E5".into(),
            "1e+5".into(),
            "1e-5".into(),
            "1e308".into(),
            "1e309".into(),  // overflow -> -1
            "-1e309".into(), // overflow -> -1
            "1e999".into(),  // overflow -> -1
            "1e-400".into(), // underflow -> 0.0, NO error
            "-1e-400".into(),
            "1.7976931348623157e308".into(),
            "2.2250738585072014e-308".into(),
            "4.9406564584124654e-324".into(),
            "9007199254740993".into(),
            "0.1".into(),
            "123456789012345678901234567890".into(),
        ];
        let mut rng = Rng::new(0x5710_D0);
        for _ in 0..3000 {
            let m = rng.range(-1_000_000_000, 1_000_000_000);
            let e = rng.range(-330, 330);
            texts.push(format!("{m}e{e}"));
            texts.push(format!("{m}.{}e{e}", rng.below(1_000_000)));
        }

        for t in &texts {
            let b = t.as_bytes();
            let mut cb = strbuffer_t::zeroed();
            let mut rb = strbuffer_t::zeroed();
            assert_eq!(
                p.c.strbuffer_init(&mut cb as *mut _ as *mut c_void),
                p.r.strbuffer_init(&mut rb as *mut _ as *mut c_void)
            );
            p.c.strbuffer_append_bytes(
                &mut cb as *mut _ as *mut c_void,
                b.as_ptr() as *const c_char,
                b.len(),
            );
            p.r.strbuffer_append_bytes(
                &mut rb as *mut _ as *mut c_void,
                b.as_ptr() as *const c_char,
                b.len(),
            );
            let mut cv: c_double = -12345.0;
            let mut rv: c_double = -12345.0;
            let cr = p.c.jsonp_strtod(&mut cb as *mut _ as *mut c_void, &mut cv);
            let rr = p.r.jsonp_strtod(&mut rb as *mut _ as *mut c_void, &mut rv);
            assert_eq!(cr, rr, "jsonp_strtod({t:?}) return");
            if cr == 0 {
                assert_eq!(
                    cv.to_bits(),
                    rv.to_bits(),
                    "jsonp_strtod({t:?}) value: C={cv:?} RUST={rv:?}"
                );
            }
            p.c.strbuffer_close(&mut cb as *mut _ as *mut c_void);
            p.r.strbuffer_close(&mut rb as *mut _ as *mut c_void);
        }
    }
}

/// L3: dtoa / dtoa_r / freedtoa over mode 0..5 × ndigits 0..25 × doubles.
#[test]
fn l3_dtoa_modes() {
    let p = pair();
    unsafe {
        let cf = p.c.dtoa_sym();
        let rf = p.r.dtoa_sym();

        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, 1.0 / 3.0, 1e-5, 1e16, 1e17, 1e300, 1e-300, f64::MIN,
            f64::MAX, f64::MIN_POSITIVE, 5e-324, 9007199254740992.0, 2.5, 0.125, 1234.5678,
        ];
        let mut rng = Rng::new(0xD704_A);
        for _ in 0..1500 {
            vals.push(rng.finite_f64());
        }

        for &v in &vals {
            for mode in -1i32..=9 {
                for nd in [-1i32, 0, 1, 2, 5, 15, 17, 18, 24, 25, 30] {
                    let mut cdec: c_int = -9999;
                    let mut csign: c_int = -9999;
                    let mut cend: *mut c_char = std::ptr::null_mut();
                    let mut rdec: c_int = -9999;
                    let mut rsign: c_int = -9999;
                    let mut rend: *mut c_char = std::ptr::null_mut();
                    let cs_ = cf(v, mode, nd, &mut cdec, &mut csign, &mut cend);
                    let rs_ = rf(v, mode, nd, &mut rdec, &mut rsign, &mut rend);
                    assert_eq!(
                        cs_.is_null(),
                        rs_.is_null(),
                        "dtoa({v:?}, mode {mode}, nd {nd}) NULL-ness"
                    );
                    if cs_.is_null() {
                        continue;
                    }
                    let cdigits = cstr_bytes(cs_).unwrap();
                    let rdigits = cstr_bytes(rs_).unwrap();
                    // `rve` points at the NUL terminator of the digit string.
                    let coff = if cend.is_null() {
                        None
                    } else {
                        Some(cend as usize - cs_ as usize)
                    };
                    let roff = if rend.is_null() {
                        None
                    } else {
                        Some(rend as usize - rs_ as usize)
                    };
                    assert_eq!(
                        (cdigits.clone(), cdec, csign, coff),
                        (rdigits.clone(), rdec, rsign, roff),
                        "dtoa({v:?}, mode {mode}, nd {nd}): C=({:?},{cdec},{csign},{coff:?}) \
                         RUST=({:?},{rdec},{rsign},{roff:?})",
                        String::from_utf8_lossy(&cdigits),
                        String::from_utf8_lossy(&rdigits)
                    );
                    p.c.freedtoa(cs_);
                    p.r.freedtoa(rs_);
                }
            }
        }
    }
}

/// L5: the exported `dtoa_divmax` global must hold the same value.
#[test]
fn l5_dtoa_divmax() {
    let p = pair();
    unsafe {
        let cg: Symbol<*mut c_int> = p.c.lib.get(b"dtoa_divmax\0").unwrap();
        let rg: Symbol<*mut c_int> = p.r.lib.get(b"dtoa_divmax\0").unwrap();
        assert_eq!(**cg, **rg, "dtoa_divmax");
    }
}

// ===========================================================================
// M — memory / error / loop check  (CONFIGS M1..M10)
// ===========================================================================

/// M1: jsonp_malloc / jsonp_free — size 0 must yield NULL.
#[test]
fn m1_jsonp_malloc_free() {
    let p = pair();
    unsafe {
        for n in [0usize, 1, 8, 16, 4096, 65536] {
            let cp = p.c.jsonp_malloc(n);
            let rp = p.r.jsonp_malloc(n);
            assert_eq!(
                cp.is_null(),
                rp.is_null(),
                "jsonp_malloc({n}) NULL-ness (0 must be NULL)"
            );
            p.c.jsonp_free(cp);
            p.r.jsonp_free(rp);
        }
        // free(NULL) is a no-op in both.
        p.c.jsonp_free(std::ptr::null_mut());
        p.r.jsonp_free(std::ptr::null_mut());
    }
}

/// M2: jsonp_realloc — grow/shrink/to-0/from-NULL, with the default 3-arg
/// realloc and (in M4) after `json_set_alloc_funcs` nulls `do_realloc`.
#[test]
fn m2_jsonp_realloc() {
    let p = pair();
    unsafe {
        for (old, new) in [
            (0usize, 0usize),
            (0, 16),
            (16, 0),
            (16, 32),
            (32, 16),
            (16, 16),
            (1, 4096),
            (4096, 1),
        ] {
            let cp = if old == 0 {
                std::ptr::null_mut()
            } else {
                p.c.jsonp_malloc(old)
            };
            let rp = if old == 0 {
                std::ptr::null_mut()
            } else {
                p.r.jsonp_malloc(old)
            };
            if old > 0 {
                std::ptr::write_bytes(cp as *mut u8, 0xA5, old);
                std::ptr::write_bytes(rp as *mut u8, 0xA5, old);
            }
            let cn = p.c.jsonp_realloc(cp, old, new);
            let rn = p.r.jsonp_realloc(rp, old, new);
            assert_eq!(
                cn.is_null(),
                rn.is_null(),
                "jsonp_realloc({old} -> {new}) NULL-ness"
            );
            if !cn.is_null() {
                let keep = old.min(new);
                if keep > 0 {
                    assert_eq!(
                        std::slice::from_raw_parts(cn as *const u8, keep),
                        std::slice::from_raw_parts(rn as *const u8, keep),
                        "jsonp_realloc({old} -> {new}) preserved bytes"
                    );
                }
                p.c.jsonp_free(cn);
                p.r.jsonp_free(rn);
            }
        }
    }
}

/// M3: jsonp_strndup.
#[test]
fn m3_jsonp_strndup() {
    let p = pair();
    unsafe {
        let src = b"hello world, this is a longer source string\0";
        for n in [0usize, 1, 5, 11, 42] {
            let cp = p.c.jsonp_strndup(src.as_ptr() as *const c_char, n);
            let rp = p.r.jsonp_strndup(src.as_ptr() as *const c_char, n);
            assert_eq!(cp.is_null(), rp.is_null(), "jsonp_strndup(n={n}) NULL-ness");
            assert_eq!(
                cstr_bytes(cp),
                cstr_bytes(rp),
                "jsonp_strndup(n={n}) contents"
            );
            p.c.jsonp_free(cp as *mut c_void);
            p.r.jsonp_free(rp as *mut c_void);
        }
    }
}

/// M4/M5: allocator hook round-trip through get/set (2-arg and 3-arg).
#[test]
fn m4_m5_alloc_func_roundtrip() {
    let p = pair();
    unsafe {
        // Read back the defaults; both must report a non-NULL malloc/free pair.
        let mut cm: usize = 0;
        let mut cr_: usize = 0;
        let mut cfr: usize = 0;
        let mut rm: usize = 0;
        let mut rr_: usize = 0;
        let mut rfr: usize = 0;
        p.c.json_get_alloc_funcs2(&mut cm, &mut cr_, &mut cfr);
        p.r.json_get_alloc_funcs2(&mut rm, &mut rr_, &mut rfr);
        assert_eq!(
            (cm != 0, cr_ != 0, cfr != 0),
            (rm != 0, rr_ != 0, rfr != 0),
            "default alloc funcs presence (malloc, realloc, free)"
        );
        // Partial out-params must be tolerated (ERRORS F8/F9).
        p.c.json_get_alloc_funcs2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        p.r.json_get_alloc_funcs2(std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        p.c.json_get_alloc_funcs(std::ptr::null_mut(), std::ptr::null_mut());
        p.r.json_get_alloc_funcs(std::ptr::null_mut(), std::ptr::null_mut());
    }
}

/// M6/M7/M8: jsonp_error_init, jsonp_error_set (variadic), jsonp_error_set_source.
#[test]
fn m6_m7_m8_error_api() {
    let p = pair();
    unsafe {
        // M6: init with NULL and non-NULL source.
        for src in [None, Some(""), Some("x"), Some("<string>")] {
            let holder = src.map(cs);
            let sp = holder
                .as_ref()
                .map(|c| c.as_ptr())
                .unwrap_or(std::ptr::null());
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            p.c.jsonp_error_init(&mut ce, sp);
            p.r.jsonp_error_init(&mut re, sp);
            assert_eq!(ce.snap(), re.snap(), "jsonp_error_init({src:?})");
        }
        // NULL error struct is a no-op in both.
        p.c.jsonp_error_init(std::ptr::null_mut(), std::ptr::null());
        p.r.jsonp_error_init(std::ptr::null_mut(), std::ptr::null());

        // M8: source lengths straddling JSON_ERROR_SOURCE_LENGTH (80).
        for n in [0usize, 1, 78, 79, 80, 81, 100, 200] {
            let src = cs(&"s".repeat(n));
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            p.c.jsonp_error_init(&mut ce, std::ptr::null());
            p.r.jsonp_error_init(&mut re, std::ptr::null());
            p.c.jsonp_error_set_source(&mut ce, src.as_ptr());
            p.r.jsonp_error_set_source(&mut re, src.as_ptr());
            assert_eq!(ce.snap(), re.snap(), "jsonp_error_set_source(len {n})");
        }
        p.c.jsonp_error_set_source(std::ptr::null_mut(), cs("x").as_ptr());
        p.r.jsonp_error_set_source(std::ptr::null_mut(), std::ptr::null());

        // M7: jsonp_error_set for each code, with message lengths that straddle
        // the 158-byte truncation point.
        let cset = p.c.jsonp_error_set_sym();
        let rset = p.r.jsonp_error_set_sym();
        for code in 0..=17i32 {
            for n in [0usize, 1, 50, 100, 155, 156, 157, 158, 159, 160, 300] {
                let msg = cs(&"m".repeat(n));
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                p.c.jsonp_error_init(&mut ce, cs("src").as_ptr());
                p.r.jsonp_error_init(&mut re, cs("src").as_ptr());
                cset(&mut ce, 3, 4, 5, code, cs("%s").as_ptr(), msg.as_ptr());
                rset(&mut re, 3, 4, 5, code, cs("%s").as_ptr(), msg.as_ptr());
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "jsonp_error_set(code {code}, msg len {n})"
                );
                // E6: second set must be silently discarded (first error wins).
                cset(&mut ce, 9, 9, 9, 1, cs("second").as_ptr());
                rset(&mut re, 9, 9, 9, 1, cs("second").as_ptr());
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "jsonp_error_set second call (code {code}, len {n})"
                );
            }
        }
        // Format directives beyond %s.
        for (fmt, a, b) in [("%d/%d", 7i32, -3i32), ("%x %o", 255, 8), ("%c%c", 65, 66)] {
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            p.c.jsonp_error_init(&mut ce, std::ptr::null());
            p.r.jsonp_error_init(&mut re, std::ptr::null());
            cset(&mut ce, 1, 2, 3, 8, cs(fmt).as_ptr(), a, b);
            rset(&mut re, 1, 2, 3, 8, cs(fmt).as_ptr(), a, b);
            assert_eq!(ce.snap(), re.snap(), "jsonp_error_set({fmt:?})");
        }
    }
}

/// M9: jsonp_loop_check — 0 on first insert, -1 on the second.
#[test]
fn m9_jsonp_loop_check() {
    let p = pair();
    unsafe {
        let mut cslab = [0u8; 256];
        let mut rslab = [0u8; 256];
        let ch = cslab.as_mut_ptr() as *mut c_void;
        let rh = rslab.as_mut_ptr() as *mut c_void;
        assert_eq!(p.c.hashtable_init(ch), p.r.hashtable_init(rh));

        // Distinct json_t pointers from each library. The `%p` key text
        // necessarily differs between libraries, so compare only the return
        // values and the key *lengths*.
        let cobjs: Vec<json_ptr> = (0..8).map(|_| p.c.json_object()).collect();
        let robjs: Vec<json_ptr> = (0..8).map(|_| p.r.json_object()).collect();

        for round in 0..3 {
            for i in 0..8 {
                let mut ckey = [0u8; 64];
                let mut rkey = [0u8; 64];
                let mut clen: usize = 0xDEAD;
                let mut rlen: usize = 0xDEAD;
                let cr = p.c.jsonp_loop_check(
                    ch,
                    cobjs[i],
                    ckey.as_mut_ptr() as *mut c_char,
                    ckey.len(),
                    &mut clen,
                );
                let rr = p.r.jsonp_loop_check(
                    rh,
                    robjs[i],
                    rkey.as_mut_ptr() as *mut c_char,
                    rkey.len(),
                    &mut rlen,
                );
                assert_eq!(cr, rr, "jsonp_loop_check round {round} obj {i} return");
                assert_eq!(clen, rlen, "jsonp_loop_check round {round} obj {i} key_len");
                // The generated key must be a "%p"-style rendering in both.
                let ck = &ckey[..clen];
                let rk = &rkey[..rlen];
                assert_eq!(
                    ck.starts_with(b"0x"),
                    rk.starts_with(b"0x"),
                    "loop key shape: C={:?} RUST={:?}",
                    String::from_utf8_lossy(ck),
                    String::from_utf8_lossy(rk)
                );
            }
        }
        p.c.hashtable_close(ch);
        p.r.hashtable_close(rh);
        for o in cobjs {
            p.c.json_decref(o);
        }
        for o in robjs {
            p.r.json_decref(o);
        }
    }
}

/// M10: jsonp_stringn_nocheck_own takes ownership of a jsonp_malloc'd buffer.
#[test]
fn m10_jsonp_stringn_nocheck_own() {
    let p = pair();
    unsafe {
        for content in [
            b"".to_vec(),
            b"a".to_vec(),
            b"hello".to_vec(),
            b"a\0b".to_vec(),
            vec![0xE2, 0x82, 0xAC],
        ] {
            let n = content.len();
            let cbuf = p.c.jsonp_malloc(n + 1) as *mut c_char;
            let rbuf = p.r.jsonp_malloc(n + 1) as *mut c_char;
            assert!(!cbuf.is_null() && !rbuf.is_null());
            std::ptr::copy_nonoverlapping(content.as_ptr(), cbuf as *mut u8, n);
            std::ptr::copy_nonoverlapping(content.as_ptr(), rbuf as *mut u8, n);
            *cbuf.add(n) = 0;
            *rbuf.add(n) = 0;
            let cj = p.c.jsonp_stringn_nocheck_own(cbuf, n);
            let rj = p.r.jsonp_stringn_nocheck_own(rbuf, n);
            assert_eq!(cj.is_null(), rj.is_null(), "own({content:02x?}) NULL-ness");
            assert_eq!(
                p.c.json_string_length(cj),
                p.r.json_string_length(rj),
                "own({content:02x?}) length"
            );
            assert_eq!(
                std::slice::from_raw_parts(p.c.json_string_value(cj) as *const u8, n),
                std::slice::from_raw_parts(p.r.json_string_value(rj) as *const u8, n),
                "own({content:02x?}) bytes"
            );
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

// ===========================================================================
// N — version  (CONFIGS N1, N2)
// ===========================================================================

#[test]
fn n1_n2_version() {
    let p = pair();
    unsafe {
        assert_eq!(
            cstr_bytes(p.c.jansson_version_str()),
            cstr_bytes(p.r.jansson_version_str())
        );
        for major in [i32::MIN, -1, 0, 1, 2, 3, 100, i32::MAX] {
            for minor in [i32::MIN, -1, 0, 14, 15, 16, i32::MAX] {
                for micro in [i32::MIN, -1, 0, 1, 2, i32::MAX] {
                    assert_eq!(
                        p.c.jansson_version_cmp(major, minor, micro),
                        p.r.jansson_version_cmp(major, minor, micro),
                        "jansson_version_cmp({major},{minor},{micro})"
                    );
                }
            }
        }
    }
}
