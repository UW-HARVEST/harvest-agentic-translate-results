use std::ptr;

#[repr(C)]
pub struct TflacMd5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[inline(always)]
unsafe fn write_word(out: *mut u8, word: *const u32) {
    unsafe {
        out.write(word.read() as u8);
        out.add(1).write((word.read() >> 8) as u8);
        out.add(2).write((word.read() >> 16) as u8);
        out.add(3).write((word.read() >> 24) as u8);
    }
}

/// Writes the MD5 state words to `out` in the byte order used by the C library.
///
/// # Safety
///
/// `m` must point to a valid `TflacMd5`, and `out` must be writable for 16
/// bytes. These requirements match the original C function's pointer contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const TflacMd5, out: *mut u8) {
    unsafe {
        write_word(out, ptr::addr_of!((*m).a));
        write_word(out.add(4), ptr::addr_of!((*m).b));
        write_word(out.add(8), ptr::addr_of!((*m).c));
        write_word(out.add(12), ptr::addr_of!((*m).d));
    }
}
