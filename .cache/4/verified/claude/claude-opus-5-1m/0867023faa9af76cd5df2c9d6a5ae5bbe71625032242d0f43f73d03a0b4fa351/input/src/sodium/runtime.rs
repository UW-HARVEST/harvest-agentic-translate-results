//! `sodium/runtime.c`
//!
//! The reference build defines no `HAVE_*` feature macros, so:
//!  * `_sodium_runtime_arm_cpu_features()` zeroes neon/armcrypto then returns -1
//!    (`#ifndef __ARM_ARCH` branch).
//!  * `_cpuid()` falls through to the `#else` that zeroes `cpu_info`, so
//!    `_sodium_runtime_intel_cpu_features()` returns -1 right after the
//!    `cpu_info[0] == 0` test, leaving every feature flag at its static zero.
//!
//! Hence every `sodium_runtime_has_*()` returns 0 and
//! `_sodium_runtime_get_cpu_features()` returns -1.

use core::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CPUFeatures {
    pub initialized: c_int,
    pub has_neon: c_int,
    pub has_armcrypto: c_int,
    pub has_sse2: c_int,
    pub has_sse3: c_int,
    pub has_ssse3: c_int,
    pub has_sse41: c_int,
    pub has_avx: c_int,
    pub has_avx2: c_int,
    pub has_avx512f: c_int,
    pub has_pclmul: c_int,
    pub has_aesni: c_int,
    pub has_rdrand: c_int,
}

static mut CPU_FEATURES: CPUFeatures = CPUFeatures {
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

#[inline]
fn cpu() -> &'static mut CPUFeatures {
    unsafe { &mut *(&raw mut CPU_FEATURES) }
}

/// `_sodium_runtime_arm_cpu_features()`
fn arm_cpu_features(f: &mut CPUFeatures) -> c_int {
    f.has_neon = 0;
    f.has_armcrypto = 0;
    // #ifndef __ARM_ARCH -> return -1
    -1
}

/// `_sodium_runtime_intel_cpu_features()`
fn intel_cpu_features(_f: &mut CPUFeatures) -> c_int {
    // _cpuid() with neither _MSC_VER nor HAVE_CPUID zeroes cpu_info, so
    // cpu_info[0] == 0 and we bail out immediately.
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    let mut ret: c_int = -1;
    let f = cpu();
    ret &= arm_cpu_features(f);
    ret &= intel_cpu_features(f);
    f.initialized = 1;
    ret
}

macro_rules! has {
    ($name:ident, $field:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> c_int {
            cpu().$field
        }
    };
}

has!(sodium_runtime_has_neon, has_neon);
has!(sodium_runtime_has_armcrypto, has_armcrypto);
has!(sodium_runtime_has_sse2, has_sse2);
has!(sodium_runtime_has_sse3, has_sse3);
has!(sodium_runtime_has_ssse3, has_ssse3);
has!(sodium_runtime_has_sse41, has_sse41);
has!(sodium_runtime_has_avx, has_avx);
has!(sodium_runtime_has_avx2, has_avx2);
has!(sodium_runtime_has_avx512f, has_avx512f);
has!(sodium_runtime_has_pclmul, has_pclmul);
has!(sodium_runtime_has_aesni, has_aesni);
has!(sodium_runtime_has_rdrand, has_rdrand);
