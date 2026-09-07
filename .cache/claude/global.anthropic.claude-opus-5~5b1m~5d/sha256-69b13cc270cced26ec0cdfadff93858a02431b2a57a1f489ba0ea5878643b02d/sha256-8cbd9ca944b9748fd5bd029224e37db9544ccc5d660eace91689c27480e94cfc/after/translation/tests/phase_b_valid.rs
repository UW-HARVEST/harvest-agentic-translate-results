//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every call goes through the dynamically
//! loaded `.so` exports of BOTH implementations; return values and captured
//! stdout bytes are compared byte-for-byte.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ===========================================================================
// Row 1 — validate_uint16_range
// ===========================================================================

#[test]
fn cfg01_validate_uint16_range_sweep() {
    let l = libs();
    let boundaries: [i32; 11] = [
        i32::MIN,
        -2,
        -1,
        0,
        1,
        2,
        65534,
        65535,
        65536,
        65537,
        i32::MAX,
    ];
    for v in boundaries {
        let c = unsafe { (l.c.validate_uint16_range)(v) };
        let r = unsafe { (l.r.validate_uint16_range)(v) };
        assert_eq!(c, r, "validate_uint16_range({v})");
    }
    let mut rng = Rng::new(SEED);
    for i in 0..4096 {
        let v = rng.spicy_i32();
        let c = unsafe { (l.c.validate_uint16_range)(v) };
        let r = unsafe { (l.r.validate_uint16_range)(v) };
        assert_eq!(c, r, "validate_uint16_range({v}) at iter {i}");
    }
}

// ===========================================================================
// Rows 2-6 — is_string_empty
// ===========================================================================

fn ise(l: &Libs, p: *const c_char, what: &str) {
    let c = unsafe { (l.c.is_string_empty)(p) };
    let r = unsafe { (l.r.is_string_empty)(p) };
    assert_eq!(c, r, "is_string_empty({what})");
}

#[test]
fn cfg02_is_string_empty_null() {
    let l = libs();
    ise(&l, std::ptr::null(), "NULL");
}

#[test]
fn cfg03_is_string_empty_empty() {
    let l = libs();
    let s = b"\0";
    ise(&l, s.as_ptr() as *const c_char, "\"\"");
}

#[test]
fn cfg04_is_string_empty_nonempty() {
    let l = libs();
    for s in [
        &b"a\0"[..],
        &b"Hello, World!\0"[..],
        &b" \0"[..],
        &b"\t\0"[..],
        &b"0\0"[..],
    ] {
        ise(&l, s.as_ptr() as *const c_char, "non-empty");
    }
}

#[test]
fn cfg05_is_string_empty_nul_then_garbage() {
    let l = libs();
    // First byte is NUL but the buffer continues: C only dereferences [0].
    let s = b"\0trailing garbage\0";
    ise(&l, s.as_ptr() as *const c_char, "NUL + garbage");
}

#[test]
fn cfg06_is_string_empty_high_bit() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..256 {
        // First byte >= 0x80 -> negative `char`; `if (*str)` must still be true.
        let mut v = vec![0x80u8 | (rng.next_u8() & 0x7F)];
        for _ in 0..rng.below(8) {
            let b = rng.next_u8();
            v.push(if b == 0 { 0xFF } else { b });
        }
        v.push(0);
        ise(&l, v.as_ptr() as *const c_char, "high-bit first byte");
    }
}

// ===========================================================================
// Rows 7-10 — create_buffer
// ===========================================================================

#[test]
fn cfg07_create_buffer_null() {
    let l = libs();
    let (cn, cb) = unsafe { l.c.create_buffer_owned(std::ptr::null()) };
    let (rn, rb) = unsafe { l.r.create_buffer_owned(std::ptr::null()) };
    assert!(cn && rn, "create_buffer(NULL) must be NULL in both (C={cn}, Rust={rn})");
    assert_eq!(cb, rb);
}

#[test]
fn cfg08_create_buffer_empty() {
    let l = libs();
    let s = b"\0";
    let (cn, cb) = unsafe { l.c.create_buffer_owned(s.as_ptr() as *const c_char) };
    let (rn, rb) = unsafe { l.r.create_buffer_owned(s.as_ptr() as *const c_char) };
    assert_eq!((cn, &cb), (rn, &rb));
    assert!(!cn, "create_buffer(\"\") should succeed");
    assert_eq!(cb, vec![0u8]);
}

#[test]
fn cfg09_create_buffer_random() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..512 {
        let s = rng.cstring(512);
        let (cn, cb) = unsafe { l.c.create_buffer_owned(s.as_ptr() as *const c_char) };
        let (rn, rb) = unsafe { l.r.create_buffer_owned(s.as_ptr() as *const c_char) };
        assert_eq!(cn, rn, "null-ness diverged at iter {i}");
        assert_eq!(cb, rb, "contents diverged at iter {i}");
        if !cn {
            assert_eq!(cb, s, "copy is not identical to the input at iter {i}");
        }
    }
}

#[test]
fn cfg10_create_buffer_bytes_after_nul() {
    let l = libs();
    let s = b"prefix\0suffix-must-not-be-copied\0";
    let (cn, cb) = unsafe { l.c.create_buffer_owned(s.as_ptr() as *const c_char) };
    let (rn, rb) = unsafe { l.r.create_buffer_owned(s.as_ptr() as *const c_char) };
    assert_eq!((cn, &cb), (rn, &rb));
    assert_eq!(cb, b"prefix\0".to_vec());
}

// ===========================================================================
// Rows 11-21 — find_char_in_buffer
// ===========================================================================

/// Compares the *offset* of the returned pointer (or `None`), since both
/// libraries are handed the very same buffer pointer.
fn fcib(l: &Libs, buf: *const c_char, size: usize, target: c_char, what: &str) -> Option<isize> {
    let cp = unsafe { (l.c.find_char_in_buffer)(buf, size, target) };
    let rp = unsafe { (l.r.find_char_in_buffer)(buf, size, target) };
    let off = |p: *mut c_char| -> Option<isize> {
        if p.is_null() {
            None
        } else {
            Some(unsafe { p.offset_from(buf) })
        }
    };
    let (co, ro) = (off(cp), off(rp));
    assert_eq!(
        co, ro,
        "find_char_in_buffer({what}, size={size}, target={target}) diverged"
    );
    co
}

#[test]
fn cfg11_find_char_null_buffer() {
    let l = libs();
    for size in [0usize, 1, 16, 4096, usize::MAX] {
        for t in [0i8, 1, 65, -1, -128, 127] {
            let got = fcib(&l, std::ptr::null(), size, t, "NULL");
            assert_eq!(got, None);
        }
    }
}

#[test]
fn cfg12_find_char_zero_size() {
    let l = libs();
    let b = b"abc\0";
    for t in [b'a' as i8, b'b' as i8, 0i8, -1i8] {
        let got = fcib(&l, b.as_ptr() as *const c_char, 0, t, "\"abc\"");
        assert_eq!(got, None, "size==0 must never match");
    }
}

#[test]
fn cfg13_find_char_size_one() {
    let l = libs();
    let b = [b'Q' as c_char, b'Q' as c_char];
    assert_eq!(fcib(&l, b.as_ptr(), 1, b'Q' as c_char, "size1 hit"), Some(0));
    assert_eq!(fcib(&l, b.as_ptr(), 1, b'Z' as c_char, "size1 miss"), None);
}

#[test]
fn cfg14_find_char_first_byte() {
    let l = libs();
    let b = b"Xyzzy plugh X";
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, b.len(), b'X' as c_char, "first"),
        Some(0)
    );
}

#[test]
fn cfg15_find_char_last_byte() {
    let l = libs();
    let b = b"abcdefgZ";
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, b.len(), b'Z' as c_char, "last"),
        Some((b.len() - 1) as isize)
    );
}

#[test]
fn cfg16_find_char_duplicated_first_wins() {
    let l = libs();
    let b = b"..k..k..k..";
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, b.len(), b'k' as c_char, "dup"),
        Some(2)
    );
}

#[test]
fn cfg17_find_char_absent() {
    let l = libs();
    let b = b"abcdefghij";
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, b.len(), b'~' as c_char, "absent"),
        None
    );
}

#[test]
fn cfg18_find_char_beyond_size() {
    let l = libs();
    let b = b"aaaaaaaaaaZ";
    // Z lives at index 10 -> a search limited to 10 bytes must miss it.
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, 10, b'Z' as c_char, "truncated"),
        None
    );
    assert_eq!(
        fcib(&l, b.as_ptr() as *const c_char, 11, b'Z' as c_char, "full"),
        Some(10)
    );
}

#[test]
fn cfg19_find_char_nul_target_embedded_nuls() {
    let l = libs();
    let b: [u8; 8] = [b'a', b'b', 0, b'c', 0, b'd', b'e', 0];
    let p = b.as_ptr() as *const c_char;
    assert_eq!(fcib(&l, p, 8, 0, "NUL target"), Some(2));
    assert_eq!(fcib(&l, p, 2, 0, "NUL target, truncated"), None);
    assert_eq!(fcib(&l, p, 3, 0, "NUL target, exact"), Some(2));
    assert_eq!(fcib(&l, p, 8, b'd' as c_char, "past a NUL"), Some(5));
}

#[test]
fn cfg20_find_char_high_bit() {
    let l = libs();
    // memchr converts its `int` argument to `unsigned char`; the C code passes
    // a (signed) `char`, so 0x80..0xFF arrive sign-extended. Both sides must
    // agree, and 0xNN must NOT alias 0xNN+0x100 etc.
    let mut buf = Vec::new();
    for b in 0x80u16..=0xFFu16 {
        buf.push(b as u8);
    }
    let p = buf.as_ptr() as *const c_char;
    for (i, b) in (0x80u16..=0xFFu16).enumerate() {
        let t = b as u8 as i8;
        assert_eq!(
            fcib(&l, p, buf.len(), t, "high-bit"),
            Some(i as isize),
            "byte 0x{b:02X} must be found at {i}"
        );
    }
    // An ASCII byte must not be found in an all-high-bit buffer.
    for t in [0i8, 1, 65, 127] {
        assert_eq!(fcib(&l, p, buf.len(), t, "ascii in high-bit buf"), None);
    }
}

#[test]
fn cfg21_find_char_randomized() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..2048 {
        let n = rng.below(129);
        let mut buf: Vec<u8> = (0..n).map(|_| rng.next_u8()).collect();
        if buf.is_empty() {
            buf.push(rng.next_u8());
        }
        // Sometimes bias the alphabet so hits are frequent.
        if rng.next_u64() % 2 == 0 {
            for b in buf.iter_mut() {
                *b %= 4;
            }
        }
        let target: i8 = match rng.next_u64() % 3 {
            0 => buf[rng.below(buf.len())] as i8, // guaranteed present
            1 => rng.next_u8() as i8,
            _ => (rng.next_u8() | 0x80) as i8,
        };
        // size from 0..=buf.len() so truncation is exercised too.
        let size = rng.below(buf.len() + 1);
        fcib(
            &l,
            buf.as_ptr() as *const c_char,
            size,
            target,
            &format!("random iter {i}"),
        );
    }
}

// ===========================================================================
// Rows 22-26 — counter operations
// ===========================================================================

fn step(l: &Libs, opidx: usize, v: c_int, what: &str) {
    let c = unsafe { (l.c.op(opidx))(v) };
    let r = unsafe { (l.r.op(opidx))(v) };
    assert_eq!(c, r, "{what}: op#{opidx}({v}) diverged");
}

/// Put both libraries into a known counter state.
fn sync_reset(l: &Libs, v: c_int) {
    let c = unsafe { (l.c.reset_counter)(v) };
    let r = unsafe { (l.r.reset_counter)(v) };
    assert_eq!(c, r, "reset_counter({v})");
}

#[test]
fn cfg22_increment_counter() {
    let l = libs();
    sync_reset(&l, 0);
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..2048 {
        step(&l, 0, rng.spicy_i32(), &format!("increment iter {i}"));
    }
}

#[test]
fn cfg23_decrement_counter() {
    let l = libs();
    sync_reset(&l, 0);
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..2048 {
        step(&l, 1, rng.spicy_i32(), &format!("decrement iter {i}"));
    }
}

#[test]
fn cfg24_multiply_counter() {
    let l = libs();
    sync_reset(&l, 1);
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..2048 {
        let v = match i % 5 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MIN,
            _ => rng.spicy_i32(),
        };
        step(&l, 2, v, &format!("multiply iter {i}"));
        if i % 7 == 0 {
            sync_reset(&l, rng.spicy_i32());
        }
    }
}

#[test]
fn cfg25_reset_counter() {
    let l = libs();
    for v in [i32::MIN, -1, 0, 1, i32::MAX] {
        sync_reset(&l, v);
    }
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..2048 {
        sync_reset(&l, rng.spicy_i32());
    }
}

#[test]
fn cfg26_counter_interleaved() {
    let l = libs();
    sync_reset(&l, 0);
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..2048 {
        let opidx = rng.below(4);
        step(&l, opidx, rng.spicy_i32(), &format!("interleaved iter {i}"));
    }
}

// ===========================================================================
// Rows 27-29 — apply_operation
// ===========================================================================

#[test]
fn cfg27_apply_operation_null() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..256 {
        let v = rng.spicy_i32();
        let c = unsafe { (l.c.apply_operation)(None, v) };
        let r = unsafe { (l.r.apply_operation)(None, v) };
        assert_eq!(c, r, "apply_operation(NULL, {v})");
        assert_eq!(c, -1, "C contract: NULL op returns -1");
    }
}

#[test]
fn cfg28_apply_operation_each_op() {
    let l = libs();
    sync_reset(&l, 0);
    let mut rng = Rng::new(SEED ^ 28);
    for i in 0..2048 {
        let opidx = rng.below(4);
        let v = rng.spicy_i32();
        // Each library's own function pointer -> each mutates its own counter.
        let c = unsafe { (l.c.apply_operation)(Some(l.c.op(opidx)), v) };
        let r = unsafe { (l.r.apply_operation)(Some(l.r.op(opidx)), v) };
        assert_eq!(c, r, "apply_operation(op#{opidx}, {v}) at iter {i}");
    }
}

#[test]
fn cfg29_apply_operation_uses_own_symbols() {
    // Documented in CONFIGS.md row 29: crossing function pointers between the
    // two libraries would make them share the wrong `static counter`, so we
    // only assert that each library's `apply_operation` really dispatches
    // through the pointer it was given (observed via the counter it moves).
    let l = libs();
    for lib in [&l.c, &l.r] {
        unsafe {
            assert_eq!((lib.apply_operation)(Some(lib.reset_counter), 100), 100);
            assert_eq!((lib.apply_operation)(Some(lib.increment_counter), 5), 105);
            assert_eq!((lib.apply_operation)(Some(lib.multiply_counter), 2), 210);
            assert_eq!((lib.apply_operation)(Some(lib.decrement_counter), 10), 200);
            assert_eq!((lib.apply_operation)(None, 999), -1);
        }
    }
}

// ===========================================================================
// Rows 30-44 — charinbuf (return value + stdout)
// ===========================================================================

fn cib(l: &Libs, mode: c_int, value: c_int, o1: c_int, o2: c_int) {
    let cres = capture_stdout(|| unsafe { (l.c.charinbuf)(mode, value, o1, o2) });
    let rres = capture_stdout(|| unsafe { (l.r.charinbuf)(mode, value, o1, o2) });
    assert_same_call(&format!("charinbuf({mode}, {value}, {o1}, {o2})"), cres, rres);
}

#[test]
fn cfg30_charinbuf_mode0_valid() {
    let l = libs();
    for v in [0, 1, 2, 100, 65534, 65535] {
        cib(&l, 0, v, 0, 0);
    }
    let mut rng = Rng::new(SEED ^ 30);
    for _ in 0..256 {
        let v = rng.range_i32(0, 65535);
        cib(&l, 0, v, rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn cfg31_charinbuf_mode0_invalid() {
    let l = libs();
    for v in [-1, -2, i32::MIN, 65536, 65537, i32::MAX] {
        cib(&l, 0, v, 0, 0);
    }
    let mut rng = Rng::new(SEED ^ 31);
    for _ in 0..256 {
        let v = if rng.next_u64() % 2 == 0 {
            rng.range_i32(i32::MIN, -1)
        } else {
            rng.range_i32(65536, i32::MAX)
        };
        cib(&l, 0, v, rng.next_i32(), rng.next_i32());
    }
}

#[test]
fn cfg32_charinbuf_mode1() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 32);
    for _ in 0..128 {
        cib(&l, 1, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn cfg33_charinbuf_mode2() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 33);
    for _ in 0..128 {
        cib(&l, 2, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn cfg34_charinbuf_mode3_zeros() {
    let l = libs();
    cib(&l, 3, 0, 0, 0);
}

#[test]
fn cfg35_charinbuf_mode3_positive() {
    let l = libs();
    for (v, a, b) in [(1, 2, 3), (10, 20, 30), (7, 0, 1), (100, 1, 2)] {
        cib(&l, 3, v, a, b);
    }
}

#[test]
fn cfg36_charinbuf_mode3_negative() {
    let l = libs();
    for (v, a, b) in [(-1, -2, -3), (-10, 5, -1), (5, -10, -2), (-100, -1, 3)] {
        cib(&l, 3, v, a, b);
    }
}

#[test]
fn cfg37_charinbuf_mode3_multiply_by_zero() {
    let l = libs();
    for v in [0, 1, -1, 12345, i32::MAX, i32::MIN] {
        cib(&l, 3, v, 7, 0);
    }
}

#[test]
fn cfg38_charinbuf_mode3_multiply_overflow() {
    let l = libs();
    for v in [i32::MAX, i32::MIN, 1 << 30, -(1 << 30), 65536] {
        for o2 in [-1, 2, 3, i32::MIN, i32::MAX, 65536] {
            cib(&l, 3, v, 0, o2);
        }
    }
}

#[test]
fn cfg39_charinbuf_mode3_add_sub_overflow() {
    let l = libs();
    for v in [i32::MAX, i32::MIN, i32::MAX - 4, i32::MIN + 4] {
        for o1 in [1, -1, i32::MAX, i32::MIN, 5] {
            cib(&l, 3, v, o1, 1);
        }
    }
}

#[test]
fn cfg40_charinbuf_mode3_randomized() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 40);
    for _ in 0..2048 {
        cib(&l, 3, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn cfg41_charinbuf_mode4() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 41);
    for _ in 0..128 {
        cib(&l, 4, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn cfg42_charinbuf_default_branch() {
    let l = libs();
    for m in [-1, 5, 6, 100, i32::MIN, i32::MAX, -2, 255, 256] {
        cib(&l, m, 0, 0, 0);
    }
    let mut rng = Rng::new(SEED ^ 42);
    for _ in 0..512 {
        let mut m = rng.spicy_i32();
        if (0..=4).contains(&m) {
            m = m.wrapping_add(1000);
        }
        cib(&l, m, rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
    }
}

#[test]
fn cfg43_charinbuf_repeated_all_modes() {
    let l = libs();
    for _round in 0..8 {
        for m in 0..=5 {
            cib(&l, m, 1234, 7, 3);
        }
    }
}

#[test]
fn cfg44_charinbuf_resets_counter_on_entry() {
    let l = libs();
    // Dirty the counter through the direct exports, then prove `charinbuf`
    // mode 3 ignores the previous state (C sets `counter = 0` at line 101).
    let mut rng = Rng::new(SEED ^ 44);
    for _ in 0..64 {
        let dirty = rng.spicy_i32();
        let c = unsafe { (l.c.reset_counter)(dirty) };
        let r = unsafe { (l.r.reset_counter)(dirty) };
        assert_eq!(c, r);
        cib(&l, 3, 11, 22, 33);
        // Reading state again after charinbuf: increment by 0 exposes it.
        let c = unsafe { (l.c.increment_counter)(0) };
        let r = unsafe { (l.r.increment_counter)(0) };
        assert_eq!(c, r, "counter state after charinbuf diverged");
    }
    // Also interleave with every other mode.
    for m in 0..=4 {
        unsafe {
            (l.c.reset_counter)(999);
            (l.r.reset_counter)(999);
        }
        cib(&l, m, 42, 9, 2);
        let c = unsafe { (l.c.increment_counter)(1) };
        let r = unsafe { (l.r.increment_counter)(1) };
        assert_eq!(c, r, "counter after mode {m}");
    }
}
