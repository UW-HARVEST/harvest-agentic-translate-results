//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (from `nm -D` on the C `.so`):
//!   * `md5_digest`
//!
//! The header declares no namespace-renaming macros, so the source-level names
//! are also the final linker symbols.

#![allow(non_camel_case_types)]

/// `typedef uint8_t tflac_u8;`
pub type tflac_u8 = u8;

/// `typedef uint32_t tflac_u32;`
pub type tflac_u32 = u32;

/// `struct tflac_md5` — four 32-bit words, `#[repr(C)]` to match the C layout.
#[repr(C)]
pub struct tflac_md5 {
    pub a: tflac_u32,
    pub b: tflac_u32,
    pub c: tflac_u32,
    pub d: tflac_u32,
}

/// Serialize the four MD5 state words into `out` as 16 little-endian bytes.
///
/// C signature: `void md5_digest(const tflac_md5 *m, tflac_u8 out[16]);`
///
/// An array parameter in C decays to a pointer, so `out` is `*mut tflac_u8`.
/// The C code performs no NULL checks; this translation reproduces that
/// behavior exactly (dereferencing NULL is UB in both languages).
///
/// # Aliasing fidelity
///
/// The C body is sixteen *separate* statements, each of which re-reads the
/// source word and then stores one byte:
///
/// ```c
/// out[0] = (tflac_u8)(m->a);
/// out[1] = (tflac_u8)(m->a >> 8);
/// ...
/// ```
///
/// `out` has type `tflac_u8 *` (i.e. `unsigned char *`), which is exempt from
/// the strict-aliasing rule, so a C compiler *may not* cache `m->a` across the
/// stores — a store through `out` can legally modify `*m`. When the caller
/// aliases `out` onto (or partially over) `m`, each store therefore feeds back
/// into the next load, and the observable output is a byte-by-byte cascade
/// rather than a straight copy of the original words.
///
/// This is reproduced here by loading the word afresh immediately before each
/// store, using volatile accesses so the optimizer cannot re-cache the loads,
/// merge the stores into wider ones, or reorder them. Loads are done a byte at
/// a time (alignment 1) so that an under-aligned `m` behaves like the C too;
/// no store intervenes between the four byte loads of one iteration, so the
/// value observed is identical to C's single 32-bit load at that point.
/// `from_ne_bytes` keeps the reconstruction host-endian, matching the C, while
/// the shifts below reproduce C's value-level little-endian serialization.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const tflac_md5, out: *mut tflac_u8) {
    let words = m.cast::<tflac_u8>();
    let mut i = 0usize;
    while i < 16 {
        // Which struct word this output byte comes from, and which byte of it:
        //   out[4*w + 0] = (u8)(word);       out[4*w + 1] = (u8)(word >> 8);
        //   out[4*w + 2] = (u8)(word >> 16); out[4*w + 3] = (u8)(word >> 24);
        let field = unsafe { words.add(i & !3) };
        let word = tflac_u32::from_ne_bytes(unsafe {
            [
                field.read_volatile(),
                field.add(1).read_volatile(),
                field.add(2).read_volatile(),
                field.add(3).read_volatile(),
            ]
        });
        let byte = (word >> (8 * (i % 4))) as tflac_u8;
        unsafe { out.add(i).write_volatile(byte) };
        i += 1;
    }
}
