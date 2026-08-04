use libloading::{Library, Symbol};

#[repr(C)]
#[derive(Clone, Copy)]
struct TflacMd5 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

type Md5DigestFn = unsafe extern "C" fn(*const TflacMd5, *mut u8);

fn c_lib_path() -> String {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so");
    p.to_string_lossy().into_owned()
}

fn rust_lib_path() -> String {
    // Try both release and debug builds.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target").join("release").join("libmd5_digest_lib.so");
    let debug = manifest.join("target").join("debug").join("libmd5_digest_lib.so");
    if release.exists() {
        release.to_string_lossy().into_owned()
    } else {
        debug.to_string_lossy().into_owned()
    }
}

unsafe fn load_md5_digest(lib: &Library) -> Symbol<Md5DigestFn> {
    lib.get(b"md5_digest").expect("md5_digest symbol")
}

fn run_case(input: TflacMd5) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C .so");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust .so");

        let c_fn = load_md5_digest(&c_lib);
        let r_fn = load_md5_digest(&r_lib);

        let mut c_out = [0u8; 16];
        let mut r_out = [0u8; 16];

        c_fn(&input as *const _, c_out.as_mut_ptr());
        r_fn(&input as *const _, r_out.as_mut_ptr());

        assert_eq!(
            c_out, r_out,
            "Mismatch for input {{a={:#x}, b={:#x}, c={:#x}, d={:#x}}}\n  C    = {:?}\n  Rust = {:?}",
            input.a, input.b, input.c, input.d, c_out, r_out
        );
    }
}

#[test]
fn test_zeroes() {
    run_case(TflacMd5 { a: 0, b: 0, c: 0, d: 0 });
}

#[test]
fn test_all_ones() {
    run_case(TflacMd5 {
        a: 0xFFFF_FFFF,
        b: 0xFFFF_FFFF,
        c: 0xFFFF_FFFF,
        d: 0xFFFF_FFFF,
    });
}

#[test]
fn test_sequential_bytes() {
    run_case(TflacMd5 {
        a: 0x03020100,
        b: 0x07060504,
        c: 0x0B0A0908,
        d: 0x0F0E0D0C,
    });
}

#[test]
fn test_distinct_bytes() {
    run_case(TflacMd5 {
        a: 0xDEAD_BEEF,
        b: 0xCAFE_BABE,
        c: 0x1234_5678,
        d: 0x89AB_CDEF,
    });
}

#[test]
fn test_high_bits_only() {
    run_case(TflacMd5 {
        a: 0x8000_0000,
        b: 0x4000_0000,
        c: 0x2000_0000,
        d: 0x1000_0000,
    });
}

#[test]
fn test_low_bits_only() {
    run_case(TflacMd5 {
        a: 0x0000_0001,
        b: 0x0000_0002,
        c: 0x0000_0004,
        d: 0x0000_0008,
    });
}

#[test]
fn test_pseudo_random() {
    // Deterministic PRNG (xorshift) covering many values.
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
    for _ in 0..256 {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u32
        };
        let m = TflacMd5 {
            a: next(),
            b: next(),
            c: next(),
            d: next(),
        };
        run_case(m);
    }
}
