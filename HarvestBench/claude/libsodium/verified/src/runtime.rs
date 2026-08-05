//! Translated from sodium/runtime.c
//! No HAVE_* SIMD macros defined, and not ARM -> all features are 0.
#![allow(dead_code)]

use core::ffi::c_int;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_runtime_get_cpu_features() -> c_int {
    // arm returns -1 (no __ARM_ARCH). intel: no HAVE_CPUID -> cpu_info all 0 -> returns -1.
    // ret = -1; ret &= (-1); ret &= (-1) => -1
    let ret: c_int = -1;
    CPU_FEATURES.has_neon = 0;
    CPU_FEATURES.has_armcrypto = 0;
    CPU_FEATURES.has_sse2 = 0;
    CPU_FEATURES.has_sse3 = 0;
    CPU_FEATURES.has_ssse3 = 0;
    CPU_FEATURES.has_sse41 = 0;
    CPU_FEATURES.has_avx = 0;
    CPU_FEATURES.has_avx2 = 0;
    CPU_FEATURES.has_avx512f = 0;
    CPU_FEATURES.has_pclmul = 0;
    CPU_FEATURES.has_aesni = 0;
    CPU_FEATURES.has_rdrand = 0;
    CPU_FEATURES.initialized = 1;
    ret
}

macro_rules! feature_getter {
    ($name:ident, $field:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() -> c_int {
            CPU_FEATURES.$field
        }
    };
}

feature_getter!(sodium_runtime_has_neon, has_neon);
feature_getter!(sodium_runtime_has_armcrypto, has_armcrypto);
feature_getter!(sodium_runtime_has_sse2, has_sse2);
feature_getter!(sodium_runtime_has_sse3, has_sse3);
feature_getter!(sodium_runtime_has_ssse3, has_ssse3);
feature_getter!(sodium_runtime_has_sse41, has_sse41);
feature_getter!(sodium_runtime_has_avx, has_avx);
feature_getter!(sodium_runtime_has_avx2, has_avx2);
feature_getter!(sodium_runtime_has_avx512f, has_avx512f);
feature_getter!(sodium_runtime_has_pclmul, has_pclmul);
feature_getter!(sodium_runtime_has_aesni, has_aesni);
feature_getter!(sodium_runtime_has_rdrand, has_rdrand);

// NOTE: In C these are declared SODIUM_EXPORT_WEAK (weak ELF binding). Rust's
// #[no_mangle] forces global binding, and combining it with a `.weak` asm
// directive is rejected by the assembler ("changed binding to STB_GLOBAL").
// The exported symbol NAMES and runtime behavior are identical; only the ELF
// binding attribute (W vs T) differs, which does not affect the ABI surface.
