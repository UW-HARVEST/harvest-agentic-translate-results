mod common;

use common::{Libraries, Rng, ptr_or_dangling};
use std::ffi::{c_int, c_uint, c_ulonglong, c_void};

#[test]
fn xxhash_one_shot_streaming_copy_and_canonical_match() {
    unsafe {
        let libs = Libraries::load();
        let (c32, r32) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint>(b"LZ4_XXH32\0");
        let (c64, r64) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize, c_ulonglong) -> c_ulonglong>(
                b"LZ4_XXH64\0",
            );

        let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15);
        let mut lengths = vec![0, 1, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 255, 256, 1024];
        lengths.extend((0..80).map(|_| (rng.next_u64() as usize) % 4097));
        for len in lengths {
            let mut storage = rng.bytes(len + 1);
            for offset in [0usize, 1] {
                let input = &storage[offset..offset + len];
                for seed in [0, 1, rng.next_u32(), u32::MAX] {
                    assert_eq!(
                        c32(ptr_or_dangling(input), len, seed),
                        r32(ptr_or_dangling(input), len, seed)
                    );
                }
                for seed in [0, 1, rng.next_u64(), u64::MAX] {
                    assert_eq!(
                        c64(ptr_or_dangling(input), len, seed),
                        r64(ptr_or_dangling(input), len, seed)
                    );
                }
            }
            storage.fill(0);
        }

        exercise_stream32(&libs, &mut rng);
        exercise_stream64(&libs, &mut rng);
    }
}

unsafe fn exercise_stream32(libs: &Libraries, rng: &mut Rng) {
    let (c_create, r_create) =
        unsafe { libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_XXH32_createState\0") };
    let (c_free, r_free) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_XXH32_freeState\0")
    };
    let (c_reset, r_reset) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, c_uint) -> c_int>(b"LZ4_XXH32_reset\0")
    };
    let (c_update, r_update) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int>(
            b"LZ4_XXH32_update\0",
        )
    };
    let (c_digest, r_digest) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_void) -> c_uint>(b"LZ4_XXH32_digest\0")
    };
    let (c_copy, r_copy) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void)>(b"LZ4_XXH32_copyState\0")
    };
    let (c_canonical, r_canonical) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, c_uint)>(b"LZ4_XXH32_canonicalFromHash\0")
    };
    let (c_from, r_from) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_void) -> c_uint>(b"LZ4_XXH32_hashFromCanonical\0")
    };

    let cs = unsafe { c_create() };
    let rs = unsafe { r_create() };
    let cc = unsafe { c_create() };
    let rc = unsafe { r_create() };
    assert_eq!(cs.is_null(), rs.is_null());
    assert_eq!(cc.is_null(), rc.is_null());
    assert!(!cs.is_null() && !cc.is_null());

    for _ in 0..40 {
        let seed = rng.next_u32();
        assert_eq!(unsafe { c_reset(cs, seed) }, unsafe { r_reset(rs, seed) });
        let len = (rng.next_u64() as usize) % 4097;
        let data = rng.bytes(len);
        let mut at = 0;
        while at < data.len() {
            let chunk = 1 + (rng.next_u64() as usize % 73).min(data.len() - at - 1);
            let part = &data[at..at + chunk];
            assert_eq!(
                unsafe { c_update(cs, ptr_or_dangling(part), part.len()) },
                unsafe { r_update(rs, ptr_or_dangling(part), part.len()) }
            );
            assert_eq!(unsafe { c_digest(cs) }, unsafe { r_digest(rs) });
            at += chunk;
        }
        assert_eq!(unsafe { c_update(cs, std::ptr::null(), 0) }, unsafe {
            r_update(rs, std::ptr::null(), 0)
        });
        unsafe {
            c_copy(cc, cs);
            r_copy(rc, rs);
        }
        let ch = unsafe { c_digest(cc) };
        let rh = unsafe { r_digest(rc) };
        assert_eq!(ch, rh);
        let mut cb = [0u8; 4];
        let mut rb = [0u8; 4];
        unsafe {
            c_canonical(cb.as_mut_ptr().cast(), ch);
            r_canonical(rb.as_mut_ptr().cast(), rh);
        }
        assert_eq!(cb, rb);
        assert_eq!(unsafe { c_from(cb.as_ptr().cast()) }, unsafe {
            r_from(rb.as_ptr().cast())
        });
    }
    assert_eq!(unsafe { c_free(cs) }, unsafe { r_free(rs) });
    assert_eq!(unsafe { c_free(cc) }, unsafe { r_free(rc) });
    assert_eq!(unsafe { c_free(std::ptr::null_mut()) }, unsafe {
        r_free(std::ptr::null_mut())
    });
}

unsafe fn exercise_stream64(libs: &Libraries, rng: &mut Rng) {
    let (c_create, r_create) =
        unsafe { libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_XXH64_createState\0") };
    let (c_free, r_free) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_XXH64_freeState\0")
    };
    let (c_reset, r_reset) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, c_ulonglong) -> c_int>(b"LZ4_XXH64_reset\0")
    };
    let (c_update, r_update) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int>(
            b"LZ4_XXH64_update\0",
        )
    };
    let (c_digest, r_digest) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_void) -> c_ulonglong>(b"LZ4_XXH64_digest\0")
    };
    let (c_copy, r_copy) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void)>(b"LZ4_XXH64_copyState\0")
    };
    let (c_canonical, r_canonical) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void, c_ulonglong)>(
            b"LZ4_XXH64_canonicalFromHash\0",
        )
    };
    let (c_from, r_from) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_void) -> c_ulonglong>(
            b"LZ4_XXH64_hashFromCanonical\0",
        )
    };

    let cs = unsafe { c_create() };
    let rs = unsafe { r_create() };
    let cc = unsafe { c_create() };
    let rc = unsafe { r_create() };
    assert!(!cs.is_null() && !rs.is_null() && !cc.is_null() && !rc.is_null());

    for _ in 0..40 {
        let seed = rng.next_u64();
        assert_eq!(unsafe { c_reset(cs, seed) }, unsafe { r_reset(rs, seed) });
        let len = (rng.next_u64() as usize) % 4097;
        let data = rng.bytes(len);
        for part in data.chunks(1 + (rng.next_u64() as usize % 97)) {
            assert_eq!(
                unsafe { c_update(cs, ptr_or_dangling(part), part.len()) },
                unsafe { r_update(rs, ptr_or_dangling(part), part.len()) }
            );
        }
        unsafe {
            c_copy(cc, cs);
            r_copy(rc, rs);
        }
        let ch = unsafe { c_digest(cc) };
        let rh = unsafe { r_digest(rc) };
        assert_eq!(ch, rh);
        let mut cb = [0u8; 8];
        let mut rb = [0u8; 8];
        unsafe {
            c_canonical(cb.as_mut_ptr().cast(), ch);
            r_canonical(rb.as_mut_ptr().cast(), rh);
        }
        assert_eq!(cb, rb);
        assert_eq!(unsafe { c_from(cb.as_ptr().cast()) }, unsafe {
            r_from(rb.as_ptr().cast())
        });
    }
    assert_eq!(unsafe { c_free(cs) }, unsafe { r_free(rs) });
    assert_eq!(unsafe { c_free(cc) }, unsafe { r_free(rc) });
}
