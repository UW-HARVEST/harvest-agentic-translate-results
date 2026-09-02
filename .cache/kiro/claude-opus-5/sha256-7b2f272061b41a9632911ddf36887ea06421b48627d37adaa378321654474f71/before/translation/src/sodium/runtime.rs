//! Translation of `libsodium/sodium/runtime.c`
//!
//! The reference build defines no `HAVE_*` feature macros, so every
//! capability probe compiles to the "unavailable" branch and all
//! `sodium_runtime_has_*()` accessors report 0.

use core::ffi::c_int;

#[derive(Copy, Clone)]
struct CpuFeatures {
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

static mut CPU_FEATURES: CpuFeatures = CpuFeatures {
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

/// `_sodium_runtime_arm_cpu_features()`: `__ARM_ARCH` is undefined on the
/// reference (x86_64) build, so the function zeroes both fields and returns -1.
fn runtime_arm_cpu_features(f: &mut CpuFeatures) -> c_int {
    f.has_neon = 0;
    f.has_armcrypto = 0;
    -1
}

/// `_sodium_runtime_intel_cpu_features()`: without any `HAVE_*INTRIN_H` macro
/// every field is set to 0. `_cpuid()` is compiled to the stub that zeroes
/// `cpu_info`, so `cpu_info[0] == 0` and the function returns -1 early.
fn runtime_intel_cpu_features(_f: &mut CpuFeatures) -> c_int {
    // _cpuid(cpu_info, 0x0) -> all zeroes (no HAVE_CPUID, not MSVC)
    // if (cpu_info[0] == 0U) return -1;
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    let mut ret: c_int = -1;
    unsafe {
        let f = &mut *core::ptr::addr_of_mut!(CPU_FEATURES);
        ret &= runtime_arm_cpu_features(f);
        ret &= runtime_intel_cpu_features(f);
        f.initialized = 1;
    }
    ret
}

macro_rules! has {
    ($name:ident, $field:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> c_int {
            unsafe { (*core::ptr::addr_of!(CPU_FEATURES)).$field }
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
