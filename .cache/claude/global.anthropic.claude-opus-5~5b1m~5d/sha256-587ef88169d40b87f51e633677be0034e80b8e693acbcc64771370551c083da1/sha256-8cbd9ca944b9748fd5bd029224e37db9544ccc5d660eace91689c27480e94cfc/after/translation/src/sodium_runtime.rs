//! Translation of `c_src/libsodium/sodium/runtime.c`.
//!
//! The reference build defines no `HAVE_CPUID`, `HAVE_*INTRIN_H`,
//! `HAVE_ANDROID_GETCPUFEATURES`, `__ARM_NEON`, etc., and targets
//! `__x86_64__`/`__linux__` without `__ARM_ARCH`. Per `$W/tools/cpp.sh`:
//! * `_sodium_runtime_arm_cpu_features` sets `has_neon`/`has_armcrypto` to 0
//!   and returns -1 immediately (no `__ARM_ARCH`); the remainder of the C
//!   function body is unreachable.
//! * `_cpuid` is a no-op that zeroes `cpu_info[0..4]` (no `HAVE_CPUID`).
//! * `_sodium_runtime_intel_cpu_features` therefore always observes
//!   `cpu_info[0] == 0` after the first `_cpuid` call and returns -1 before
//!   touching any `has_*` field; all of those fields keep their
//!   zero-initialized values.
//! * Consequently every `sodium_runtime_has_*` function always returns 0.

use core::ffi::c_int;

#[repr(C)]
struct CPUFeatures {
    initialized: i32,
    has_neon: i32,
    has_armcrypto: i32,
    has_sse2: i32,
    has_sse3: i32,
    has_ssse3: i32,
    has_sse41: i32,
    has_avx: i32,
    has_avx2: i32,
    has_avx512f: i32,
    has_pclmul: i32,
    has_aesni: i32,
    has_rdrand: i32,
}

static mut _cpu_features: CPUFeatures = CPUFeatures {
    initialized: 0,
    has_neon: 0,
    has_armcrypto: 0,
    has_sse2: 0,
    has_sse3: 0,
    has_ssse3: 0,
    has_sse41: 0,
    has_avx: 0,
    has_avx2: 0,
    has_avx512f: 0,
    has_pclmul: 0,
    has_aesni: 0,
    has_rdrand: 0,
};

unsafe fn _sodium_runtime_arm_cpu_features(cpu_features: *mut CPUFeatures) -> c_int {
    (*cpu_features).has_neon = 0;
    (*cpu_features).has_armcrypto = 0;

    /* `__ARM_ARCH` is not defined for this build. */
    return -1; /* LCOV_EXCL_LINE */

    // The remainder of the C function body is unreachable in this
    // configuration (no `__ARM_ARCH`), so it is intentionally not
    // reproduced here.
}

unsafe fn _cpuid(cpu_info: *mut u32, cpu_info_type: u32) {
    let _ = cpu_info_type;
    *cpu_info.add(0) = 0;
    *cpu_info.add(1) = 0;
    *cpu_info.add(2) = 0;
    *cpu_info.add(3) = 0;
}

unsafe fn _sodium_runtime_intel_cpu_features(cpu_features: *mut CPUFeatures) -> c_int {
    let mut cpu_info: [u32; 4] = [0; 4];
    let mut xcr0: u32 = 0;

    _cpuid(cpu_info.as_mut_ptr(), 0x0);
    if cpu_info[0] == 0 {
        return -1;
    }
    _cpuid(cpu_info.as_mut_ptr(), 0x00000001);

    (*cpu_features).has_sse2 = 0;
    (*cpu_features).has_sse3 = 0;
    (*cpu_features).has_ssse3 = 0;
    (*cpu_features).has_sse41 = 0;

    (*cpu_features).has_avx = 0;

    let _ = xcr0;

    (*cpu_features).has_avx2 = 0;
    (*cpu_features).has_avx512f = 0;

    (*cpu_features).has_pclmul = 0;
    (*cpu_features).has_aesni = 0;

    (*cpu_features).has_rdrand = 0;

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    let mut ret: c_int = -1;

    ret &= _sodium_runtime_arm_cpu_features(core::ptr::addr_of_mut!(_cpu_features));
    ret &= _sodium_runtime_intel_cpu_features(core::ptr::addr_of_mut!(_cpu_features));
    _cpu_features.initialized = 1;

    ret
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_neon() -> c_int {
    _cpu_features.has_neon
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_armcrypto() -> c_int {
    _cpu_features.has_armcrypto
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_sse2() -> c_int {
    _cpu_features.has_sse2
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_sse3() -> c_int {
    _cpu_features.has_sse3
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_ssse3() -> c_int {
    _cpu_features.has_ssse3
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_sse41() -> c_int {
    _cpu_features.has_sse41
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_avx() -> c_int {
    _cpu_features.has_avx
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_avx2() -> c_int {
    _cpu_features.has_avx2
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_avx512f() -> c_int {
    _cpu_features.has_avx512f
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_pclmul() -> c_int {
    _cpu_features.has_pclmul
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_aesni() -> c_int {
    _cpu_features.has_aesni
}

#[no_mangle]
pub unsafe extern "C" fn sodium_runtime_has_rdrand() -> c_int {
    _cpu_features.has_rdrand
}
