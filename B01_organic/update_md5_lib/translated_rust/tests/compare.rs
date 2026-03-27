use libloading::{Library, Symbol};
use std::path::PathBuf;

type TflacU8 = u8;
type TflacS32 = i32;
type TflacU32 = u32;
type TflacU64 = u64;

#[repr(C)]
#[derive(Clone)]
struct TflacMd5 {
    pos: TflacU32,
    total: TflacU64,
    buffer: [TflacU8; 64 + 8],
}

#[repr(C)]
#[derive(Clone)]
struct Tflac {
    md5_ctx: TflacMd5,
    cur_blocksize: TflacU32,
    channels: TflacU32,
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libupdate_md5_lib.so");
    p
}

// ---- Test 1: tflac_pack_u64le ----
#[test]
fn test_pack_u64le() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut TflacU8, TflacU64)> =
            c_lib.get(b"tflac_pack_u64le").expect("c sym");
        let r_fn: Symbol<unsafe extern "C" fn(*mut TflacU8, TflacU64)> =
            rust_lib.get(b"tflac_pack_u64le").expect("rust sym");

        let test_vals: &[TflacU64] = &[
            0,
            1,
            0xFF,
            0x0102030405060708,
            0xFFFFFFFFFFFFFFFF,
            0xDEADBEEFCAFEBABE,
        ];

        for &val in test_vals {
            let mut c_buf = [0u8; 8];
            let mut r_buf = [0u8; 8];
            c_fn(c_buf.as_mut_ptr(), val);
            r_fn(r_buf.as_mut_ptr(), val);
            assert_eq!(c_buf, r_buf, "pack_u64le mismatch for val=0x{:016X}", val);
        }
    }
}

// ---- Test 2: tflac_md5_addsample ----
#[test]
fn test_md5_addsample() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut TflacMd5, TflacU32, TflacU64)> =
            c_lib.get(b"tflac_md5_addsample").expect("c sym");
        let r_fn: Symbol<unsafe extern "C" fn(*mut TflacMd5, TflacU32, TflacU64)> =
            rust_lib.get(b"tflac_md5_addsample").expect("rust sym");

        // Test case 1: simple add, no wrap
        let base = TflacMd5 {
            pos: 0,
            total: 0,
            buffer: [0u8; 72],
        };

        let test_cases: &[(TflacU32, TflacU32, TflacU64)] = &[
            // (initial_pos, bits, val)
            (0, 64, 0x0102030405060708),
            (56, 64, 0xAABBCCDDEEFF0011), // will wrap past 64
            (60, 32, 0x12345678),
            (0, 16, 0xABCD),
            (62, 64, 0xDEADBEEFCAFEBABE), // wrap
        ];

        for &(pos, bits, val) in test_cases {
            let mut c_md5 = base.clone();
            c_md5.pos = pos;
            let mut r_md5 = c_md5.clone();

            c_fn(&mut c_md5, bits, val);
            r_fn(&mut r_md5, bits, val);

            let c_bytes: &[u8] = std::slice::from_raw_parts(
                &c_md5 as *const _ as *const u8,
                std::mem::size_of::<TflacMd5>(),
            );
            let r_bytes: &[u8] = std::slice::from_raw_parts(
                &r_md5 as *const _ as *const u8,
                std::mem::size_of::<TflacMd5>(),
            );
            assert_eq!(
                c_bytes, r_bytes,
                "md5_addsample mismatch for pos={}, bits={}, val=0x{:016X}",
                pos, bits, val
            );
        }

        // Test case: multiple sequential calls
        let mut c_md5 = base.clone();
        let mut r_md5 = base.clone();
        for i in 0u64..20 {
            c_fn(&mut c_md5, 64, i * 0x0101010101010101);
            r_fn(&mut r_md5, 64, i * 0x0101010101010101);
        }
        let c_bytes: &[u8] = std::slice::from_raw_parts(
            &c_md5 as *const _ as *const u8,
            std::mem::size_of::<TflacMd5>(),
        );
        let r_bytes: &[u8] = std::slice::from_raw_parts(
            &r_md5 as *const _ as *const u8,
            std::mem::size_of::<TflacMd5>(),
        );
        assert_eq!(c_bytes, r_bytes, "md5_addsample sequential calls mismatch");
    }
}

// ---- Test 3: update_md5 ----
#[test]
fn test_update_md5() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(*mut Tflac, *const TflacS32) -> TflacU32> =
            c_lib.get(b"update_md5").expect("c sym");
        let r_fn: Symbol<unsafe extern "C" fn(*mut Tflac, *const TflacS32) -> TflacU32> =
            rust_lib.get(b"update_md5").expect("rust sym");

        // Need 5 iterations * 32 elements per iteration = 160 samples minimum
        let samples: Vec<TflacS32> = (0..256).map(|i| (i * 7 + 13) as TflacS32).collect();

        let base = Tflac {
            md5_ctx: TflacMd5 {
                pos: 0,
                total: 0,
                buffer: [0u8; 72],
            },
            cur_blocksize: 100,
            channels: 2,
        };

        // Test with different initial states
        let configs: &[(TflacU32, TflacU32)] = &[
            (100, 2),
            (50, 4),
            (1, 1),
            (0, 0),
        ];

        for &(blocksize, channels) in configs {
            let mut c_t = base.clone();
            c_t.cur_blocksize = blocksize;
            c_t.channels = channels;
            let mut r_t = c_t.clone();

            let c_ret = c_fn(&mut c_t, samples.as_ptr());
            let r_ret = r_fn(&mut r_t, samples.as_ptr());

            assert_eq!(
                c_ret, r_ret,
                "update_md5 return mismatch for blocksize={}, channels={}",
                blocksize, channels
            );

            let c_bytes: &[u8] = std::slice::from_raw_parts(
                &c_t as *const _ as *const u8,
                std::mem::size_of::<Tflac>(),
            );
            let r_bytes: &[u8] = std::slice::from_raw_parts(
                &r_t as *const _ as *const u8,
                std::mem::size_of::<Tflac>(),
            );
            assert_eq!(
                c_bytes, r_bytes,
                "update_md5 struct mismatch for blocksize={}, channels={}",
                blocksize, channels
            );
        }
    }
}
