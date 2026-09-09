use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::path::PathBuf;
use std::process::Command;

fn paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libzstd.so"),
        root.join("target/release/libzstd.so"),
    )
}

fn libs() -> (Library, Library) {
    let (c, r) = paths();
    unsafe { (Library::new(c).unwrap(), Library::new(r).unwrap()) }
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe { *lib.get::<T>(name).unwrap() }
}

fn bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            *seed as u8
        })
        .collect()
}

type Bound = unsafe extern "C" fn(usize) -> usize;
type Compress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
type Decompress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
type IsError = unsafe extern "C" fn(usize) -> c_uint;

#[test]
fn dynamic_symbol_surface_is_identical() {
    let (c, r) = paths();
    let list = |path: PathBuf| {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .unwrap();
        assert!(out.status.success());
        let mut names: Vec<_> = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
            .collect();
        names.sort();
        names
    };
    assert_eq!(list(c), list(r));
}

#[test]
fn metadata_and_exported_data_match() {
    type Version = unsafe extern "C" fn() -> c_uint;
    type VersionString = unsafe extern "C" fn() -> *const c_char;
    let (c, r) = libs();
    unsafe {
        let cv: Version = sym(&c, b"ZSTD_versionNumber\0");
        let rv: Version = sym(&r, b"ZSTD_versionNumber\0");
        assert_eq!(cv(), rv());
        let cs: VersionString = sym(&c, b"ZSTD_versionString\0");
        let rs: VersionString = sym(&r, b"ZSTD_versionString\0");
        assert_eq!(CStr::from_ptr(cs()).to_bytes(), CStr::from_ptr(rs()).to_bytes());

        for name in [b"g_debuglevel\0".as_slice(), b"g_ZSTD_threading_useless_symbol\0"] {
            let cp = *c.get::<*mut c_int>(name).unwrap();
            let rp = *r.get::<*mut c_int>(name).unwrap();
            *cp = 7;
            *rp = 7;
            assert_eq!(*cp, *rp);
        }
    }
}

#[test]
fn randomized_one_shot_roundtrips_match_byte_for_byte() {
    let (c, r) = libs();
    unsafe {
        let cb: Bound = sym(&c, b"ZSTD_compressBound\0");
        let rb: Bound = sym(&r, b"ZSTD_compressBound\0");
        let cc: Compress = sym(&c, b"ZSTD_compress\0");
        let rc: Compress = sym(&r, b"ZSTD_compress\0");
        let cd: Decompress = sym(&c, b"ZSTD_decompress\0");
        let rd: Decompress = sym(&r, b"ZSTD_decompress\0");
        let mut seed = 0x4d595df4d0f33173;
        for &len in &[0, 1, 2, 7, 31, 255, 4096, 131071, 131072, 131073, 300000] {
            for &level in &[-5, -1, 0, 1, 3, 9, 19, 22] {
                for variant in 0..3 {
                    let input = if variant == 0 { bytes(&mut seed, len) } else if variant == 1 {
                        vec![0x5a; len]
                    } else {
                        (0..len).map(|i| (i % 251) as u8).collect()
                    };
                    assert_eq!(cb(len), rb(len));
                    let cap = cb(len);
                    let mut co = vec![0u8; cap];
                    let mut ro = vec![0u8; cap];
                    let cn = cc(co.as_mut_ptr().cast(), cap, input.as_ptr().cast(), len, level);
                    let rn = rc(ro.as_mut_ptr().cast(), cap, input.as_ptr().cast(), len, level);
                    assert_eq!(cn, rn, "len={len} level={level} variant={variant}");
                    assert_eq!(&co[..cn], &ro[..rn]);
                    let mut cplain = vec![0u8; len.max(1)];
                    let mut rplain = vec![0u8; len.max(1)];
                    let cpn = cd(cplain.as_mut_ptr().cast(), len, co.as_ptr().cast(), cn);
                    let rpn = rd(rplain.as_mut_ptr().cast(), len, ro.as_ptr().cast(), rn);
                    assert_eq!(cpn, rpn);
                    assert_eq!(&cplain[..cpn], &rplain[..rpn]);
                    assert_eq!(&cplain[..cpn], input.as_slice());
                }
            }
        }
    }
}

#[test]
fn context_options_and_invalid_enums_match() {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Set = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
    type Reset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
    type Compress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
    let (c, r) = libs();
    unsafe {
        let ccreate: Create = sym(&c, b"ZSTD_createCCtx\0");
        let rcreate: Create = sym(&r, b"ZSTD_createCCtx\0");
        let cfree: Free = sym(&c, b"ZSTD_freeCCtx\0");
        let rfree: Free = sym(&r, b"ZSTD_freeCCtx\0");
        let cset: Set = sym(&c, b"ZSTD_CCtx_setParameter\0");
        let rset: Set = sym(&r, b"ZSTD_CCtx_setParameter\0");
        let creset: Reset = sym(&c, b"ZSTD_CCtx_reset\0");
        let rreset: Reset = sym(&r, b"ZSTD_CCtx_reset\0");
        let ccompress: Compress2 = sym(&c, b"ZSTD_compress2\0");
        let rcompress: Compress2 = sym(&r, b"ZSTD_compress2\0");
        let cb: Bound = sym(&c, b"ZSTD_compressBound\0");
        let cx = ccreate();
        let rx = rcreate();
        assert!(!cx.is_null() && !rx.is_null());
        for &(param, value) in &[
            (100, 5), (200, 0), (200, 1), (201, 0), (201, 1), (202, 0), (202, 1),
            (160, 0), (160, 1), (999_999, 0),
        ] {
            assert_eq!(cset(cx, param, value), rset(rx, param, value));
        }
        for reset in [1, 2, 3, 0, 4, -1, c_int::MAX] {
            assert_eq!(creset(cx, reset), rreset(rx, reset));
        }
        let input = b"context configured payload context configured payload";
        let cap = cb(input.len());
        let mut co = vec![0; cap];
        let mut ro = vec![0; cap];
        let cn = ccompress(cx, co.as_mut_ptr().cast(), cap, input.as_ptr().cast(), input.len());
        let rn = rcompress(rx, ro.as_mut_ptr().cast(), cap, input.as_ptr().cast(), input.len());
        assert_eq!(cn, rn);
        assert_eq!(&co[..cn], &ro[..rn]);
        assert_eq!(cfree(cx), rfree(rx));
        assert_eq!(cfree(std::ptr::null_mut()), rfree(std::ptr::null_mut()));
    }
}

#[test]
fn malformed_inputs_and_capacity_errors_match_exactly() {
    let (c, r) = libs();
    unsafe {
        let cc: Compress = sym(&c, b"ZSTD_compress\0");
        let rc: Compress = sym(&r, b"ZSTD_compress\0");
        let cd: Decompress = sym(&c, b"ZSTD_decompress\0");
        let rd: Decompress = sym(&r, b"ZSTD_decompress\0");
        let ci: IsError = sym(&c, b"ZSTD_isError\0");
        let ri: IsError = sym(&r, b"ZSTD_isError\0");
        let input = b"capacity";
        let mut one = [0u8; 1];
        let ce = cc(one.as_mut_ptr().cast(), 1, input.as_ptr().cast(), input.len(), 3);
        let re = rc(one.as_mut_ptr().cast(), 1, input.as_ptr().cast(), input.len(), 3);
        assert_eq!(ce, re);
        assert_eq!(ci(ce), ri(re));
        for bad in [&[][..], &[0][..], &[0x28, 0xb5, 0x2f][..], b"not a frame".as_slice()] {
            let mut co = [0u8; 64];
            let mut ro = [0u8; 64];
            let ce = cd(co.as_mut_ptr().cast(), co.len(), bad.as_ptr().cast(), bad.len());
            let re = rd(ro.as_mut_ptr().cast(), ro.len(), bad.as_ptr().cast(), bad.len());
            assert_eq!(ce, re, "bad={bad:?}");
            assert_eq!(ci(ce), ri(re));
        }
    }
}

#[test]
fn hashes_and_skippable_frames_match() {
    type Xxh32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
    type Write = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
    type Read = unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize;
    let (c, r) = libs();
    unsafe {
        let ch: Xxh32 = sym(&c, b"ZSTD_XXH32\0");
        let rh: Xxh32 = sym(&r, b"ZSTD_XXH32\0");
        let cw: Write = sym(&c, b"ZSTD_writeSkippableFrame\0");
        let rw: Write = sym(&r, b"ZSTD_writeSkippableFrame\0");
        let cr: Read = sym(&c, b"ZSTD_readSkippableFrame\0");
        let rr: Read = sym(&r, b"ZSTD_readSkippableFrame\0");
        let mut seed = 77;
        for len in [0, 1, 3, 16, 31, 32, 33, 4096] {
            let input = bytes(&mut seed, len);
            for hash_seed in [0, 1, u32::MAX] {
                assert_eq!(ch(input.as_ptr().cast(), len, hash_seed), rh(input.as_ptr().cast(), len, hash_seed));
            }
            for variant in [0, 1, 15, 16, u32::MAX] {
                let mut co = vec![0u8; len + 8];
                let mut ro = vec![0u8; len + 8];
                let cn = cw(co.as_mut_ptr().cast(), co.len(), input.as_ptr().cast(), len, variant);
                let rn = rw(ro.as_mut_ptr().cast(), ro.len(), input.as_ptr().cast(), len, variant);
                assert_eq!(cn, rn);
                if cn <= co.len() {
                    assert_eq!(&co[..cn], &ro[..rn]);
                    let mut cp = vec![0; len.max(1)];
                    let mut rp = vec![0; len.max(1)];
                    let mut cv = 99;
                    let mut rv = 99;
                    let cpn = cr(cp.as_mut_ptr().cast(), len, &mut cv, co.as_ptr().cast(), cn);
                    let rpn = rr(rp.as_mut_ptr().cast(), len, &mut rv, ro.as_ptr().cast(), rn);
                    assert_eq!((cpn, cv), (rpn, rv));
                    assert_eq!(&cp[..cpn], &rp[..rpn]);
                }
            }
        }
    }
}
