//! Level 3: the string arena -- `stbds_stralloc` / `stbds_strreset`.

mod common;

use common::*;
use std::ffi::{c_char, CStr};

fn empty_arena() -> StringArena {
    StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    }
}

/// Everything about an arena that must match: the two scalar fields, whether a
/// block list exists, and (for the returned pointer) the offset of the string
/// inside its owning block plus the bytes it points at.
fn snap_arena(a: &StringArena, ret: *mut c_char) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&a.remaining.to_le_bytes());
    out.push(a.block);
    out.push(a.mode);
    out.push(a.storage.is_null() as u8);
    if ret.is_null() {
        out.push(0);
    } else {
        out.push(1);
        // offset of the returned pointer within the head block's storage[]
        let base = unsafe { (a.storage as *mut u8).add(std::mem::size_of::<*mut u8>()) };
        let delta = (ret as isize) - (base as isize);
        out.extend_from_slice(&delta.to_le_bytes());
        let s = unsafe { CStr::from_ptr(ret) };
        out.extend_from_slice(s.to_bytes_with_nul());
    }
    out
}

#[test]
fn stralloc_small_strings() {
    let p = load_pair();
    let mut ca = empty_arena();
    let mut ra = empty_arena();

    for i in 0..400usize {
        let mut s: Vec<u8> = format!("key_{i}_{}", "x".repeat(i % 37)).into_bytes();
        s.push(0);
        let cp = unsafe { (p.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char) };
        let rp = unsafe { (p.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc small #{i}"),
            &snap_arena(&ca, cp),
            &snap_arena(&ra, rp),
        );
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    assert_bytes_eq(
        "strreset",
        &snap_arena(&ca, std::ptr::null_mut()),
        &snap_arena(&ra, std::ptr::null_mut()),
    );
}

#[test]
fn stralloc_oversized_strings() {
    // strings longer than the current block size take the "own block" path
    let p = load_pair();
    for &len in &[
        0usize, 1, 7, 8, 9, 511, 512, 513, 1023, 1024, 2000, 4096, 100_000,
    ] {
        let mut ca = empty_arena();
        let mut ra = empty_arena();
        let mut s: Vec<u8> = vec![b'z'; len];
        s.push(0);
        let cp = unsafe { (p.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char) };
        let rp = unsafe { (p.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc oversized len={len}"),
            &snap_arena(&ca, cp),
            &snap_arena(&ra, rp),
        );

        // a second, small allocation afterwards exercises the mixed state
        let mut t = b"tail\0".to_vec();
        let cp2 = unsafe { (p.c.stralloc)(&mut ca, t.as_mut_ptr() as *mut c_char) };
        let rp2 = unsafe { (p.r.stralloc)(&mut ra, t.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc oversized len={len} then small"),
            &snap_arena(&ca, cp2),
            &snap_arena(&ra, rp2),
        );

        unsafe {
            (p.c.strreset)(&mut ca);
            (p.r.strreset)(&mut ra);
        }
        assert_bytes_eq(
            &format!("strreset after len={len}"),
            &snap_arena(&ca, std::ptr::null_mut()),
            &snap_arena(&ra, std::ptr::null_mut()),
        );
    }
}

#[test]
fn stralloc_block_escalation() {
    // repeatedly fill blocks so that a->block walks up to the 1MB clamp
    let p = load_pair();
    let mut ca = empty_arena();
    let mut ra = empty_arena();
    let mut s = vec![b'q'; 400];
    s.push(0);
    for i in 0..2000usize {
        let cp = unsafe { (p.c.stralloc)(&mut ca, s.as_mut_ptr() as *mut c_char) };
        let rp = unsafe { (p.r.stralloc)(&mut ra, s.as_mut_ptr() as *mut c_char) };
        assert_bytes_eq(
            &format!("stralloc escalation #{i}"),
            &snap_arena(&ca, cp),
            &snap_arena(&ra, rp),
        );
    }
    assert!(ca.block > 1, "block counter never advanced");
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
}

#[test]
fn strreset_on_empty_arena() {
    let p = load_pair();
    let mut ca = empty_arena();
    let mut ra = empty_arena();
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.r.strreset)(&mut ra);
    }
    assert_bytes_eq(
        "strreset(empty)",
        &snap_arena(&ca, std::ptr::null_mut()),
        &snap_arena(&ra, std::ptr::null_mut()),
    );
}
