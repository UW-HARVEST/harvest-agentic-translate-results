//! Translation of `c_src/libsodium/sodium/runtime.c`.
//!
//! The reference build defines **no** `HAVE_*` macros (no `HAVE_CPUID`,
//! `HAVE_ANDROID_GETCPUFEATURES`, `HAVE_EMMINTRIN_H`, `HAVE_PMMINTRIN_H`,
//! `HAVE_TMMINTRIN_H`, `HAVE_SMMINTRIN_H`, `HAVE_AVXINTRIN_H`,
//! `HAVE_AVX2INTRIN_H`, `HAVE_AVX512FINTRIN_H`, `HAVE_WMMINTRIN_H`,
//! `HAVE_RDRAND`, `HAVE_SYS_AUXV_H`, `HAVE_GETAUXVAL`, ...), no `__ARM_ARCH`,
//! no `_MSC_VER` and no `__APPLE__`.  Preprocessing `runtime.c` therefore
//! collapses `_cpuid()` to "zero the four output words", collapses
//! `_sodium_runtime_arm_cpu_features()` to "clear the two ARM flags and return
//! -1", and collapses `_sodium_runtime_intel_cpu_features()` to "clear every
//! Intel flag" -- and because `_cpuid()` zeroes `cpu_info[0]`, the leaf-0 check
//! bails out with -1 before even reaching those assignments.
//!
//! Consequence (matching the C build byte for byte): every
//! `sodium_runtime_has_*()` returns 0, and `_sodium_runtime_get_cpu_features()`
//! returns -1 while setting `_cpu_features.initialized` to 1.
//!
//! Note: in C the `sodium_runtime_has_*` entry points are declared
//! `SODIUM_EXPORT_WEAK`, i.e. `__attribute__((weak))`, so they show up as `W`
//! in `nm`.  Stable Rust cannot emit weak function symbols (`#[linkage]` is
//! nightly-only), so they are emitted as ordinary global symbols with the exact
//! same names and behaviour.

use core::ffi::c_int;
use core::ptr::{addr_of, addr_of_mut};

/// `typedef struct CPUFeatures_ { ... } CPUFeatures;`
#[repr(C)]
struct CPUFeatures {
    initialized: c_int,
    has_neon: c_int,
    has_armcrypto: c_int,
    has_sse2: c_int,
    has_sse3: c_int,
    has_ssse3: c_int,
    has_sse41: c_int,
    has_avx: c_int,
    has_avx2: c_int,
    has_avx512f: c_int,
    has_pclmul: c_int,
    has_aesni: c_int,
    has_rdrand: c_int,
}

/// `static CPUFeatures _cpu_features;` -- zero-initialised (.bss) in C.
/// Private symbol: no `#[no_mangle]`.
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

const CPUID_EBX_AVX2: u32 = 0x00000020;
const CPUID_EBX_AVX512F: u32 = 0x00010000;

const CPUID_ECX_SSE3: u32 = 0x00000001;
const CPUID_ECX_PCLMUL: u32 = 0x00000002;
const CPUID_ECX_SSSE3: u32 = 0x00000200;
const CPUID_ECX_SSE41: u32 = 0x00080000;
const CPUID_ECX_AESNI: u32 = 0x02000000;
const CPUID_ECX_XSAVE: u32 = 0x04000000;
const CPUID_ECX_OSXSAVE: u32 = 0x08000000;
const CPUID_ECX_AVX: u32 = 0x10000000;
const CPUID_ECX_RDRAND: u32 = 0x40000000;

const CPUID_EDX_SSE2: u32 = 0x04000000;

const XCR0_SSE: u32 = 0x00000002;
const XCR0_AVX: u32 = 0x00000004;
const XCR0_OPMASK: u32 = 0x00000020;
const XCR0_ZMM_HI256: u32 = 0x00000040;
const XCR0_HI16_ZMM: u32 = 0x00000080;

/// `static int _sodium_runtime_arm_cpu_features(CPUFeatures * const)`
///
/// With `__ARM_ARCH` undefined the `#ifndef __ARM_ARCH` block survives, so the
/// function unconditionally returns -1 right after clearing the two flags.
/// Everything after the `return` in the C source is unreachable.
unsafe fn _sodium_runtime_arm_cpu_features(cpu_features: *mut CPUFeatures) -> c_int {
    (*cpu_features).has_neon = 0;
    (*cpu_features).has_armcrypto = 0;

    -1 /* LCOV_EXCL_LINE */
}

/// `static void _cpuid(unsigned int cpu_info[4U], const unsigned int cpu_info_type)`
///
/// Neither `_MSC_VER` nor `HAVE_CPUID` is defined, so the `#else` branch is
/// taken: the type is ignored and all four output words are zeroed.
unsafe fn _cpuid(cpu_info: *mut u32, cpu_info_type: u32) {
    let _ = cpu_info_type; /* (void) cpu_info_type; */
    *cpu_info.add(3) = 0;
    *cpu_info.add(2) = 0;
    *cpu_info.add(1) = 0;
    *cpu_info.add(0) = 0;
}

/// `static int _sodium_runtime_intel_cpu_features(CPUFeatures * const)`
unsafe fn _sodium_runtime_intel_cpu_features(cpu_features: *mut CPUFeatures) -> c_int {
    let mut cpu_info: [u32; 4] = [0u32; 4];
    let xcr0: u32 = 0;

    _cpuid(cpu_info.as_mut_ptr(), 0x0);
    if cpu_info[0] == 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _cpuid(cpu_info.as_mut_ptr(), 0x00000001);

    /* HAVE_EMMINTRIN_H undefined */
    (*cpu_features).has_sse2 = 0;
    /* HAVE_PMMINTRIN_H undefined */
    (*cpu_features).has_sse3 = 0;
    /* HAVE_TMMINTRIN_H undefined */
    (*cpu_features).has_ssse3 = 0;
    /* HAVE_SMMINTRIN_H undefined */
    (*cpu_features).has_sse41 = 0;

    (*cpu_features).has_avx = 0;

    let _ = xcr0; /* (void) xcr0; -- HAVE_AVXINTRIN_H undefined */

    (*cpu_features).has_avx2 = 0; /* HAVE_AVX2INTRIN_H undefined */
    (*cpu_features).has_avx512f = 0; /* HAVE_AVX512FINTRIN_H undefined */

    /* HAVE_WMMINTRIN_H undefined */
    (*cpu_features).has_pclmul = 0;
    (*cpu_features).has_aesni = 0;

    /* HAVE_RDRAND undefined */
    (*cpu_features).has_rdrand = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    let mut ret: c_int = -1;

    ret &= _sodium_runtime_arm_cpu_features(addr_of_mut!(_cpu_features));
    ret &= _sodium_runtime_intel_cpu_features(addr_of_mut!(_cpu_features));
    (*addr_of_mut!(_cpu_features)).initialized = 1;

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_neon() -> c_int {
    (*addr_of!(_cpu_features)).has_neon
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_armcrypto() -> c_int {
    (*addr_of!(_cpu_features)).has_armcrypto
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_sse2() -> c_int {
    (*addr_of!(_cpu_features)).has_sse2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_sse3() -> c_int {
    (*addr_of!(_cpu_features)).has_sse3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_ssse3() -> c_int {
    (*addr_of!(_cpu_features)).has_ssse3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_sse41() -> c_int {
    (*addr_of!(_cpu_features)).has_sse41
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_avx() -> c_int {
    (*addr_of!(_cpu_features)).has_avx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_avx2() -> c_int {
    (*addr_of!(_cpu_features)).has_avx2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_avx512f() -> c_int {
    (*addr_of!(_cpu_features)).has_avx512f
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_pclmul() -> c_int {
    (*addr_of!(_cpu_features)).has_pclmul
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_aesni() -> c_int {
    (*addr_of!(_cpu_features)).has_aesni
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_runtime_has_rdrand() -> c_int {
    (*addr_of!(_cpu_features)).has_rdrand
}
