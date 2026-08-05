// Included into dtoa.rs. Provides gethex and strtod__unused.
// (Temporary minimal versions; replaced with full translations after
// dtoa_r is verified byte-identical.)

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    _sp: *mut *const c_char,
    _rvp: *mut c_void,
    _rounding: c_int,
    _sign: c_int,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(_s00: *const c_char, _se: *mut *mut c_char) -> f64 {
    0.0
}
