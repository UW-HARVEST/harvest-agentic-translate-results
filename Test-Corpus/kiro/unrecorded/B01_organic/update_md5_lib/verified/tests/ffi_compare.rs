use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
struct tflac_md5 {
    pos: u32,
    total: u64,
    buffer: [u8; 72],
}

#[repr(C)]
#[derive(Clone)]
struct tflac {
    md5_ctx: tflac_md5,
    cur_blocksize: u32,
    channels: u32,
}

impl tflac_md5 {
    fn zeroed() -> Self {
        Self { pos: 0, total: 0, buffer: [0u8; 72] }
    }
}

impl tflac {
    fn zeroed() -> Self {
        Self { md5_ctx: tflac_md5::zeroed(), cur_blocksize: 0, channels: 0 }
    }
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built alongside tests in the deps directory
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libupdate_md5_lib.so");
    p
}

type PackU64Le = unsafe extern "C" fn(*mut u8, u64);
type Md5AddSample = unsafe extern "C" fn(*mut tflac_md5, u32, u64);
type UpdateMd5 = unsafe extern "C" fn(*mut tflac, *const i32) -> u32;

// ---- tflac_pack_u64le tests ----

#[test]
fn test_pack_u64le_basic() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<PackU64Le> = c.get(b"tflac_pack_u64le").unwrap();
        let r_fn: Symbol<PackU64Le> = r.get(b"tflac_pack_u64le").unwrap();

        let vals: &[u64] = &[
            0, 1, 0xFF, 0x0102030405060708, 0xFFFFFFFFFFFFFFFF,
            0xDEADBEEFCAFEBABE, 0x8000000000000000,
        ];
        for &v in vals {
            let mut c_buf = [0u8; 8];
            let mut r_buf = [0u8; 8];
            c_fn(c_buf.as_mut_ptr(), v);
            r_fn(r_buf.as_mut_ptr(), v);
            assert_eq!(c_buf, r_buf, "pack_u64le mismatch for val={v:#x}");
        }
    }
}

// ---- tflac_md5_addsample tests ----

#[test]
fn test_md5_addsample_single() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<Md5AddSample> = c.get(b"tflac_md5_addsample").unwrap();
        let r_fn: Symbol<Md5AddSample> = r.get(b"tflac_md5_addsample").unwrap();

        // Test with various bits and val combinations
        let cases: &[(u32, u64)] = &[
            (64, 0x0102030405060708),
            (32, 0xDEADBEEF),
            (16, 0xCAFE),
            (8, 0x42),
            (64, 0xFFFFFFFFFFFFFFFF),
        ];
        for &(bits, val) in cases {
            let mut c_md5 = tflac_md5::zeroed();
            let mut r_md5 = tflac_md5::zeroed();
            c_fn(&mut c_md5, bits, val);
            r_fn(&mut r_md5, bits, val);
            assert_eq!(c_md5.pos, r_md5.pos, "pos mismatch bits={bits} val={val:#x}");
            assert_eq!(c_md5.total, r_md5.total, "total mismatch bits={bits} val={val:#x}");
            assert_eq!(c_md5.buffer, r_md5.buffer, "buffer mismatch bits={bits} val={val:#x}");
        }
    }
}

#[test]
fn test_md5_addsample_overflow() {
    // Test the case where pos wraps past 64
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<Md5AddSample> = c.get(b"tflac_md5_addsample").unwrap();
        let r_fn: Symbol<Md5AddSample> = r.get(b"tflac_md5_addsample").unwrap();

        let mut c_md5 = tflac_md5::zeroed();
        let mut r_md5 = tflac_md5::zeroed();
        // Set pos near boundary
        c_md5.pos = 60;
        r_md5.pos = 60;

        // Adding 8 bytes should trigger overflow (60 + 8 = 68 >= 64)
        c_fn(&mut c_md5, 64, 0xAABBCCDDEEFF0011);
        r_fn(&mut r_md5, 64, 0xAABBCCDDEEFF0011);
        assert_eq!(c_md5.pos, r_md5.pos, "pos mismatch after overflow");
        assert_eq!(c_md5.total, r_md5.total, "total mismatch after overflow");
        assert_eq!(c_md5.buffer, r_md5.buffer, "buffer mismatch after overflow");
    }
}

#[test]
fn test_md5_addsample_sequential() {
    // Multiple sequential calls to accumulate state
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<Md5AddSample> = c.get(b"tflac_md5_addsample").unwrap();
        let r_fn: Symbol<Md5AddSample> = r.get(b"tflac_md5_addsample").unwrap();

        let mut c_md5 = tflac_md5::zeroed();
        let mut r_md5 = tflac_md5::zeroed();

        for i in 0u64..20 {
            c_fn(&mut c_md5, 64, i * 0x0101010101010101);
            r_fn(&mut r_md5, 64, i * 0x0101010101010101);
            assert_eq!(c_md5.pos, r_md5.pos, "pos mismatch at iter {i}");
            assert_eq!(c_md5.total, r_md5.total, "total mismatch at iter {i}");
            assert_eq!(c_md5.buffer, r_md5.buffer, "buffer mismatch at iter {i}");
        }
    }
}

// ---- update_md5 tests ----

#[test]
fn test_update_md5_basic() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<UpdateMd5> = c.get(b"update_md5").unwrap();
        let r_fn: Symbol<UpdateMd5> = r.get(b"update_md5").unwrap();

        // Need enough samples: 5 iterations, each reads 8 samples then advances by 32.
        // Max offset = 4*32 + 7 = 135. Allocate 160 i32s.
        let samples: Vec<i32> = (0..160).map(|i| (i * 7 + 3) as i32).collect();

        let mut c_t = tflac::zeroed();
        let mut r_t = tflac::zeroed();
        c_t.cur_blocksize = 100;
        c_t.channels = 2;
        r_t.cur_blocksize = 100;
        r_t.channels = 2;

        let c_ret = c_fn(&mut c_t, samples.as_ptr());
        let r_ret = r_fn(&mut r_t, samples.as_ptr());

        assert_eq!(c_ret, r_ret, "return value mismatch");
        assert_eq!(c_t.md5_ctx.pos, r_t.md5_ctx.pos, "md5 pos mismatch");
        assert_eq!(c_t.md5_ctx.total, r_t.md5_ctx.total, "md5 total mismatch");
        assert_eq!(c_t.md5_ctx.buffer, r_t.md5_ctx.buffer, "md5 buffer mismatch");
        assert_eq!(c_t.cur_blocksize, r_t.cur_blocksize);
        assert_eq!(c_t.channels, r_t.channels);
    }
}

#[test]
fn test_update_md5_negative_samples() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<UpdateMd5> = c.get(b"update_md5").unwrap();
        let r_fn: Symbol<UpdateMd5> = r.get(b"update_md5").unwrap();

        let samples: Vec<i32> = (0..160).map(|i| -(i as i32) - 1).collect();

        let mut c_t = tflac::zeroed();
        let mut r_t = tflac::zeroed();
        c_t.cur_blocksize = 50;
        c_t.channels = 1;
        r_t.cur_blocksize = 50;
        r_t.channels = 1;

        let c_ret = c_fn(&mut c_t, samples.as_ptr());
        let r_ret = r_fn(&mut r_t, samples.as_ptr());

        assert_eq!(c_ret, r_ret, "return value mismatch");
        assert_eq!(c_t.md5_ctx.pos, r_t.md5_ctx.pos, "md5 pos mismatch");
        assert_eq!(c_t.md5_ctx.total, r_t.md5_ctx.total, "md5 total mismatch");
        assert_eq!(c_t.md5_ctx.buffer, r_t.md5_ctx.buffer, "md5 buffer mismatch");
    }
}

#[test]
fn test_update_md5_with_preexisting_state() {
    unsafe {
        let c = Library::new(c_lib_path()).unwrap();
        let r = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<UpdateMd5> = c.get(b"update_md5").unwrap();
        let r_fn: Symbol<UpdateMd5> = r.get(b"update_md5").unwrap();

        let samples: Vec<i32> = (0..160).map(|i| (i * 13 + 7) as i32).collect();

        // Start with non-zero md5 state
        let mut c_t = tflac::zeroed();
        let mut r_t = tflac::zeroed();
        c_t.cur_blocksize = 80;
        c_t.channels = 3;
        c_t.md5_ctx.pos = 30;
        c_t.md5_ctx.total = 240;
        r_t.cur_blocksize = 80;
        r_t.channels = 3;
        r_t.md5_ctx.pos = 30;
        r_t.md5_ctx.total = 240;

        let c_ret = c_fn(&mut c_t, samples.as_ptr());
        let r_ret = r_fn(&mut r_t, samples.as_ptr());

        assert_eq!(c_ret, r_ret, "return value mismatch");
        assert_eq!(c_t.md5_ctx.pos, r_t.md5_ctx.pos, "md5 pos mismatch");
        assert_eq!(c_t.md5_ctx.total, r_t.md5_ctx.total, "md5 total mismatch");
        assert_eq!(c_t.md5_ctx.buffer, r_t.md5_ctx.buffer, "md5 buffer mismatch");
    }
}
