//! Phase C — read-extent contract (`ERRORS.md` rows 11..15, `CONFIGS.md` row 20).
//!
//! The C `&&` chain short-circuits, so `hdr_compare` provably does **not**
//! dereference certain bytes on certain paths:
//!
//! * `h1` is not touched at all while `hdr_valid(h2)` is false;
//! * `h1[0]` is *never* read on any path;
//! * `h2[1]` is not read when `h2[0] != 0xff`;
//! * `h2[2]` is not read when the `h2[1]` gates fail;
//! * `h1[2]` is not read when the `h1[1]`/`h2[1]` gate fails;
//! * nothing at index >= 3 is ever read.
//!
//! Those are observable through the FFI boundary: place the header so the byte
//! after it lives in an unmapped guard page and pass `NULL` for pointers the C
//! must not dereference. If the Rust translation reads one byte further than
//! the C, the process faults.
//!
//! Because a divergence here is a `SIGSEGV` rather than a failed assertion, the
//! work runs in a **child process** (a re-exec of this very test binary with
//! `--ignored --exact <worker>`), so the crash is reported as a test failure
//! instead of taking down the whole test binary. A companion "self-check"
//! worker deliberately touches a guard byte and MUST crash — that proves the
//! guard pages really are unmapped and the test has teeth.

mod common;

use common::*;
use libloading::os::unix::Library as UnixLibrary;
use std::ffi::{c_int, c_void};
use std::process::Command;

// ---------------------------------------------------------------------------
// parent side
// ---------------------------------------------------------------------------

fn run_worker(name: &str) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args([
            "--exact",
            name,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HDR_GUARD_CHILD", "1")
        .output()
        .expect("spawn worker")
}

/// `ERRORS.md` rows 11–15 / `CONFIGS.md` row 20.
#[test]
fn read_extent_matches_c() {
    let out = run_worker("guard_page_worker");
    assert!(
        out.status.success(),
        "guard-page worker did not succeed: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("GUARD-WORKER-OK"),
        "worker did not report completion:\n{stdout}"
    );
}

/// Proves the guard pages are genuinely unmapped: the worker touches the byte
/// right after the mapped region and must die from `SIGSEGV`.
#[test]
fn guard_pages_really_fault() {
    let out = run_worker("guard_page_selfcheck");
    assert!(
        !out.status.success(),
        "self-check unexpectedly survived reading a guard byte -- the guard \
         pages are NOT protected, so `read_extent_matches_c` would be \
         vacuous.\n--- stdout ---\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// ---------------------------------------------------------------------------
// child side
// ---------------------------------------------------------------------------

const PROT_NONE: c_int = 0;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;

type MmapFn = unsafe extern "C" fn(*mut c_void, usize, c_int, c_int, c_int, i64) -> *mut c_void;
type MprotectFn = unsafe extern "C" fn(*mut c_void, usize, c_int) -> c_int;
type GetPageSizeFn = unsafe extern "C" fn() -> c_int;

struct Libc {
    mmap: MmapFn,
    mprotect: MprotectFn,
    page: usize,
}

impl Libc {
    fn load() -> Libc {
        unsafe {
            // RTLD_DEFAULT: libc is already in the process' global namespace.
            let this = UnixLibrary::this();
            let mmap: MmapFn = *this.get::<MmapFn>(b"mmap\0").expect("mmap");
            let mprotect: MprotectFn =
                *this.get::<MprotectFn>(b"mprotect\0").expect("mprotect");
            let page = match this.get::<GetPageSizeFn>(b"getpagesize\0") {
                Ok(f) => (*f)() as usize,
                Err(_) => 4096,
            };
            std::mem::forget(this);
            Libc { mmap, mprotect, page }
        }
    }

    /// Two consecutive pages, both `PROT_NONE`.
    fn map_two_pages(&self) -> *mut u8 {
        unsafe {
            let p = (self.mmap)(
                std::ptr::null_mut(),
                2 * self.page,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p as isize != -1 && !p.is_null(), "mmap failed");
            p as *mut u8
        }
    }

    /// A `len`-byte readable region whose *last* byte is the last byte of a
    /// mapped page; everything at `ptr[len]` and beyond faults.
    fn guard_after(&self, len: usize) -> *mut u8 {
        let base = self.map_two_pages();
        unsafe {
            assert_eq!(
                (self.mprotect)(base as *mut c_void, self.page, PROT_READ | PROT_WRITE),
                0,
                "mprotect failed"
            );
            base.add(self.page - len)
        }
    }

    /// A region whose byte `0` is **unmapped** and whose bytes `1..=len` are
    /// readable; `ptr[0]` faults, `ptr[1..=len]` do not.
    fn guard_before(&self, len: usize) -> *mut u8 {
        let base = self.map_two_pages();
        unsafe {
            assert_eq!(
                (self.mprotect)(
                    base.add(self.page) as *mut c_void,
                    self.page,
                    PROT_READ | PROT_WRITE
                ),
                0,
                "mprotect failed"
            );
            assert!(len <= self.page);
            base.add(self.page - 1)
        }
    }
}

unsafe fn put(p: *mut u8, offset: usize, v: u8) {
    unsafe { p.add(offset).write(v) };
}

fn call_both(im: &Impls, h1: *const u8, h2: *const u8, ctx: &str) -> c_int {
    let (c, r) = im.both(h1, h2);
    assert_eq!(c, r, "DIVERGENCE {ctx}: C={c} Rust={r}");
    c
}

#[test]
#[ignore = "spawned as a child process by read_extent_matches_c"]
fn guard_page_worker() {
    assert!(
        std::env::var("HDR_GUARD_CHILD").is_ok(),
        "must be run as the guard child"
    );
    let im = load();
    let libc = Libc::load();
    let null: *const u8 = std::ptr::null();

    // -- Row 11: h2[0] != 0xff -> h2[1], h2[2] and all of h1 are never read.
    let h2_1 = libc.guard_after(1);
    for v in 0u16..=255 {
        if v == 0xFF {
            continue;
        }
        unsafe { put(h2_1, 0, v as u8) };
        let got = call_both(&im, null, h2_1, "row11/h2[0]!=0xff, 1 mapped byte, h1=NULL");
        assert_eq!(got, 0, "row11: h2[0]={v:#04x}");
    }
    println!("row11 ok (255 x 1-byte h2, h1=NULL)");

    // -- Row 12: the h2[1] gates fail -> h2[2] and all of h1 are never read.
    let h2_2 = libc.guard_after(2);
    unsafe { put(h2_2, 0, 0xFF) };
    let mut n12 = 0;
    for v in 0u16..=255 {
        let b1 = v as u8;
        // every b1 that hdr_valid rejects: bad sync bits OR reserved layer 0
        if sync_ok(b1) && ((b1 >> 1) & 3) != 0 {
            continue;
        }
        unsafe { put(h2_2, 1, b1) };
        let got = call_both(&im, null, h2_2, "row12/h2[1] gate fails, 2 mapped bytes, h1=NULL");
        assert_eq!(got, 0, "row12: h2[1]={b1:#04x}");
        n12 += 1;
    }
    assert_eq!(n12, 242, "expected 238 bad-sync + 4 layer-0 values");
    println!("row12 ok ({n12} x 2-byte h2, h1=NULL)");

    // -- Row 13: the h2[2] gates fail -> all of h1 is never read.
    let h2_3 = libc.guard_after(3);
    unsafe {
        put(h2_3, 0, 0xFF);
        put(h2_3, 1, 0xFB);
    }
    let mut n13 = 0;
    for v in 0u16..=255 {
        let b2 = v as u8;
        if (b2 >> 4) != 15 && ((b2 >> 2) & 3) != 3 {
            continue;
        }
        unsafe { put(h2_3, 2, b2) };
        let got = call_both(&im, null, h2_3, "row13/h2[2] gate fails, 3 mapped bytes, h1=NULL");
        assert_eq!(got, 0, "row13: h2[2]={b2:#04x}");
        n13 += 1;
    }
    assert!(n13 >= 16 + 60, "expected the reserved-bitrate/srate families, got {n13}");
    println!("row13 ok ({n13} x 3-byte h2, h1=NULL)");

    // -- Row 14: h1[1] gate fails -> h1[2] is never read; nothing at index 3.
    unsafe {
        put(h2_3, 0, 0xFF);
        put(h2_3, 1, 0xFB);
        put(h2_3, 2, 0x90); // valid: bitrate 9, srate 0
    }
    let h1_2 = libc.guard_after(2);
    unsafe { put(h1_2, 0, 0x00) };
    let mut n14 = 0;
    for v in 0u16..=255 {
        let b1 = v as u8;
        if (b1 ^ 0xFB) & 0xFE == 0 {
            continue; // would pass the gate and go on to read h1[2]
        }
        unsafe { put(h1_2, 1, b1) };
        let got = call_both(&im, h1_2, h2_3, "row14/h1[1] gate fails, h1[2] unmapped");
        assert_eq!(got, 0, "row14: h1[1]={b1:#04x}");
        n14 += 1;
    }
    assert_eq!(n14, 254, "expected 254 mismatching h1[1] values");
    println!("row14 ok ({n14} x 2-byte h1 with unmapped h1[2])");

    // -- Row 15: success with exactly 3 mapped bytes on both sides (nothing at
    //    index >= 3 is read).
    let h1_3 = libc.guard_after(3);
    unsafe {
        put(h1_3, 0, 0x00);
        put(h1_3, 1, 0xFA); // == 0xFB under the 0xFE mask
        put(h1_3, 2, 0x93); // same srate bits, non-zero bitrate
    }
    let got = call_both(&im, h1_3, h2_3, "row15/success, 3 mapped bytes each");
    assert_eq!(got, 1, "row15: expected a match");
    // and a rejection that still reads h1[2]
    unsafe { put(h1_3, 2, 0x94) }; // srate bits differ
    let got = call_both(&im, h1_3, h2_3, "row15/srate mismatch, 3 mapped bytes each");
    assert_eq!(got, 0);
    unsafe { put(h1_3, 2, 0x03) }; // bitrate index 0 -> free-format mismatch
    let got = call_both(&im, h1_3, h2_3, "row15/free-format mismatch, 3 mapped bytes each");
    assert_eq!(got, 0);
    println!("row15 ok (3-byte h1 and h2, no read at index >= 3)");

    // -- Row 15b: h1[0] is never read (its page is unmapped).
    let h1_pre = libc.guard_before(2);
    unsafe {
        put(h1_pre, 1, 0xFA);
        put(h1_pre, 2, 0x93);
    }
    let got = call_both(&im, h1_pre, h2_3, "row15b/h1[0] unmapped, success path");
    assert_eq!(got, 1, "row15b: expected a match without reading h1[0]");
    unsafe { put(h1_pre, 1, 0xF9) }; // mismatch under 0xFE
    let got = call_both(&im, h1_pre, h2_3, "row15b/h1[0] unmapped, mismatch path");
    assert_eq!(got, 0);
    println!("row15b ok (h1[0] never dereferenced)");

    println!("GUARD-WORKER-OK");
}

#[test]
#[ignore = "spawned as a child process by guard_pages_really_fault; MUST crash"]
fn guard_page_selfcheck() {
    assert!(
        std::env::var("HDR_GUARD_CHILD").is_ok(),
        "must be run as the guard child"
    );
    let libc = Libc::load();
    let p = libc.guard_after(3);
    // the three mapped bytes are readable ...
    unsafe {
        put(p, 0, 1);
        put(p, 1, 2);
        put(p, 2, 3);
        let sum = p.read_volatile() + p.add(1).read_volatile() + p.add(2).read_volatile();
        assert_eq!(sum, 6);
        println!("mapped bytes readable, now touching the guard byte ...");
        // ... and this one must fault.
        let boom = p.add(3).read_volatile();
        println!("NO FAULT: guard byte read as {boom}");
    }
    // Also check the guard_before layout faults at index 0.
    let q = libc.guard_before(2);
    unsafe {
        let boom = q.read_volatile();
        println!("NO FAULT: guard_before byte 0 read as {boom}");
    }
}
