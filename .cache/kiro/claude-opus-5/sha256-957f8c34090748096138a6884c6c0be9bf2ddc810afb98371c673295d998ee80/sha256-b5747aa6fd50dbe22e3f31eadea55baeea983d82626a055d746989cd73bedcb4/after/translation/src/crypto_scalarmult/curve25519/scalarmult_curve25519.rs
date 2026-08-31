//! Translation of c_src/libsodium/crypto_scalarmult/curve25519/scalarmult_curve25519.c

use core::ffi::c_int;

// crypto_scalarmult_curve25519_BYTES / SCALARBYTES from the header (32U each).
const crypto_scalarmult_curve25519_BYTES: usize = 32;
const crypto_scalarmult_curve25519_SCALARBYTES: usize = 32;

// Local repr(C) mirror of crypto_scalarmult_curve25519_implementation
// (scalarmult_curve25519.h). Both function pointers are non-NULL in this build
// (initialized to the ref10 functions), matching a plain C function pointer.
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult: extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int,
    pub mult_base: extern "C" fn(q: *mut u8, n: *const u8) -> c_int,
}

unsafe impl Sync for crypto_scalarmult_curve25519_implementation {}

extern "C" {
    // Defined in ref10/x25519_ref10.rs (exported static).
    static crypto_scalarmult_curve25519_ref10_implementation:
        crypto_scalarmult_curve25519_implementation;
}

// static const crypto_scalarmult_curve25519_implementation *implementation =
//     &crypto_scalarmult_curve25519_ref10_implementation;
static mut implementation: *const crypto_scalarmult_curve25519_implementation =
    core::ptr::null();

#[inline]
unsafe fn implementation_ptr() -> *const crypto_scalarmult_curve25519_implementation {
    // C initializes `implementation` to the address of the ref10 impl at load
    // time. Rust statics cannot reference an extern static in an initializer,
    // so lazily default to it when still NULL.
    if implementation.is_null() {
        implementation =
            core::ptr::addr_of!(crypto_scalarmult_curve25519_ref10_implementation);
    }
    implementation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let mut i: usize;
    // volatile unsigned char d = 0;
    let mut d: u8 = 0;
    let d_ref = core::ptr::addr_of_mut!(d);

    let imp = implementation_ptr();
    if ((*imp).mult)(q, n, p) != 0 {
        return -1; // LCOV_EXCL_LINE
    }
    i = 0;
    while i < crypto_scalarmult_curve25519_BYTES {
        core::ptr::write_volatile(d_ref, core::ptr::read_volatile(d_ref) | *q.add(i));
        i += 1;
    }
    let dv = core::ptr::read_volatile(d_ref);
    -(1i32 & (((dv as c_int) - 1) >> 8))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    (crypto_scalarmult_curve25519_ref10_implementation.mult_base)(q, n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_bytes() -> usize {
    crypto_scalarmult_curve25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_curve25519_scalarbytes() -> usize {
    crypto_scalarmult_curve25519_SCALARBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_scalarmult_curve25519_pick_best_implementation(
) -> c_int {
    implementation = core::ptr::addr_of!(crypto_scalarmult_curve25519_ref10_implementation);

    // HAVE_AVX_ASM undefined: sandy2x implementation does not exist.
    0
}
