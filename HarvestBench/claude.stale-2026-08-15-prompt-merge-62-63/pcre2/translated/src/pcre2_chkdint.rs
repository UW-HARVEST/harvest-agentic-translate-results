use crate::pcre2_internal::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: i32, b: i32) -> BOOL {
    // Uses 64-bit multiply and checks overflow against PCRE2_SIZE_MAX, as C does
    // in the non-builtin path. a and b are asserted >= 0 in C.
    let m: i64 = (a as i64) * (b as i64);
    // sizeof(i64) > sizeof(usize) only on 32-bit; on 64-bit both are 8 bytes.
    if (m as u64) > PCRE2_SIZE_MAX as u64 {
        return TRUE;
    }
    *r = m as PCRE2_SIZE;
    FALSE
}
