//! Translation of `sodium/runtime.c`
//!
//! No `HAVE_*` feature macros are defined by the reference build, so every
//! feature probe selects the portable fallback: `_cpuid()` returns all zeros
//! and every `has_*` field stays 0.

use core::ffi::c_int;

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

static mut _CPU_FEATURES: CPUFeatures = CPUFeatures {
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

/// `_sodium_runtime_arm_cpu_features()`: sets neon/armcrypto to 0 and, because
/// `__ARM_ARCH` is not defined on the reference target, returns -1 immediately.
fn _sodium_runtime_arm_cpu_features(cpu_features: &mut CPUFeatures) -> c_int {
    cpu_features.has_neon = 0;
    cpu_features.has_armcrypto = 0;
    -1
}

/// `_cpuid()`: without `HAVE_CPUID` the function zeroes the output array.
fn _cpuid(cpu_info: &mut [u32; 4], _cpu_info_type: u32) {
    cpu_info[0] = 0;
    cpu_info[1] = 0;
    cpu_info[2] = 0;
    cpu_info[3] = 0;
}

/// `_sodium_runtime_intel_cpu_features()`. `_cpuid(cpu_info, 0)` yields
/// `cpu_info[0] == 0`, so the function bails out with -1 before touching any
/// feature flag.
fn _sodium_runtime_intel_cpu_features(_cpu_features: &mut CPUFeatures) -> c_int {
    let mut cpu_info: [u32; 4] = [0; 4];

    _cpuid(&mut cpu_info, 0x0);
    if cpu_info[0] == 0 {
        return -1;
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    let mut ret: c_int = -1;

    unsafe {
        let f = &mut *(&raw mut _CPU_FEATURES);
        ret &= _sodium_runtime_arm_cpu_features(f);
        ret &= _sodium_runtime_intel_cpu_features(f);
        f.initialized = 1;
    }

    ret
}

macro_rules! has {
    ($name:ident, $field:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name() -> c_int {
            unsafe { (*(&raw const _CPU_FEATURES)).$field }
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

// The public header annotates the `sodium_runtime_has_*` probes with
// `SODIUM_EXPORT_WEAK`, so the C library exports them with weak binding.
// rustc builds a cdylib with an explicit exported-symbol list, which forces
// STB_GLOBAL for every `#[no_mangle]` item and localises anything defined only
// in `global_asm!`. The symbol names and behaviour are identical; only the ELF
// binding differs (weak vs global), which has no effect on dynamic linking.
