//! Translation of `randombytes/randombytes.c`.
//!
//! Reference build: Linux/glibc x86-64, no `config.h`, no
//! `RANDOMBYTES_CUSTOM_IMPLEMENTATION`, no `RANDOMBYTES_DEFAULT_IMPLEMENTATION`,
//! no `__EMSCRIPTEN__`.  The preprocessor therefore keeps exactly the
//! `&randombytes_sysrandom_implementation` default and drops all of the
//! JavaScript backend.

//! The reference build compiles with `-O3 -DNDEBUG -std=gnu99`, so every
//! `assert()` collapses to `((void) (0))` and is reproduced here as a comment.

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------ */
/*  include/sodium/randombytes.h                                      */
/* ------------------------------------------------------------------ */

/// `typedef struct randombytes_implementation { ... } randombytes_implementation;`
///
/// Six function pointers, 0x30 bytes on LP64.  `Option<extern "C" fn(..)>` has
/// the same representation as the corresponding C function pointer thanks to
/// the null-pointer optimisation, so `None` == `NULL`.
#[repr(C)]
pub struct randombytes_implementation {
    /* required */
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    /* required */
    pub random: Option<extern "C" fn() -> u32>,
    /* optional */
    pub stir: Option<extern "C" fn()>,
    /* optional */
    pub uniform: Option<extern "C" fn(upper_bound: u32) -> u32>,
    /* required */
    pub buf: Option<extern "C" fn(buf: *mut c_void, size: usize)>,
    /* optional */
    pub close: Option<extern "C" fn() -> c_int>,
}

/// `#define randombytes_SEEDBYTES 32U`
const randombytes_SEEDBYTES: usize = 32;

/// `#define crypto_stream_chacha20_ietf_NONCEBYTES 12U`
const crypto_stream_chacha20_ietf_NONCEBYTES: usize = 12;

/* ------------------------------------------------------------------ */
/*  externals                                                         */
/* ------------------------------------------------------------------ */

extern "C" {
    /// Exported data symbol defined by `randombytes/sysrandom/randombytes_sysrandom.c`.
    static randombytes_sysrandom_implementation: randombytes_implementation;

    fn crypto_stream_chacha20_ietf(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    fn sodium_misuse() -> !;
}

/* ------------------------------------------------------------------ */
/*  static const randombytes_implementation *implementation;          */
/* ------------------------------------------------------------------ */

static mut implementation: *const randombytes_implementation = core::ptr::null();

/// `static void randombytes_init_if_needed(void)`
unsafe fn randombytes_init_if_needed() {
    if implementation.is_null() {
        /* RANDOMBYTES_DEFAULT_IMPLEMENTATION == &randombytes_sysrandom_implementation */
        implementation = core::ptr::addr_of!(randombytes_sysrandom_implementation);
        randombytes_stir();
    }
}

/* ------------------------------------------------------------------ */
/*  public API                                                        */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_set_implementation(
    impl_: *const randombytes_implementation,
) -> c_int {
    implementation = impl_;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    ((*implementation).implementation_name.unwrap_unchecked())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    ((*implementation).random.unwrap_unchecked())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    if let Some(stir) = (*implementation).stir {
        stir();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    let min: u32;
    let mut r: u32;

    randombytes_init_if_needed();
    if let Some(uniform) = (*implementation).uniform {
        return uniform(upper_bound);
    }
    if upper_bound < 2 {
        return 0;
    }
    /* min = (1U + ~upper_bound) % upper_bound;  == 2**32 mod upper_bound */
    min = (!upper_bound).wrapping_add(1) % upper_bound;
    loop {
        r = randombytes_random();
        if !(r < min) {
            break;
        }
    }
    /* r is now clamped to a set whose size mod upper_bound == 0
     * the worst case (2**31+1) requires ~ 2 attempts */

    r % upper_bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf(buf: *mut c_void, size: usize) {
    randombytes_init_if_needed();
    if size > 0 {
        ((*implementation).buf.unwrap_unchecked())(buf, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8, /* [randombytes_SEEDBYTES] */
) {
    static nonce: [u8; crypto_stream_chacha20_ietf_NONCEBYTES] = [
        b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G',
    ];

    /* COMPILER_ASSERT(randombytes_SEEDBYTES == crypto_stream_chacha20_ietf_KEYBYTES) */
    const _: () = assert!(randombytes_SEEDBYTES == 32);
    /* COMPILER_ASSERT(randombytes_BYTES_MAX <= 0x4000000000ULL) */
    if size as u64 > 0x4000000000u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_stream_chacha20_ietf(
        buf as *mut u8,
        size as c_ulonglong,
        nonce.as_ptr(),
        seed,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_seedbytes() -> usize {
    randombytes_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_close() -> c_int {
    if !implementation.is_null() {
        if let Some(close) = (*implementation).close {
            return close();
        }
    }
    0
}

/* -- NaCl compatibility interface -- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(buf: *mut u8, buf_len: c_ulonglong) {
    /* assert(buf_len <= SIZE_MAX);  ->  ((void) (0))  under NDEBUG */
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}
