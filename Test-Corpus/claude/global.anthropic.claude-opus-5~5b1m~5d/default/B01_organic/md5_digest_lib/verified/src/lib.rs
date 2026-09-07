//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C shared object):
//!   * `md5_digest`
//!
//! Header (`include/lib.h`) declares:
//! ```c
//! typedef uint8_t  tflac_u8;
//! typedef uint32_t tflac_u32;
//!
//! struct tflac_md5 {
//!     tflac_u32 a;
//!     tflac_u32 b;
//!     tflac_u32 c;
//!     tflac_u32 d;
//! };
//! typedef struct tflac_md5 tflac_md5;
//!
//! void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);
//! ```
//!
//! There are no namespace-renaming preprocessor macros in the header, so the
//! linker symbol name is exactly `md5_digest`.

#![allow(non_camel_case_types)]

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac_md5` — layout-compatible with the C struct (four
/// naturally-aligned 32-bit words, no padding).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// Serialize the four MD5 state words little-endian into `out[0..16]`.
///
/// Faithful translation of:
///
/// ```c
/// void md5_digest(const tflac_md5 *m, tflac_u8 out[16]) {
///     out[0] = (tflac_u8)(m->a);
///     out[1] = (tflac_u8)(m->a >> 8);
///     ...
///     out[15] = (tflac_u8)(m->d >> 24);
/// }
/// ```
///
/// Exactly like the C original, no null / alignment validation is performed on
/// either argument; passing invalid pointers is undefined behaviour in both
/// versions.
///
/// ## Aliasing fidelity (why this is not a "read struct once, then write" loop)
///
/// Neither parameter is `restrict`, and a store through a character-type lvalue
/// may alias an object of any type, so `out` overlapping `*m` is a *well
/// defined* call in C. The compiled C therefore re-loads the relevant 32-bit
/// field from memory immediately before **every one of the 16 byte stores**
/// (visible in the disassembly as `mov -0x8(%rbp),%rax; mov (%rax),%eax`
/// repeated 16 times). With overlapping buffers this produces a byte cascade —
/// each store can change the value the next load observes.
///
/// To stay byte-identical, this translation performs the same interleaving:
/// one raw (possibly unaligned) load of the field, then one raw byte store,
/// sixteen times. Only raw pointers are used — no references and no
/// intermediate struct copy — so nothing here asserts `noalias` and the
/// load/store order is preserved.
///
/// # Safety
///
/// `m` must point to at least 16 readable bytes and `out` to at least 16
/// writable bytes. The two ranges MAY overlap (see above).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    // `$field` is re-read from memory for each byte, mirroring the C exactly.
    //
    // The load is assembled from four plain byte reads and `from_ne_bytes`
    // (native byte order == what the C's `m->a` lvalue load does) rather than
    // from `ptr::read`/`read_unaligned`, for two reasons:
    //   * it tolerates a misaligned `m`, which the C does not check for either;
    //   * a plain `*p` deref and `ptr::read_unaligned` both carry rustc's
    //     debug-only null/alignment UB check, which turns a null `m` into a
    //     `SIGABRT`, whereas the C faults with `SIGSEGV`. `read_volatile` /
    //     `write_volatile` on `u8` (alignment 1, so never misaligned) carry no
    //     such check and additionally forbid the compiler from caching or
    //     reordering the accesses, so the fault *and* the aliasing cascade are
    //     identical to the C's in BOTH the debug and release profiles.
    // All four bytes of a field are read before the single byte store, exactly
    // like the C's 32-bit load followed by one 8-bit store, so the overlapping
    // buffer cascade is reproduced byte for byte.
    macro_rules! store_byte {
        ($field:ident, $idx:expr, $shift:expr) => {{
            let p: *const u8 = core::ptr::addr_of!((*m).$field) as *const u8;
            let v: tflac_u32 = tflac_u32::from_ne_bytes([
                core::ptr::read_volatile(p),
                core::ptr::read_volatile(p.wrapping_add(1)),
                core::ptr::read_volatile(p.wrapping_add(2)),
                core::ptr::read_volatile(p.wrapping_add(3)),
            ]);
            core::ptr::write_volatile(out.wrapping_add($idx), (v >> $shift) as tflac_u8);
        }};
    }

    unsafe {
        store_byte!(a, 0, 0);
        store_byte!(a, 1, 8);
        store_byte!(a, 2, 16);
        store_byte!(a, 3, 24);
        store_byte!(b, 4, 0);
        store_byte!(b, 5, 8);
        store_byte!(b, 6, 16);
        store_byte!(b, 7, 24);
        store_byte!(c, 8, 0);
        store_byte!(c, 9, 8);
        store_byte!(c, 10, 16);
        store_byte!(c, 11, 24);
        store_byte!(d, 12, 0);
        store_byte!(d, 13, 8);
        store_byte!(d, 14, 16);
        store_byte!(d, 15, 24);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_c() {
        assert_eq!(core::mem::size_of::<tflac_md5>(), 16);
        assert_eq!(core::mem::align_of::<tflac_md5>(), 4);
    }

    #[test]
    fn little_endian_serialization() {
        let m = tflac_md5 {
            a: 0x04030201,
            b: 0x08070605,
            c: 0x0c0b0a09,
            d: 0x100f0e0d,
        };
        let mut out = [0u8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn truncation_and_extremes() {
        let m = tflac_md5 {
            a: 0xFFFF_FFFF,
            b: 0x0000_0000,
            c: 0xDEAD_BEEF,
            d: 0x0000_00FF,
        };
        let mut out = [0xAAu8; 16];
        unsafe { md5_digest(&m, out.as_mut_ptr()) };
        assert_eq!(
            out,
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xEF, 0xBE, 0xAD, 0xDE, 0xFF,
                0x00, 0x00, 0x00,
            ]
        );
    }
}
