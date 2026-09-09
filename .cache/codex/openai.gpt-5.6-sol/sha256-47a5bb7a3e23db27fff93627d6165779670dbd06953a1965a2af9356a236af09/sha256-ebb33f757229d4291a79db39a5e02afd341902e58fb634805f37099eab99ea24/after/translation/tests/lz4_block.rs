mod common;

use common::{Libraries, Rng, ptr_or_dangling};
use std::ffi::{c_char, c_int, c_void};

type Compress4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
type Compress5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type CompressState6 =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type Decompress4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;

fn corpus(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut values = vec![
        vec![],
        vec![0],
        vec![1, 2, 3],
        vec![7; 4],
        vec![7; 12],
        vec![7; 13],
        vec![0; 255],
        (0..256).map(|x| x as u8).collect(),
        vec![b'a'; 65_535],
        (0..65_537).map(|x| (x % 251) as u8).collect(),
    ];
    for _ in 0..50 {
        let len = (rng.next_u64() as usize) % 20_000;
        let mut data = rng.bytes(len);
        if rng.next_u64() & 1 == 0 {
            for byte in &mut data {
                *byte &= 7;
            }
        }
        values.push(data);
    }
    values
}

#[test]
fn block_compressors_and_safe_decompressors_match_byte_for_byte() {
    unsafe {
        let libs = Libraries::load();
        let (c_bound, _) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let mut rng = Rng::new(0x243f_6a88_85a3_08d3);

        for input in corpus(&mut rng) {
            let bound = c_bound(input.len() as c_int);
            assert!(bound > 0);
            for name in [
                b"LZ4_compress_default\0".as_slice(),
                b"LZ4_compress_limitedOutput\0".as_slice(),
            ] {
                compare_compress4(&libs, name, &input, bound);
                if bound > 1 {
                    compare_compress4(&libs, name, &input, bound / 2);
                }
            }
            for acceleration in [-7, 0, 1, 2, 17, 65_537, 65_538, c_int::MAX] {
                compare_compress5(&libs, b"LZ4_compress_fast\0", &input, bound, acceleration);
            }
            for level in [-5, 0, 1, 2, 9, 10, 12, 13, c_int::MAX] {
                compare_compress5(&libs, b"LZ4_compress_HC\0", &input, bound, level);
                if bound > 1 {
                    compare_compress5(&libs, b"LZ4_compress_HC\0", &input, bound / 2, level);
                }
            }

            let (c_default, r_default) = libs.pair::<Compress4>(b"LZ4_compress_default\0");
            let mut cc = vec![0u8; bound as usize];
            let mut rc = vec![0u8; bound as usize];
            let cn = c_default(
                ptr_or_dangling(&input).cast(),
                cc.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            let rn = r_default(
                ptr_or_dangling(&input).cast(),
                rc.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            assert_eq!(cn, rn);
            assert_eq!(&cc[..cn as usize], &rc[..rn as usize]);

            let (c_decompress, r_decompress) = libs.pair::<Decompress4>(b"LZ4_decompress_safe\0");
            for capacity in [input.len(), input.len().saturating_add(17)] {
                let mut co = vec![0xa5u8; capacity.max(1)];
                let mut ro = vec![0xa5u8; capacity.max(1)];
                let cd = c_decompress(
                    cc.as_ptr().cast(),
                    co.as_mut_ptr().cast(),
                    cn,
                    capacity as c_int,
                );
                let rd = r_decompress(
                    rc.as_ptr().cast(),
                    ro.as_mut_ptr().cast(),
                    rn,
                    capacity as c_int,
                );
                assert_eq!(cd, rd);
                assert_eq!(&co[..cd.max(0) as usize], &ro[..rd.max(0) as usize]);
                if capacity >= input.len() {
                    assert_eq!(&co[..input.len()], input);
                }
            }

            let (c_partial, r_partial) = libs.pair::<unsafe extern "C" fn(
                *const c_char,
                *mut c_char,
                c_int,
                c_int,
                c_int,
            ) -> c_int>(
                b"LZ4_decompress_safe_partial\0"
            );
            for target in [0, 1, input.len() / 2, input.len(), input.len() + 1] {
                let cap = input.len() + 1;
                let mut co = vec![0u8; cap.max(1)];
                let mut ro = vec![0u8; cap.max(1)];
                let cd = c_partial(
                    cc.as_ptr().cast(),
                    co.as_mut_ptr().cast(),
                    cn,
                    target as c_int,
                    cap as c_int,
                );
                let rd = r_partial(
                    rc.as_ptr().cast(),
                    ro.as_mut_ptr().cast(),
                    rn,
                    target as c_int,
                    cap as c_int,
                );
                assert_eq!(cd, rd);
                assert_eq!(&co[..cd.max(0) as usize], &ro[..rd.max(0) as usize]);
            }
        }
    }
}

unsafe fn compare_compress4(libs: &Libraries, name: &[u8], input: &[u8], capacity: c_int) {
    let (c, r) = unsafe { libs.pair::<Compress4>(name) };
    let mut co = vec![0xa5u8; capacity.max(1) as usize];
    let mut ro = vec![0xa5u8; capacity.max(1) as usize];
    let cn = unsafe {
        c(
            ptr_or_dangling(input).cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
        )
    };
    let rn = unsafe {
        r(
            ptr_or_dangling(input).cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
        )
    };
    assert_eq!(cn, rn, "{name:?}");
    if cn > 0 {
        assert_eq!(&co[..cn as usize], &ro[..rn as usize], "{name:?}");
    }
}

unsafe fn compare_compress5(
    libs: &Libraries,
    name: &[u8],
    input: &[u8],
    capacity: c_int,
    option: c_int,
) {
    let (c, r) = unsafe { libs.pair::<Compress5>(name) };
    let mut co = vec![0xa5u8; capacity.max(1) as usize];
    let mut ro = vec![0xa5u8; capacity.max(1) as usize];
    let cn = unsafe {
        c(
            ptr_or_dangling(input).cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
            option,
        )
    };
    let rn = unsafe {
        r(
            ptr_or_dangling(input).cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
            option,
        )
    };
    assert_eq!(cn, rn, "{name:?}, option={option}");
    if cn > 0 {
        assert_eq!(&co[..cn as usize], &ro[..rn as usize], "{name:?}");
    }
}

#[test]
fn external_state_and_dest_size_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let (c_size, r_size) = libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofState\0");
        let (c_hc_size, r_hc_size) =
            libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofStateHC\0");
        assert_eq!(c_size(), r_size());
        assert_eq!(c_hc_size(), r_hc_size());
        let mut rng = Rng::new(0x1319_8a2e_0370_7344);

        for _ in 0..30 {
            let len = (rng.next_u64() as usize) % 12_000;
            let input = rng.bytes(len);
            let bound_fn = libs
                .c
                .get::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0")
                .unwrap();
            let bound = bound_fn(input.len() as c_int);
            let mut c_state = vec![0u64; (c_size() as usize + 7) / 8];
            let mut r_state = vec![0u64; (r_size() as usize + 7) / 8];
            let (c_ext, r_ext) = libs.pair::<CompressState6>(b"LZ4_compress_fast_extState\0");
            for acceleration in [0, 1, 7, 65_538] {
                let mut co = vec![0u8; bound as usize];
                let mut ro = vec![0u8; bound as usize];
                let cn = c_ext(
                    c_state.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    input.len() as c_int,
                    bound,
                    acceleration,
                );
                let rn = r_ext(
                    r_state.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    input.len() as c_int,
                    bound,
                    acceleration,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
            }

            let mut c_hc_state = vec![0u64; (c_hc_size() as usize + 7) / 8];
            let mut r_hc_state = vec![0u64; (r_hc_size() as usize + 7) / 8];
            let (c_hc, r_hc) = libs.pair::<CompressState6>(b"LZ4_compress_HC_extStateHC\0");
            for level in [2, 9, 10, 12, 99] {
                let mut co = vec![0u8; bound as usize];
                let mut ro = vec![0u8; bound as usize];
                let cn = c_hc(
                    c_hc_state.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    input.len() as c_int,
                    bound,
                    level,
                );
                let rn = r_hc(
                    r_hc_state.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    input.len() as c_int,
                    bound,
                    level,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
            }

            let (c_dest, r_dest) = libs.pair::<unsafe extern "C" fn(
                *const c_char,
                *mut c_char,
                *mut c_int,
                c_int,
            ) -> c_int>(b"LZ4_compress_destSize\0");
            for target in [1, 8, (bound / 3).max(1), bound] {
                let mut co = vec![0u8; target as usize];
                let mut ro = vec![0u8; target as usize];
                let mut cs = input.len() as c_int;
                let mut rs = input.len() as c_int;
                let cn = c_dest(
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    &mut cs,
                    target,
                );
                let rn = r_dest(
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    &mut rs,
                    target,
                );
                assert_eq!((cn, cs), (rn, rs));
                if cn > 0 {
                    assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
                }
            }
        }
    }
}

#[test]
fn malformed_and_boundary_block_errors_match_exactly() {
    unsafe {
        let libs = Libraries::load();
        let (c_compress, r_compress) = libs.pair::<Compress4>(b"LZ4_compress_default\0");
        let mut out_c = [0u8; 32];
        let mut out_r = [0u8; 32];
        let readable_input = [0u8; 128];
        for (size, cap) in [(-1, 32), (0, 0), (1, 0), (100, 1)] {
            assert_eq!(
                c_compress(
                    readable_input.as_ptr().cast(),
                    out_c.as_mut_ptr().cast(),
                    size,
                    cap,
                ),
                r_compress(
                    readable_input.as_ptr().cast(),
                    out_r.as_mut_ptr().cast(),
                    size,
                    cap,
                )
            );
        }

        let (c_decompress, r_decompress) = libs.pair::<Decompress4>(b"LZ4_decompress_safe\0");
        let mut rng = Rng::new(0xa409_3822_299f_31d0);
        for len in [0, 1, 2, 3, 4, 7, 16, 64, 255] {
            for _ in 0..20 {
                let malformed = rng.bytes(len);
                let mut co = [0u8; 512];
                let mut ro = [0u8; 512];
                let cd = c_decompress(
                    ptr_or_dangling(&malformed).cast(),
                    co.as_mut_ptr().cast(),
                    len as c_int,
                    co.len() as c_int,
                );
                let rd = r_decompress(
                    ptr_or_dangling(&malformed).cast(),
                    ro.as_mut_ptr().cast(),
                    len as c_int,
                    ro.len() as c_int,
                );
                assert_eq!(cd, rd);
                if cd >= 0 {
                    assert_eq!(&co[..cd as usize], &ro[..rd as usize]);
                }
            }
        }
    }
}
