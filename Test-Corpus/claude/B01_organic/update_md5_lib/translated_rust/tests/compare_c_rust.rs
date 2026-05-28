//! Integration tests that compare the C implementation and the Rust
//! implementation by loading both shared libraries with `libloading` and
//! invoking their exported FFI symbols. Outputs (return values + mutated
//! state of the structs) must match byte-for-byte.

use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
struct TflacMd5 {
    pos: u32,
    total: u64,
    buffer: [u8; 64 + 8],
}

#[repr(C)]
#[derive(Clone)]
struct Tflac {
    md5_ctx: TflacMd5,
    cur_blocksize: u32,
    channels: u32,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The Rust [lib] is named `update_md5_lib`, producing libupdate_md5_lib.so.
    // Cargo runs tests from the manifest dir.
    let candidates = [
        manifest_dir().join("target/debug/libupdate_md5_lib.so"),
        manifest_dir().join("target/release/libupdate_md5_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust .so not found, looked in: {:?}",
        candidates
    );
}

fn load_c() -> Library {
    unsafe { Library::new(c_lib_path()).expect("failed to load C .so") }
}

fn load_rust() -> Library {
    unsafe { Library::new(rust_lib_path()).expect("failed to load Rust .so") }
}

// Ensure the Rust .so exists before the test runs by issuing a build hint via
// the build script. To keep this simple, we rely on the user running
// `cargo build` before `cargo test`. The test setup explicitly checks.

unsafe fn pack_u64le(lib: &Library, d: *mut u8, n: u64) {
    let f: Symbol<unsafe extern "C" fn(*mut u8, u64)> =
        unsafe { lib.get(b"tflac_pack_u64le").expect("missing tflac_pack_u64le") };
    unsafe { f(d, n) }
}

unsafe fn md5_addsample(lib: &Library, m: *mut TflacMd5, bits: u32, val: u64) {
    let f: Symbol<unsafe extern "C" fn(*mut TflacMd5, u32, u64)> =
        unsafe { lib.get(b"tflac_md5_addsample").expect("missing tflac_md5_addsample") };
    unsafe { f(m, bits, val) }
}

unsafe fn update_md5(lib: &Library, t: *mut Tflac, samples: *const i32) -> u32 {
    let f: Symbol<unsafe extern "C" fn(*mut Tflac, *const i32) -> u32> =
        unsafe { lib.get(b"update_md5").expect("missing update_md5") };
    unsafe { f(t, samples) }
}

#[test]
fn test_pack_u64le_various_values() {
    let c = load_c();
    let r = load_rust();

    let cases = [
        0u64,
        1,
        0xFFu64,
        0x1234_5678_9ABC_DEF0u64,
        u64::MAX,
        0xDEAD_BEEF_CAFE_BABEu64,
        0x0102_0304_0506_0708u64,
        0x8000_0000_0000_0000u64,
    ];

    for &n in &cases {
        let mut c_buf = [0u8; 8];
        let mut r_buf = [0u8; 8];
        unsafe {
            pack_u64le(&c, c_buf.as_mut_ptr(), n);
            pack_u64le(&r, r_buf.as_mut_ptr(), n);
        }
        assert_eq!(c_buf, r_buf, "pack_u64le diverges for n={:#x}", n);
    }
}

fn fresh_md5() -> TflacMd5 {
    TflacMd5 {
        pos: 0,
        total: 0,
        buffer: [0u8; 72],
    }
}

fn md5_eq(a: &TflacMd5, b: &TflacMd5) -> bool {
    a.pos == b.pos && a.total == b.total && a.buffer == b.buffer
}

#[test]
fn test_md5_addsample_single_call() {
    let c = load_c();
    let r = load_rust();

    let cases: &[(u32, u64)] = &[
        (8, 0xAA),
        (16, 0x1234),
        (32, 0xDEAD_BEEF),
        (64, 0x0102_0304_0506_0708),
        (64, u64::MAX),
        (64, 0),
    ];

    for &(bits, val) in cases {
        let mut cm = fresh_md5();
        let mut rm = fresh_md5();
        unsafe {
            md5_addsample(&c, &mut cm, bits, val);
            md5_addsample(&r, &mut rm, bits, val);
        }
        assert!(
            md5_eq(&cm, &rm),
            "md5_addsample diverges for bits={} val={:#x}: C pos={} total={} buf={:?} ; Rust pos={} total={} buf={:?}",
            bits, val, cm.pos, cm.total, cm.buffer, rm.pos, rm.total, rm.buffer
        );
    }
}

#[test]
fn test_md5_addsample_many_calls_wrap_buffer() {
    let c = load_c();
    let r = load_rust();

    let mut cm = fresh_md5();
    let mut rm = fresh_md5();

    // Loop enough to make `pos` wrap several times.
    let mut v: u64 = 0xCAFE_BABE_DEAD_BEEFu64;
    for _ in 0..40 {
        unsafe {
            md5_addsample(&c, &mut cm, 64, v);
            md5_addsample(&r, &mut rm, 64, v);
        }
        v = v.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0x12345);
        assert!(
            md5_eq(&cm, &rm),
            "diverged at pos C={} R={} total C={} R={}",
            cm.pos, rm.pos, cm.total, rm.total
        );
    }
}

#[test]
fn test_md5_addsample_unaligned_bits() {
    let c = load_c();
    let r = load_rust();

    let mut cm = fresh_md5();
    let mut rm = fresh_md5();

    // 24-bit samples — pos won't be a multiple of 8, exercising buffer wrap.
    let mut v: u64 = 0x123456;
    for _ in 0..30 {
        unsafe {
            md5_addsample(&c, &mut cm, 24, v);
            md5_addsample(&r, &mut rm, 24, v);
        }
        v = v.wrapping_add(0x111111);
        assert!(md5_eq(&cm, &rm));
    }
}

fn fresh_tflac(blocksize: u32, channels: u32) -> Tflac {
    Tflac {
        md5_ctx: fresh_md5(),
        cur_blocksize: blocksize,
        channels,
    }
}

fn tflac_eq(a: &Tflac, b: &Tflac) -> bool {
    a.cur_blocksize == b.cur_blocksize
        && a.channels == b.channels
        && md5_eq(&a.md5_ctx, &b.md5_ctx)
}

#[test]
fn test_update_md5_basic() {
    let c = load_c();
    let r = load_rust();

    // The C `update_md5` function loops 5 iterations, each iteration reading
    // 8 samples and advancing the pointer by `8 * sizeof(tflac_s32)` = 32
    // *elements*. So we need 4 * 32 + 8 = 136 sample slots minimum.
    // Use 256 to be safe.
    let n = 256usize;
    let samples: Vec<i32> = (0..n as i32)
        .map(|i| i.wrapping_mul(0x01010101))
        .collect();

    let mut tc = fresh_tflac(1024, 2);
    let mut tr = fresh_tflac(1024, 2);

    let rc = unsafe { update_md5(&c, &mut tc, samples.as_ptr()) };
    let rr = unsafe { update_md5(&r, &mut tr, samples.as_ptr()) };

    assert_eq!(rc, rr, "update_md5 return value mismatch");
    assert!(tflac_eq(&tc, &tr), "update_md5 state mismatch");
}

#[test]
fn test_update_md5_various_blocksizes_channels() {
    let c = load_c();
    let r = load_rust();

    let n = 512usize;
    let samples: Vec<i32> = (0..n as i32)
        .map(|i| (i.wrapping_mul(2654435761u32 as i32)).wrapping_add(7))
        .collect();

    for &(blocksize, channels) in &[
        (40u32, 1u32),
        (40, 2),
        (1024, 1),
        (1024, 6),
        (1, 1),
        (0, 0),
        (8, 8),
    ] {
        let mut tc = fresh_tflac(blocksize, channels);
        let mut tr = fresh_tflac(blocksize, channels);

        let rc = unsafe { update_md5(&c, &mut tc, samples.as_ptr()) };
        let rr = unsafe { update_md5(&r, &mut tr, samples.as_ptr()) };

        assert_eq!(rc, rr, "rc mismatch for ({}, {})", blocksize, channels);
        assert!(
            tflac_eq(&tc, &tr),
            "state mismatch for ({}, {}); C pos={} total={}, R pos={} total={}",
            blocksize, channels,
            tc.md5_ctx.pos, tc.md5_ctx.total,
            tr.md5_ctx.pos, tr.md5_ctx.total
        );
    }
}

#[test]
fn test_update_md5_repeated_calls() {
    let c = load_c();
    let r = load_rust();

    let n = 1024usize;
    let samples: Vec<i32> = (0..n as i32).collect();

    let mut tc = fresh_tflac(2048, 4);
    let mut tr = fresh_tflac(2048, 4);

    for _ in 0..5 {
        let rc = unsafe { update_md5(&c, &mut tc, samples.as_ptr()) };
        let rr = unsafe { update_md5(&r, &mut tr, samples.as_ptr()) };
        assert_eq!(rc, rr);
        assert!(tflac_eq(&tc, &tr));
    }
}
