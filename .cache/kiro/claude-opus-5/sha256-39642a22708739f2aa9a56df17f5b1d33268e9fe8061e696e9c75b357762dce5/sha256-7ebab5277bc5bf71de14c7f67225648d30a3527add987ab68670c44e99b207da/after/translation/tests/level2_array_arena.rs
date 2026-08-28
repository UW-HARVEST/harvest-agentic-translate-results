//! Level 2: the dynamic-array primitive (`stbds_arrgrowf` / `stbds_arrfreef`)
//! and the string arena (`stbds_stralloc` / `stbds_strreset`).

mod harness;

use harness::*;
use snap::{ArenaSnap, HeaderSnap};
use std::ffi::{c_char, c_void};

unsafe fn set_length(a: *mut c_void, n: usize) {
    unsafe { *((a as *mut u8).sub(HEADER_SIZE) as *mut usize) = n }
}

unsafe fn set_temp(a: *mut c_void, n: isize) {
    unsafe { *((a as *mut u8).sub(HEADER_SIZE).add(24) as *mut isize) = n }
}

fn grow_sequence(p: &Pair, elemsize: usize, ops: &[(usize, usize)]) {
    unsafe {
        let mut ca: *mut c_void = std::ptr::null_mut();
        let mut ra: *mut c_void = std::ptr::null_mut();
        for (step, &(addlen, min_cap)) in ops.iter().enumerate() {
            let cn = (p.c.arrgrowf)(ca, elemsize, addlen, min_cap);
            let rn = (p.r.arrgrowf)(ra, elemsize, addlen, min_cap);
            // NB: pointer identity is *not* comparable here — `realloc` may or
            // may not grow a block in place.  The no-op path (min_cap <= cap)
            // is checked separately in `arrgrowf_noop_matches`.
            assert_eq!(cn.is_null(), rn.is_null(), "null-ness differs at step {step}");
            ca = cn;
            ra = rn;
            if ca.is_null() {
                continue;
            }
            let ch = snap::snap_header(ca);
            let rh = snap::snap_header(ra);
            assert_eq!(
                ch, rh,
                "elemsize={elemsize} step={step} op={:?}: header differs",
                (addlen, min_cap)
            );
            // emulate what stbds_arraddn does after growing
            let newlen = ch.length + addlen;
            set_length(ca, newlen);
            set_length(ra, newlen);
        }
        if !ca.is_null() {
            (p.c.arrfreef)(ca);
        }
        if !ra.is_null() {
            (p.r.arrfreef)(ra);
        }
    }
}

#[test]
fn arrgrowf_matches() {
    let p = pair();
    let ops: Vec<(usize, usize)> = vec![
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 0),
        (1, 0),
        (1, 0),
        (1, 0),
        (0, 0),
        (0, 3),
        (5, 0),
        (0, 100),
        (1, 0),
        (30, 0),
        (0, 7),
        (64, 0),
        (1, 1),
        (0, 1000),
        (500, 0),
        (0, 0),
    ];
    for elemsize in [1usize, 2, 3, 4, 8, 12, 16, 20, 24, 64] {
        grow_sequence(&p, elemsize, &ops);
    }
}

#[test]
fn arrgrowf_from_null_matches() {
    let p = pair();
    unsafe {
        for elemsize in [1usize, 4, 8, 20] {
            for &(addlen, min_cap) in &[
                (0usize, 0usize),
                (0, 1),
                (0, 4),
                (0, 5),
                (1, 0),
                (3, 0),
                (4, 0),
                (9, 0),
                (2, 7),
                (7, 2),
            ] {
                let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                assert_eq!(ca.is_null(), ra.is_null());
                if ca.is_null() {
                    continue;
                }
                assert_eq!(
                    snap::snap_header(ca),
                    snap::snap_header(ra),
                    "fresh grow elemsize={elemsize} addlen={addlen} min_cap={min_cap}"
                );
                (p.c.arrfreef)(ca);
                (p.r.arrfreef)(ra);
            }
        }
    }
}

/// `stbds_arrgrowf` must return the array unchanged when capacity suffices.
#[test]
fn arrgrowf_noop_matches() {
    let p = pair();
    unsafe {
        let ca = (p.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 16);
        let ra = (p.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 16);
        set_length(ca, 3);
        set_length(ra, 3);
        set_temp(ca, -5);
        set_temp(ra, -5);
        for min_cap in [0usize, 1, 3, 16] {
            let cn = (p.c.arrgrowf)(ca, 8, 0, min_cap);
            let rn = (p.r.arrgrowf)(ra, 8, 0, min_cap);
            assert_eq!(cn, ca, "C should not move the array (min_cap={min_cap})");
            assert_eq!(rn, ra, "Rust should not move the array (min_cap={min_cap})");
            assert_eq!(snap::snap_header(ca), snap::snap_header(ra));
        }
        assert_eq!(
            snap::snap_header(ca),
            HeaderSnap {
                length: 3,
                capacity: 16,
                has_hash_table: false,
                temp: -5
            }
        );
        (p.c.arrfreef)(ca);
        (p.r.arrfreef)(ra);
    }
}

// --- string arena -----------------------------------------------------------

/// 8-byte-aligned zeroed `stbds_string_arena`.
struct Arena(Box<[u64; 3]>);

impl Arena {
    fn new() -> Arena {
        Arena(Box::new([0u64; 3]))
    }
    fn ptr(&mut self) -> *mut c_void {
        self.0.as_mut_ptr() as *mut c_void
    }
    fn snap(&self) -> ArenaSnap {
        unsafe { snap::snap_arena_at(self.0.as_ptr() as *const u8) }
    }
}

fn cbuf(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

fn stralloc_sequence(p: &Pair, strings: &[String]) {
    let mut ca = Arena::new();
    let mut ra = Arena::new();
    unsafe {
        for (i, s) in strings.iter().enumerate() {
            let mut buf = cbuf(s);
            let cp = (p.c.stralloc)(ca.ptr(), buf.as_mut_ptr());
            let rp = (p.r.stralloc)(ra.ptr(), buf.as_mut_ptr());
            assert_eq!(
                snap::cstr(cp),
                snap::cstr(rp),
                "stralloc #{i} content for {:?}",
                &s[..s.len().min(40)]
            );
            assert_eq!(
                snap::cstr(cp),
                Some(s.as_bytes().to_vec()),
                "stralloc #{i} did not round-trip"
            );
            assert_eq!(ca.snap(), ra.snap(), "arena state after stralloc #{i}");
        }
        (p.c.strreset)(ca.ptr());
        (p.r.strreset)(ra.ptr());
        assert_eq!(ca.snap(), ra.snap(), "arena state after strreset");
        assert_eq!(
            ca.snap(),
            ArenaSnap {
                remaining: 0,
                block: 0,
                mode: 0,
                chain_len: 0
            }
        );
    }
}

#[test]
fn stralloc_short_strings() {
    let p = pair();
    let strings: Vec<String> = (0..400).map(|n| format!("key_{n}")).collect();
    stralloc_sequence(&p, &strings);
}

#[test]
fn stralloc_mixed_sizes() {
    let p = pair();
    let mut strings: Vec<String> = Vec::new();
    for n in 0..60 {
        strings.push("a".repeat(n * 13 + 1));
    }
    // force the "len > blocksize" oversized-block path several times
    strings.push("b".repeat(600));
    strings.push("c".repeat(5));
    strings.push("d".repeat(2000));
    strings.push("e".repeat(1));
    strings.push(String::new());
    for n in 0..40 {
        strings.push("f".repeat(n * 97));
    }
    stralloc_sequence(&p, &strings);
}

/// First call ever being an oversized string hits the `a->storage == NULL`
/// branch of the big-block path.
#[test]
fn stralloc_oversized_first() {
    let p = pair();
    stralloc_sequence(&p, &["z".repeat(4096), "y".repeat(3), "x".repeat(700)].map(|s| s).to_vec());
}

#[test]
fn strreset_on_empty_arena() {
    let p = pair();
    let mut ca = Arena::new();
    let mut ra = Arena::new();
    unsafe {
        (p.c.strreset)(ca.ptr());
        (p.r.strreset)(ra.ptr());
    }
    assert_eq!(ca.snap(), ra.snap());
}
